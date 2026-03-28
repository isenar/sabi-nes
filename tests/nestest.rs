mod common;

use crate::common::trace;
use pretty_assertions::assert_eq;
use sabi_nes::{Address, Bus, Cpu, Result, Rom};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};
use std::path::Path;

fn read_lines(filename: impl AsRef<Path>) -> io::Result<Lines<BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}

#[test]
fn cpu_validation_test() -> Result<()> {
    let rom = Rom::from_file("../sabi-nes/tests/test_roms/nestest.nes")?;
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);

    // PC starts here (as seen in nestest.log).
    // This specific value enables running the test ROM in "automation" mode.
    cpu.program_counter = Address::new(0xc000);

    let lines: Vec<String> = read_lines("../sabi-nes/tests/expected_logs/nestest.log")?
        .collect::<io::Result<_>>()?;

    let mut traces = Vec::with_capacity(lines.len());
    for _ in 0..lines.len() {
        traces.push(trace(&mut cpu)?);
        cpu.step()?;
    }

    for (line, (expected_trace, actual_trace)) in lines.iter().zip(traces.iter()).enumerate() {
        let line = line + 1;
        assert_eq!(expected_trace, actual_trace, "Mismatch at line#{line}");
    }

    Ok(())
}
