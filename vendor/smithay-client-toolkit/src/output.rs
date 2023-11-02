//! Types and functions related to graphical outputs
//!
//! This modules provides two main elements. The first is the
//! [`OutputHandler`](struct.OutputHandler.html) type, which is a
//! [`MultiGlobalHandler`](../environment/trait.MultiGlobalHandler.html) for
//! use with the [`init_environment!`](../macro.init_environment.html) macro. It is automatically
//! included if you use the [`new_default_environment!`](../macro.new_default_environment.html).
//!
//! The second is the [`with_output_info`](fn.with_output_info.html) with allows you to
//! access the information associated to this output, as an [`OutputInfo`](struct.OutputInfo.html).

use std::{
    cell::RefCell,
    fmt,
    rc::{self, Rc},
    sync::{self, Arc, Mutex},
};

use wayland_client::{
    protocol::{
        wl_output::{self, Event, WlOutput},
        wl_registry,
    },
    Attached, DispatchData, Main,
};

use wayland_protocols::unstable::xdg_output::v1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};

pub use wayland_client::protocol::wl_output::{Subpixel, Transform};

/// A possible mode for an output
#[derive(Copy, Clone, Debug)]
pub struct Mode {
    /// Number of pixels of this mode in format `(width, height)`
    ///
    /// for example `(1920, 1080)`
    pub dimensions: (i32, i32),
    /// Refresh rate for this mode, in mHz
    pub refresh_rate: i32,
    /// Whether this is the current mode for this output
    pub is_current: bool,
    /// Whether this is the preferred mode for this output
    pub is_preferred: bool,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// Compiled information about an output
pub struct OutputInfo {
    /// The ID of this output as a global
    pub id: u32,
    /// The model name of this output as advertised by the server
    pub model: String,
    /// The make name of this output as advertised by the server
    pub make: String,
    /// The name of this output as advertised by the server
    ///
    /// Each name is unique among all wl_output globals, but if a wl_output
    /// global is destroyed the same name may be reused later. The names will
    /// also remain consistent across sessions with the same hardware and
    /// software configuration.
    ///
    /// Examples of names include 'HDMI-A-1', 'WL-1', 'X11-1', etc. However, do
    /// not assume that the name is a reflection of an underlying DRM connector,
    /// X11 connection, etc.
    ///
    /// Note that this is not filled in by version 3 of the wl_output protocol,
    /// but it has been proposed for inclusion in version 4.  Until then, it is
    /// only filled in if your environment has an [XdgOutputHandler] global
    /// handler for [ZxdgOutputManagerV1].
    pub name: String,
    /// The description of this output as advertised by the server
    ///
    /// The description is a UTF-8 string with no convention defined for its
    /// contents. The description is not guaranteed to be unique among all
    /// wl_output globals. Examples might include 'Foocorp 11" Display' or
    /// 'Virtual X11 output via :1'.
    ///
    /// Note that this is not filled in by version 3 of the wl_output protocol,
    /// but it has been proposed for inclusion in version 4.  Until then, it is
    /// only filled in if your environment has an [XdgOutputHandler] global
    /// handler for [ZxdgOutputManagerV1].
    pub description: String,
    /// Location of the top-left corner of this output in compositor
    /// space
    ///
    /// Note that the compositor may decide to always report (0,0) if
    /// it decides clients are not allowed to know this information.
    pub location: (i32, i32),
    /// Physical dimensions of this output, in unspecified units
    pub physical_size: (i32, i32),
    /// The subpixel layout for this output
    pub subpixel: Subpixel,
    /// The current transformation applied to this output
    ///
    /// You can pre-render your buffers taking this information
    /// into account and advertising it via `wl_buffer.set_tranform`
    /// for better performances.
    pub transform: Transform,
    /// The scaling factor of this output
    ///
    /// Any buffer whose scaling factor does not match the one
    /// of the output it is displayed on will be rescaled accordingly.
    ///
    /// For example, a buffer of scaling factor 1 will be doubled in
    /// size if the output scaling factor is 2.
    pub scale_factor: i32,
    /// Possible modes for an output
    pub modes: Vec<Mode>,
    /// Has this output been unadvertized by the registry
    ///
    /// If this is the case, it has become inert, you might want to
    /// call its `release()` method if you don't plan to use it any
    /// longer.
    pub obsolete: bool,
}

impl OutputInfo {
    fn new(id: u32) -> OutputInfo {
        OutputInfo {
            id,
            model: String::new(),
            make: String::new(),
            name: String::new(),
            description: String::new(),
            location: (0, 0),
            physical_size: (0, 0),
            subpixel: Subpixel::Unknown,
            transform: Transform::Normal,
            scale_factor: 1,
            modes: Vec::new(),
            obsolete: false,
        }
    }
}

type OutputCallback = dyn Fn(WlOutput, &OutputInfo, DispatchData) + Send + Sync;

enum OutputData {
    Ready {
        info: OutputInfo,
        callbacks: Vec<sync::Weak<OutputCallback>>,
    },
    Pending {
        id: u32,
        has_xdg: bool,
        events: Vec<Event>,
        callbacks: Vec<sync::Weak<OutputCallback>>,
    },
    PendingXDG {
        info: OutputInfo,
        callbacks: Vec<sync::Weak<OutputCallback>>,
    },
}

type OutputStatusCallback = dyn FnMut(WlOutput, &OutputInfo, DispatchData) + 'static;

/// A handler for `wl_output`
///
/// This handler can be used for managing `wl_output` in the
/// [`init_environment!`](../macro.init_environment.html) macro, and is automatically
/// included in [`new_default_environment!`](../macro.new_default_environment.html).
///
/// It aggregates the output information and makes it available via the
/// [`with_output_info`](fn.with_output_info.html) function.
pub struct OutputHandler {
    outputs: Vec<(u32, Attached<WlOutput>)>,
    status_listeners: Rc<RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>>,
    xdg_listener: Option<rc::Weak<RefCell<XdgOutputHandlerInner>>>,
}

impl OutputHandler {
    /// Create a new instance of this handler
    pub fn new() -> OutputHandler {
        OutputHandler {
            outputs: Vec::new(),
            status_listeners: Rc::new(RefCell::new(Vec::new())),
            xdg_listener: None,
        }
    }
}

impl crate::environment::MultiGlobalHandler<WlOutput> for OutputHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        // We currently support wl_output up to version 4
        let version = std::cmp::min(version, 4);
        let output = registry.bind::<WlOutput>(version, id);
        let has_xdg = if let Some(xdg) = self.xdg_listener.as_ref().and_then(rc::Weak::upgrade) {
            xdg.borrow_mut().new_xdg_output(&output, &self.status_listeners)
        } else {
            false
        };
        if version > 1 {
            // wl_output.done event was only added at version 2
            // In case of an old version 1, we just behave as if it was send at the start
            output.as_ref().user_data().set_threadsafe(|| {
                Mutex::new(OutputData::Pending { id, has_xdg, events: vec![], callbacks: vec![] })
            });
        } else {
            output.as_ref().user_data().set_threadsafe(|| {
                Mutex::new(OutputData::Ready { info: OutputInfo::new(id), callbacks: vec![] })
            });
        }
        let status_listeners_handle = self.status_listeners.clone();
        let xdg_listener_handle = self.xdg_listener.clone();
        output.quick_assign(move |output, event, ddata| {
            process_output_event(
                output,
                event,
                ddata,
                &status_listeners_handle,
                &xdg_listener_handle,
            )
        });
        self.outputs.push((id, (*output).clone()));
    }
    fn removed(&mut self, id: u32, mut ddata: DispatchData) {
        let status_listeners_handle = &self.status_listeners;
        let xdg_listener_handle = &self.xdg_listener;
        self.outputs.retain(|(i, o)| {
            if *i != id {
                true
            } else {
                make_obsolete(o, ddata.reborrow(), status_listeners_handle, xdg_listener_handle);
                false
            }
        });
    }
    fn get_all(&self) -> Vec<Attached<WlOutput>> {
        self.outputs.iter().map(|(_, o)| o.clone()).collect()
    }
}

