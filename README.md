# executor

<a href="https://docs.rs/executor"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>

```toml
[dependencies]
executor = "0.9.0"
```
## Features
- [x] `#![no_std]` + `alloc`
- [x] simple enough to learn from! (~ 200 lines)
- [x] works with WebAssembly

## WebAssembly

```rust
use web::*;
use executor;

#[no_mangle]
fn main() {
    executor::add_async(async {
        loop {
            set_inner_html(DOM_BODY, "⏰ tic");
            sleep(1000).await;
            set_inner_html(DOM_BODY, "⏰ tock");
            sleep(1000).await;
        }
    });
    while !executor::is_done() {
        executor::update();
    }
}
```

See this working [here](https://richardanaya.github.io/executor/examples/timer/).

## CLI

Even `async-std` can be used if you add something to stop it from exiting too early.

```rust
use async_std::task::sleep;
use std::time::Duration;

fn main() {
    let complete = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
    let ender = complete.clone();
    executor::add_async(async move {
        println!("hello");
        sleep(Duration::from_secs(1)).await;
        println!("world!");
        ender.store(true, core::sync::atomic::Ordering::Release);
    });
    while !complete.load(core::sync::atomic::Ordering::Acquire) {executor::update();}
}
```

## Testing / platform support

Host tests cover completion, `is_done`, nested `add_async`, `wake` and `wake_by_ref` (including wakes from another thread), waking during poll, drop of finished tasks, and a short many-wake stress run.

```bash
cargo test -- --test-threads=1
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo doc --no-deps
cargo +nightly miri test -- --test-threads=1
```

The library builds on the declared MSRV, Rust 1.63:

```bash
cargo +1.63.0 build --lib
cargo +1.63.0 check --target wasm32-unknown-unknown --lib
cargo +1.63.0 check --target thumbv7m-none-eabi --lib
```

Behavior tests and the `async-std` examples run on stable. Current `async-std` releases pull crates that need a newer compiler than 1.63; the library does not.

`no_std` + `alloc` needs pointer-width atomics (`alloc::sync::Arc` and `spin::Mutex`). Checked targets:

| Target | Support |
| --- | --- |
| `wasm32-unknown-unknown` | `cargo check --target wasm32-unknown-unknown --lib` |
| `thumbv7m-none-eabi` | builds, and `nostd-test` actually runs under QEMU |
| `thumbv6m-none-eabi` (Cortex-M0) | not supported: no pointer-width atomics, so `Arc` is not in `alloc` |

The QEMU harness is a `#![no_std]` `#![no_main]` binary with a bump allocator. It checks `is_done`, a task that completes and is dropped, a task spawned from another task, a self-wake, several tasks, and a `u32` output. Nothing from `std` is linked.

```bash
cargo build --manifest-path nostd-test/Cargo.toml --release --target thumbv7m-none-eabi
qemu-system-arm -cpu cortex-m3 -machine lm3s6965evb -nographic \
  -semihosting-config enable=on,target=native \
  -kernel nostd-test/target/thumbv7m-none-eabi/release/executor-nostd-test
```

Do not call `update` or `update_woken` from inside a task. Both take a `spin::Mutex` that is not reentrant. Spawn with `add_async`, and wake the waker you were given; the queues exist so that is safe during `update`.

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
