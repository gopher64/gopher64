use jni::objects::JString;
use jni::refs::Global;
use jni::{Env, JavaVM, bind_java_type};

use crate::ui;

pub static ANDROID_APP: std::sync::Mutex<Option<slint::android::AndroidApp>> =
    std::sync::Mutex::new(None);

bind_java_type! {
    AndroidActivity => "android.app.Activity",
    type_map = {
        AndroidIntent => "android.content.Intent",
    },
    methods {
        fn start_activity(intent: AndroidIntent) -> (),
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
        static FLAG_ACTIVITY_NEW_TASK: jint,
    },
    constructors {
        fn new(action: JString),
    },
    methods {
        fn set_data(uri: AndroidUri) -> AndroidIntent,
        fn add_flags(flags: jint) -> AndroidIntent,
    },
}

bind_java_type! {
    AndroidUri => "android.net.Uri",
    methods {
        static fn parse(uri_string: JString) -> AndroidUri,
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

/// Lists connected gamepads and joysticks using the Android framework.
pub fn list_controllers() -> Vec<ControllerInfo> {
    let vm = if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }
    } else {
        eprintln!("Android app not initialized");
        return Vec::new();
    };

    match vm.attach_current_thread(list_controllers_on_jvm) {
        Ok(controllers) => controllers,
        Err(err) => {
            eprintln!("JNI error while listing controllers: {err:?}");
            Vec::new()
        }
    }
}

fn list_controllers_on_jvm(env: &mut Env<'_>) -> jni::errors::Result<Vec<ControllerInfo>> {
    let source_gamepad = AndroidInputDevice::SOURCE_GAMEPAD(env)?;
    let source_joystick = AndroidInputDevice::SOURCE_JOYSTICK(env)?;

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

        if device.is_virtual(env)? {
            continue;
        }

        if !device.is_external(env)? {
            continue;
        }

        if !device.supports_source(env, source_gamepad)? {
            continue;
        }

        if !device.supports_source(env, source_joystick)? {
            continue;
        }

        if device.get_vendor_id(env)? == 0 {
            continue;
        }

        if device.get_product_id(env)? == 0 {
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
    let path = path.to_string();

    let vm = if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }
    } else {
        eprintln!("Android app not initialized");
        return;
    };
    if let Err(err) = vm.attach_current_thread(|env| open_uri_on_jvm(env, &path)) {
        eprintln!("JNI error while opening URI: {err:?}");
    }
}

fn open_uri_on_jvm(env: &mut Env<'_>, path: &str) -> jni::errors::Result<()> {
    if let Ok(app) = ANDROID_APP.lock()
        && let Some(app) = app.as_ref()
    {
        let raw_activity_global = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<Global<AndroidActivity>>(&raw_activity_global)? };

        let uri_string = JString::from_str(env, path.to_string())?;
        let uri = AndroidUri::parse(env, &uri_string)?;

        let action_view = AndroidIntent::ACTION_VIEW(env)?;
        let intent = AndroidIntent::new(env, &action_view)?.set_data(env, &uri)?;

        activity.as_ref().start_activity(env, &intent)?;
        Ok(())
    } else {
        Err(jni::errors::Error::UninitializedJavaVM)
    }
}

pub async fn select_rom(_rom_dir: slint::SharedString) -> Option<std::path::PathBuf> {
    None
}

pub async fn select_gb_rom(_player: i32) -> Option<std::path::PathBuf> {
    None
}

pub async fn select_gb_ram(_player: i32) -> Option<std::path::PathBuf> {
    None
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
