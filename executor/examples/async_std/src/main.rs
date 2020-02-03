use async_std::task::sleep;
use std::time::Duration;

#[executor::main]
async fn main() {
    println!("hello");
    sleep(Duration::from_secs(1)).await;
    println!("world!");
}
