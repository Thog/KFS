//! Time Service
//!
//! This service takes care of anything related with time.

#![feature(alloc, alloc_prelude, maybe_uninit, untagged_unions)]
#![no_std]

// rustc warnings
#![warn(unused)]
#![warn(missing_debug_implementations)]
#![allow(unused_unsafe)]
#![allow(unreachable_code)]
#![allow(dead_code)]
#![cfg_attr(test, allow(unused_imports))]

// rustdoc warnings
#![warn(missing_docs)] // hopefully this will soon become deny(missing_docs)
#![deny(intra_doc_link_resolution_failure)]

#[macro_use]
extern crate sunrise_libuser;

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate log;

mod timezone;

use alloc::prelude::v1::*;

use sunrise_libuser::syscalls;
use sunrise_libuser::ipc::server::{WaitableManager, PortHandler, IWaitable, SessionWrapper};
use sunrise_libuser::types::*;
use sunrise_libuser::error::Error;


use timezone::TimeZoneService;

capabilities!(CAPABILITIES = Capabilities {
    svcs: [
        sunrise_libuser::syscalls::nr::SleepThread,
        sunrise_libuser::syscalls::nr::ExitProcess,
        sunrise_libuser::syscalls::nr::CloseHandle,
        sunrise_libuser::syscalls::nr::WaitSynchronization,
        sunrise_libuser::syscalls::nr::OutputDebugString,

        sunrise_libuser::syscalls::nr::ReplyAndReceiveWithUserBuffer,
        sunrise_libuser::syscalls::nr::AcceptSession,
        sunrise_libuser::syscalls::nr::CreateSession,

        sunrise_libuser::syscalls::nr::ConnectToNamedPort,
        sunrise_libuser::syscalls::nr::SendSyncRequestWithUserBuffer,

        sunrise_libuser::syscalls::nr::SetHeapSize,

        sunrise_libuser::syscalls::nr::QueryMemory,

        sunrise_libuser::syscalls::nr::MapSharedMemory,
        sunrise_libuser::syscalls::nr::UnmapSharedMemory,

        sunrise_libuser::syscalls::nr::MapFramebuffer,
    ],
});

/// Entry point interface.
#[derive(Default, Debug)]
struct StaticService;

object! {
    impl StaticService {
        #[cmdid(3)]
        fn get_timezone_service(&mut self, manager: &WaitableManager,) -> Result<(Handle,), Error> {
            let timezone_instance = TimeZoneService::default();
            let (server, client) = syscalls::create_session(false, 0)?;
            let wrapper = SessionWrapper::new(server, timezone_instance);
            manager.add_waitable(Box::new(wrapper) as Box<dyn IWaitable>);
            Ok((client.into_handle(),))
        }
    }
}

fn main() {
    let man = WaitableManager::new();
    let user_handler = Box::new(PortHandler::<StaticService>::new("time:u\0").unwrap());
    let applet_handler = Box::new(PortHandler::<StaticService>::new("time:a\0").unwrap());
    let system_handler = Box::new(PortHandler::<StaticService>::new("time:s\0").unwrap());

    man.add_waitable(user_handler as Box<dyn IWaitable>);
    man.add_waitable(applet_handler as Box<dyn IWaitable>);
    man.add_waitable(system_handler as Box<dyn IWaitable>);

    man.run();
}