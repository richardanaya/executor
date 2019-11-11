use async_std::task;
use executor::*;
use std::thread;
use std::time::Duration;

async fn run() {
    println!("hello");
    task::sleep(Duration::from_secs(1)).await;
    println!("world!");
    complete::mark_complete();
}

fn main() -> () {
    thread::spawn(move || {
        Executor::spawn(run());
    });
    complete::block_until_complete();
}
