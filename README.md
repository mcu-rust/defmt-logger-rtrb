# defmt-logger-rtrb

A [`defmt`](https://crates.io/crates/defmt) global logger based on [`rtrb`](https://crates.io/crates/rtrb) ring buffer.

This crate needs a global allocator. If you are using it on a bare-metal platform, you can use [`embedded-alloc`](https://crates.io/crates/embedded-alloc) or [`heap1`](https://crates.io/crates/heap1) as global allocator.

# Usage
```sh
cargo add defmt-logger-rtrb
```

```rust ignore
use defmt_logger_rtrb;

fn main() {
    // Initialize it before any `defmt` interfaces are called.
    let mut log_buf = defmt_logger_rtrb::init(128);

    defmt::info!("foo");

    // get log data from buffer and send it via UART or something similar
    let n = log_buf.slots();
    if n > 0 && let Ok(chunk) = log_buf.read_chunk(n) {
        let (data, _) = chunk.as_slices();
        // send data ...
        chunk.commit(n);
    }
}
```
