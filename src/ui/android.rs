use crate::ui;
use jni::objects::{JClass, JObject, JString};
use jni::refs::Global;
use jni::sys::jint;
use jni::{Env, EnvUnowned, JavaVM, bind_java_type};
use std::os::fd::FromRawFd;

const REQUEST_SELECT_ROM: jint = 1;
const CONFIGURE_INPUT_PROFILE: jint = 2;

pub static ANDROID_APP: std::sync::Mutex<Option<slint::android::AndroidApp>> =
    std::sync::Mutex::new(None);

pub static SELECT_ROM_TX: std::sync::Mutex<
    Option<tokio::sync::oneshot::Sender<Option<std::path::PathBuf>>>,
> = std::sync::Mutex::new(None);

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
        fn put_extra_boolean {
            sig = (extra: JString, value: jboolean) -> AndroidIntent,
            name = "putExtra",
        },
        fn put_extra_int {
            sig = (extra: JString, value: jint) -> AndroidIntent,
            name = "putExtra",
        },
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
        fn get_descriptor() -> JString,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerInfo {
    pub name: String,
    /// Stable ID from [`InputDevice.getDescriptor`](https://developer.android.com/reference/android/view/InputDevice#getDescriptor()).
    pub descriptor: String,
}

pub fn configure_input_profile(profile_name: slint::SharedString, dinput: bool, deadzone: i32) {
    if let Some(vm) = get_vm() {
        if let Err(err) = vm.attach_current_thread(|env| {
            start_n64_activity_on_jvm(env, profile_name.to_string(), dinput, deadzone)
        }) {
            eprintln!("JNI error while starting N64Activity: {err:?}");
        }
    }
}

fn start_n64_activity_on_jvm(
    env: &mut Env<'_>,
    profile_name: String,
    dinput: bool,
    deadzone: i32,
) -> jni::errors::Result<()> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };

        let package_name = JString::from_str(env, "io.github.gopher64.gopher64")?;
        let class_name = JString::from_str(env, "N64Activity")?;

        let profile_name_key = JString::from_str(env, "profile_name")?;
        let profile_name_value = JString::from_str(env, profile_name)?;
        let dinput_key = JString::from_str(env, "dinput")?;
        let deadzone_key = JString::from_str(env, "deadzone")?;
        let mode_key = JString::from_str(env, "mode")?;
        let intent = AndroidIntent::new(env)?
            .set_class_name(env, &package_name, &class_name)?
            .put_extra_string(env, &profile_name_key, &profile_name_value)?
            .put_extra_boolean(env, &dinput_key, dinput)?
            .put_extra_int(env, &deadzone_key, deadzone)?
            .put_extra_int(env, &mode_key, 1 /* configure input profile */)?;

        activity
            .as_ref()
            .start_activity_for_result(env, &intent, CONFIGURE_INPUT_PROFILE)?;
        Ok(())
    } else {
        Err(jni::errors::Error::UninitializedJavaVM)
    }
}

pub fn run_rom(
    _file_path: std::path::PathBuf,
    _game_settings: ui::GameSettings,
    netplay: Option<ui::gui::NetplayDevice>,
) {
    if let Some(netplay) = netplay {
        println!(
            "Netplay peer addr: {} player number: {}",
            netplay.peer_addr, netplay.player_number
        );
    }
}

fn get_vm() -> Option<JavaVM> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        Some(unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) })
    } else {
        None
    }
}

/// Lists connected gamepads and joysticks using the Android framework.
pub fn list_controllers() -> Vec<ControllerInfo> {
    if let Some(vm) = get_vm() {
        match vm.attach_current_thread(list_controllers_on_jvm) {
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
            "Unknown controller".to_string()
        };
        let descriptor = if let Ok(descriptor) = device.get_descriptor(env)
            && let Ok(descriptor) = descriptor.try_to_string(env)
        {
            descriptor
        } else {
            String::new()
        };

        controllers.push(ControllerInfo { name, descriptor });
    }

    Ok(controllers)
}

/// Opens a URI in the user's default app via [`Intent::ACTION_VIEW`](https://developer.android.com/reference/android/content/Intent#ACTION_VIEW).
pub fn open_uri(path: &str) {
    if let Some(vm) = get_vm() {
        let path = path.to_string();
        if let Err(err) = vm.attach_current_thread(|env| open_uri_on_jvm(env, &path)) {
            eprintln!("JNI error while opening URI: {err:?}");
        }
    }
}

fn open_uri_on_jvm(env: &mut Env<'_>, path: &str) -> jni::errors::Result<()> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };

        let uri_string = JString::from_str(env, path)?;
        let uri = AndroidUri::parse(env, &uri_string)?;

        let action_view = AndroidIntent::ACTION_VIEW(env)?;
        let intent = AndroidIntent::with_action(env, &action_view)?.set_data(env, &uri)?;

        activity.as_ref().start_activity(env, &intent)?;
        Ok(())
    } else {
        Err(jni::errors::Error::UninitializedJavaVM)
    }
}

pub async fn select_rom(rom_dir: slint::SharedString) -> Option<std::path::PathBuf> {
    if let Some(vm) = get_vm() {
        if let Err(err) =
            vm.attach_current_thread(|env| select_rom_on_jvm(env, rom_dir.to_string()))
        {
            eprintln!("JNI error while opening URI: {err:?}");
            return None;
        }
        let (tx, rx) = tokio::sync::oneshot::channel::<Option<std::path::PathBuf>>();
        if let Ok(mut tx_lock) = SELECT_ROM_TX.lock() {
            tx_lock.replace(tx);
        } else {
            eprintln!("Error locking SELECT_ROM_TX");
            return None;
        }
        rx.await.unwrap_or(None)
    } else {
        eprintln!("Android app not initialized");
        None
    }
}

pub async fn select_gb_rom(_player: i32) -> Option<std::path::PathBuf> {
    select_rom(slint::SharedString::new()).await
}

pub async fn select_gb_ram(_player: i32) -> Option<std::path::PathBuf> {
    select_rom(slint::SharedString::new()).await
}

fn select_rom_on_jvm(env: &mut Env<'_>, rom_dir: String) -> jni::errors::Result<()> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
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
    } else {
        Err(jni::errors::Error::UninitializedJavaVM)
    }
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
    if let Some(vm) = get_vm() {
        let path = path.to_str().unwrap().into();
        match vm.attach_current_thread(|env| get_file_from_uri_on_jvm(env, path)) {
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
    path: String,
) -> jni::errors::Result<Option<std::fs::File>> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
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
    } else {
        eprintln!("Android app not initialized");
        return Err(jni::errors::Error::UninitializedJavaVM);
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
        if result_code == AndroidActivity::RESULT_OK(env)? && !intent_data.is_null() {
            match request_code {
                REQUEST_SELECT_ROM => {
                    if let Ok(mut tx_lock) = SELECT_ROM_TX.lock()
                        && let Some(tx) = tx_lock.take()
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
                CONFIGURE_INPUT_PROFILE => {
                    if let Ok(weak_app_window) = WEAK_SLINT_WINDOW.lock()
                        && let Some(weak_app_window) = weak_app_window.as_ref()
                    {
                        let config = ui::config::Config::new();
                        ui::gui::update_input_profiles(&weak_app_window, &config);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    });
    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
