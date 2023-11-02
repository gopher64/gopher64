//! Smithay Client Toolkit
//!
//! Provides various utilities and abstractions for comunicating with various
//! Wayland compositors.
//!
//! ## `Environment`
//!
//! The crate is structured around the [`Environment`](environment/struct.Environment.html) type,
//! which binds the wayland globals for you using a set of modular handlers. This type is used in conjunction
//! with the [`environment!`](macro.environment.html) if you want full control, or by using the
//! [`default_environment!`](macro.default_environment.html) macro to automatically bring in all
//! SCTK modules.
//!
//! The various modules work by adding methods to the [`Environment`](environment/struct.Environment.html)
//! type, giving you more capabilities as more modules are activated.
//!
//! ## Event Loops
//!
//! SCTK integrates with `calloop` to provide an event loop abstraction. Indeed most Wayland
//! apps will need to handle more event sources than the single Wayland connection. These are
//! necessary to handle things like keyboard repetition, copy-paste, or animated cursors.
//!
//! [`WaylandSource`](struct.WaylandSource.html) is an adapter to insert a Wayland `EventQueue` into
//! a calloop event loop. And some of the modules of SCTK will provide you with other event sources
//! that you need to insert into calloop for them to work correctly.
#![warn(missing_docs, missing_debug_implementations)]
#![allow(clippy::new_without_default)]

#[macro_use]
extern crate dlib;

/// Re-exports of some crates, for convenience
pub mod reexports {
    #[cfg(feature = "calloop")]
    pub use calloop;
    pub use wayland_client as client;
    pub use wayland_protocols as protocols;
}

pub mod data_device;
pub mod environment;
mod lazy_global;
pub mod output;
pub mod primary_selection;
pub mod seat;
pub mod shell;
pub mod shm;
pub mod window;

#[cfg(feature = "calloop")]
mod event_loop;
mod surface;

#[cfg(feature = "calloop")]
pub use event_loop::WaylandSource;
pub use surface::{get_surface_outputs, get_surface_scale_factor};

