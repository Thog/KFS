//! IPC Sessions
//!
//! A Session represents an established connection. It implements a rendez-vous
//! style Remote Procedure Call interface. The ClientSession has a `send_request`
//! operation, which will wait for the counterpart ServerSession's `reply`. A
//! ServerSession can also `receive` the pending requests.
//!
//! Note that a single Session can only process a single request at a time - it
//! is an inherently sequential construct. If multiple threads attempt receiving
//! on the same handle, they will have to wait for the current request to be
//! replied to before being able to receive the next request in line.
//!
//! ```rust
//! use kernel::ipc::session;
//! let (server, client) = session::new();
//! ```
//!
//! The requests are encoded in a byte buffer under a specific format. For
//! documentation on the format, [switchbrew] is your friend.
//!
//! [switchbrew]: https://switchbrew.org/w/index.php?title=IPC_Marshalling

use crate::scheduler;
use alloc::vec::Vec;
use alloc::sync::{Arc, Weak};
use crate::sync::SpinLock;
use crate::error::UserspaceError;
use crate::event::Waitable;
use crate::process::ThreadStruct;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::slice;
use byteorder::{LE, ByteOrder};
use crate::paging::{MappingFlags, mapping::MappingType, process_memory::ProcessMemory};
use crate::mem::{UserSpacePtr, UserSpacePtrMut, VirtualAddress};
use bit_field::BitField;

#[derive(Debug)]
struct InternalSession {
    active_request: Option<Request>,
    incoming_requests: Vec<Request>,
}

#[derive(Debug)]
struct Session {
    internal: SpinLock<InternalSession>,
    accepters: SpinLock<Vec<Weak<ThreadStruct>>>,
    servercount: AtomicUsize,
}

/// The client side of a Session.
#[derive(Debug, Clone)]
pub struct ClientSession(Arc<Session>);

/// The server side of a Session.
#[derive(Debug)]
pub struct ServerSession(Arc<Session>);

impl Clone for ServerSession {
    fn clone(&self) -> Self {
        assert!(self.0.servercount.fetch_add(1, Ordering::SeqCst) != usize::max_value(), "Overflow when incrementing servercount");
        ServerSession(self.0.clone())
    }
}

impl Drop for ServerSession {
    fn drop(&mut self) {
        let count = self.0.servercount.fetch_sub(1, Ordering::SeqCst);
        assert!(count != 0, "Overflow when decrementing servercount");
        if count == 1 {
            info!("Last ServerSession dropped");
            // We're dead jim.
            let mut internal = self.0.internal.lock();

            if let Some(request) = internal.active_request.take() {
                *request.answered.lock() = Some(Err(UserspaceError::PortRemoteDead));
                scheduler::add_to_schedule_queue(request.sender.clone());
            }

            for request in internal.incoming_requests.drain(..) {
                *request.answered.lock() = Some(Err(UserspaceError::PortRemoteDead));
                scheduler::add_to_schedule_queue(request.sender.clone());
            }
        }
    }
}

bitfield! {
    /// Represenens the header of an HIPC command.
    ///
    /// The kernel uses this header to figure out how to send the IPC message.
    pub struct MsgPackedHdr(u64);
    impl Debug;
    u16, ty, _: 15, 0;
    u8, num_x_descriptors, set_num_x_descriptors: 19, 16;
    u8, num_a_descriptors, set_num_a_descriptors: 23, 20;
    u8, num_b_descriptors, set_num_b_descriptors: 27, 24;
    u8, num_w_descriptors, set_num_w_descriptors: 31, 28;
    u16, raw_section_size, set_raw_section_size: 41, 32;
    u8, c_descriptor_flags, set_c_descriptor_flags: 45, 42;
    u32, c_list_offset, set_c_list_offset: 52, 62;
    enable_handle_descriptor, set_enable_handle_descriptor: 63;
}

bitfield! {
    /// Part of an HIPC command. Sent only when
    /// `MsgPackedHdr::enable_handle_descriptor` is true.
    pub struct HandleDescriptorHeader(u32);
    impl Debug;
    send_pid, set_send_pid: 0;
    u8, num_copy_handles, set_num_copy_handles: 4, 1;
    u8, num_move_handles, set_num_move_handles: 8, 5;
}

