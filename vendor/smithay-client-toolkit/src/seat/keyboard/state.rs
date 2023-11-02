use memmap2::MmapOptions;
use std::{env, ffi::CString, fs::File, os::raw::c_char, os::unix::ffi::OsStringExt, ptr};

#[cfg(feature = "dlopen")]
use super::ffi::XKBCOMMON_HANDLE as XKBH;
#[cfg(not(feature = "dlopen"))]
use super::ffi::*;
use super::ffi::{self, xkb_state_component};
use super::Error;

#[derive(Debug)]
pub(crate) struct KbState {
    xkb_context: *mut ffi::xkb_context,
    xkb_keymap: *mut ffi::xkb_keymap,
    xkb_state: *mut ffi::xkb_state,
    xkb_compose_table: *mut ffi::xkb_compose_table,
    xkb_compose_state: *mut ffi::xkb_compose_state,
    mods_state: ModifiersState,
    locked: bool,
}

/// The RMLVO description of a keymap
///
/// All fields are optional, and the system default
/// will be used if set to `None`.
#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct RMLVO {
    /// The rules file to use
    pub rules: Option<String>,
    /// The keyboard model by which to interpret keycodes and LEDs
    pub model: Option<String>,
    /// A comma separated list of layouts (languages) to include in the keymap
    pub layout: Option<String>,
    /// A comma separated list of variants, one per layout, which may modify or
    /// augment the respective layout in various ways
    pub variant: Option<String>,
    /// A comma separated list of options, through which the user specifies
    /// non-layout related preferences, like which key combinations are
    /// used for switching layouts, or which key is the Compose key.
    pub options: Option<String>,
}

/// Represents the current state of the keyboard modifiers
///
/// Each field of this struct represents a modifier and is `true` if this modifier is active.
///
/// For some modifiers, this means that the key is currently pressed, others are toggled
/// (like caps lock).
#[derive(Copy, Clone, Debug, Default)]
pub struct ModifiersState {
    /// The "control" key
    pub ctrl: bool,
    /// The "alt" key
    pub alt: bool,
    /// The "shift" key
    pub shift: bool,
    /// The "Caps lock" key
    pub caps_lock: bool,
    /// The "logo" key
    ///
    /// Also known as the "windows" key on most keyboards
    pub logo: bool,
    /// The "Num lock" key
    pub num_lock: bool,
}

impl ModifiersState {
    fn new() -> ModifiersState {
        ModifiersState::default()
    }

    fn update_with(&mut self, state: *mut ffi::xkb_state) {
        self.ctrl = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_mod_name_is_active,
                state,
                ffi::XKB_MOD_NAME_CTRL.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE
            ) > 0
        };
        self.alt = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_mod_name_is_active,
                state,
                ffi::XKB_MOD_NAME_ALT.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE
            ) > 0
        };
        self.shift = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_mod_name_is_active,
                state,
                ffi::XKB_MOD_NAME_SHIFT.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE
            ) > 0
        };
        self.caps_lock = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_mod_name_is_active,
                state,
                ffi::XKB_MOD_NAME_CAPS.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE
            ) > 0
        };
        self.logo = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_mod_name_is_active,
                state,
                ffi::XKB_MOD_NAME_LOGO.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE
            ) > 0
        };
        self.num_lock = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_mod_name_is_active,
                state,
                ffi::XKB_MOD_NAME_NUM.as_ptr() as *const c_char,
                xkb_state_component::XKB_STATE_MODS_EFFECTIVE
            ) > 0
        };
    }
}

