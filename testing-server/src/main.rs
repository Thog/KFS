#![feature(alloc, const_let)]
#![no_std]

#![warn(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]

#[macro_use]
extern crate kfs_libuser as libuser;
#[macro_use]
extern crate alloc;

extern crate kfs_libkern as libkern;

use alloc::prelude::*;
use alloc::sync::{Arc, Weak};

use libuser::terminal::{Terminal, WindowSize};
use libuser::io::{self, Io};
use libuser::syscalls;
use core::fmt::Write;

#[macro_use]
extern crate log;
extern crate kfs_libuser;

use libuser::error::Error;
use libuser::ipc::server::{WaitableManager, PortHandler, IWaitable, SessionWrapper};
use libuser::types::*;
use libuser::ipc::Message;

#[derive(Default)]
struct LogService;

#[derive(Default)]
struct LoggerInterface;

object! {
    impl LogService {
        #[cmdid(0)]
        fn open_logger(&mut self, manager: &WaitableManager, val: u32, pid: Pid, ) ->  Result<(Handle,), Error> {
            let logger_interface = LoggerInterface {};

            let (server, client) = syscalls::create_session(false, 0)?;
            let wrapper = SessionWrapper::new(server, logger_interface);
            manager.add_waitable(Box::new(wrapper) as Box<dyn IWaitable>);
            Ok((client.into_handle(),))
        }
    }
}

/*object! {
    impl LoggerInterface {
        #[cmdid(0)]
        fn log(&mut self, data: /*InPointer<[u8; 0x1000]>,*/ ) -> Result<((),), Error> {
            Ok(((),))
        }

        #[cmdid(1)]
        fn set_destination(&mut self, destination: u32,) -> Result<((),), Error> {
            info!("Destination: {}", destination);
            Ok(((),))
        }
    }
}*/

impl libuser::ipc::server::Object for LoggerInterface {
    fn dispatch(
        &mut self,
        manager: &WaitableManager,
        cmdid: u32,
        buf: &mut [u8]
    ) -> Result<(), Error> {
        Ok(())
    }
}


fn main() {
    let man = WaitableManager::new();
    let handler = Box::new(PortHandler::<LogService>::new("lm\0").unwrap());
    man.add_waitable(handler as Box<dyn IWaitable>);

    man.run();
}

capabilities!(CAPABILITIES = Capabilities {
    svcs: [
        kfs_libuser::syscalls::nr::SleepThread,
        kfs_libuser::syscalls::nr::ExitProcess,
        kfs_libuser::syscalls::nr::CloseHandle,
        kfs_libuser::syscalls::nr::WaitSynchronization,
        kfs_libuser::syscalls::nr::OutputDebugString,

        kfs_libuser::syscalls::nr::ReplyAndReceiveWithUserBuffer,
        kfs_libuser::syscalls::nr::AcceptSession,
        kfs_libuser::syscalls::nr::CreateSession,

        kfs_libuser::syscalls::nr::ConnectToNamedPort,
        kfs_libuser::syscalls::nr::SendSyncRequestWithUserBuffer,

        kfs_libuser::syscalls::nr::SetHeapSize,

        kfs_libuser::syscalls::nr::QueryMemory,

        kfs_libuser::syscalls::nr::MapSharedMemory,
        kfs_libuser::syscalls::nr::UnmapSharedMemory,

        kfs_libuser::syscalls::nr::MapFramebuffer,
    ],
});
