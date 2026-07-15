# Rust USBTMC

## Single instrument

```rust
use rust_usbtmc::instrument::Instrument;

const VID: u16 = 0x0699; // Vendor
const PID: u16 = 0x0368; // Product

fn main() {
    let mut instr = Instrument::new(VID, PID);

    // Command Osciloscope
    instr.write("SELECT:CH1 1").unwrap();
    println!("Ask: {}", instr.ask("*IDN?").unwrap());
}
```

## Multiple instruments

You are able to connect to multiple devices with same `VID` and `PID` by filtering them on `Bus` and `Device` entries from `lsusb`.

```rust
use rust_usbtmc::instrument::Instrument;

const VID: u16 = 0x0699; // Vendor
const PID: u16 = 0x0368; // Product

// Bus and Device settings of the first instrument
const FIRST_BUS: u8 = 0x01;
const FIRST_ADDRESS: u8 = 0x01;

// Bus and Device settings of the second instrument
const SECOND_BUS: u8 = 0x03;
const SECOND_ADDRESS: u8 = 0x02;

fn main() {
    // Select first specific Instrument.
    let mut instr1 = Instrument::new_filtered(VID, PID, FIRST_BUS, FIRST_ADDRESS);

    // Select second specific Instrument.
    let mut instr2 = Instrument::new_filtered(VID, PID, SECOND_BUS, SECOND_ADDRESS);

    // Execute commands on the first Instrument
    instr1.write("SELECT:CH1 1").unwrap();
    println!("Ask: {}", instr1.ask("*IDN?").unwrap());

    // Execute commands on the second Instrument
    instr2.write("SELECT:CH2 1").unwrap();
    println!("Ask: {}", instr2.ask("*IDN?").unwrap());
}
```

## Identification

`info()` is a shortcut for `ask("*IDN?")`:

```rust
println!("Info: {}", instr.info().unwrap());
```

## Timeout and transfer size

Both default to 1 second and 1 MiB respectively; override them before the first `read`/`write`/`ask` call:

```rust
use std::time::Duration;

instr.set_timeout(Duration::from_secs(5));
instr.set_max_transfer_size(64 * 1024);
```

## Connection lifetime

The USB device is opened once, on the first `read`/`write`/`ask`/`info` call, and the handle is reused for every call after that instead of reopening the device each time. The connection closes (and the kernel driver reattaches) when the `Instrument` is dropped.

## Errors

All fallible calls return `Result<T, UsbtmcError>`. `UsbtmcError` implements `std::error::Error`/`Display`, so it composes with `?` and error-handling crates like `anyhow`.
