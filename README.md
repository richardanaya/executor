# Executor: Async-Await in Web Assembly

```toml
[dependencies]
executor = "0.0.2"
```

## Features
- [x] handle single spawn (i.e. don't spawn beyond your initial entry point)
- [x] handle basic futures (i.e only futures that resolve once like most JS promises)
- [ ] handle multi-entrant futures
- [ ] handle more than one spawn

## Example

```rust
use executor::Executor;

pub fn main() -> () {
    Executor::spawn(async {
        say_num(1);
        sleep(1000).await;
        say_num(2);
        sleep(1000).await;
        say_num(3);
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
for inclusion in woke by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
