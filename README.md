# executor

<a href="https://docs.rs/executor"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>

```toml
[dependencies]
executor = "0.5"
```
## Features
- [x] `#![no_std]` + `alloc`
- [x] global executor for easy spawning
- [x] simple enough to learn from! (~ 100 lines)

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

fn main() {
    block_on(async {
        println!("hello");
        async_std::task::sleep(Duration::from_secs(1)).await;
        println!("world!");
    })
}

fn block_on(f:impl std::future::Future+Sync+Send+'static){
    let complete = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
    let ender = complete.clone();
    thread::spawn(||{
        executor::spawn(async move {
            f.await;
            ender.store(true, core::sync::atomic::Ordering::Release);
        });
    });
    while !complete.load(core::sync::atomic::Ordering::Acquire) {}
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
