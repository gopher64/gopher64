use crate::Args;
use crate::create_runtime;
use crate::run;
use crate::ui;
use clap::Parser;
use jni::objects::{JClass, JObject, JObjectArray, JString};
use jni::refs::Global;
use jni::sys::{jfloat, jint};
use jni::{Env, EnvUnowned, JavaVM, bind_java_type};
use std::os::fd::FromRawFd;

const REQUEST_SELECT_ROM: jint = 1;
const RUN_ROM: jint = 3;

pub static ANDROID_APP: std::sync::Mutex<Option<slint::android::AndroidApp>> =
    std::sync::Mutex::new(None);

pub static SELECT_ROM_TX: tokio::sync::Mutex<
    Option<tokio::sync::oneshot::Sender<Option<std::path::PathBuf>>>,
> = tokio::sync::Mutex::const_new(None);

pub static WEAK_SLINT_WINDOW: std::sync::Mutex<Option<slint::Weak<ui::gui::AppWindow>>> =
    std::sync::Mutex::new(None);

bind_java_type! {
    DocumentsContract => "android.provider.DocumentsContract",
    fields {
        #[allow(non_snake_case)]
        static EXTRA_INITIAL_URI: JString,
    },
}

bind_java_type! {
    AndroidActivity => "android.app.Activity",
    type_map = {
        AndroidIntent => "android.content.Intent",
        AndroidContentResolver => "android.content.ContentResolver",
    },
    methods {
        fn start_activity(intent: AndroidIntent) -> (),
        fn start_activity_for_result(intent: AndroidIntent, request_code: jint) -> (),
        fn get_content_resolver() -> AndroidContentResolver,
    },
    fields {
        #[allow(non_snake_case)]
        static RESULT_OK: jint,
    },
}