impl MsgPackedHdr {
    pub fn message_size(&self, descriptor: &HandleDescriptorHeader) -> usize {
        let padding_size = if self.enable_handle_descriptor() {
            3
        } else {
            2
        };

        let pid_size = if descriptor.send_pid() {
            2
        } else {
            0
        };

        let handle_size_word: u16 = (self.num_x_descriptors() * 2) as u16
            + ((self.num_a_descriptors()
            + self.num_b_descriptors() + self.num_w_descriptors()) * 3) as u16;

        ((handle_size_word + self.raw_section_size() + padding_size
            + (descriptor.num_copy_handles() as u16)
            + (descriptor.num_move_handles() as u16) + pid_size) * 4) as usize
    }
}

impl Session {
    fn new() -> (ServerSession, ClientSession) {
        let sess = Arc::new(Session {
            internal: SpinLock::new(InternalSession {
                incoming_requests: Vec::new(),
                active_request: None
            }),
            accepters: SpinLock::new(Vec::new()),
            servercount: AtomicUsize::new(0)
        });

        (Session::server(sess.clone()), Session::client(sess))
    }

    /// Returns a ClientPort from this Port.
    fn client(this: Arc<Self>) -> ClientSession {
        ClientSession(this)
    }

    /// Returns a ServerSession from this Port.
    fn server(this: Arc<Self>) -> ServerSession {
        this.servercount.fetch_add(1, Ordering::SeqCst);
        ServerSession(this)
    }
}

/// Create a new Session pair. Those sessions are linked to each-other: The
/// server will receive requests sent through the client.
pub fn new() -> (ServerSession, ClientSession) {
    Session::new()
}

impl Waitable for ServerSession {
    fn is_signaled(&self) -> bool {
        let mut internal = self.0.internal.lock();
        if internal.active_request.is_none() {
            if let Some(s) = internal.incoming_requests.pop() {
                internal.active_request = Some(s);
                true
            } else {
                false
            }
        } else {
            true
        }
    }

    fn register(&self) {
        let mut accepters = self.0.accepters.lock();
        let curproc = scheduler::get_current_thread();

        if !accepters.iter().filter_map(|v| v.upgrade()).any(|v| Arc::ptr_eq(&curproc, &v)) {
            accepters.push(Arc::downgrade(&curproc));
        }
    }
}

#[derive(Debug)]
struct Request {
    sender_buf: VirtualAddress,
    sender_bufsize: usize,
    sender: Arc<ThreadStruct>,
    answered: Arc<SpinLock<Option<Result<(), UserspaceError>>>>,
}

// TODO finish buf_map
#[allow(unused)]
fn buf_map(from_buf: &[u8], to_buf: &mut [u8], curoff: &mut usize, from_mem: &mut ProcessMemory, to_mem: &mut ProcessMemory, flags: MappingFlags) -> Result<(), UserspaceError> {
    let lowersize = LE::read_u32(&from_buf[*curoff..*curoff + 4]);
    let loweraddr = LE::read_u32(&from_buf[*curoff + 4..*curoff + 8]);
    let rest = LE::read_u32(&from_buf[*curoff + 8..*curoff + 12]);

    let bufflags = rest.get_bits(0..2);

    let addr = *(loweraddr as u64)
        .set_bits(32..36, rest.get_bits(28..32) as u64)
        .set_bits(36..39, rest.get_bits(2..5) as u64);

    let size = *(lowersize as u64)
        .set_bits(32..36, rest.get_bits(24..28) as u64);

    // 64-bit address on a 32-bit kernel!
    if (usize::max_value() as u64) < addr {
        return Err(UserspaceError::InvalidAddress);
    }

    // 64-bit size on a 32-bit kernel!
    if (usize::max_value() as u64) < size {
        return Err(UserspaceError::InvalidSize);
    }

    // 64-bit address on a 32-bit kernel
    if (usize::max_value() as u64) < addr.saturating_add(size) {
        return Err(UserspaceError::InvalidSize);
    }

    let addr = addr as usize;
    let size = size as usize;

    // Map the descriptor in the other process.
    let to_addr : VirtualAddress = unimplemented!("Needs the equivalent to find_available_virtual_space");
    /*let to_addr = to_mem.find_available_virtual_space_runtime(size / paging::PAGE_SIZE)
        .ok_or(UserspaceError::MemoryFull)?;*/

    let mapping = from_mem.unmap(VirtualAddress(addr), size)?;
    let flags = mapping.flags();
    let phys = match mapping.mtype() {
        MappingType::Available | MappingType::Guarded | MappingType::SystemReserved =>
            // todo remap it D:
            return Err(UserspaceError::InvalidAddress),
        MappingType::Regular(vec) /*| MappingType::Stack(vec) */ => Arc::new(vec),
        MappingType::Shared(arc) => arc,
    };

    from_mem.map_shared_mapping(phys, VirtualAddress(addr), flags)?;

    let loweraddr = to_addr.addr() as u32;
    let rest = *0u32
        .set_bits(0..2, bufflags)
        .set_bits(2..5, (to_addr.addr() as u64).get_bits(36..39) as u32)
        .set_bits(24..28, (size as u64).get_bits(32..36) as u32)
        .set_bits(28..32, (to_addr.addr() as u64).get_bits(32..36) as u32);

    LE::write_u32(&mut to_buf[*curoff + 0..*curoff + 4], lowersize);
    LE::write_u32(&mut to_buf[*curoff + 4..*curoff + 8], loweraddr);
    LE::write_u32(&mut to_buf[*curoff + 8..*curoff + 12], rest);

    *curoff += 12;
    Ok(())
}

