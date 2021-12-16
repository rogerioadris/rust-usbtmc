use rust_usbtmc::instrument::Instrument;
use std::time::Instant;

const VID: u16 = 0x0699;
const PID: u16 = 0x0368;

const BUS: u8 = 0x01;
const ADDRESS: u8 = 0x01;

fn main() {
    // Single device
    let mut instr_single = Instrument::new(VID, PID);
    let start = Instant::now();
    instr_single.write("SELECT:CH1 1").unwrap();
    println!("Ask: {}", instr_single.ask("*IDN?").unwrap());

    let duration = start.elapsed();
    println!("Time elapsed is: {:?}", duration);

    // Multiple devices
    let mut instr_multiple = Instrument::new_filtered(VID, PID, BUS, ADDRESS);

    let start = Instant::now();
    instr_multiple.write("SELECT:CH1 1").unwrap();
    println!("Ask: {}", instr_multiple.ask("*IDN?").unwrap());

    let duration = start.elapsed();
    println!("Time elapsed is: {:?}", duration);
}
