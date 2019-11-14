use std::thread;
use std::time::Duration;

fn main() {
    block_on(async {
        println!("hello");
        async_std::task::sleep(Duration::from_secs(1)).await;
        println!("world!");
    })
}

fn block_on(f:impl std::future::Future+Sync+Send+'static){
    let complete = std::sync::Arc::new(std::sync::Mutex::new(core::sync::atomic::AtomicBool::new(false)));
    let ender = complete.clone();
    thread::spawn(||{
        executor::spawn(async move {
            f.await;
            ender.lock().unwrap().store(true, core::sync::atomic::Ordering::Release);
        });
    });
    while !complete.lock().unwrap().load(core::sync::atomic::Ordering::Acquire) {}
}