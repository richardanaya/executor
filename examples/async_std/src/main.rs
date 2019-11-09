use executor::*;
use std::time::Duration;
use async_std::task;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::thread;

static IS_COMPLETE:AtomicBool = AtomicBool::new(false);

async fn run() {
    println!("hello");
    task::sleep(Duration::from_secs(1)).await;
    println!("world!");
    IS_COMPLETE.store(true,Ordering::Release);
}

fn main() -> () {
    thread::spawn(move || {
        Executor::spawn(run());
    });
    while !IS_COMPLETE.load(Ordering::Acquire) {}
}
