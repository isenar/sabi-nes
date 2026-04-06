mod common;

use crate::common::trace;
use pretty_assertions::assert_eq;
use sabi_nes_core::{Address, Bus, Cpu, Result, Rom};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::iter;
use std::path::Path;

fn read_lines(filename: impl AsRef<Path>) -> Result<Vec<String>> {
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines().collect::<io::Result<_>>()?)
}

#[test]
fn cpu_validation_test() -> Result<()> {
    let rom = Rom::from_file("../sabi-nes-core/tests/test_roms/nestest.nes")?;
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);

    // PC starts here (as seen in nestest.log).
    // This specific value enables running the test ROM in "automation" mode.
    cpu.program_counter = Address::new(0xc000);

    let trace_step = || -> Result<String> {
        let trace = trace(&mut cpu);
        cpu.step()?;
        trace
    };
    let lines = read_lines("../sabi-nes-core/tests/test_data/nestest_expected_logs.txt")?;
    let traces = iter::repeat_with(trace_step)
        .take(lines.len())
        .collect::<Result<Vec<_>>>()?;

    let results = lines.iter().zip(traces.iter()).enumerate();
    for (line, (expected_trace, actual_trace)) in results {
        let line = line + 1;
        assert_eq!(expected_trace, actual_trace, "Mismatch at line#{line}");
    }

    Ok(())
}
