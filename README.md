# Executor: Async-Await in Web Assembly

```toml
[dependencies]
executor = "0.0.2"
```

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

## Features
- [x] handle spawn (i.e. don't spawn beyond your initial entry point)
- [x] handle non self waking futures (i.e futures that immediately return values and JS futures operate asynchronously)
- [ ] handle more than one spawn ( needed for callbacks that start async work )
- [ ] handle self waking futures (i.e. Since JS is single threaded, self waking futures cause a mutex lock, futures must call some other mechanism to asynchronously re-enter a web assembly module)

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in woke by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
