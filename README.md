# executor

<a href="https://docs.rs/executor"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>

```toml
[dependencies]
executor = "0.7"
```
## Features
- [x] `#![no_std]` + `alloc`
- [x] simple enough to learn from! (~ 100 lines)

## WebAssembly

```rust
use web::{log, set_timeout_async};

async fn main() {
    executor::run(async move {
        log("hello");
        set_timeout_async(1000).await;
        log("world");
    });
}
```

## CLI

Even `async-std` can be used if you add something to stop it from exiting too early.

```rust
use async_std::task::sleep;
use std::time::Duration;

fn main() {
    let complete = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
    let ender = complete.clone();
    executor::run(async move {
        println!("hello");
        sleep(Duration::from_secs(1)).await;
        println!("world!");
        ender.store(true, core::sync::atomic::Ordering::Release);
    });
    while !complete.load(core::sync::atomic::Ordering::Acquire) {}
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
