use crate::FileDialog;

use std::ops::Deref;
use std::path::Path;
use std::{ops::DerefMut, path::PathBuf};

use objc::{class, msg_send, sel, sel_impl};
use objc_id::Id;

use super::super::utils::{INSURL, NSURL};

use crate::backend::macos::utils::{INSWindow, NSWindow};
use objc::runtime::{Object, YES};
use objc::runtime::{BOOL, NO};
use objc_foundation::{INSArray, INSString, NSArray, NSString};
use raw_window_handle::RawWindowHandle;

use super::super::{
    utils::{FocusManager, PolicyManager},
    AsModal,
};

extern "C" {
    pub fn CGShieldingWindowLevel() -> i32;
}

fn make_nsstring(s: &str) -> Id<NSString> {
    NSString::from_str(s)
}

pub struct Panel {
    pub(crate) panel: Id<Object>,
    _focus_manager: FocusManager,
    _policy_manager: PolicyManager,
}

impl AsModal for Panel {
    fn modal_ptr(&mut self) -> *mut Object {
        self.panel.deref_mut()
    }
}

impl Panel {
    pub fn new(panel: *mut Object) -> Self {
        let _policy_manager = PolicyManager::new();

        let _focus_manager = FocusManager::new();

        let _: () = unsafe { msg_send![panel, setLevel: CGShieldingWindowLevel()] };
        Self {
            panel: unsafe { Id::from_ptr(panel) },
            _focus_manager,
            _policy_manager,
        }
    }

    pub fn open_panel() -> Self {
        Self::new(unsafe { msg_send![class!(NSOpenPanel), openPanel] })
    }

    pub fn save_panel() -> Self {
        Self::new(unsafe { msg_send![class!(NSSavePanel), savePanel] })
    }

    pub fn run_modal(&self) -> i32 {
        unsafe { msg_send![self.panel, runModal] }
    }

    pub fn set_can_choose_directories(&self, v: BOOL) {
        let _: () = unsafe { msg_send![self.panel, setCanChooseDirectories: v] };
    }

    pub fn set_can_create_directories(&self, v: BOOL) {
        let _: () = unsafe { msg_send![self.panel, setCanCreateDirectories: v] };
    }

    pub fn set_can_choose_files(&self, v: BOOL) {
        let _: () = unsafe { msg_send![self.panel, setCanChooseFiles: v] };
    }

    pub fn set_allows_multiple_selection(&self, v: BOOL) {
        let _: () = unsafe { msg_send![self.panel, setAllowsMultipleSelection: v] };
    }

    pub fn add_filters(&self, params: &FileDialog) {
        let mut exts: Vec<String> = Vec::new();

        for filter in params.filters.iter() {
            exts.append(&mut filter.extensions.to_vec());
        }

        unsafe {
            let f_raw: Vec<_> = exts.iter().map(|ext| make_nsstring(&ext)).collect();
            let array = NSArray::from_vec(f_raw);

            let _: () = msg_send![self.panel, setAllowedFileTypes: array];
        }
    }

    pub fn set_path(&self, path: &Path, file_name: Option<&str>) {
        // if file_name is some, and path is a dir
        let path = if let (Some(name), true) = (file_name, path.is_dir()) {
            let mut path = path.to_owned();
            // add a name to the end of path
            path.push(name);
            path
        } else {
            path.to_owned()
        };

        if let Some(path) = path.to_str() {
            unsafe {
                let url = NSURL::file_url_with_path(path, true);
                let () = msg_send![self.panel, setDirectoryURL: url];
            }
        }
    }

    pub fn set_file_name(&self, file_name: &str) {
        unsafe {
            let file_name = make_nsstring(file_name);
            let () = msg_send![self.panel, setNameFieldStringValue: file_name];
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let title = make_nsstring(title);
            let () = msg_send![self.panel, setTitle: title];
        }
    }

    pub fn set_parent(&self, parent: &RawWindowHandle) {
        let id = NSWindow::from_raw_window_handle(parent);
        unsafe {
            let () = msg_send![id, addChildWindow: self.panel.deref() ordered: 1];
        }
    }

    pub fn get_result(&self) -> PathBuf {
        unsafe {
            let url = msg_send![self.panel, URL];
            let url: Id<NSURL> = Id::from_ptr(url);
            url.to_path_buf()
        }
    }

    pub fn get_results(&self) -> Vec<PathBuf> {
        unsafe {
            let urls = msg_send![self.panel, URLs];
            let urls: Id<NSArray<NSURL>> = Id::from_ptr(urls);

            let mut res = Vec::new();
            for url in urls.to_vec() {
                res.push(url.to_path_buf());
            }

            res
        }
    }
}

impl Panel {
    pub fn build_pick_file(opt: &FileDialog) -> Self {
        let panel = Panel::open_panel();

        if !opt.filters.is_empty() {
            panel.add_filters(&opt);
        }

        if let Some(path) = &opt.starting_directory {
            panel.set_path(path, opt.file_name.as_deref());
        }

        if let Some(file_name) = &opt.file_name {
            panel.set_file_name(file_name);
        }

        if let Some(title) = &opt.title {
            panel.set_title(title);
        }

        if let Some(parent) = &opt.parent {
            panel.set_parent(parent);
        }

        panel.set_can_choose_directories(NO);
        panel.set_can_choose_files(YES);

        panel
    }

    pub fn build_save_file(opt: &FileDialog) -> Self {
        let panel = Panel::save_panel();

        if !opt.filters.is_empty() {
            panel.add_filters(&opt);
        }

        if let Some(path) = &opt.starting_directory {
            panel.set_path(path, opt.file_name.as_deref());
        }

        if let Some(file_name) = &opt.file_name {
            panel.set_file_name(file_name);
        }

        if let Some(title) = &opt.title {
            panel.set_title(title);
        }

        if let Some(parent) = &opt.parent {
            panel.set_parent(parent);
        }

        panel
    }

    pub fn build_pick_folder(opt: &FileDialog) -> Self {
        let panel = Panel::open_panel();

        if let Some(path) = &opt.starting_directory {
            panel.set_path(path, opt.file_name.as_deref());
        }

        if let Some(title) = &opt.title {
            panel.set_title(title);
        }

        if let Some(parent) = &opt.parent {
            panel.set_parent(parent);
        }

        panel.set_can_choose_directories(YES);
        panel.set_can_create_directories(YES);
        panel.set_can_choose_files(NO);

        panel
    }

    pub fn build_pick_folders(opt: &FileDialog) -> Self {
        let panel = Panel::open_panel();

        if let Some(path) = &opt.starting_directory {
            panel.set_path(path, opt.file_name.as_deref());
        }

        if let Some(title) = &opt.title {
            panel.set_title(title);
        }

        if let Some(parent) = &opt.parent {
            panel.set_parent(parent);
        }

        panel.set_can_choose_directories(YES);
        panel.set_can_create_directories(YES);
        panel.set_can_choose_files(NO);
        panel.set_allows_multiple_selection(YES);

        panel
    }

    pub fn build_pick_files(opt: &FileDialog) -> Self {
        let panel = Panel::open_panel();

        if !opt.filters.is_empty() {
            panel.add_filters(&opt);
        }

        if let Some(path) = &opt.starting_directory {
            panel.set_path(path, opt.file_name.as_deref());
        }

        if let Some(title) = &opt.title {
            panel.set_title(title);
        }

        if let Some(parent) = &opt.parent {
            panel.set_parent(parent);
        }

        panel.set_can_choose_directories(NO);
        panel.set_can_choose_files(YES);
        panel.set_allows_multiple_selection(YES);

        panel
    }
}
