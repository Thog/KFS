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

#![feature(alloc, const_let)]
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
extern crate kfs_libuser as libuser;
#[macro_use]
extern crate alloc;


#[macro_use]
extern crate lazy_static;

use alloc::prelude::*;
use crate::libuser::syscalls;
use crate::libuser::ipc::server::{WaitableManager, PortHandler, IWaitable};
use crate::libuser::types::*;
use crate::libuser::error::Error;
use crate::libuser::error::SmError;
use hashmap_core::map::{HashMap, Entry};
use spin::Mutex;

/// `sm:` service interface.
/// The main interface to the Service Manager. Clients can use it to connect to
/// or register new services (assuming they have the appropriate capabilities).
///
/// Make sure to call the [UserInterface::initialize] method before using it.
#[derive(Debug, Default)]
struct UserInterface;

lazy_static! {
    /// Global mapping of Service Name -> ClientPort.
    static ref SERVICES: Mutex<HashMap<u64, ClientPort>> = Mutex::new(HashMap::new());
}
// TODO: global event when services are accessed.

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

object! {
    impl UserInterface {
        /// Initialize the UserInterface, acquiring the Pid of the remote
        /// process, which will then be used to validate the permissions of each
        /// calls.
        #[cmdid(0)]
        fn initialize(&mut self, _pid: Pid,) -> Result<(), Error> {
            Ok(())
        }

        /// Get a ClientSession to this service.
        #[cmdid(1)]
        fn get_service(&mut self, servicename: u64,) -> Result<(Handle,), Error> {
            match SERVICES.lock().get(&servicename) {
                Some(port) => port.connect().map(|v| (v.into_handle(),)),
                None => Err(SmError::ServiceNotRegistered.into())
            }
        }

        /// Register a new service, returning a ServerPort to the newly
        /// registered service.
        #[cmdid(2)]
        fn register_service(&mut self, servicename: u64, is_light: u8, max_handles: u32,) -> Result<(Handle,), Error> {
            let (clientport, serverport) = syscalls::create_port(max_handles, is_light != 0, get_service_str(&servicename))?;
            match SERVICES.lock().entry(servicename) {
                Entry::Occupied(_) => Err(SmError::ServiceAlreadyRegistered.into()),
                Entry::Vacant(vacant) => {
                    vacant.insert(clientport);
                    Ok((serverport.0,))
                }
            }
        }

        /// Unregister a service.
        #[cmdid(3)]
        fn unregister_service(&mut self, servicename: u64,) -> Result<(), Error> {
            match SERVICES.lock().remove(&servicename) {
                Some(_) => Ok(()),
                None => Err(SmError::ServiceNotRegistered.into())
            }
        }
    }
}

fn main() {
    let man = WaitableManager::new();
    let handler = Box::new(PortHandler::<UserInterface>::new_managed("sm:\0").unwrap());
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

        kfs_libuser::syscalls::nr::SetHeapSize,
        kfs_libuser::syscalls::nr::ManageNamedPort,
        kfs_libuser::syscalls::nr::AcceptSession,
        kfs_libuser::syscalls::nr::ReplyAndReceiveWithUserBuffer,
        kfs_libuser::syscalls::nr::CreatePort,
        kfs_libuser::syscalls::nr::ConnectToPort,
    ]
});