impl ClientSession {
    /// Send an IPC request through the client pipe. Takes a userspace buffer
    /// containing the packed IPC request. When returning, the buffer will
    /// contain the IPC answer (unless an error occured).
    ///
    /// This function is blocking - it will wait until the server receives and
    /// replies to the request before returning.
    ///
    /// Note that the buffer needs to live until send_request returns, which may
    /// take an arbitrary long time. We do not eagerly read the buffer - it will
    /// be read from when the server asks to receive a request.
    pub fn send_request(&self, buf: UserSpacePtrMut<[u8]>) -> Result<(), UserspaceError> {
        // TODO: Unmapping is out of the question. Ideally, I should just affect the bookkeeping
        // in order to remap it in the kernel.
        let answered = Arc::new(SpinLock::new(None));

        {
            // Be thread-safe: First we lock the internal mutex. Then check whether there's
            // a server left or not, in which case fail-fast. Otherwise, add the incoming
            // request.
            let mut internal = self.0.internal.lock();

            if self.0.servercount.load(Ordering::SeqCst) == 0 {
                return Err(UserspaceError::PortRemoteDead);
            }

            internal.incoming_requests.push(Request {
                sender_buf: VirtualAddress(buf.as_ptr() as usize),
                sender_bufsize: buf.len(),
                answered: answered.clone(),
                sender: scheduler::get_current_thread(),
            })
        }

        let mut guard = answered.lock();

        while let None = *guard {
            while let Some(item) = self.0.accepters.lock().pop() {
                if let Some(process) = item.upgrade() {
                    scheduler::add_to_schedule_queue(process);
                    break;
                }
            }

            guard = scheduler::unschedule(&*answered, guard)?;
        }

        (*guard).unwrap()
    }
}

impl ServerSession {
    /// Receive an IPC request through the server pipe. Takes a userspace buffer
    /// containing an empty IPC message. The request may optionally contain a
    /// C descriptor in order to receive X descriptors. The buffer will be filled
    /// with an IPC request.
    ///
    /// This function does **not** wait. It assumes an active_request has already
    /// been set by a prior call to wait.
    pub fn receive(&self, mut buf: UserSpacePtrMut<[u8]>) -> Result<(), UserspaceError> {
        // Read active session
        let internal = self.0.internal.lock();

        // TODO: In case of a race, we might want to check that receive is only called once.
        // Can races even happen ?
        let active = internal.active_request.as_ref().unwrap();

        let sender = active.sender.process.clone();
        let memlock = sender.pmemory.lock();

        let mapping = memlock.mirror_mapping(active.sender_buf, active.sender_bufsize)?;
        let sender_buf = unsafe {
            slice::from_raw_parts_mut(mapping.addr().addr() as *mut u8, mapping.len())
        };

        pass_message(sender_buf, active.sender.clone(), &mut *buf, scheduler::get_current_thread())?;

        Ok(())
    }

