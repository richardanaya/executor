# 💀 Executor

<a href="https://docs.rs/executor"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>

```toml
[dependencies]
executor = "0.1"
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
        console_log("World");
        set_timeout(1000).await;
        console_log("!");
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

```
async fn run() {
    println!("hello");
    async_std::task::sleep(std::time::Duration::from_secs(1)).await;
    println!("world!");
    complete::mark_complete();
}

fn main() -> () {
    std::thread::spawn(move || {
        executor::spawn(run());
    });
    complete::block_until_complete();
}
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