impl fmt::Debug for OutputHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputHandler")
            .field("outputs", &self.outputs)
            .field("status_listeners", &"Fn() -> { ... }")
            .field("xdg_listener", &self.xdg_listener)
            .finish()
    }
}

fn process_output_event(
    output: Main<WlOutput>,
    event: Event,
    mut ddata: DispatchData,
    listeners: &Rc<RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>>,
    xdg_listener: &Option<rc::Weak<RefCell<XdgOutputHandlerInner>>>,
) {
    let udata_mutex = output
        .as_ref()
        .user_data()
        .get::<Mutex<OutputData>>()
        .expect("SCTK: wl_output has invalid UserData");
    let mut udata = udata_mutex.lock().unwrap();
    if let Event::Done = event {
        let (id, has_xdg, pending_events, mut callbacks) = match *udata {
            OutputData::Pending { id, has_xdg, events: ref mut v, callbacks: ref mut cb } => {
                (id, has_xdg, std::mem::take(v), std::mem::take(cb))
            }
            OutputData::PendingXDG { ref mut info, ref mut callbacks } => {
                notify(&output, info, ddata.reborrow(), callbacks);
                notify_status_listeners(&output, info, ddata, listeners);
                let info = info.clone();
                let callbacks = std::mem::take(callbacks);
                *udata = OutputData::Ready { info, callbacks };
                return;
            }
            OutputData::Ready { ref mut info, ref mut callbacks } => {
                // a Done event on an output that is already ready was due to a
                // status change (which was already merged)
                notify(&output, info, ddata, callbacks);
                return;
            }
        };
        let mut info = OutputInfo::new(id);
        for evt in pending_events {
            merge_event(&mut info, evt);
        }
        notify(&output, &info, ddata.reborrow(), &mut callbacks);
        if let Some(xdg) = xdg_listener.as_ref().and_then(rc::Weak::upgrade) {
            if has_xdg || xdg.borrow_mut().new_xdg_output(&output, listeners) {
                *udata = OutputData::PendingXDG { info, callbacks };
                return;
            }
        }
        notify_status_listeners(&output, &info, ddata, listeners);
        *udata = OutputData::Ready { info, callbacks };
    } else {
        match *udata {
            OutputData::Pending { events: ref mut v, .. } => v.push(event),
            OutputData::PendingXDG { ref mut info, .. }
            | OutputData::Ready { ref mut info, .. } => {
                merge_event(info, event);
            }
        }
    }
}

