mod common;

use crate::common::trace;
use pretty_assertions::assert_eq;
use sabi_nes::{Bus, Cpu, Result, Rom};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};
use std::path::Path;

// TODO: remove this once APU is implemented and whole test passes
const VALID_LINES_SO_FAR: usize = 8980;

fn read_lines(filename: impl AsRef<Path>) -> io::Result<Lines<BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[test]
fn cpu_validation_test() -> Result<()> {
    let rom = Rom::from_file("../sabi-nes/tests/test_roms/nestest.nes")?;
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);

    // PC starts here (as seen in nestest.log).
    // If it's not set, the test ROM won't work properly.
    cpu.program_counter = 0xc000;

    let mut traces = Vec::with_capacity(VALID_LINES_SO_FAR);
    loop {
        traces.push(trace(&mut cpu)?);
        if cpu.step()? {
            break;
        }
    }

    let lines = read_lines("../sabi-nes/tests/expected_logs/nestest.log")?;
    let expected_traces = lines.zip(traces).enumerate().take(VALID_LINES_SO_FAR);

    for (line_num, (expected_trace, actual_trace)) in expected_traces {
        let expected_trace = expected_trace?;

        assert_eq!(
            expected_trace,
            actual_trace,
            "Failed at line#{}",
            line_num + 1
        );
    }

    Ok(())
}
