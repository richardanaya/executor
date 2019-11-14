# 💀 executor

<a href="https://docs.rs/executor"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>

```toml
[dependencies]
executor = "0.5"
```
## Features
- [x] `#![no_std]` and minimally uses `alloc` (`Box` and `Arc`)
- [x] handle more than one spawn to parallelize work
- [x] simple enough to learn from! (< 100 lines)

## Example

A web assembly example

```rust
[no_mangle]
pub fn main() -> () {
    executor::spawn(async {
        console_log("Hello");
        set_timeout(1000).await;
        console_log("World!");
    });
}

fn set_timeout(milliseconds:u32) -> TimeoutFuture {
   // create a timeout future and store globally
}

[no_mangle]
pub fn timeout_complete() -> () {
    // find your timeout future and wake it's waker
}
```

## async-std

Want to use [async-std](https://async.rs/)?

```rust
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

fn main_on() {
    block(async {
        println!("hello");
        async_std::task::sleep(Duration::from_secs(1)).await;
        println!("world!");
    })
}
```

## Want to replace the global executor?

Write your own with this trait

```rust
pub trait GlobalExecutor {
    fn spawn(&mut self, future: Box<dyn Future<Output = ()> + 'static + Send + Unpin>);
}
```

```rust
executor::set_global_executor(MY_EXECUTOR);
```

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `executor` by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
