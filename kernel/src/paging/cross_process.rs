//! Cross Process Mapping
//!
//! Provides mechanics for temporarily mirroring a Userland mapping in KernelLand.
//!
//! When kernel has to access memory in UserLand (for example a user has provided a buffer as an
//! argument of a syscall), it can do it in two ways:
//!
//! * Either the buffer is mapped in the same page tables that the kernel is currently using,
//!   in this case it accesses it directly, through a UserspacePtr.
//! * Either the buffer is only mapped in the page tables of another process, in which case
//!   it has to temporarily map it to KernelLand, make the modifications, and unmap it from KernelLand.
//!
//! This module covers the second case.
//!
//! The remapping is represented by a [`CrossProcessMapping`] structure. It is created from a reference
//! to the mapping being mirrored, and the KernelLand address where it will be remapped.
//! When this struct is dropped, the frames are unmap'd from KernelLand.
//!
//! A `CrossProcessMapping` is temporary by nature, and has the same lifetime as the reference to the
//! mapping it remaps, which is chained to the lifetime of the lock protecting [`ProcessMemory`].
//!
//! Because of this, a `CrossProcessMapping` cannot outlive the `ProcessMemory` lock guard held by the
//! function that created it. This ensures that:
//!
//! * All `CrossProcessMapping`s will be unmapped before returning to UserSpace.
//! * Another thread cannot make any modification to a `ProcessMemory` while a `CrossProcessMapping`
//!   exists for this `ProcessMemory`.
//! * The UserLand side of the mapping cannot be deleted while it is still being mirrored,
//!   as this would require a mutable borrow of the `ProcessMemory` lock,
//!   and it is currently (constly) borrowed by the `CrossProcessMapping`.
//!
//! [`CrossProcessMapping`]: self::CrossProcessMapping<'a>
//! [`ProcessMemory`]: crate::paging::process_memory::ProcessMemory

use crate::mem::VirtualAddress;
use super::{PAGE_SIZE, MappingAccessRights};
use super::mapping::{Mapping, MappingType};
use super::kernel_memory::get_kernel_memory;
use super::error::MmError;
use crate::utils::check_nonzero_length;
use failure::Backtrace;
use crate::error::KernelError;

/// A struct representing a UserLand mapping temporarily mirrored in KernelSpace.
#[derive(Debug)]
pub struct CrossProcessMapping<'a> {
    /// The KernelLand address it was remapped to. Has the desired offset.
    kernel_address: VirtualAddress,
    /// Stores the desired length.
    len: usize,
    /// The mapping we remapped from.
    ///
    /// Note that a `CrossProcessMapping` has the same lifetime as the mapping it remaps.
    mapping: &'a Mapping,
}

#[allow(clippy::len_without_is_empty)] // len *cannot* be zero
impl<'a> CrossProcessMapping<'a> {
    /// Creates a `CrossProcessMapping`.
    ///
    /// Temporarily remaps a subsection of the mapping in KernelLand.
    ///
    /// # Errors
    ///
    /// * `InvalidSize`:
    ///     * `offset + len > mapping.length()`.
    ///     * `offset + len - 1` would overflow.
    ///     * `len` is 0.
    ///
    /// * Error if the mapping is Available/Guarded/SystemReserved, as there would be
    /// no point to remap it, and dereferencing the pointer would cause the kernel to page-fault.
    pub fn mirror_mapping(mapping: &Mapping, offset: usize, len: usize) -> Result<CrossProcessMapping<'_>, KernelError> {
        check_nonzero_length(len)?;
        offset.checked_add(len - 1)
            .ok_or_else(|| KernelError::InvalidSize { size: usize::max_value(), backtrace: Backtrace::new() })
            .and_then(|sum| if sum >= mapping.length() {
                Err(KernelError::InvalidSize { size: sum, backtrace: Backtrace::new() })
            } else {
                Ok(())
            })?;
        let regions = match mapping.mtype_ref() {
            MappingType::Guarded | MappingType::Available | MappingType::SystemReserved
                => return Err(KernelError::MmError(MmError::InvalidMapping { backtrace: Backtrace::new() })),
            MappingType::Regular(ref f) => f,
            //MappingType::Stack(ref f) => f,
            MappingType::Shared(ref f) => f
        };
        let map_start = (mapping.address() + offset).floor();
        let map_end = (mapping.address() + offset + len).ceil();
        // iterator[map_start..map_end]
        let frames_iterator = regions.iter().flatten()
            .skip((map_start - mapping.address()) / PAGE_SIZE)
            .take((map_end - map_start) / PAGE_SIZE);
        let kernel_map_start = unsafe {
            // safe, the frames won't be dropped, they still are tracked by the userspace mapping.
            get_kernel_memory().map_frame_iterator(frames_iterator, MappingAccessRights::k_rw())
        };
        Ok(CrossProcessMapping {
            kernel_address: kernel_map_start + (offset % PAGE_SIZE),
            mapping,
            len,
        })
    }

    /// The address of the region asked to be remapped.
    pub fn addr(&self) -> VirtualAddress {
        self.kernel_address
    }

    /// The length of the region asked to be remapped.
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a> Drop for CrossProcessMapping<'a> {
    /// Unmaps itself from KernelLand when dropped.
    fn drop(&mut self) {
        let map_start = self.kernel_address.floor();
        let map_len = (self.kernel_address + self.len).ceil() - map_start;
        // don't dealloc the frames, they are still tracked by the mapping
        get_kernel_memory().unmap_no_dealloc(map_start, map_len)
    }
}
