use web::{log, sleep};

#[no_mangle]
fn main() {
    executor::run(async {
        log("hello");
        sleep(1000).await;
        log("world");
    });
}