fn make_obsolete(
    output: &Attached<WlOutput>,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>,
    xdg_listener: &Option<rc::Weak<RefCell<XdgOutputHandlerInner>>>,
) {
    let udata_mutex = output
        .as_ref()
        .user_data()
        .get::<Mutex<OutputData>>()
        .expect("SCTK: wl_output has invalid UserData");
    let mut udata = udata_mutex.lock().unwrap();
    if let Some(xdg) = xdg_listener.as_ref().and_then(rc::Weak::upgrade) {
        xdg.borrow_mut().destroy_xdg_output(output);
    }
    let (id, mut callbacks) = match *udata {
        OutputData::PendingXDG { ref mut info, ref mut callbacks }
        | OutputData::Ready { ref mut info, ref mut callbacks } => {
            info.obsolete = true;
            notify(output, info, ddata.reborrow(), callbacks);
            notify_status_listeners(output, info, ddata, listeners);
            return;
        }
        OutputData::Pending { id, callbacks: ref mut cb, .. } => (id, std::mem::take(cb)),
    };
    let mut info = OutputInfo::new(id);
    info.obsolete = true;
    notify(output, &info, ddata.reborrow(), &mut callbacks);
    notify_status_listeners(output, &info, ddata, listeners);
    *udata = OutputData::Ready { info, callbacks };
}

