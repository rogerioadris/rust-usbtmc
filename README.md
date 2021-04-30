# Rust USBTMC

Exemplo de uso

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
