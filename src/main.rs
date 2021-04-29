use rust_usbtmc::instrument::Instrument;
use std::time::Instant;

const VID: u16 = 0x0699;
const PID: u16 = 0x0368;

fn main() {
    let mut instr = Instrument::new(VID, PID);

    let start = Instant::now();
    instr.write("SELECT:CH1 1").unwrap();
    println!("Ask: {}", instr.ask("*IDN?").unwrap());

    let duration = start.elapsed();
    println!("Time elapsed is: {:?}", duration);
}
