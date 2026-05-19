use jni::{Env, JavaVM, bind_java_type};

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
    let Some(app) = crate::ui::ANDROID_APP.get() else {
        eprintln!("Android app not initialized; cannot list controllers");
        return Vec::new();
    };

    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };

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

        if !device.supports_source(env, source_gamepad & source_joystick)? {
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

pub fn open_uri(_path: &str) {}
