//! Integration tests using the AccuracyCoin test ROM.
//! ROM source: https://github.com/100thCoin/AccuracyCoin (MIT License)

use sabi_nes::input::joypad::JoypadButton;
use sabi_nes::{Address, Bus, Byte, Cpu, Result, Rom};
use serde::Deserialize;

/// Result codes written by the AccuracyCoin test runner to CPU RAM.
///
/// Results are encoded in the bottom 2 bits:
///   xx_00 = not started  (only $00 in practice)
///   xx_01 = pass         ($01 standard, $41 "revision G", etc.)
///   xx_10 = fail         (error_code << 2) | 0x02; error_code >= 1
///   xx_11 = in progress  (only $03 in practice)
const RESULT_NOT_STARTED: Byte = Byte::new(0x00);
const RESULT_IN_PROGRESS: Byte = Byte::new(0x03);

fn has_passed(result: Byte) -> bool {
    result.value() & 0x03 == 0x01
}

#[derive(Deserialize)]
struct TestCase {
    name: String,
    #[serde(deserialize_with = "deserialize_hex_u16")]
    address: u16,
    known_failure: bool,
}

fn deserialize_hex_u16<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u16, D::Error> {
    let s: String = serde::Deserialize::deserialize(d)?;
    let hex = s.trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(hex, 16).map_err(serde::de::Error::custom)
}

fn load_test_case_data() -> Vec<TestCase> {
    let csv = include_str!("test_data/accuracy_coin_test_cases.csv");
    csv::Reader::from_reader(csv.as_bytes())
        .deserialize()
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to parse accuracycoin_tests.csv")
}

fn run_frames(cpu: &mut Cpu, frames: usize) {
    let mut completed = 0;
    while completed < frames {
        cpu.step().unwrap();
        if cpu.bus().is_frame_ready() {
            cpu.bus_mut().clear_frame_ready();
            completed += 1;
        }
    }
}

fn decode_result(result: Byte) -> String {
    match result.value() & 0x03 {
        0x01 => format!("pass ({:#04x})", result.value()),
        0x02 => format!("failed (subtest {})", result.value() >> 2),
        _ if result == RESULT_IN_PROGRESS => "in progress".to_owned(),
        _ if result == RESULT_NOT_STARTED => "not started".to_owned(),
        _ => format!("unknown ({:#04x})", result.value()),
    }
}

#[test]
fn accuracy_coin_tests() {
    let test_cases = load_test_case_data();

    let rom = Rom::from_file("tests/test_roms/AccuracyCoin.nes").unwrap();
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);
    cpu.reset().unwrap();

    // Make sure the test ROM is initialised properly and ready to take input.
    run_frames(&mut cpu, 5);

    // Press Start once to trigger AutomaticallyRunEveryTestInROM.
    cpu.bus_mut().joypad_mut().press_button(JoypadButton::START);

    loop {
        cpu.step().unwrap();
        if cpu.bus().is_frame_ready() {
            cpu.bus_mut().clear_frame_ready();
            cpu.bus_mut()
                .joypad_mut()
                .release_button(JoypadButton::START);
            break;
        }
    }

    loop {
        cpu.step().unwrap();
        if cpu.bus().is_frame_ready() {
            cpu.bus_mut().clear_frame_ready();
        }
        let all_done = test_cases.iter().all(|t| {
            let result = cpu.peek_byte(Address::new(t.address));
            result != RESULT_NOT_STARTED && result != RESULT_IN_PROGRESS
        });
        if all_done {
            break;
        }
    }

    let regressions: Vec<String> = test_cases
        .iter()
        .filter(|t| !t.known_failure)
        .filter_map(|t| {
            let v = cpu.peek_byte(Address::new(t.address));
            if has_passed(v) {
                None
            } else {
                Some(format!(
                    "  REGRESSION  '{}' (${:04X}): {:#04x} ({})",
                    t.name,
                    t.address,
                    v,
                    decode_result(v),
                ))
            }
        })
        .collect();

    // Known failures that unexpectedly passed — progress worth tracking.
    let unexpected_passes: Vec<String> = test_cases
        .iter()
        .filter(|t| t.known_failure)
        .filter_map(|t| {
            let test_result = cpu.peek_byte(Address::new(t.address));
            has_passed(test_result).then(|| format!("  '{}' (${:04X})", t.name, t.address))
        })
        .collect();

    if !unexpected_passes.is_empty() {
        eprintln!(
            "{} known-failure test(s) now pass — update known_failure in accuracycoin_tests.csv:\n{}",
            unexpected_passes.len(),
            unexpected_passes.join("\n"),
        );
    }

    let total = test_cases.len();
    let known_failures = test_cases.iter().filter(|t| t.known_failure).count();
    let passed = test_cases
        .iter()
        .filter(|t| has_passed(cpu.peek_byte(Address::new(t.address))))
        .count();
    eprintln!(
        "AccuracyCoin: {}/{} passed ({} known failures)",
        passed, total, known_failures,
    );

    assert!(
        regressions.is_empty(),
        "{} regression(s) detected:\n{}",
        regressions.len(),
        regressions.join("\n"),
    );

    assert!(
        unexpected_passes.is_empty(),
        "{} known-failure test(s) now pass — update known_failure in accuracy_coin_tests_cases.csv:\n{}",
        unexpected_passes.len(),
        unexpected_passes.join("\n"),
    );
}