bind_java_type! {
    AndroidIntent => "android.content.Intent",
    type_map = {
        AndroidUri => "android.net.Uri",
    },
    fields {
        #[allow(non_snake_case)]
        static ACTION_VIEW: JString,
        #[allow(non_snake_case)]
        static ACTION_OPEN_DOCUMENT: JString,
        #[allow(non_snake_case)]
        static CATEGORY_OPENABLE: JString,
        #[allow(non_snake_case)]
        static FLAG_ACTIVITY_NEW_TASK: jint,
        #[allow(non_snake_case)]
        static FLAG_GRANT_READ_URI_PERMISSION: jint,
    },
    constructors {
        fn new(),
        fn with_action(action: JString),
    },
    methods {
        fn set_data(uri: AndroidUri) -> AndroidIntent,
        fn set_type(r#type: JString) -> AndroidIntent,
        fn add_category(category: JString) -> AndroidIntent,
        fn get_data() -> AndroidUri,
        fn put_extra_string {
            sig = (extra: JString, value: JString) -> AndroidIntent,
            name = "putExtra",
        },
        fn put_extra_string_array {
            sig = (extra: JString, value: JString[]) -> AndroidIntent,
            name = "putExtra",
        },
        fn get_string_extra(name: JString) -> JString,
        fn set_class_name(package_name: JString, class_name: JString) -> AndroidIntent,
    },
}

bind_java_type! {
    ParcelFileDescriptor => "android.os.ParcelFileDescriptor",
    methods {
        fn close() -> (),
        fn detach_fd() -> jint,
    },
}

bind_java_type! {
    AndroidContentResolver => "android.content.ContentResolver",
    type_map = {
        AndroidUri => "android.net.Uri",
        ParcelFileDescriptor => "android.os.ParcelFileDescriptor",
    },
    methods {
        fn take_persistable_uri_permission(uri: AndroidUri, flags: jint) -> (),
        fn open_file_descriptor(uri: AndroidUri, mode: JString) -> ParcelFileDescriptor,
    },
}

bind_java_type! {
    AndroidUri => "android.net.Uri",
    methods {
        static fn parse(uri_string: JString) -> AndroidUri,
        fn to_string() -> JString,
        static fn decode(path: JString) -> JString,
    },
}

bind_java_type! {
    pub AndroidInputDevice => "android.view.InputDevice",
    fields {
        #[allow(non_snake_case)]
        static SOURCE_JOYSTICK: jint,
        #[allow(non_snake_case)]
        static SOURCE_GAMEPAD: jint,
    },
    methods {
        static fn get_device_ids() -> jint[],
        static fn get_device(device_id: jint) -> AndroidInputDevice,
        fn supports_source(source: jint) -> jboolean,
        fn is_virtual() -> jboolean,
        fn is_external() -> jboolean,
        fn get_vendor_id() -> jint,
        fn get_product_id() -> jint,
        fn get_name() -> JString,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerInfo {
    pub name: String,
    /// Stable ID from [`InputDevice.getDescriptor`](https://developer.android.com/reference/android/view/InputDevice#getDescriptor()).
    pub descriptor: String,
}

fn argv_to_strings(argc: std::ffi::c_int, argv: *mut *mut std::ffi::c_char) -> Vec<String> {
    if argc <= 0 || argv.is_null() {
        return Vec::new();
    }
    unsafe {
        (0..argc as usize)
            .map(|i| {
                std::ffi::CStr::from_ptr(*argv.add(i))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gopher64_sdl_main(
    argc: std::ffi::c_int,
    argv: *mut *mut std::ffi::c_char,
) -> std::ffi::c_int {
    let (close_tx, handle) = create_runtime();
    let _guard = handle.enter();

    let raw = argv_to_strings(argc, argv);
    let args = Args::try_parse_from(raw).unwrap();
    if let Err(err) = run(args, argc as usize) {
        close_tx.send(()).unwrap();
        eprintln!("Error running game: {err:?}");
        return 1;
    }
    close_tx.send(()).unwrap();
    0
}

bind_java_type! {
    SlintActivity => "io.github.gopher64.gopher64.SlintActivity",
    methods {
        fn set_capture_active(active: bool) -> (),
    },
}

/// Toggle `SlintActivity`'s input-capture forwarding (dispatch overrides +
/// focused overlay). Called by the wizard on the Slint event-loop thread
/// when a capture session opens/closes.
pub fn set_capture_active(active: bool) {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        if let Err(err) = get_vm(app).attach_current_thread(|env| {
            let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
            let activity =
                unsafe { env.as_cast_raw::<Global<SlintActivity>>(&raw_activity_global)? };
            activity.as_ref().set_capture_active(env, active)
        }) {
            eprintln!("JNI error while toggling input capture: {err:?}");
        }
    }
}

/// Input forwarded by `SlintActivity.nativeOnCaptureInput` while capture is
/// active. Runs on an Android UI/binder thread — hop to the Slint event loop
/// and feed the wizard. `type` discriminates: 0 = key (ACTION_DOWN, repeat 0;
/// `code` = Android keycode, `value` unused), 1 = motion axis (`code` =
/// `MotionEvent.AXIS_*`, `value` normalized to -1..=1 exactly like
/// `SDLControllerManager`, `action` = resting-position sign -1/0/+1).
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_gopher64_gopher64_SlintActivity_nativeOnCaptureInput<
    'caller,
>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    _device_id: jint,
    source: jint,
    r#type: jint,
    code: jint,
    value: jfloat,
    action: jint,
) {
    let ev = match r#type {
        0 => ui::input_capture::AndroidEvent::Key {
            source,
            keycode: code,
        },
        1 => ui::input_capture::AndroidEvent::Axis {
            axis: code,
            value,
            rest: action.clamp(-1, 1) as i8,
        },
        _ => return,
    };
    if let Err(err) =
        slint::invoke_from_event_loop(move || ui::gui::wizard::apply_android_input(ev))
    {
        eprintln!("Dropping capture input (event loop not running): {err:?}");
    }
}

pub fn run_rom(
    file_path: std::path::PathBuf,
    game_settings: ui::GameSettings,
    netplay: Option<ui::gui::NetplayDevice>,
    weak: slint::Weak<ui::gui::AppWindow>,
) {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        if let Err(err) = get_vm(app).attach_current_thread(|env| {
            start_run_rom_on_jvm(env, app, file_path, game_settings, netplay, weak)
        }) {
            eprintln!("JNI error while starting N64Activity: {err:?}");
        }
    }
}

fn start_run_rom_on_jvm(
    env: &mut Env<'_>,
    app: &slint::android::AndroidApp,
    file_path: std::path::PathBuf,
    game_settings: ui::GameSettings,
    netplay: Option<ui::gui::NetplayDevice>,
    weak: slint::Weak<ui::gui::AppWindow>,
) -> jni::errors::Result<()> {
    let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
    let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };

    let package_name = JString::from_str(env, "io.github.gopher64.gopher64")?;
    let class_name = JString::from_str(env, "io.github.gopher64.gopher64.N64Activity")?;

    let file_path_key = JString::from_str(env, "file_path")?;
    let file_path = file_path.to_str().unwrap();
    let cheats_path_key = JString::from_str(env, "cheats_path")?;
    let cheats_path = app
        .internal_data_path()
        .unwrap()
        .join("cache")
        .join("cheats.json");

    let args_key = JString::from_str(env, "args")?;
    let mut args = vec![
        JString::from_str(env, file_path)?,
        JString::from_str(env, "--fullscreen")?,
        JString::from_str(env, "--overclock")?,
        JString::from_str(env, &game_settings.overclock.to_string())?,
        JString::from_str(env, "--disable-expansion-pak")?,
        JString::from_str(env, &game_settings.disable_expansion_pak.to_string())?,
    ];

    if let Some(netplay) = netplay {
        args.push(JString::from_str(env, "--netplay-server-addr")?);
        args.push(JString::from_str(env, &netplay.server_addr)?);
        args.push(JString::from_str(env, "--netplay-player-number")?);
        args.push(JString::from_str(env, &netplay.player_number.to_string())?);
        args.push(JString::from_str(env, "--netplay-number-of-players")?);
        args.push(JString::from_str(
            env,
            &netplay.number_of_players.to_string(),
        )?);
        args.push(JString::from_str(env, "--netplay-input-delay")?);
        args.push(JString::from_str(env, &netplay.input_delay.to_string())?);
        args.push(JString::from_str(env, "--cheats")?);
        args.push(JString::from_str(env, cheats_path.to_str().unwrap())?);

        let f = std::fs::File::create(&cheats_path).unwrap();
        serde_json::to_writer_pretty(f, &game_settings.cheats).unwrap();
    }

    let empty_arg = JString::from_str(env, "")?;
    let j_args = JObjectArray::<JString>::new(env, args.len(), empty_arg)?;
    for (i, arg) in args.iter().enumerate() {
        j_args.set_element(env, i, arg)?;
    }

    let file_path_string = JString::from_str(env, file_path)?;
    let cheats_path_string = JString::from_str(env, cheats_path.to_str().unwrap())?;
    let intent = AndroidIntent::new(env)?
        .set_class_name(env, &package_name, &class_name)?
        .put_extra_string(env, &file_path_key, &file_path_string)?
        .put_extra_string(env, &cheats_path_key, &cheats_path_string)?
        .put_extra_string_array(env, &args_key, &j_args)?;

    weak.upgrade_in_event_loop(move |handle| handle.set_game_running(true))
        .unwrap();

    activity
        .as_ref()
        .start_activity_for_result(env, &intent, RUN_ROM)?;
    Ok(())
}

fn get_vm(app: &slint::android::AndroidApp) -> JavaVM {
    unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }
}

pub fn decode_path(path: &str) -> String {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        match get_vm(app).attach_current_thread(|env| decode_path_on_jvm(env, path)) {
            Ok(decoded_path) => decoded_path,
            Err(err) => {
                eprintln!("JNI error while decoding path: {err:?}");
                String::new()
            }
        }
    } else {
        eprintln!("Android app not initialized");
        String::new()
    }
}

