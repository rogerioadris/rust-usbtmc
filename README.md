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
