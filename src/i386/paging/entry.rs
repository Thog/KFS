///! # Page table entry

use super::PhysicalAddress;
use ::frame_alloc::Frame;

bitflags! {
    pub struct EntryFlags: u32 {
        const PRESENT =         1 << 0;
        const WRITABLE =        1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH =   1 << 3;
        const NO_CACHE =        1 << 4;
        const ACCESSED =        1 << 5;
        const DIRTY =           1 << 6;
        const HUGE_PAGE =       1 << 7;
        const GLOBAL =          1 << 8;
        const USER_DEFINED_1 =  1 << 9;
        const USER_DEFINED_2 =  1 << 10;
        const USER_DEFINED_3 =  1 << 11;
    }
}

const ENTRY_PHYS_ADDRESS_MASK: usize = 0xffff_f000;

/// An entry in a page table or page directory. An unused entry is 0
#[repr(transparent)]
pub struct Entry(u32);

impl Entry {
    /// Is the entry unused ?
    pub fn is_unused(&self) -> bool { self.0 == 0 }

    /// Clear the entry
    pub fn set_unused(&mut self) { self.0 = 0; }

    /// Get the current entry flags
    pub fn flags(&self) -> EntryFlags { EntryFlags::from_bits_truncate(self.0) }

    /// Get the associated frame, if available
    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(EntryFlags::PRESENT) {
            let frame_phys_addr = self.0 as usize & ENTRY_PHYS_ADDRESS_MASK;
            Some( Frame { physical_addr: frame_phys_addr })
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        let frame_phys_addr = frame.dangerous_as_physical_ptr() as *mut u8 as PhysicalAddress;
        assert_eq!(frame_phys_addr & !ENTRY_PHYS_ADDRESS_MASK, 0);
        self.0 = (frame_phys_addr as u32) | flags.bits();
    }
}