fn decode_path_on_jvm(env: &mut Env<'_>, path: &str) -> jni::errors::Result<String> {
    let path = JString::from_str(env, path)?;
    let decoded_path = AndroidUri::decode(env, &path)?;
    Ok(decoded_path.try_to_string(env)?)
}

/// Lists connected gamepads and joysticks using the Android framework.
pub fn list_controllers() -> Vec<ControllerInfo> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        match get_vm(app).attach_current_thread(list_controllers_on_jvm) {
            Ok(controllers) => controllers,
            Err(err) => {
                eprintln!("JNI error while listing controllers: {err:?}");
                Vec::new()
            }
        }
    } else {
        eprintln!("Android app not initialized");
        Vec::new()
    }
}

fn list_controllers_on_jvm(env: &mut Env<'_>) -> jni::errors::Result<Vec<ControllerInfo>> {
    let device_ids = AndroidInputDevice::get_device_ids(env)?;
    let count = device_ids.len(env)?;
    let mut ids = vec![0i32; count];
    if count > 0 {
        device_ids.get_region(env, 0, &mut ids)?;
    }

    let mut controllers = Vec::new();
    for device_id in ids {
        let device = AndroidInputDevice::get_device(env, device_id)?;
        if device.is_null() {
            continue;
        }

        let supports_gamepad =
            device.supports_source(env, AndroidInputDevice::SOURCE_GAMEPAD(env)?)?;
        let supports_joystick =
            device.supports_source(env, AndroidInputDevice::SOURCE_JOYSTICK(env)?)?;
        if !supports_gamepad && !supports_joystick {
            continue;
        }

        let name = if let Ok(name) = device.get_name(env)
            && let Ok(name) = name.try_to_string(env)
        {
            name
        } else {
            ui::input::UNKNOWN_CONTROLLER_NAME.to_string()
        };
        let descriptor = if let Ok(product_id) = device.get_product_id(env)
            && let Ok(vendor_id) = device.get_vendor_id(env)
        {
            format!("{}:{}:{}", name, vendor_id, product_id)
        } else {
            String::new()
        };

        controllers.push(ControllerInfo { name, descriptor });
    }

    Ok(controllers)
}