#[macro_export]
/// Declare a batteries-included SCTK environment
///
/// Similar to the [`environment!`](macro.environment.html) macro, but creates the type for you and
/// includes all the handlers provided by SCTK, for use with the rest of the library. Its sister
/// macro [`new_default_environment!`](macro.new_default_environment.html) needs to be used to
/// initialize it.
///
/// This includes handlers for the following globals:
///
/// - `wl_compositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_data_device_manager` as a [`DataDeviceHandler`](data_device/struct.DataDeviceHandler.html)
/// - `wl_output` with the [`OutputHandler`](output/struct.OutputHandler.html)
/// - `wl_seat` with the [`SeatHandler`](seat/struct.SeatHandler.html)
/// - `wl_subcompositor` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
/// - `wl_shm` as a [`ShmHandler`](shm/struct.ShmHandler.html)
/// - `zwp` and `gtk` primary selection device manager as a [`PrimarySelectionHandler`](primary_selection/struct.PrimarySelectionHandler.html)
///
/// If you don't need to add anything more, using it is as simple as:
///
/// ```no_run
/// # use smithay_client_toolkit::default_environment;
/// default_environment!(MyEnv);
/// ```
///
/// The macro also provides some presets including more globals depending on your use-case:
///
/// - the `desktop` preset, invoked as `default_environment!(MyEnv, desktop);` additionally
/// includes:
///   - `xdg_shell` and `wl_shell` with the [`ShellHandler`](shell/struct.ShellHandler.html)
///   - `xdg_decoration_manager` as a [`SimpleGlobal`](environment/struct.SimpleGlobal.html)
///
/// You can also add the `fields` argument to add additional fields to the generated struct, and
/// the `singles` and `multis` arguments to route additional globals like with the
/// [`environment!`](macro.environment.html) macro. These three fields are optional, but they must
/// appear in this order, and after the optional preset
///
/// ```no_run
/// # use smithay_client_toolkit::default_environment;
/// default_environment!(MyEnv,
///     desktop, // the chosen preset, can be ommited
///     fields=[
///         somefield: u32,
///         otherfield: String,
///     ],
///     singles=[
///         // Add some routing here
///     ],
///     multis=[
///         // add some routing here
///     ]
/// );
/// ```
macro_rules! default_environment {
    ($env_name:ident, desktop
        $(,fields = [$($fname:ident : $fty:ty),* $(,)?])?
        $(,singles = [$($sty:ty => $sname: ident),* $(,)?])?
        $(,multis = [$($mty:ty => $mname:ident),* $(,)?])?
        $(,)?
    ) => {
        $crate::default_environment!($env_name,
            fields=[
                // shell
                sctk_shell: $crate::shell::ShellHandler,
                // decoration
                sctk_decoration_mgr: $crate::environment::SimpleGlobal<$crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
                // others
                $($($fname : $fty,)*)?
            ],
            singles = [
                // shell globals
                $crate::reexports::client::protocol::wl_shell::WlShell => sctk_shell,
                $crate::reexports::protocols::xdg_shell::client::xdg_wm_base::XdgWmBase => sctk_shell,
                $crate::reexports::protocols::unstable::xdg_shell::v6::client::zxdg_shell_v6::ZxdgShellV6 => sctk_shell,
                // decoration
                $crate::reexports::protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1 => sctk_decoration_mgr,
                // others
                $($($sty => $sname,)*)?
            ],
            multis = [ $($($mty => $mname,)*)?  ],
        );

        // Shell utility
        impl $crate::shell::ShellHandling for $env_name {
            fn get_shell(&self) -> Option<$crate::shell::Shell> {
                self.sctk_shell.get_shell()
            }
        }
    };
    ($env_name:ident
        $(,fields = [$($fname:ident : $fty:ty),* $(,)?])?
        $(,singles = [$($sty:ty => $sname:ident),* $(,)?])?
        $(,multis = [$($mty:ty => $mname:ident),* $(,)?])?
        $(,)?
    ) => {
        /*
         * Declare the type
         */
        pub struct $env_name {
            // SimpleGlobals
            sctk_compositor: $crate::environment::SimpleGlobal<$crate::reexports::client::protocol::wl_compositor::WlCompositor>,
            sctk_subcompositor: $crate::environment::SimpleGlobal<$crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor>,
            // shm
            sctk_shm: $crate::shm::ShmHandler,
            // output
            sctk_outputs: $crate::output::OutputHandler,
            // seat
            sctk_seats: $crate::seat::SeatHandler,
            // data device
            sctk_data_device_manager: $crate::data_device::DataDeviceHandler,
            // primary selection
            sctk_primary_selection_manager: $crate::primary_selection::PrimarySelectionHandler,
            // user added
            $($(
                $fname : $fty,
            )*)?
        }

        // SHM utility
        impl $crate::shm::ShmHandling for $env_name {
            fn shm_formats(&self) -> Vec<$crate::reexports::client::protocol::wl_shm::Format> {
                self.sctk_shm.shm_formats()
            }
        }

        // Seat utility
        impl $crate::seat::SeatHandling for $env_name {
            fn listen<F>(&mut self, f: F) -> $crate::seat::SeatListener
            where F: FnMut(
                $crate::reexports::client::Attached<$crate::reexports::client::protocol::wl_seat::WlSeat>,
                &$crate::seat::SeatData,
                $crate::reexports::client::DispatchData
            ) + 'static
            {
                self.sctk_seats.listen(f)
            }
        }

        // Output utility
        impl $crate::output::OutputHandling for $env_name {
            fn listen<F>(&mut self, f: F) -> $crate::output::OutputStatusListener
            where F: FnMut(
                $crate::reexports::client::protocol::wl_output::WlOutput,
                &$crate::output::OutputInfo,
                $crate::reexports::client::DispatchData,
            ) + 'static
            {
                self.sctk_outputs.listen(f)
            }
        }

        // Data device utility
        impl $crate::data_device::DataDeviceHandling for $env_name {
            fn set_callback<F>(&mut self, callback: F) -> ::std::result::Result<(), $crate::MissingGlobal>
            where F: FnMut(
                $crate::reexports::client::protocol::wl_seat::WlSeat,
                $crate::data_device::DndEvent,
                $crate::reexports::client::DispatchData
            ) + 'static
            {
                self.sctk_data_device_manager.set_callback(callback)
            }

            fn with_device<F: FnOnce(&$crate::data_device::DataDevice)>(
                &self,
                seat: &$crate::reexports::client::protocol::wl_seat::WlSeat,
                f: F
            ) -> ::std::result::Result<(), $crate::MissingGlobal> {
                self.sctk_data_device_manager.with_device(seat, f)
            }
        }

        // Primary selection utility
        impl $crate::primary_selection::PrimarySelectionHandling for $env_name {
            fn with_primary_selection<F>(
                &self,
                seat: &$crate::reexports::client::protocol::wl_seat::WlSeat,
                f: F,
            ) -> ::std::result::Result<(), $crate::MissingGlobal>
            where F: FnOnce(&$crate::primary_selection::PrimarySelectionDevice)
            {
                self.sctk_primary_selection_manager.with_primary_selection(seat, f)
            }

            fn get_primary_selection_manager(&self) -> Option<$crate::primary_selection::PrimarySelectionDeviceManager> {
                self.sctk_primary_selection_manager.get_primary_selection_manager()
            }
        }

        //
        // Final macro delegation
        //
        $crate::environment!($env_name,
            singles = [
                // SimpleGlobals
                $crate::reexports::client::protocol::wl_compositor::WlCompositor => sctk_compositor,
                $crate::reexports::client::protocol::wl_subcompositor::WlSubcompositor => sctk_subcompositor,
                // shm
                $crate::reexports::client::protocol::wl_shm::WlShm => sctk_shm,
                // data device
                $crate::reexports::client::protocol::wl_data_device_manager::WlDataDeviceManager => sctk_data_device_manager,
                // primary selection
                $crate::reexports::protocols::unstable::primary_selection::v1::client::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1 => sctk_primary_selection_manager,
                $crate::reexports::protocols::misc::gtk_primary_selection::client::gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager => sctk_primary_selection_manager,
                // user added
                $($($sty => $sname),*)?
            ],
            multis = [
                // output globals
                $crate::reexports::client::protocol::wl_output::WlOutput => sctk_outputs,
                // seat globals
                $crate::reexports::client::protocol::wl_seat::WlSeat => sctk_seats,
                // user added
                $($($mty => $mname),*)?
            ]
        );
    };
}

