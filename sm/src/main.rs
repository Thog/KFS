//! Service Manager
//!
//! Services are system processes running in the background which wait for
//! incoming requests. When a process wants to communicate with a service, it
//! first needs to get a handle to the named service, and then it can communicate
//! with the service via inter-process communication (each service has a name up
//! to 7 characters followed by a \0).
//!
//! Handles for services are retrieved from the service manager port, "sm:", and
//! are released via svcCloseHandle or when a process is terminated or crashes.
//!
//! Manager service "sm:m" allows the Process Manager to tell sm: about the
//! permissions of each process. By default, SM assumes a process has no
//! permissions, and as such cannot access any service. "sm:m" RegisterProcess
//! calls allows PM to tell the Service Manager about which services a certain
//! process is allowed to access or host.
//!
//! A Service is very similar to a kernel-managed Named Port: You can connect to
//! it, and it returns a ClientSession. The difference is that a Service handled
//! by "sm:" has an additional permission check done to ensure it isn't accessed
//! by an unprivileged process.
//! Service Manager

#![feature(async_await)]
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
extern crate sunrise_libuser as libuser;
extern crate alloc;


#[macro_use]
extern crate lazy_static;

use log::*;
use alloc::boxed::Box;
use crate::libuser::syscalls;
use crate::libuser::futures::{WaitableManager, WorkQueue};
use crate::libuser::ipc::server::managed_port_handler;
use crate::libuser::types::*;
use crate::libuser::error::Error;
use crate::libuser::error::SmError;
use crate::libuser::sm::IUserInterfaceAsync;
use crate::libuser::loop_future::{Loop, loop_fn};
use hashbrown::hash_map::{HashMap, Entry};
use spin::Mutex;
use futures::future::{FutureExt, FutureObj};

/// `sm:` service interface.
/// The main interface to the Service Manager. Clients can use it to connect to
/// or register new services (assuming they have the appropriate capabilities).
///
/// Make sure to call the `IUserInterface::initialize` method before using it.
#[derive(Debug, Default)]
struct UserInterface;

lazy_static! {
    /// Global mapping of Service Name -> ClientPort.
    static ref SERVICES: Mutex<HashMap<u64, ClientPort>> = Mutex::new(HashMap::new());
    // TODO: Implement a futures-based condvar instead of using event for in-process eventing.
    // BODY: A futures-based condvar can easily be implemented entirely in userspace, without
    // BODY: the need for any kernel help. It would have a lot less overhead than using a kernel Event.
    static ref SERVICES_EVENT: (WritableEvent, ReadableEvent) = {
        crate::libuser::syscalls::create_event().unwrap()
    };
}

/// Get the length of a service encoded as an u64.
#[allow(clippy::verbose_bit_mask)] // More readable this way...
fn get_service_length(servicename: u64) -> usize{
    for i in 0..8 {
        if (servicename >> (8*i)) & 0xFF == 0 {
            return i;
        }
    }
    8
}

/// Casts an &u64 into an &str.
///
/// # Panics
///
/// Panics if the bytes of the u64 don't match valid UTF-8.
fn get_service_str(servicename: &u64) -> &str {
    // TODO: Don't fail, return an error (invalid servicename or something).
    // TODO: Maybe I should use &[u8] instead?
    let len = get_service_length(*servicename);
    unsafe {
        core::str::from_utf8(core::slice::from_raw_parts(servicename as *const u64 as *const u8, len)).unwrap()
    }
}