/// Opens a URI in the user's default app via [`Intent::ACTION_VIEW`](https://developer.android.com/reference/android/content/Intent#ACTION_VIEW).
pub fn open_uri(path: &str) {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        let path = path.to_string();
        if let Err(err) = get_vm(app).attach_current_thread(|env| open_uri_on_jvm(env, app, &path))
        {
            eprintln!("JNI error while opening URI: {err:?}");
        }
    }
}

fn open_uri_on_jvm(
    env: &mut Env<'_>,
    app: &slint::android::AndroidApp,
    path: &str,
) -> jni::errors::Result<()> {
    let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
    let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };

    let uri_string = JString::from_str(env, path)?;
    let uri = AndroidUri::parse(env, &uri_string)?;

    let action_view = AndroidIntent::ACTION_VIEW(env)?;
    let intent = AndroidIntent::with_action(env, &action_view)?.set_data(env, &uri)?;

    activity.as_ref().start_activity(env, &intent)?;
    Ok(())
}

pub async fn select_rom(rom_dir: slint::SharedString) -> Option<std::path::PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<std::path::PathBuf>>();
    SELECT_ROM_TX.lock().await.replace(tx);

    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        if let Err(err) = get_vm(app)
            .attach_current_thread(|env| select_rom_on_jvm(env, app, rom_dir.to_string()))
        {
            eprintln!("JNI error while opening URI: {err:?}");
            return None;
        }
    } else {
        eprintln!("Android app not initialized");
        return None;
    }

    rx.await.unwrap_or(None)
}

pub async fn select_gb_rom(_player: i32) -> Option<std::path::PathBuf> {
    select_rom(slint::SharedString::new()).await
}

pub async fn select_gb_ram(_player: i32) -> Option<std::path::PathBuf> {
    select_rom(slint::SharedString::new()).await
}

fn select_rom_on_jvm(
    env: &mut Env<'_>,
    app: &slint::android::AndroidApp,
    rom_dir: String,
) -> jni::errors::Result<()> {
    let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
    let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };

    let action = AndroidIntent::ACTION_OPEN_DOCUMENT(env)?;
    let category = AndroidIntent::CATEGORY_OPENABLE(env)?;
    let mime_type = JString::from_str(env, "*/*")?;
    let mut intent = AndroidIntent::with_action(env, &action)?
        .set_type(env, &mime_type)?
        .add_category(env, &category)?;
    if !rom_dir.is_empty() {
        let start_dir = JString::from_str(env, rom_dir)?;
        let extra_initial_uri = DocumentsContract::EXTRA_INITIAL_URI(env)?;
        intent = intent.put_extra_string(env, &extra_initial_uri, &start_dir)?;
    }

    activity.start_activity_for_result(env, &intent, REQUEST_SELECT_ROM)?;
    Ok(())
}

pub fn get_dirs() -> ui::Dirs {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        ui::Dirs {
            config_dir: app.internal_data_path().unwrap().join("config"),
            data_dir: app.external_data_path().unwrap().join("data"),
            cache_dir: app.internal_data_path().unwrap().join("cache"),
        }
    } else {
        panic!("Android app not initialized");
    }
}

pub fn get_controller_names() -> Vec<String> {
    let mut controllers: Vec<String> = list_controllers().into_iter().map(|c| c.name).collect();
    controllers.insert(0, "None".into());
    controllers
}