    /// Replies to the currently active IPC request on the server pipe. Takes a
    /// userspace buffer containing the IPC reply. The kernel will copy the reply
    /// to the sender's IPC buffer, before waking the sender so it may return to
    /// userspace.
    ///
    /// # Panics
    ///
    /// Panics if there is no currently active request on the pipe.
    // TODO: Don't panic in Session::reply if active_request is not set.
    // BODY: Session::reply currently asserts that an active session is set. This
    // BODY: assertion can be trivially triggered by userspace, by calling
    // BODY: the reply_and_receive syscall with reply_target set to a Session
    // BODY: that hasn't received any request.
    pub fn reply(&self, buf: UserSpacePtr<[u8]>) -> Result<(), UserspaceError> {
        // TODO: This probably has an errcode.
        assert!(self.0.internal.lock().active_request.is_some(), "Called reply without an active session");

        let active = self.0.internal.lock().active_request.take().unwrap();

        let sender = active.sender.process.clone();

        let memlock = sender.pmemory.lock();

        let mapping = memlock.mirror_mapping(active.sender_buf, active.sender_bufsize)?;
        let sender_buf = unsafe {
            slice::from_raw_parts_mut(mapping.addr().addr() as *mut u8, mapping.len())
        };

        pass_message(&*buf, scheduler::get_current_thread(), sender_buf, active.sender.clone())?;

        *active.answered.lock() = Some(Ok(()));

        scheduler::add_to_schedule_queue(active.sender.clone());

        Ok(())
    }
}

