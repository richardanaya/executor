use async_std::task::sleep;
use std::time::Duration;

fn main() {
    let complete = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
    let ender = complete.clone();
    executor::run(async move {
        println!("hello");
        sleep(Duration::from_secs(1)).await;
        println!("world!");
        ender.store(true, core::sync::atomic::Ordering::Release);
    });
    while !complete.load(core::sync::atomic::Ordering::Acquire) {}
}