pub fn get_controller_paths() -> Vec<String> {
    let mut controller_paths: Vec<String> = list_controllers()
        .into_iter()
        .map(|c| c.descriptor)
        .collect();
    controller_paths.insert(0, String::new());
    controller_paths
}

pub fn rom_exists(path: &str) -> bool {
    get_file_from_uri(&std::path::PathBuf::from(path)).is_some()
}

pub fn get_file_from_uri(path: &std::path::PathBuf) -> Option<std::fs::File> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        let path = path.to_str().unwrap().into();
        match get_vm(app).attach_current_thread(|env| get_file_from_uri_on_jvm(env, app, path)) {
            Ok(file) => file,
            Err(err) => {
                eprintln!("JNI error while opening URI: {err:?}");
                return None;
            }
        }
    } else {
        eprintln!("Android app not initialized");
        None
    }
}

fn get_file_from_uri_on_jvm(
    env: &mut Env<'_>,
    app: &slint::android::AndroidApp,
    path: String,
) -> jni::errors::Result<Option<std::fs::File>> {
    let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
    let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };
    let path = JString::from_str(env, path)?;
    let mode = JString::from_str(env, "r")?;
    let uri = AndroidUri::parse(env, &path)?;

    let content_resolver = activity.as_ref().get_content_resolver(env)?;
    let parcel_file_descriptor = content_resolver.open_file_descriptor(env, &uri, &mode);
    if let Ok(descriptor) = parcel_file_descriptor
        && !descriptor.is_null()
    {
        let owned_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(descriptor.detach_fd(env)?) };
        let file = std::fs::File::from(owned_fd);
        descriptor.close(env)?;
        return Ok(Some(file));
    } else {
        return Ok(None);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_gopher64_gopher64_SlintActivity_nativeOnActivityResult<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    request_code: jint,
    result_code: jint,
    intent_data: JObject<'caller>,
) {
    let outcome = unowned_env.with_env(|env| -> Result<_, jni::errors::Error> {
        if result_code == AndroidActivity::RESULT_OK(env)? {
            match request_code {
                REQUEST_SELECT_ROM => {
                    if let Some(tx) = SELECT_ROM_TX.blocking_lock().take()
                        && !intent_data.is_null()
                    {
                        let result_intent =
                            unsafe { env.as_cast_raw::<AndroidIntent>(&intent_data)? };

                        let uri = result_intent.as_ref().get_data(env)?;
                        if uri.is_null() {
                            return Ok(());
                        }

                        if let Ok(app) = ANDROID_APP.lock()
                            && let Some(app) = app.as_ref()
                        {
                            let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
                            let activity = unsafe {
                                env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)?
                            };

                            let content_resolver = activity.as_ref().get_content_resolver(env)?;
                            let take_flags = AndroidIntent::FLAG_GRANT_READ_URI_PERMISSION(env)?;
                            content_resolver
                                .take_persistable_uri_permission(env, &uri, take_flags)?;

                            let path = uri.to_string(env)?;

                            let _ = tx.send(Some(std::path::PathBuf::from(path.to_string())));
                        } else {
                            eprintln!("Android app not initialized");
                            return Ok(());
                        }
                    }
                }
                RUN_ROM => {
                    let result_intent = unsafe { env.as_cast_raw::<AndroidIntent>(&intent_data)? };

                    let file_path_key = JString::from_str(env, "file_path")?;
                    let file_path = result_intent
                        .as_ref()
                        .get_string_extra(env, &file_path_key)?
                        .try_to_string(env)?;

                    let cheats_path_key = JString::from_str(env, "cheats_path")?;
                    if let Ok(cheats_path) = result_intent
                        .as_ref()
                        .get_string_extra(env, &cheats_path_key)
                        && let Ok(cheats_path) = cheats_path.try_to_string(env)
                    {
                        let _ = std::fs::remove_file(cheats_path);
                    }
                    if let Ok(weak_app_window) = WEAK_SLINT_WINDOW.lock()
                        && let Some(weak_app_window) = weak_app_window.as_ref()
                    {
                        weak_app_window
                            .upgrade_in_event_loop(move |handle| {
                                ui::gui::update_recent_roms(&handle, file_path.into());
                                handle.set_game_running(false)
                            })
                            .unwrap();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
