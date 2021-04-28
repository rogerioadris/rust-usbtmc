use rust_usbtmc::instrument::Instrument;
use std::time::Instant;

const VID: u16 = 0x0699;
const PID: u16 = 0x0368;

fn main() {
    let start = Instant::now();

    let mut instr = Instrument::new(VID, PID);

    instr.write("SELECT:CH1 1").unwrap();
    instr.ask("*IDN?").unwrap();
    instr.read().unwrap();

    let duration = start.elapsed();
    println!("Time elapsed is: {:?}", duration);
}