fn merge_event(info: &mut OutputInfo, event: Event) {
    match event {
        Event::Geometry {
            x,
            y,
            physical_width,
            physical_height,
            subpixel,
            model,
            make,
            transform,
        } => {
            info.location = (x, y);
            info.physical_size = (physical_width, physical_height);
            info.subpixel = subpixel;
            info.transform = transform;
            info.model = model;
            info.make = make;
        }
        Event::Scale { factor } => {
            info.scale_factor = factor;
        }
        Event::Mode { width, height, refresh, flags } => {
            let mut found = false;
            if let Some(mode) = info
                .modes
                .iter_mut()
                .find(|m| m.dimensions == (width, height) && m.refresh_rate == refresh)
            {
                // this mode already exists, update it
                mode.is_preferred = flags.contains(wl_output::Mode::Preferred);
                mode.is_current = flags.contains(wl_output::Mode::Current);
                found = true;
            }
            if !found {
                // otherwise, add it
                info.modes.push(Mode {
                    dimensions: (width, height),
                    refresh_rate: refresh,
                    is_preferred: flags.contains(wl_output::Mode::Preferred),
                    is_current: flags.contains(wl_output::Mode::Current),
                })
            }
        }
        Event::Name { name } => {
            info.name = name;
        }
        Event::Description { description } => {
            info.description = description;
        }
        // ignore all other events
        _ => (),
    }
}

fn notify(
    output: &WlOutput,
    info: &OutputInfo,
    mut ddata: DispatchData,
    callbacks: &mut Vec<sync::Weak<OutputCallback>>,
) {
    callbacks.retain(|weak| {
        if let Some(arc) = sync::Weak::upgrade(weak) {
            (*arc)(output.clone(), info, ddata.reborrow());
            true
        } else {
            false
        }
    });
}

fn notify_status_listeners(
    output: &WlOutput,
    info: &OutputInfo,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>,
) {
    // Notify the callbacks listening for new outputs
    listeners.borrow_mut().retain(|lst| {
        if let Some(cb) = rc::Weak::upgrade(lst) {
            (cb.borrow_mut())(output.clone(), info, ddata.reborrow());
            true
        } else {
            false
        }
    })
}

/// Access the info associated with this output
///
/// The provided closure is given the [`OutputInfo`](struct.OutputInfo.html) as argument,
/// and its return value is returned from this function.
///
/// If the provided `WlOutput` has not yet been initialized or is not managed by SCTK, `None` is returned.
///
/// If the output has been removed by the compositor, the `obsolete` field of the `OutputInfo`
/// will be set to `true`. This handler will not automatically detroy the output by calling its
/// `release` method, to avoid interfering with your logic.
pub fn with_output_info<T, F: FnOnce(&OutputInfo) -> T>(output: &WlOutput, f: F) -> Option<T> {
    if let Some(udata_mutex) = output.as_ref().user_data().get::<Mutex<OutputData>>() {
        let udata = udata_mutex.lock().unwrap();
        match *udata {
            OutputData::PendingXDG { ref info, .. } | OutputData::Ready { ref info, .. } => {
                Some(f(info))
            }
            OutputData::Pending { .. } => None,
        }
    } else {
        None
    }
}

/// Add a listener to this output
///
/// The provided closure will be called whenever a property of the output changes,
/// including when it is removed by the compositor (in this case it'll be marked as
/// obsolete).
///
/// The returned [`OutputListener`](struct.OutputListener) keeps your callback alive,
/// dropping it will disable the callback and free the closure.
pub fn add_output_listener<F: Fn(WlOutput, &OutputInfo, DispatchData) + Send + Sync + 'static>(
    output: &WlOutput,
    f: F,
) -> OutputListener {
    let arc = Arc::new(f) as Arc<_>;

    if let Some(udata_mutex) = output.as_ref().user_data().get::<Mutex<OutputData>>() {
        let mut udata = udata_mutex.lock().unwrap();

        match *udata {
            OutputData::Pending { ref mut callbacks, .. }
            | OutputData::PendingXDG { ref mut callbacks, .. }
            | OutputData::Ready { ref mut callbacks, .. } => {
                callbacks.push(Arc::downgrade(&arc));
            }
        }
    }

    OutputListener { _cb: arc }
}

