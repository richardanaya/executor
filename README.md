# 💀 Executor

```toml
[dependencies]
executor = "0.1.0"
```
## Features
- [x] `#![no_std]` and minimally uses `alloc` (`Box` and `Arc`)
- [x] handle more than one spawn to parallelize work
- [x] simple enough to learn from!

## Example

```rust
use executor::Executor;

pub fn main() -> () {
    Executor::spawn(async {
        console_log("Hello");
        window_set_timeout(1000).await;
        console_log("World");
        window_set_timeout(1000).await;
        console_log("!");
    });
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