// TODO: Kernel IPC: Implement X and C descriptor support in pass_message.
// BODY: X and C descriptors are complicated and support for them is put off for
// BODY: the time being. We'll likely want them when implementing FS though.
#[allow(unused)]
fn pass_message(from_buf: &[u8], from_proc: Arc<ThreadStruct>, to_buf: &mut [u8], to_proc: Arc<ThreadStruct>) -> Result<(), UserspaceError> {
    // TODO: pass_message deadlocks when sending message to the same process.
    // BODY: If from_proc and to_proc are the same process, pass_message will
    // BODY: deadlock trying to acquire the locks to the handle table or the
    // BODY: page tables.

    let mut curoff = 0;
    let from_hdr = MsgPackedHdr(LE::read_u64(&from_buf[curoff..curoff + 8]));
    let to_hdr = MsgPackedHdr(LE::read_u64(&to_buf[curoff..curoff + 8]));

    let to_descriptor = if from_hdr.enable_handle_descriptor() {
        let descriptor = HandleDescriptorHeader(LE::read_u32(&to_buf[curoff + 8..curoff + 12]));
        descriptor
    } else {
        HandleDescriptorHeader(0)
    };

    LE::write_u64(&mut to_buf[curoff..curoff + 8], from_hdr.0);

    curoff += 8;

    let from_descriptor = if from_hdr.enable_handle_descriptor() {
        let descriptor = HandleDescriptorHeader(LE::read_u32(&from_buf[curoff..curoff + 4]));
        LE::write_u32(&mut to_buf[curoff..curoff + 4], descriptor.0);
        curoff += 4;
        descriptor
    } else {
        HandleDescriptorHeader(0)
    };

    if from_descriptor.send_pid() {
        // TODO: Atmosphere patch for fs_mitm.
        LE::write_u64(&mut to_buf[curoff..curoff + 8], from_proc.process.pid as u64);
        curoff += 8;
    }

    if from_descriptor.num_copy_handles() != 0 || from_descriptor.num_move_handles() != 0 {
        let mut from_handle_table = from_proc.process.phandles.lock();
        let mut to_handle_table = to_proc.process.phandles.lock();

        for i in 0..from_descriptor.num_copy_handles() {
            let handle = LE::read_u32(&from_buf[curoff..curoff + 4]);
            let handle = from_handle_table.get_handle(handle)?;
            let handle = to_handle_table.add_handle(handle);
            LE::write_u32(&mut to_buf[curoff..curoff + 4], handle);
            curoff += 4;
        }
        for i in 0..from_descriptor.num_move_handles() {
            let handle = LE::read_u32(&from_buf[curoff..curoff + 4]);
            let handle = from_handle_table.delete_handle(handle)?;
            let handle = to_handle_table.add_handle(handle);
            LE::write_u32(&mut to_buf[curoff..curoff + 4], handle);
            curoff += 4;
        }
    }

    let c_list_offset: usize = if to_hdr.c_list_offset() == 0 {
        to_hdr.message_size(&to_descriptor)
    } else {
        (to_hdr.c_list_offset() * 4) as usize
    };

    for i in 0..from_hdr.num_x_descriptors() {
        let x_dword = LE::read_u64(&from_buf[curoff..curoff + 8]);
        let counter: u64 = x_dword & 0xFF;
        let buffer_size = x_dword >> 16;
        let mut buffer_address = (((x_dword >> 2) & (0x70 | (x_dword >> 12) & 0xF)) << 32) | x_dword >> 32;

        if buffer_size != 0 {

            let c_flags = to_hdr.c_descriptor_flags();

            // TODO: find receiver address
            let receiver_address: Result<u64, UserspaceError> = if c_flags == 0 {
                // TODO: check this error
                return Err(UserspaceError::SlabheapFull);
            } else if c_flags == 1 || c_flags == 2 {

                let mut receiver_list_start_address;
                let mut receiver_list_end_address;

                if c_flags == 1 {
                    let message_address = to_buf.as_ptr() as u64;

                    receiver_list_start_address = message_address + to_hdr.message_size(&to_descriptor) as u64;
                    receiver_list_end_address = message_address + to_buf.len() as u64;

                } else {
                    let c_dword = LE::read_u64(&to_buf[c_list_offset..c_list_offset + 8]);

                    let size = c_dword >> 48;

                    if size == 0 {
                        return Err(UserspaceError::SlabheapFull);
                    }

                    receiver_list_start_address = c_dword & 0x7fffffffff;
                    receiver_list_end_address = receiver_list_start_address + size;
                }

                unimplemented!("1 or 2 flags")

            } else {
                let num_c_descriptors: u64 = (c_flags - 2) as u64;

                if counter >= num_c_descriptors {
                    return Err(UserspaceError::SlabheapFull);
                }

                let c_list_entry_offset = c_list_offset + (counter * 8) as usize;

                let c_dword = LE::read_u64(&to_buf[c_list_entry_offset..c_list_entry_offset + 8]);

                let address = c_dword & 0x7fffffffff;
                let size = c_dword >> 48;

                if address == 0 || size == 0 || size < buffer_size {
                    return Err(UserspaceError::SlabheapFull);
                }

                Ok(address)
            };

            // TODO: buf_map with data from before

            buffer_address = receiver_address?;

            unimplemented!("Implement X descriptors entirely");

        } else {
            buffer_address = 0;
        }

        let num = *(counter.get_bits(0..4) as u32)
            .set_bits(6..9, buffer_address.get_bits(36..39) as u32)
            .set_bits(12..16, buffer_address.get_bits(32..36) as u32)
            .set_bits(16..32, buffer_size as u32);

        LE::write_u32(&mut to_buf[curoff..curoff + 4], num);
        LE::write_u32(&mut to_buf[curoff + 4..curoff + 8], (buffer_address & 0xFFFFFFFF) as u32);

        curoff += 8;

    }

    if from_hdr.num_a_descriptors() != 0 || from_hdr.num_b_descriptors() != 0 {
        let mut from_mem = from_proc.process.pmemory.lock();
        let mut to_mem = to_proc.process.pmemory.lock();

        for i in 0..from_hdr.num_a_descriptors() {
            buf_map(from_buf, to_buf, &mut curoff, &mut *from_mem, &mut *to_mem, MappingFlags::empty())?;
        }

        for i in 0..from_hdr.num_b_descriptors() {
            buf_map(from_buf, to_buf, &mut curoff, &mut *from_mem, &mut *to_mem, MappingFlags::WRITABLE)?;
        }

        for i in 0..from_hdr.num_w_descriptors() {
            buf_map(from_buf, to_buf, &mut curoff, &mut *from_mem, &mut *to_mem, MappingFlags::WRITABLE)?;
        }
    }

    (&mut to_buf[curoff..curoff + (from_hdr.raw_section_size() as usize) * 4])
        .copy_from_slice(&from_buf[curoff..curoff + (from_hdr.raw_section_size() as usize) * 4]);

    if from_hdr.c_descriptor_flags() == 1 {
        unimplemented!("Inline C Descriptor");
    } else if from_hdr.c_descriptor_flags() == 2 {
        unimplemented!("Single C Descriptor");
    } else if from_hdr.c_descriptor_flags() != 0 {
        unimplemented!("Multi C Descriptor");
        for i in 0..from_hdr.c_descriptor_flags() - 2 {
        }
    }

    Ok(())
}
