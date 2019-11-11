use std::thread;
use std::time::Duration;

async fn run() {
    println!("hello");
    async_std::task::sleep(Duration::from_secs(1)).await;
    println!("world!");
    complete::mark_complete();
}

fn main() -> () {
    thread::spawn(move || {
        executor::spawn(run());
    });
    complete::block_until_complete();
}
