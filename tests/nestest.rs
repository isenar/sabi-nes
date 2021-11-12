use sabi_nes::Result;

mod common;

#[test]
fn basic_cpu_test() -> Result<()> {
    let test_rom_bytes = std::fs::read("tests/test_roms/nestest.nes")?;

    println!("len: {}", test_rom_bytes.len());

    Ok(())
}
