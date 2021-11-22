use sabi_nes::Result;

mod common;

#[test]
fn cpu_validation_test() -> Result<()> {
    // TODO
    let _test_rom_bytes = std::fs::read("tests/test_roms/nestest.nes")?;

    Ok(())
}
