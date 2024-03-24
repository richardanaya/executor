use async_std::task::sleep;
use std::time::Duration;

async fn async_sleep(print_name: &str, time: Duration) {
    let sleep_time = time.as_millis();
    println!("{print_name}: before sleep {sleep_time} ms");
    sleep(time).await;
    println!("{print_name}: after sleep {sleep_time} ms");
}

fn main() -> () {
    println!("hello");

    executor::add_async(async {
        async_sleep("call_1", Duration::from_secs(1)).await;

        // We also can add new futures "on fly" at the another futures.
        executor::add_async(async_sleep("call_2", Duration::from_secs(2)));
        async_sleep("call_3", Duration::from_secs(3)).await;

        println!("world!")
    });

    while !executor::is_done() {
        executor::update();
    }
}