/// A handle to an output listener callback
///
/// Dropping it disables the associated callback and frees the closure.
pub struct OutputListener {
    _cb: Arc<dyn Fn(WlOutput, &OutputInfo, DispatchData) + Send + Sync + 'static>,
}

impl fmt::Debug for OutputListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputListener").field("_cb", &"fn() -> { ... }").finish()
    }
}

/// A handle to an output status callback
///
/// Dropping it disables the associated callback and frees the closure.
pub struct OutputStatusListener {
    _cb: Rc<RefCell<OutputStatusCallback>>,
}

impl fmt::Debug for OutputStatusListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputStatusListener").field("_cb", &"fn() -> { ... }").finish()
    }
}

/// Trait representing the OutputHandler functions
///
/// Implementing this trait on your inner environment struct used with the
/// [`environment!`](../macro.environment.html) by delegating it to its
/// [`OutputHandler`](struct.OutputHandler.html) field will make available the output-associated
/// method on your [`Environment`](../environment/struct.Environment.html).
pub trait OutputHandling {
    /// Insert a listener for output creation and removal events
    fn listen<F: FnMut(WlOutput, &OutputInfo, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> OutputStatusListener;
}

impl OutputHandling for OutputHandler {
    fn listen<F: FnMut(WlOutput, &OutputInfo, DispatchData) + 'static>(
        &mut self,
        f: F,
    ) -> OutputStatusListener {
        let rc = Rc::new(RefCell::new(f)) as Rc<_>;
        self.status_listeners.borrow_mut().push(Rc::downgrade(&rc));
        OutputStatusListener { _cb: rc }
    }
}

impl<E: OutputHandling> crate::environment::Environment<E> {
    /// Insert a new listener for outputs
    ///
    /// The provided closure will be invoked whenever a `wl_output` is created or removed.
    ///
    /// Note that if outputs already exist when this callback is setup, it'll not be invoked on them.
    /// For you to be notified of them as well, you need to first process them manually by calling
    /// `.get_all_outputs()`.
    ///
    /// The returned [`OutputStatusListener`](../output/struct.OutputStatusListener.hmtl) keeps your
    /// callback alive, dropping it will disable it.
    #[must_use = "the returned OutputStatusListener keeps your callback alive, dropping it will disable it"]
    pub fn listen_for_outputs<F: FnMut(WlOutput, &OutputInfo, DispatchData) + 'static>(
        &self,
        f: F,
    ) -> OutputStatusListener {
        self.with_inner(move |inner| OutputHandling::listen(inner, f))
    }
}

impl<E: crate::environment::MultiGlobalHandler<WlOutput>> crate::environment::Environment<E> {
    /// Shorthand method to retrieve the list of outputs
    pub fn get_all_outputs(&self) -> Vec<WlOutput> {
        self.get_all_globals::<WlOutput>().into_iter().map(|o| o.detach()).collect()
    }
}

/// A handler for `zxdg_output_manager_v1`
///
/// This handler adds additional information to the OutputInfo struct that is
/// available through the xdg_output interface.  Because this requires binding
/// the two handlers together when they are being created, it does not work with
/// [`new_default_environment!`](../macro.new_default_environment.html); you
/// must use [`default_environment!`](../macro.default_environment.html) and
/// create the [OutputHandler] outside the constructor.
///
/// ```no_compile
///  let (sctk_outputs, sctk_xdg_out) = smithay_client_toolkit::output::XdgOutputHandler::new_output_handlers();
///
///  let env = smithay_client_toolkit::environment::Environment::new(&wl_display, &mut wl_queue, Globals {
///      sctk_compositor: SimpleGlobal::new(),
///      sctk_shm: smithay_client_toolkit::shm::ShmHandler::new(),
///      sctk_seats : smithay_client_toolkit::seat::SeatHandler::new(),
///      sctk_shell : smithay_client_toolkit::shell::ShellHandler::new(),
///      sctk_outputs,
///      sctk_xdg_out,
///      // ...
///  })?;
///
/// ```
#[derive(Debug)]
pub struct XdgOutputHandler {
    inner: Rc<RefCell<XdgOutputHandlerInner>>,
}

