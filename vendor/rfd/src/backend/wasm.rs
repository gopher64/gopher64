mod file_dialog;

use crate::{
    file_dialog::FileDialog, file_handle::WasmFileHandleKind, FileHandle, MessageDialogResult,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlAnchorElement, HtmlButtonElement, HtmlElement, HtmlInputElement};

#[derive(Clone, Debug)]
pub enum FileKind<'a> {
    In(FileDialog),
    Out(FileDialog, &'a [u8]),
}

#[derive(Clone, Debug)]
enum HtmlIoElement<'a> {
    Input(HtmlInputElement),
    Output {
        element: HtmlAnchorElement,
        name: String,
        data: &'a [u8],
    },
}

pub struct WasmDialog<'a> {
    overlay: Element,
    card: Element,
    title: Option<HtmlElement>,
    io: HtmlIoElement<'a>,
    button: HtmlButtonElement,

    style: Element,
}

impl<'a> WasmDialog<'a> {
    pub fn new(opt: &FileKind<'a>) -> Self {
        let window = web_sys::window().expect("Window not found");
        let document = window.document().expect("Document not found");

        let overlay = document.create_element("div").unwrap();
        overlay.set_id("rfd-overlay");

        let card = {
            let card = document.create_element("div").unwrap();
            card.set_id("rfd-card");
            overlay.append_child(&card).unwrap();

            card
        };

        let title = match opt {
            FileKind::In(dialog) => &dialog.title,
            FileKind::Out(dialog, _) => &dialog.title,
        }
        .as_ref()
        .map(|title| {
            let title_el: HtmlElement = document.create_element("div").unwrap().dyn_into().unwrap();

            title_el.set_id("rfd-title");
            title_el.set_inner_html(title);

            card.append_child(&title_el).unwrap();
            title_el
        });

        let io = match opt {
            FileKind::In(dialog) => {
                let input_el = document.create_element("input").unwrap();
                let input: HtmlInputElement = wasm_bindgen::JsCast::dyn_into(input_el).unwrap();

                input.set_id("rfd-input");
                input.set_type("file");

                let mut accept: Vec<String> = Vec::new();

                for filter in dialog.filters.iter() {
                    accept.append(&mut filter.extensions.to_vec());
                }

                accept.iter_mut().for_each(|ext| ext.insert_str(0, "."));

                input.set_accept(&accept.join(","));

                card.append_child(&input).unwrap();
                HtmlIoElement::Input(input)
            }
            FileKind::Out(dialog, data) => {
                let output_el = document.create_element("a").unwrap();
                let output: HtmlAnchorElement = wasm_bindgen::JsCast::dyn_into(output_el).unwrap();

                output.set_id("rfd-output");
                output.set_inner_text("click here to download your file");

                card.append_child(&output).unwrap();
                HtmlIoElement::Output {
                    element: output,
                    name: dialog.file_name.clone().unwrap_or_default(),
                    data,
                }
            }
        };

        let button = {
            let btn_el = document.create_element("button").unwrap();
            let btn: HtmlButtonElement = wasm_bindgen::JsCast::dyn_into(btn_el).unwrap();

            btn.set_id("rfd-button");
            btn.set_inner_text("Ok");

            card.append_child(&btn).unwrap();
            btn
        };

        let style = document.create_element("style").unwrap();
        style.set_inner_html(include_str!("./wasm/style.css"));
        overlay.append_child(&style).unwrap();

        Self {
            overlay,
            card,
            title,
            button,
            io,

            style,
        }
    }

    async fn show(&self) {
        let window = web_sys::window().expect("Window not found");
        let document = window.document().expect("Document not found");
        let body = document.body().expect("Document should have a body");

        let overlay = self.overlay.clone();
        let button = self.button.clone();

        let promise = match &self.io {
            HtmlIoElement::Input(_) => js_sys::Promise::new(&mut move |res, _rej| {
                let resolve_promise = Closure::wrap(Box::new(move || {
                    res.call0(&JsValue::undefined()).unwrap();
                }) as Box<dyn FnMut()>);

                button.set_onclick(Some(resolve_promise.as_ref().unchecked_ref()));
                resolve_promise.forget();
                body.append_child(&overlay).ok();
            }),
            HtmlIoElement::Output {
                element,
                name,
                data,
            } => {
                js_sys::Promise::new(&mut |res, _rej| {
                    // Moved to keep closure as FnMut
                    let output = element.clone();
                    let file_name = name.clone();

                    let resolve_promise = Closure::wrap(Box::new(move || {
                        res.call1(&JsValue::undefined(), &JsValue::from(true))
                            .unwrap();
                    }) as Box<dyn FnMut()>);

                    // Resolve the promise once the user clicks the download link or the button.
                    output.set_onclick(Some(resolve_promise.as_ref().unchecked_ref()));
                    button.set_onclick(Some(resolve_promise.as_ref().unchecked_ref()));
                    resolve_promise.forget();

                    let set_download_link = move |in_array: &[u8], name: &str| {
                        // See <https://stackoverflow.com/questions/69556755/web-sysurlcreate-object-url-with-blobblob-not-formatting-binary-data-co>
                        let array = js_sys::Array::new();
                        let uint8arr = js_sys::Uint8Array::new(
                            // Safety: No wasm allocations happen between creating the view and consuming it in the array.push
                            &unsafe { js_sys::Uint8Array::view(&in_array) }.into(),
                        );
                        array.push(&uint8arr.buffer());
                        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
                            &array,
                            web_sys::BlobPropertyBag::new().type_("application/octet-stream"),
                        )
                        .unwrap();
                        let download_url =
                            web_sys::Url::create_object_url_with_blob(&blob).unwrap();

                        output.set_href(&download_url);
                        output.set_download(&name);
                    };

                    set_download_link(&*data, &file_name);

                    body.append_child(&overlay).ok();
                })
            }
        };

