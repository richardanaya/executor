use core::future::Future;
use js_ffi::*;
use once_cell::sync::Lazy;
use spin::Mutex;

static JS_API: Lazy<Mutex<API>> = Lazy::new(|| {
    let a = API {
        log_handle: js!(console.log),
        set_interval_handle: js!(window.setInterval),
        set_timeout_handle: js!(window.setTimeout),
    };
    Mutex::new(a)
});

#[no_mangle]
pub fn main() -> () {
    JS_API.lock().window_set_interval(
        || {
            executor::run(async {
                let api = JS_API.lock();
                api.console_log("Tic");
                api.window_set_timeout(500).await;
                api.console_log("Toc");
            });
        },
        1000,
    );
}

struct API {
    log_handle: JSInvoker,
    set_timeout_handle: JSInvoker,
    set_interval_handle: JSInvoker,
}

impl API {
    fn console_log(&self, msg: &str) {
        self.log_handle.invoke_1(msg);
    }

    fn window_set_interval(&self, cb: impl FnMut() -> () + Send + 'static, millis: i32) {
        let cb = create_callback_0(cb);
        self.set_interval_handle.invoke_2(cb, millis);
    }

    fn window_set_timeout(&self, millis: i32) -> impl Future {
        let (future, cb) = create_callback_future_0();
        self.set_timeout_handle.invoke_2(cb, millis);
        future
    }
}