impl IUserInterfaceAsync for UserInterface {
    /// Initialize the UserInterface, acquiring the Pid of the remote
    /// process, which will then be used to validate the permissions of each
    /// calls.
    fn initialize(&mut self, _manager: WorkQueue<'static>, _pid: Pid) -> FutureObj<'_, Result<(), Error>> {
        FutureObj::new(Box::new(futures::future::ok(())))
    }

    /// Get a ClientSession to this service.
    //
    // Implementation note: There is a possibility of deadlocking in `port.connect()` here!
    //
    // Here's what can happen: Someone calls `register_service()`, but then never accepts on it.
    // The call to `connect()` will block waiting for an accepter, and freeze `sm:` forever.
    //
    // It gets worse when you consider the old way of registering: We would get a handle to `sm:`,
    // register our handle, close the handle to `sm:`, and finally accept. The problem is, closing
    // the handle requires `sm:` to handle it, but by the time `sm:` gets there, it might already be
    // trying to connect to the port!
    //
    // For this reason, it is recommended for processes to use a global `sm:` handle.
    fn get_service<'a>(&mut self, work_queue: WorkQueue<'a>, servicename: u64) -> FutureObj<'a, Result<ClientSession, Error>> {
        FutureObj::new(Box::new(loop_fn(work_queue, move |work_queue| {
            info!("Trying to acquire {:x}", servicename);
            if let Some(port) = SERVICES.lock().get(&servicename) {
                info!("Success!");
                // Synchronous connect. This can block.
                let client = port.connect();
                futures::future::ready(Loop::Break(client)).left_future()
            } else {
                info!("Service not currently registered. Sleeping.");
                SERVICES_EVENT.1.wait_async(work_queue.clone())
                    .map(|_| {
                        info!("Waking up, clearing the service event");
                        SERVICES_EVENT.1.clear().unwrap();
                        Loop::Continue(work_queue)
                    }).right_future()
            }
        })))
    }
    /// Register a new service, returning a ServerPort to the newly
    /// registered service.
    fn register_service(&mut self, _work_queue: WorkQueue<'static>, servicename: u64, is_light: bool, max_handles: u32) -> FutureObj<'_, Result<ServerPort, Error>> {
        let (clientport, serverport) = match syscalls::create_port(max_handles, is_light, get_service_str(&servicename)) {
            Ok(v) => v,
            Err(err) => return FutureObj::new(Box::new(futures::future::err(err.into())))
        };
        match SERVICES.lock().entry(servicename) {
            Entry::Occupied(_) => return FutureObj::new(Box::new(futures::future::err(SmError::ServiceAlreadyRegistered.into()))),
            Entry::Vacant(vacant) => {
                vacant.insert(clientport);

                // Wake up potential get_service.
                SERVICES_EVENT.0.signal().unwrap();

                FutureObj::new(Box::new(futures::future::ok(serverport)))
            }
        }
    }

    /// Unregister a service.
    fn unregister_service<'a>(&mut self, _work_queue: WorkQueue<'static>, servicename: u64) -> FutureObj<'_, Result<(), Error>> {
        match SERVICES.lock().remove(&servicename) {
            Some(_) => FutureObj::new(Box::new(futures::future::ok(()))),
            None => FutureObj::new(Box::new(futures::future::err(SmError::ServiceNotRegistered.into())))
        }
    }
}

fn main() {
    let mut man = WaitableManager::new();
    let handler = managed_port_handler(man.work_queue(), "sm:\0", UserInterface::dispatch).unwrap();

    man.work_queue().spawn(FutureObj::new(Box::new(handler)));

    man.run();
}

capabilities!(CAPABILITIES = Capabilities {
    svcs: [
        sunrise_libuser::syscalls::nr::SleepThread,
        sunrise_libuser::syscalls::nr::ExitProcess,
        sunrise_libuser::syscalls::nr::CloseHandle,
        sunrise_libuser::syscalls::nr::WaitSynchronization,
        sunrise_libuser::syscalls::nr::OutputDebugString,
        sunrise_libuser::syscalls::nr::SetThreadArea,

        sunrise_libuser::syscalls::nr::SetHeapSize,
        sunrise_libuser::syscalls::nr::ManageNamedPort,
        sunrise_libuser::syscalls::nr::AcceptSession,
        sunrise_libuser::syscalls::nr::ReplyAndReceiveWithUserBuffer,
        sunrise_libuser::syscalls::nr::CreatePort,
        sunrise_libuser::syscalls::nr::ConnectToPort,
        sunrise_libuser::syscalls::nr::CreateEvent,
        sunrise_libuser::syscalls::nr::SignalEvent,
        sunrise_libuser::syscalls::nr::ClearEvent,
        sunrise_libuser::syscalls::nr::ResetSignal,
    ]
});
