use async_std::task::sleep;
use std::time::Duration;

async fn async_sleep(print_name: &str, time: Duration) {
    let sleep_time = time.as_millis();
    println!("{print_name}: before sleep {sleep_time} ms");
    sleep(time).await;
    println!("{print_name}: after sleep {sleep_time} ms");
}

fn main() {
    // Creates new task. that wiill sleep 1 sec and complete.
    // Execute only after update() method.
    executor::add_async(async_sleep("call_1", Duration::from_secs(1)));

    // Poll all tasks once, that added by add_async or add_async.
    // We can also do other stuff after it.
    executor::update();

    // Previous sleep at progress and it dont need extra polling, so we can continue do our stuff.
    // After this function executor will execute 2 tasks simultaneously.
    executor::add_async(async_sleep("call_2", Duration::from_secs(2)));

    // Another one task for fun :)
    executor::add_async(async_sleep("call_2", Duration::from_secs(3)));

    // Checks if we have incompleted tasks.
    // Drops completed task from internal task collection.
    while !executor::is_done() {
        executor::update();
    }
}