        let future = wasm_bindgen_futures::JsFuture::from(promise);
        future.await.unwrap();
    }

    fn get_results(&self) -> Option<Vec<FileHandle>> {
        let input = match &self.io {
            HtmlIoElement::Input(input) => input,
            _ => panic!("Internal Error: Results only exist for input dialog"),
        };
        if let Some(files) = input.files() {
            let len = files.length();
            if len > 0 {
                let mut file_handles = Vec::new();
                for id in 0..len {
                    let file = files.get(id).unwrap();
                    file_handles.push(FileHandle::wrap(file));
                }
                Some(file_handles)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_result(&self) -> Option<FileHandle> {
        let files = self.get_results();
        files.and_then(|mut f| f.pop())
    }

    async fn pick_files(self) -> Option<Vec<FileHandle>> {
        if let HtmlIoElement::Input(input) = &self.io {
            input.set_multiple(true);
        } else {
            panic!("Internal error: Pick files only on input wasm dialog")
        }

        self.show().await;

        self.get_results()
    }

    async fn pick_file(self) -> Option<FileHandle> {
        if let HtmlIoElement::Input(input) = &self.io {
            input.set_multiple(false);
        } else {
            panic!("Internal error: Pick file only on input wasm dialog")
        }

        self.show().await;

        self.get_result()
    }

    fn io_element(&self) -> Element {
        match self.io.clone() {
            HtmlIoElement::Input(element) => element.unchecked_into(),
            HtmlIoElement::Output { element, .. } => element.unchecked_into(),
        }
    }
}

impl<'a> Drop for WasmDialog<'a> {
    fn drop(&mut self) {
        self.button.remove();
        self.io_element().remove();
        self.title.as_ref().map(|elem| elem.remove());
        self.card.remove();

        self.style.remove();
        self.overlay.remove();
    }
}

use super::{AsyncFilePickerDialogImpl, DialogFutureType};

impl AsyncFilePickerDialogImpl for FileDialog {
    fn pick_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        let dialog = WasmDialog::new(&FileKind::In(self));
        Box::pin(dialog.pick_file())
    }
    fn pick_files_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        let dialog = WasmDialog::new(&FileKind::In(self));
        Box::pin(dialog.pick_files())
    }
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    fn confirm(s: &str) -> bool;
}

use crate::backend::MessageDialogImpl;
use crate::message_dialog::{MessageButtons, MessageDialog};

impl MessageDialogImpl for MessageDialog {
    fn show(self) -> MessageDialogResult {
        let text = format!("{}\n{}", self.title, self.description);
        match self.buttons {
            MessageButtons::Ok | MessageButtons::OkCustom(_) => {
                alert(&text);
                MessageDialogResult::Ok
            }
            MessageButtons::OkCancel
            | MessageButtons::OkCancelCustom(..)
            | MessageButtons::YesNo
            | MessageButtons::YesNoCancel
            | MessageButtons::YesNoCancelCustom(..) => {
                if confirm(&text) {
                    MessageDialogResult::Ok
                } else {
                    MessageDialogResult::Cancel
                }
            }
        }
    }
}

impl crate::backend::AsyncMessageDialogImpl for MessageDialog {
    fn show_async(self) -> DialogFutureType<MessageDialogResult> {
        let val = MessageDialogImpl::show(self);
        Box::pin(std::future::ready(val))
    }
}

impl FileHandle {
    pub async fn write(&self, data: &[u8]) -> std::io::Result<()> {
        let dialog = match &self.0 {
            WasmFileHandleKind::Writable(dialog) => dialog,
            _ => panic!("This File Handle doesn't support writing. Use `save_file` to get a writeable FileHandle in Wasm"),
        };
        let dialog = WasmDialog::new(&FileKind::Out(dialog.clone(), data));
        dialog.show().await;
        Ok(())
    }
}
