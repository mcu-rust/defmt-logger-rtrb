# defmt-logger-rtrb

[![CI](https://github.com/mcu-rust/defmt-logger-rtrb/workflows/CI/badge.svg)](https://github.com/mcu-rust/defmt-logger-rtrb/actions)
[![Crates.io](https://img.shields.io/crates/v/defmt-logger-rtrb.svg)](https://crates.io/crates/defmt-logger-rtrb)
[![Docs.rs](https://docs.rs/defmt-logger-rtrb/badge.svg)](https://docs.rs/defmt-logger-rtrb)

A [`defmt`](https://crates.io/crates/defmt) global logger based on [`rtrb`](https://crates.io/crates/rtrb) ring buffer.

This crate needs a global allocator. If you are using it on a bare-metal platform, you can use [`embedded-alloc`](https://crates.io/crates/embedded-alloc) or [`heap1`](https://crates.io/crates/heap1) as global allocator.

# Usage
```sh
cargo add defmt-logger-rtrb
```

Add following to your `.cargo\config.toml`:

```toml
[target.thumbv7m-none-eabi]
linker = "flip-link"
rustflags = [
    "-C", "link-arg=-Tlink.x",
    "-C", "link-arg=-Tdefmt.x", # add this
]

[env]
DEFMT_LOG = "info" # add this
```

Your code:

```rust ignore
fn main() {
    // Initialize it before any `defmt` interfaces are called.
    let mut log_consumer = defmt_logger_rtrb::init(1024);

    defmt::info!("foo");

    // get log data from buffer and send it via UART or something similar
    loop {
        let n = log_consumer.slots();
        if n > 0 && let Ok(chunk) = log_consumer.read_chunk(n) {
            let (data, _) = chunk.as_slices();
            // send data ...
            chunk.commit(data.len());
        }
    }
}
```

## Global Consumer

You can also use the global log consumer:

```rust ignore
fn main() {
    // Initialize it before any `defmt` interfaces are called.
    defmt_logger_rtrb::init_global(1024);

    defmt::info!("foo");

    // get log data from buffer and send it via UART or something similar
    loop {
        if let Some(chunk) = unsafe { defmt_logger_rtrb::get_read_chunk() } {
            let (data, _) = chunk.as_slices();
            // send data ...
            chunk.commit(data.len());
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {
        if let Some(chunk) = unsafe { defmt_logger_rtrb::get_read_chunk() } {
            let (data, _) = chunk.as_slices();
            // send data ...
            chunk.commit(data.len());
        }
    }
}
```