impl KbState {
    pub(crate) fn update_modifiers(
        &mut self,
        mods_depressed: u32,
        mods_latched: u32,
        mods_locked: u32,
        group: u32,
    ) {
        if !self.ready() {
            return;
        }
        let mask = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_update_mask,
                self.xkb_state,
                mods_depressed,
                mods_latched,
                mods_locked,
                0,
                0,
                group
            )
        };
        if mask.contains(xkb_state_component::XKB_STATE_MODS_EFFECTIVE) {
            // effective value of mods have changed, we need to update our state
            self.mods_state.update_with(self.xkb_state);
        }
    }

    pub(crate) fn get_one_sym_raw(&mut self, keycode: u32) -> u32 {
        if !self.ready() {
            return 0;
        }
        unsafe { ffi_dispatch!(XKBH, xkb_state_key_get_one_sym, self.xkb_state, keycode + 8) }
    }

    pub(crate) fn get_utf8_raw(&mut self, keycode: u32) -> Option<String> {
        if !self.ready() {
            return None;
        }
        let size = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_key_get_utf8,
                self.xkb_state,
                keycode + 8,
                ptr::null_mut(),
                0
            )
        } + 1;
        if size <= 1 {
            return None;
        };
        let mut buffer = vec![0; size as usize];
        unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_state_key_get_utf8,
                self.xkb_state,
                keycode + 8,
                buffer.as_mut_ptr() as *mut _,
                size as usize
            );
        };
        // remove the final `\0`
        buffer.pop();
        // libxkbcommon will always provide valid UTF8
        Some(unsafe { String::from_utf8_unchecked(buffer) })
    }

    pub(crate) fn compose_feed(&mut self, keysym: u32) -> Option<ffi::xkb_compose_feed_result> {
        if !self.ready() || self.xkb_compose_state.is_null() {
            return None;
        }
        Some(unsafe { ffi_dispatch!(XKBH, xkb_compose_state_feed, self.xkb_compose_state, keysym) })
    }

    pub(crate) fn compose_status(&mut self) -> Option<ffi::xkb_compose_status> {
        if !self.ready() || self.xkb_compose_state.is_null() {
            return None;
        }
        Some(unsafe { ffi_dispatch!(XKBH, xkb_compose_state_get_status, self.xkb_compose_state) })
    }

    pub(crate) fn compose_get_utf8(&mut self) -> Option<String> {
        if !self.ready() || self.xkb_compose_state.is_null() {
            return None;
        }
        let size = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_compose_state_get_utf8,
                self.xkb_compose_state,
                ptr::null_mut(),
                0
            )
        } + 1;
        if size <= 1 {
            return None;
        };
        let mut buffer = vec![0; size as usize];
        unsafe {
            buffer.set_len(size as usize);
            ffi_dispatch!(
                XKBH,
                xkb_compose_state_get_utf8,
                self.xkb_compose_state,
                buffer.as_mut_ptr() as *mut _,
                size as usize
            );
        };
        // remove the final `\0`
        buffer.pop();
        // libxkbcommon will always provide valid UTF8
        Some(unsafe { String::from_utf8_unchecked(buffer) })
    }

    pub(crate) fn new() -> Result<KbState, Error> {
        #[cfg(feature = "dlopen")]
        {
            if ffi::XKBCOMMON_OPTION.as_ref().is_none() {
                return Err(Error::XKBNotFound);
            }
        }
        let context = unsafe {
            ffi_dispatch!(XKBH, xkb_context_new, ffi::xkb_context_flags::XKB_CONTEXT_NO_FLAGS)
        };
        if context.is_null() {
            return Err(Error::XKBNotFound);
        }

        let mut me = KbState {
            xkb_context: context,
            xkb_keymap: ptr::null_mut(),
            xkb_state: ptr::null_mut(),
            xkb_compose_table: ptr::null_mut(),
            xkb_compose_state: ptr::null_mut(),
            mods_state: ModifiersState::new(),
            locked: false,
        };

        unsafe {
            me.init_compose();
        }

        Ok(me)
    }

    pub(crate) fn from_rmlvo(rmlvo: RMLVO) -> Result<KbState, Error> {
        fn to_cstring(s: Option<String>) -> Result<Option<CString>, Error> {
            s.map_or(Ok(None), |s| CString::new(s).map(Option::Some)).map_err(|_| Error::BadNames)
        }

        let mut state = KbState::new()?;

        let rules = to_cstring(rmlvo.rules)?;
        let model = to_cstring(rmlvo.model)?;
        let layout = to_cstring(rmlvo.layout)?;
        let variant = to_cstring(rmlvo.variant)?;
        let options = to_cstring(rmlvo.options)?;

        let xkb_names = ffi::xkb_rule_names {
            rules: rules.map_or(ptr::null(), |s| s.as_ptr()),
            model: model.map_or(ptr::null(), |s| s.as_ptr()),
            layout: layout.map_or(ptr::null(), |s| s.as_ptr()),
            variant: variant.map_or(ptr::null(), |s| s.as_ptr()),
            options: options.map_or(ptr::null(), |s| s.as_ptr()),
        };

        unsafe {
            state.init_with_rmlvo(xkb_names)?;
        }

        state.locked = true;
        Ok(state)
    }

    pub(crate) unsafe fn init_compose(&mut self) {
        let locale = env::var_os("LC_ALL")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LC_CTYPE"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .or_else(|| env::var_os("LANG"))
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .unwrap_or_else(|| "C".into());
        let locale = CString::new(locale.into_vec()).unwrap();

        let compose_table = ffi_dispatch!(
            XKBH,
            xkb_compose_table_new_from_locale,
            self.xkb_context,
            locale.as_ptr(),
            ffi::xkb_compose_compile_flags::XKB_COMPOSE_COMPILE_NO_FLAGS
        );

        if compose_table.is_null() {
            // init of compose table failed, continue without compose
            return;
        }

        let compose_state = ffi_dispatch!(
            XKBH,
            xkb_compose_state_new,
            compose_table,
            ffi::xkb_compose_state_flags::XKB_COMPOSE_STATE_NO_FLAGS
        );

        if compose_state.is_null() {
            // init of compose state failed, continue without compose
            ffi_dispatch!(XKBH, xkb_compose_table_unref, compose_table);
            return;
        }

        self.xkb_compose_table = compose_table;
        self.xkb_compose_state = compose_state;
    }

    pub(crate) unsafe fn post_init(&mut self, keymap: *mut ffi::xkb_keymap) {
        let state = ffi_dispatch!(XKBH, xkb_state_new, keymap);
        self.xkb_keymap = keymap;
        self.xkb_state = state;
        self.mods_state.update_with(state);
    }

    pub(crate) unsafe fn de_init(&mut self) {
        ffi_dispatch!(XKBH, xkb_state_unref, self.xkb_state);
        self.xkb_state = ptr::null_mut();
        ffi_dispatch!(XKBH, xkb_keymap_unref, self.xkb_keymap);
        self.xkb_keymap = ptr::null_mut();
    }

    pub(crate) unsafe fn init_with_fd(&mut self, fd: File, size: usize) {
        let map = MmapOptions::new().len(size).map(&fd).unwrap();

        let keymap = ffi_dispatch!(
            XKBH,
            xkb_keymap_new_from_string,
            self.xkb_context,
            map.as_ptr() as *const _,
            ffi::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
            ffi::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS
        );

        if keymap.is_null() {
            panic!("Received invalid keymap from compositor.");
        }

        self.post_init(keymap);
    }

    pub(crate) unsafe fn init_with_rmlvo(
        &mut self,
        names: ffi::xkb_rule_names,
    ) -> Result<(), Error> {
        let keymap = ffi_dispatch!(
            XKBH,
            xkb_keymap_new_from_names,
            self.xkb_context,
            &names,
            ffi::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS
        );

        if keymap.is_null() {
            return Err(Error::BadNames);
        }

        self.post_init(keymap);

        Ok(())
    }

    pub(crate) unsafe fn key_repeats(&mut self, xkb_keycode_t: ffi::xkb_keycode_t) -> bool {
        ffi_dispatch!(XKBH, xkb_keymap_key_repeats, self.xkb_keymap, xkb_keycode_t) == 1
    }

    #[inline]
    pub(crate) fn ready(&self) -> bool {
        !self.xkb_state.is_null()
    }

    #[inline]
    pub(crate) fn locked(&self) -> bool {
        self.locked
    }

    #[inline]
    pub(crate) fn mods_state(&self) -> ModifiersState {
        self.mods_state
    }
}

impl Drop for KbState {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(XKBH, xkb_compose_state_unref, self.xkb_compose_state);
            ffi_dispatch!(XKBH, xkb_compose_table_unref, self.xkb_compose_table);
            ffi_dispatch!(XKBH, xkb_state_unref, self.xkb_state);
            ffi_dispatch!(XKBH, xkb_keymap_unref, self.xkb_keymap);
            ffi_dispatch!(XKBH, xkb_context_unref, self.xkb_context);
        }
    }
}