#[macro_export]
/// Initialize a batteries-included SCTK environment
///
/// Sister macro of [`default_environment!`](macro.default_environment.html). You need
/// to use it to initialize the environment instead of
/// [`Envrionment::init`](environment/struct.Environment.html). It has the same semantics.
///
/// If a preset was used for [`default_environment!`](macro.default_environment.html), it
/// must be provided here as well.
///
/// The macro will automatically setup a Wayland connection and evaluate to a `Result`
/// containing either `Ok((env, display, queue))`, providing you the initialized `Environment`
/// as well as the wayland `Display` and `EventQueue` associated to it, or to an error
/// if the connection failed.
///
/// ```no_run
/// # use smithay_client_toolkit::{default_environment, new_default_environment};
/// # default_environment!(MyEnv, desktop, fields=[somefield: u32, otherfield: String]);
/// let (env, display, queue) = new_default_environment!(MyEnv,
///     desktop,           // the optional preset
///     /* initializers for your extra fields if any, can be ommited if no fields are added */
///     fields=[
///         somefield: 42,
///         otherfield: String::from("Hello World"),
///     ]
/// ).expect("Unable to connect to the wayland compositor");
/// ```
///
/// If you instead want the macro to use some pre-existing display and event queue, you can
/// add the `with` argument providing them. In that case the macro will evaluate to
/// a `Result<Environment, io::Error>`, forwarding to you any error that may have occured
/// during the initial roundtrips.
///
/// ```no_run
/// # use smithay_client_toolkit::{default_environment, new_default_environment};
/// # default_environment!(MyEnv, desktop, fields=[somefield: u32, otherfield: String]);
/// # let display = smithay_client_toolkit::reexports::client::Display::connect_to_env().unwrap();
/// # let mut queue = display.create_event_queue();
/// let env = new_default_environment!(MyEnv,
///     desktop,                 // the optional preset
///     with=(display, queue),   // the display and event queue to use
///     /* initializers for your extra fields if any, can be ommited if no fields are added */
///     fields=[
///         somefield: 42,
///         otherfield: String::from("Hello World"),
///     ]
/// ).expect("Initial roundtrips failed!");
/// ```
macro_rules! new_default_environment {
    ($env_name:ident, desktop
        $(, with=($display:expr, $queue:expr))?
        $(,fields = [$($fname:ident : $fval:expr),* $(,)?])?
        $(,)?
    ) => {
        $crate::new_default_environment!($env_name,
            $(with=($display, $queue),)?
            fields = [
                sctk_shell: $crate::shell::ShellHandler::new(),
                sctk_decoration_mgr: $crate::environment::SimpleGlobal::new(),
                $($(
                    $fname: $fval,
                )*)?
            ]
        )
    };
    ($env_name:ident, with=($display:expr, $queue:expr)
        $(,fields = [$($fname:ident : $fval:expr),* $(,)?])?
        $(,)?
    ) => {
        {
            let mut sctk_seats = $crate::seat::SeatHandler::new();
            let sctk_data_device_manager = $crate::data_device::DataDeviceHandler::init(&mut sctk_seats);
            let sctk_primary_selection_manager = $crate::primary_selection::PrimarySelectionHandler::init(&mut sctk_seats);

            let display = $crate::reexports::client::Proxy::clone(&$display);
            let env = $crate::environment::Environment::new(&display.attach($queue.token()), &mut $queue,$env_name {
                sctk_compositor: $crate::environment::SimpleGlobal::new(),
                sctk_subcompositor: $crate::environment::SimpleGlobal::new(),
                sctk_shm: $crate::shm::ShmHandler::new(),
                sctk_outputs: $crate::output::OutputHandler::new(),
                sctk_seats,
                sctk_data_device_manager,
                sctk_primary_selection_manager,
                $($(
                    $fname: $fval,
                )*)?
            });

            if let Ok(env) = env.as_ref() {
                // Bind primary selection manager.
                let _psm = env.get_primary_selection_manager();
            }

            env
        }
    };
    ($env_name:ident
        $(,fields = [$($fname:ident : $fval:expr),* $(,)?])?
        $(,)?
    ) => {
        $crate::reexports::client::Display::connect_to_env().and_then(|display| {
            let mut queue = display.create_event_queue();
            let ret = $crate::new_default_environment!(
                $env_name,
                with=(display, queue),
                fields=[$($($fname: $fval),*)?],
            );
            match ret {
                Ok(env) => Ok((env, display, queue)),
                Err(e) => {
                    if let Some(perr) = display.protocol_error() {
                        panic!("[SCTK] A protocol error occured during initial setup: {}", perr);
                    } else {
                        // For some other reason the connection with the compositor was lost
                        // This should not arrive unless maybe the compositor was shutdown during
                        // the initial setup...
                        Err($crate::reexports::client::ConnectError::NoCompositorListening)
                    }
                }
            }
        })
    };
}

/// An error representing the fact that a required global was missing
#[derive(Debug, Copy, Clone)]
pub struct MissingGlobal;

impl std::error::Error for MissingGlobal {}

impl std::fmt::Display for MissingGlobal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("missing global")
    }
}
