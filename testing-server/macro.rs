#![feature(prelude_import)]
#![no_std]
#![feature(alloc, const_let)]
#![no_std]
#![warn(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]
#[prelude_import]
use core::prelude::v1::*;
#[macro_use]
extern crate core;
#[macro_use]
extern crate compiler_builtins;
#[macro_use]
extern crate kfs_libuser as libuser;
#[macro_use]
extern crate alloc;
use alloc::prelude::*;
use alloc::sync::{Arc, Weak};
use core::fmt::Write;
use libuser::io::{self, Io};
use libuser::syscalls;
use libuser::terminal::{Terminal, WindowSize};
#[macro_use]
extern crate log;
extern crate kfs_libuser;
use libuser::error::Error;
use libuser::ipc::server::{IWaitable, PortHandler, SessionWrapper, WaitableManager};
use libuser::ipc::Message;
use libuser::types::*;
struct LogService;
#[automatically_derived]
#[allow(unused_qualifications)]
impl $crate::default::Default for LogService {
    #[inline]
    fn default() -> LogService {
        LogService
    }
}
struct LoggerInterface;
#[automatically_derived]
#[allow(unused_qualifications)]
impl $crate::default::Default for LoggerInterface {
    #[inline]
    fn default() -> LoggerInterface {
        LoggerInterface
    }
}
impl LogService {
    fn open_logger(
        &mut self,
        manager: &WaitableManager,
        val: u32,
        pid: Pid,
    ) -> Result<(Handle,), Error> {
        let logger_interface = LoggerInterface {};
        let (server, client) = syscalls::create_session(false, 0)?;
        let wrapper = SessionWrapper::new(server, logger_interface);
        manager.add_waitable(Box::new(wrapper) as Box<dyn IWaitable>);
        Ok((client.into_handle(),))
    }
}
impl $crate::ipc::server::Object for LogService {
    fn dispatch(
        &mut self,
        manager: &$crate::ipc::server::WaitableManager,
        cmdid: u32,
        buf: &mut [u8],
    ) -> Result<(), $crate::error::Error> {
        match cmdid {
            0 => {
                #[repr(C)]
                #[rustc_copy_clone_marker]
                struct Args {
                    val: u32,
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::default::Default for Args {
                    #[inline]
                    fn default() -> Args {
                        Args {
                            val: $crate::default::Default::default(),
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::fmt::Debug for Args {
                    fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
                        match *self {
                            Args {
                                val: ref __self_0_0,
                            } => {
                                let mut debug_trait_builder = f.debug_struct("Args");
                                let _ = debug_trait_builder.field("val", &&(*__self_0_0));
                                debug_trait_builder.finish()
                            }
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::clone::Clone for Args {
                    #[inline]
                    fn clone(&self) -> Args {
                        {
                            let _: $crate::clone::AssertParamIsClone<u32>;
                            *self
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::marker::Copy for Args {}
                let mut msgin = $crate::ipc::Message::<Args, [_; 0], [_; 0], [_; 0]>::unpack(buf);
                let ret = self.open_logger(manager, msgin.raw().val, msgin.pop_pid()?);
                let mut msgout =
                    $crate::ipc::Message::<_, [_; 0], [_; 0], [_; 1 + 0]>::new_response(
                        msgin.token(),
                    );
                match ret {
                    Ok(ret) => {
                        #[repr(C)]
                        #[rustc_copy_clone_marker]
                        struct Ret();
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::default::Default for Ret {
                            #[inline]
                            fn default() -> Ret {
                                Ret()
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::fmt::Debug for Ret {
                            fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
                                match *self {
                                    Ret() => {
                                        let mut debug_trait_builder = f.debug_tuple("Ret");
                                        debug_trait_builder.finish()
                                    }
                                }
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::clone::Clone for Ret {
                            #[inline]
                            fn clone(&self) -> Ret {
                                {
                                    *self
                                }
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::marker::Copy for Ret {}
                        let (arg,) = ret;
                        msgout.push_handle_move(arg);
                        msgout.push_raw(Ret());
                    }
                    Err(err) => {
                        msgout.set_error(err.into_code());
                    }
                }
                msgout.pack(buf);
                Ok(())
            }
            cmd => {
                let _ = $crate::syscalls::output_debug_string(&$crate::fmt::format(
                    $crate::fmt::Arguments::new_v1_formatted(
                        &["Unknown cmdid: "],
                        &match (&cmd,) {
                            (arg0,) => [$crate::fmt::ArgumentV1::new(
                                arg0,
                                $crate::fmt::Display::fmt,
                            )],
                        },
                        &[$crate::fmt::rt::v1::Argument {
                            position: $crate::fmt::rt::v1::Position::At(0usize),
                            format: $crate::fmt::rt::v1::FormatSpec {
                                fill: ' ',
                                align: $crate::fmt::rt::v1::Alignment::Unknown,
                                flags: 0u32,
                                precision: $crate::fmt::rt::v1::Count::Implied,
                                width: $crate::fmt::rt::v1::Count::Implied,
                            },
                        }],
                    ),
                ));
                Err($crate::error::KernelError::PortRemoteDead.into())
            }
        }
    }
}
impl LoggerInterface {
    fn log(&mut self) -> Result<((),), Error> {
        Ok(((),))
    }
    fn set_destination(&mut self, destination: u32) -> Result<((),), Error> {
        {
            let lvl = $crate::Level::Info;
            if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
                $crate::__private_api_log(
                    $crate::fmt::Arguments::new_v1_formatted(
                        &["Destination: "],
                        &match (&destination,) {
                            (arg0,) => [$crate::fmt::ArgumentV1::new(
                                arg0,
                                $crate::fmt::Display::fmt,
                            )],
                        },
                        &[$crate::fmt::rt::v1::Argument {
                            position: $crate::fmt::rt::v1::Position::At(0usize),
                            format: $crate::fmt::rt::v1::FormatSpec {
                                fill: ' ',
                                align: $crate::fmt::rt::v1::Alignment::Unknown,
                                flags: 0u32,
                                precision: $crate::fmt::rt::v1::Count::Implied,
                                width: $crate::fmt::rt::v1::Count::Implied,
                            },
                        }],
                    ),
                    lvl,
                    &(
                        "kfs_testing_server",
                        "kfs_testing_server",
                        "testing-server/src/main.rs",
                        58u32,
                    ),
                );
            }
        };
        Ok(((),))
    }
}
impl $crate::ipc::server::Object for LoggerInterface {
    fn dispatch(
        &mut self,
        manager: &$crate::ipc::server::WaitableManager,
        cmdid: u32,
        buf: &mut [u8],
    ) -> Result<(), $crate::error::Error> {
        match cmdid {
            0 => {
                #[repr(C)]
                #[rustc_copy_clone_marker]
                struct Args {}
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::default::Default for Args {
                    #[inline]
                    fn default() -> Args {
                        Args {}
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::fmt::Debug for Args {
                    fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
                        match *self {
                            Args {} => {
                                let mut debug_trait_builder = f.debug_struct("Args");
                                debug_trait_builder.finish()
                            }
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::clone::Clone for Args {
                    #[inline]
                    fn clone(&self) -> Args {
                        {
                            *self
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::marker::Copy for Args {}
                let mut msgin = $crate::ipc::Message::<Args, [_; 0], [_; 0], [_; 0]>::unpack(buf);
                let ret = self.log();
                let mut msgout =
                    $crate::ipc::Message::<_, [_; 0], [_; 0], [_; 0]>::new_response(msgin.token());
                match ret {
                    Ok(ret) => {
                        #[repr(C)]
                        #[rustc_copy_clone_marker]
                        struct Ret(());
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::default::Default for Ret {
                            #[inline]
                            fn default() -> Ret {
                                Ret($crate::default::Default::default())
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::fmt::Debug for Ret {
                            fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
                                match *self {
                                    Ret(ref __self_0_0) => {
                                        let mut debug_trait_builder = f.debug_tuple("Ret");
                                        let _ = debug_trait_builder.field(&&(*__self_0_0));
                                        debug_trait_builder.finish()
                                    }
                                }
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::clone::Clone for Ret {
                            #[inline]
                            fn clone(&self) -> Ret {
                                {
                                    let _: $crate::clone::AssertParamIsClone<()>;
                                    *self
                                }
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::marker::Copy for Ret {}
                        let (arg,) = ret;
                        ();
                        msgout.push_raw(Ret(arg));
                    }
                    Err(err) => {
                        msgout.set_error(err.into_code());
                    }
                }
                msgout.pack(buf);
                Ok(())
            }
            1 => {
                #[repr(C)]
                #[rustc_copy_clone_marker]
                struct Args {
                    destination: u32,
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::default::Default for Args {
                    #[inline]
                    fn default() -> Args {
                        Args {
                            destination: $crate::default::Default::default(),
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::fmt::Debug for Args {
                    fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
                        match *self {
                            Args {
                                destination: ref __self_0_0,
                            } => {
                                let mut debug_trait_builder = f.debug_struct("Args");
                                let _ = debug_trait_builder.field("destination", &&(*__self_0_0));
                                debug_trait_builder.finish()
                            }
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::clone::Clone for Args {
                    #[inline]
                    fn clone(&self) -> Args {
                        {
                            let _: $crate::clone::AssertParamIsClone<u32>;
                            *self
                        }
                    }
                }
                #[automatically_derived]
                #[allow(unused_qualifications)]
                impl $crate::marker::Copy for Args {}
                let mut msgin = $crate::ipc::Message::<Args, [_; 0], [_; 0], [_; 0]>::unpack(buf);
                let ret = self.set_destination(msgin.raw().destination);
                let mut msgout =
                    $crate::ipc::Message::<_, [_; 0], [_; 0], [_; 0]>::new_response(msgin.token());
                match ret {
                    Ok(ret) => {
                        #[repr(C)]
                        #[rustc_copy_clone_marker]
                        struct Ret(());
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::default::Default for Ret {
                            #[inline]
                            fn default() -> Ret {
                                Ret($crate::default::Default::default())
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::fmt::Debug for Ret {
                            fn fmt(&self, f: &mut $crate::fmt::Formatter) -> $crate::fmt::Result {
                                match *self {
                                    Ret(ref __self_0_0) => {
                                        let mut debug_trait_builder = f.debug_tuple("Ret");
                                        let _ = debug_trait_builder.field(&&(*__self_0_0));
                                        debug_trait_builder.finish()
                                    }
                                }
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::clone::Clone for Ret {
                            #[inline]
                            fn clone(&self) -> Ret {
                                {
                                    let _: $crate::clone::AssertParamIsClone<()>;
                                    *self
                                }
                            }
                        }
                        #[automatically_derived]
                        #[allow(unused_qualifications)]
                        impl $crate::marker::Copy for Ret {}
                        let (arg,) = ret;
                        ();
                        msgout.push_raw(Ret(arg));
                    }
                    Err(err) => {
                        msgout.set_error(err.into_code());
                    }
                }
                msgout.pack(buf);
                Ok(())
            }
            cmd => {
                let _ = $crate::syscalls::output_debug_string(&$crate::fmt::format(
                    $crate::fmt::Arguments::new_v1_formatted(
                        &["Unknown cmdid: "],
                        &match (&cmd,) {
                            (arg0,) => [$crate::fmt::ArgumentV1::new(
                                arg0,
                                $crate::fmt::Display::fmt,
                            )],
                        },
                        &[$crate::fmt::rt::v1::Argument {
                            position: $crate::fmt::rt::v1::Position::At(0usize),
                            format: $crate::fmt::rt::v1::FormatSpec {
                                fill: ' ',
                                align: $crate::fmt::rt::v1::Alignment::Unknown,
                                flags: 0u32,
                                precision: $crate::fmt::rt::v1::Count::Implied,
                                width: $crate::fmt::rt::v1::Count::Implied,
                            },
                        }],
                    ),
                ));
                Err($crate::error::KernelError::PortRemoteDead.into())
            }
        }
    }
}
fn main() {
    let man = WaitableManager::new();
    let handler = Box::new(PortHandler::<LogService>::new("lm\u{0}").unwrap());
    man.add_waitable(handler as Box<dyn IWaitable>);
    man.run();
}
#[link_section = ".kernel_caps"]
#[used]
static CAPABILITIES: [u32; 6] = {
    let mut kacs = [
        0 << 29 | 15,
        1 << 29 | 15,
        2 << 29 | 15,
        3 << 29 | 15,
        4 << 29 | 15,
        5 << 29 | 15,
    ];
    kacs[kfs_libuser::syscalls::nr::SleepThread / 24] |=
        1 << ((kfs_libuser::syscalls::nr::SleepThread % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::ExitProcess / 24] |=
        1 << ((kfs_libuser::syscalls::nr::ExitProcess % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::CloseHandle / 24] |=
        1 << ((kfs_libuser::syscalls::nr::CloseHandle % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::WaitSynchronization / 24] |=
        1 << ((kfs_libuser::syscalls::nr::WaitSynchronization % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::OutputDebugString / 24] |=
        1 << ((kfs_libuser::syscalls::nr::OutputDebugString % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::ReplyAndReceiveWithUserBuffer / 24] |=
        1 << ((kfs_libuser::syscalls::nr::ReplyAndReceiveWithUserBuffer % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::AcceptSession / 24] |=
        1 << ((kfs_libuser::syscalls::nr::AcceptSession % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::CreateSession / 24] |=
        1 << ((kfs_libuser::syscalls::nr::CreateSession % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::ConnectToNamedPort / 24] |=
        1 << ((kfs_libuser::syscalls::nr::ConnectToNamedPort % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::SendSyncRequestWithUserBuffer / 24] |=
        1 << ((kfs_libuser::syscalls::nr::SendSyncRequestWithUserBuffer % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::SetHeapSize / 24] |=
        1 << ((kfs_libuser::syscalls::nr::SetHeapSize % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::QueryMemory / 24] |=
        1 << ((kfs_libuser::syscalls::nr::QueryMemory % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::MapSharedMemory / 24] |=
        1 << ((kfs_libuser::syscalls::nr::MapSharedMemory % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::UnmapSharedMemory / 24] |=
        1 << ((kfs_libuser::syscalls::nr::UnmapSharedMemory % 24) + 5);
    kacs[kfs_libuser::syscalls::nr::MapFramebuffer / 24] |=
        1 << ((kfs_libuser::syscalls::nr::MapFramebuffer % 24) + 5);
    kacs
};
