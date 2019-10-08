# Executor: Async-Await in Web Assembly

```toml
[dependencies]
executor = "0.0.1"
```

## Features
- [x] handle single spawn
- [x] handle basic futures
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
