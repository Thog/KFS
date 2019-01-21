//! Log Manager service

use core::mem;
use crate::types::*;
use crate::sm;
use crate::error::{Error, SmError};
use crate::ipc::IPCBuffer;
use crate::ipc::IPCBufferType;
use crate::ipc::Message;

use core::marker::PhantomData;

/// Main log interface.
pub struct LogService(ClientSession);

impl LogService {
    /// Connects to the vi service.
    pub fn raw_new() -> Result<LogService, Error> {
        use crate::syscalls;

        loop {
            let svcname = unsafe {
                mem::transmute(*b"lm\0\0\0\0\0\0")
            };
            let _ = match sm::IUserInterface::raw_new()?.get_service(svcname) {
                Ok(s) => return Ok(LogService(s)),
                Err(Error::Sm(SmError::ServiceNotRegistered, ..)) => syscalls::sleep_thread(0),
                Err(err) => return Err(err)
            };
        }
    }

    pub fn open_logger(&mut self) -> Result<LogInterface, Error> {
        let mut buf = [0; 0x100];

        #[repr(C)] #[derive(Clone, Copy, Default)]
        struct InRaw {
            unknown: u64
        }
        let mut msg = Message::<_, [_; 0], [_; 1], [_; 0]>::new_request(None, 0);
        msg.set_send_pid();
        msg.push_raw(InRaw { unknown: 0 });
        msg.pack(&mut buf[..]);

        self.0.send_sync_request_with_user_buffer(&mut buf[..])?;
        let mut res : Message<(), [_; 0], [_; 0], [_; 1]> = Message::unpack(&buf[..]);
        res.error()?;
        Ok(LogInterface(ClientSession(res.pop_handle_move().unwrap())))
    }
}

pub struct LogInterface(ClientSession);

impl LogInterface {
    pub fn log(&mut self, line: &[u8; 0x10]) -> Result<(), Error> {
        let mut buf = [0; 0x100];

        #[repr(C)] #[derive(Clone, Copy, Default)]
        struct InRaw { }
        let mut msg = Message::<_, [IPCBuffer; 1], [u32; 0], [u32; 0]>::new_request(None, 1);

        let buffer_x = IPCBuffer::from_ref(line, IPCBufferType::X {counter: 0});
        info!("{:x}", buffer_x.addr);
        info!("{:x}", buffer_x.size >> 16);
        msg.buffers.push(buffer_x);
        msg.push_raw(InRaw {});
        msg.pack(&mut buf[..]);

        self.0.send_sync_request_with_user_buffer(&mut buf[..])?;
        let mut res : Message<(), [_; 1], [_; 0], [_; 0]> = Message::unpack(&buf[..]);
        res.error()?;

        Ok(())
    }

    pub fn set_destination(&mut self, destination: u32) -> Result<(), Error> {
        let mut buf = [0; 0x100];

        #[repr(C)] #[derive(Clone, Copy, Default)]
        struct InRaw {
            destination: u32
        }
        let mut msg = Message::<_, [_; 0], [_; 1], [_; 0]>::new_request(None, 1);
        msg.push_raw(InRaw { destination });
        msg.pack(&mut buf[..]);

        self.0.send_sync_request_with_user_buffer(&mut buf[..])?;
        let mut res : Message<(), [_; 0], [_; 0], [_; 0]> = Message::unpack(&buf[..]);
        res.error()?;

        Ok(())
    }
}