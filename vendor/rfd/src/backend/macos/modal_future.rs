use objc::{msg_send, sel, sel_impl};
use objc_id::Id;

use std::sync::{Arc, Mutex};
use std::{mem, pin::Pin};

use std::task::{Context, Poll, Waker};

use super::AsModal;

use super::utils::{
    activate_cocoa_multithreading, is_main_thread, INSApplication, NSApplication, NSWindow,
};

struct FutureState<R, D> {
    waker: Option<Waker>,
    data: Option<R>,
    modal: Option<D>,
}

unsafe impl<R, D> Send for FutureState<R, D> {}

pub(super) struct ModalFuture<R, D> {
    state: Arc<Mutex<FutureState<R, D>>>,
}

unsafe impl<R, D> Send for ModalFuture<R, D> {}

impl<R: 'static + Default, D: AsModal + 'static> ModalFuture<R, D> {
    pub fn new<F, DBULD: FnOnce() -> D + Send>(
        win: Option<Id<NSWindow>>,
        build_modal: DBULD,
        cb: F,
    ) -> Self
    where
        F: Fn(&mut D, i64) -> R + Send + 'static,
    {
        activate_cocoa_multithreading();

        let state = Arc::new(Mutex::new(FutureState {
            waker: None,
            data: None,
            modal: None,
        }));

        let dialog_callback = move |state: Arc<Mutex<FutureState<R, D>>>, result: i64| {
            let mut state = state.lock().unwrap();
            // take() to drop it when it's safe to do so
            state.data = if let Some(mut modal) = state.modal.take() {
                Some((&cb)(&mut modal, result))
            } else {
                Some(Default::default())
            };
            if let Some(waker) = state.waker.take() {
                waker.wake();
            }
        };

        let app = NSApplication::shared_application();

        let win = if let Some(win) = win {
            Some(win.share())
        } else {
            app.get_window()
        };

        // if async exec is possible start sheet modal
        // otherwise fallback to sync
        if app.is_running() && win.is_some() {
            let state = state.clone();
            let main_runner = move || {
                let completion = {
                    let state = state.clone();
                    block::ConcreteBlock::new(move |result: i64| {
                        dialog_callback(state.clone(), result);
                    })
                };

                let window = win.unwrap();

                let mut modal = build_modal();
                let modal_ptr = modal.modal_ptr();

                state.lock().unwrap().modal = Some(modal);

                let _: () = unsafe {
                    msg_send![
                        modal_ptr,
                        beginSheetModalForWindow: window completionHandler: &completion
                    ]
                };

                mem::forget(completion);
            };

            if !is_main_thread() {
                let main = dispatch::Queue::main();
                main.exec_sync(main_runner);
            } else {
                main_runner();
            }
        } else {
            eprintln!("\n Hi! It looks like you are running async dialog in unsupported environment, I will fallback to sync dialog for you. \n");

            if is_main_thread() {
                let mut modal = build_modal();
                let modal_ptr = modal.modal_ptr();

                state.lock().unwrap().modal = Some(modal);

                let ret: i64 = unsafe { msg_send![modal_ptr, runModal] };

                dialog_callback(state.clone(), ret);
            } else {
                panic!("Fallback Sync Dialog Must Be Spawned On Main Thread (Why? If async dialog is unsupported in this env, it also means that spawning dialogs outside of main thread is also inpossible");
            }
        }

        Self { state }
    }
}

impl<R, D> std::future::Future for ModalFuture<R, D> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();

        if state.data.is_some() {
            Poll::Ready(state.data.take().unwrap())
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