#[derive(Debug)]
struct XdgOutputHandlerInner {
    xdg_manager: Option<Attached<ZxdgOutputManagerV1>>,
    outputs: Vec<(WlOutput, Attached<ZxdgOutputV1>)>,
}

impl XdgOutputHandler {
    /// Create a new instance of this handler bound to the given OutputHandler.
    pub fn new(output_handler: &mut OutputHandler) -> Self {
        let inner =
            Rc::new(RefCell::new(XdgOutputHandlerInner { xdg_manager: None, outputs: Vec::new() }));
        output_handler.xdg_listener = Some(Rc::downgrade(&inner));
        XdgOutputHandler { inner }
    }

    /// Helper function to create a bound pair of OutputHandler and XdgOutputHandler.
    pub fn new_output_handlers() -> (OutputHandler, Self) {
        let mut oh = OutputHandler::new();
        let xh = XdgOutputHandler::new(&mut oh);
        (oh, xh)
    }
}

impl XdgOutputHandlerInner {
    fn new_xdg_output(
        &mut self,
        output: &WlOutput,
        listeners: &Rc<RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>>,
    ) -> bool {
        if let Some(xdg_manager) = &self.xdg_manager {
            let xdg_main = xdg_manager.get_xdg_output(output);
            let wl_out = output.clone();
            let listeners = listeners.clone();
            xdg_main.quick_assign(move |_xdg_out, event, ddata| {
                process_xdg_event(&wl_out, event, ddata, &listeners)
            });
            self.outputs.push((output.clone(), xdg_main.into()));
            true
        } else {
            false
        }
    }
    fn destroy_xdg_output(&mut self, output: &WlOutput) {
        self.outputs.retain(|(out, xdg_out)| {
            if out.as_ref().is_alive() && out != output {
                true
            } else {
                xdg_out.destroy();
                false
            }
        });
    }
}

fn process_xdg_event(
    wl_out: &WlOutput,
    event: zxdg_output_v1::Event,
    mut ddata: DispatchData,
    listeners: &RefCell<Vec<rc::Weak<RefCell<OutputStatusCallback>>>>,
) {
    use zxdg_output_v1::Event;
    let udata_mutex = wl_out
        .as_ref()
        .user_data()
        .get::<Mutex<OutputData>>()
        .expect("SCTK: wl_output has invalid UserData");
    let mut udata = udata_mutex.lock().unwrap();
    let (info, callbacks, pending) = match &mut *udata {
        OutputData::Ready { info, callbacks } => (info, callbacks, false),
        OutputData::PendingXDG { info, callbacks } => (info, callbacks, true),
        OutputData::Pending { .. } => unreachable!(),
    };
    match event {
        Event::Name { name } => {
            info.name = name;
        }
        Event::Description { description } => {
            info.description = description;
        }
        Event::Done => {
            notify(wl_out, info, ddata.reborrow(), callbacks);
            if pending {
                notify_status_listeners(wl_out, info, ddata, listeners);
                let info = info.clone();
                let callbacks = std::mem::take(callbacks);
                *udata = OutputData::Ready { info, callbacks };
            }
        }
        _ => (),
    }
}

impl crate::environment::GlobalHandler<ZxdgOutputManagerV1> for XdgOutputHandler {
    fn created(
        &mut self,
        registry: Attached<wl_registry::WlRegistry>,
        id: u32,
        version: u32,
        _: DispatchData,
    ) {
        let version = std::cmp::min(version, 3);
        let mut inner = self.inner.borrow_mut();
        let xdg_manager: Main<ZxdgOutputManagerV1> = registry.bind(version, id);
        inner.xdg_manager = Some(xdg_manager.into());
    }
    fn get(&self) -> Option<Attached<ZxdgOutputManagerV1>> {
        let inner = self.inner.borrow();
        inner.xdg_manager.clone()
    }
}
