use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

fn block_on(f:impl std::future::Future+Sync+Send+'static){
    let complete = Arc::new(Mutex::new(AtomicBool::new(false)));
    let ender = complete.clone();
    thread::spawn(||{
        executor::spawn(async move {
            f.await;
            ender.lock().unwrap().store(true, Ordering::Release);
        });
    });
    while !complete.lock().unwrap().load(Ordering::Acquire) {}
}

fn main() {
    block_on(async {
        println!("hello");
        async_std::task::sleep(Duration::from_secs(1)).await;
        println!("world!");
    })
}
