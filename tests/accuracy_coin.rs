//! Integration tests using the AccuracyCoin test ROM.
//! ROM source: https://github.com/100thCoin/AccuracyCoin (MIT License)

use sabi_nes::input::joypad::JoypadButton;
use sabi_nes::{Address, Bus, Byte, Cpu, Result, Rom};
use serde::Deserialize;

/// Result codes written by the AccuracyCoin test runner to CPU RAM.
///
/// $00 = not started
/// $03 = in progress
/// $01 = pass
/// >= $06 = fail (encoded as (error_code << 2) | 0x02)
const RESULT_NOT_STARTED: Byte = Byte::new(0x00);
const RESULT_IN_PROGRESS: Byte = Byte::new(0x03);
const RESULT_PASS: Byte = Byte::new(0x01);

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

fn load_tests() -> Vec<TestCase> {
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
    match result {
        RESULT_PASS => "pass".to_owned(),
        RESULT_IN_PROGRESS => "in progress".to_owned(),
        RESULT_NOT_STARTED => "not started".to_owned(),
        r if r & 0x03 == 0x02 => format!("failed (subtest {})", r >> 2),
        other => format!("unknown ({other:#04x})"),
    }
}

#[test]
fn accuracy_coin_tests() {
    let tests = load_tests();

    let rom = Rom::from_file("tests/test_roms/AccuracyCoin.nes").unwrap();
    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);
    cpu.reset().unwrap();

    // Let the ROM initialize and show its menu.
    run_frames(&mut cpu, 5);

    // Press Start once to trigger AutomaticallyRunEveryTestInROM.
    cpu.bus_mut().joypad_mut().press_button(JoypadButton::START);

    let mut start_released = false;

    loop {
        cpu.step().unwrap();

        if cpu.bus().is_frame_ready() {
            cpu.bus_mut().clear_frame_ready();
            if !start_released {
                cpu.bus_mut()
                    .joypad_mut()
                    .release_button(JoypadButton::START);
                start_released = true;
            }
        }

        let all_done = tests.iter().all(|t| {
            let test_result = cpu.peek_byte(Address::new(t.address));
            test_result != RESULT_NOT_STARTED && test_result != RESULT_IN_PROGRESS
        });

        if all_done {
            break;
        }
    }

    let regressions: Vec<String> = tests
        .iter()
        .filter(|t| !t.known_failure)
        .filter_map(|t| {
            let v = cpu.peek_byte(Address::new(t.address));
            if v == RESULT_PASS {
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
    let unexpected_passes: Vec<String> = tests
        .iter()
        .filter(|t| t.known_failure)
        .filter_map(|t| {
            let test_result = cpu.peek_byte(Address::new(t.address));
            (test_result == RESULT_PASS).then(|| format!("  '{}' (${:04X})", t.name, t.address))
        })
        .collect();

    if !unexpected_passes.is_empty() {
        eprintln!(
            "{} known-failure test(s) now pass — update known_failure in accuracycoin_tests.csv:\n{}",
            unexpected_passes.len(),
            unexpected_passes.join("\n"),
        );
    }

    let total = tests.len();
    let known_failures = tests.iter().filter(|t| t.known_failure).count();
    let passed = tests
        .iter()
        .filter(|t| cpu.peek_byte(Address::new(t.address)) == RESULT_PASS)
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
