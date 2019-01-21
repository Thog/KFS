#![feature(alloc, const_let)]
#![no_std]

#![warn(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]

#[macro_use]
extern crate kfs_libuser;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;

use kfs_libuser::terminal::{Terminal, WindowSize};
use kfs_libuser::io::{self, Io};
use kfs_libuser::syscalls;
use kfs_libuser::lm::*;
use core::fmt::Write;

fn main() {
    info!("Hello, world!");

    let mut lm = LogService::raw_new().unwrap();
    let mut logger = lm.open_logger().unwrap();

    loop {
        syscalls::sleep_thread(1 * 1000 * 1_000_000);
        //logger.set_destination(3).unwrap();
        logger.log(b"Hello, world!!!!").unwrap();
        info!("Hello, world!");
    }

}

capabilities!(CAPABILITIES = Capabilities {
    svcs: [
        kfs_libuser::syscalls::nr::SleepThread,
        kfs_libuser::syscalls::nr::ExitProcess,
        kfs_libuser::syscalls::nr::CloseHandle,
        kfs_libuser::syscalls::nr::WaitSynchronization,
        kfs_libuser::syscalls::nr::OutputDebugString,

        kfs_libuser::syscalls::nr::ConnectToNamedPort,
        kfs_libuser::syscalls::nr::CreateInterruptEvent,
        kfs_libuser::syscalls::nr::SetHeapSize,
        kfs_libuser::syscalls::nr::SendSyncRequestWithUserBuffer,
        kfs_libuser::syscalls::nr::QueryMemory,
        kfs_libuser::syscalls::nr::CreateSharedMemory,
        kfs_libuser::syscalls::nr::MapSharedMemory,
        kfs_libuser::syscalls::nr::UnmapSharedMemory,
    ]
});