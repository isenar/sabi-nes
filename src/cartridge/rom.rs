use crate::Byte;
use crate::cartridge::mappers::{Mapper, Nrom128, Nrom256};
use crate::cartridge::{CHR_ROM_BANK_SIZE, MirroringType, PRG_ROM_BANK_SIZE};
use anyhow::{Result, anyhow, bail};
use bitflags::bitflags;

/// "NES" followed by MS-DOS end-of-file used to recognize .NES (iNES) files
const NES_TAG: [Byte; 4] = [0x4e, 0x45, 0x53, 0x1a];

bitflags! {
    #[derive(Debug, Copy, Clone)]
    struct ControlByte1: Byte {
        const MIRRORING               = 0b0000_0001; // 1 for vertical, 0 for horizontal
        const BATTERY_BACKED_RAM      = 0b0000_0010;
        const HAS_TRAINER             = 0b0000_0100;
        const FOUR_SCREEN_VRAM_LAYOUT = 0b0000_1000;
        const MAPPER_TYPE1            = 0b0001_0000; // first bit of mapper type
        const MAPPER_TYPE2            = 0b0010_0000; // second bit of mapper type
        const MAPPER_TYPE3            = 0b0100_0000; // third bit of mapper type
        const MAPPER_TYPE4            = 0b1000_0000; // fourth bit of mapper type
    }
}

impl ControlByte1 {
    pub fn mapper_bits_lo(&self) -> Byte {
        self.bits() >> 4
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    struct ControlByte2: Byte {
        const INES_V1_FIRST   = 0b0000_0001; // 0 for iNES v1 format
        const INES_V1_SECOND  = 0b0000_0010; // 0 for iNES v1 format
        const INES_FMT_FIRST  = 0b0000_0100; // if INES_FMT bits are == 10, then it's NES2.0 format,
        const INES_FMT_SECOND = 0b0000_1000; // if they are == 00, then it's iNES v1 format
        const MAPPER_TYPE5    = 0b0001_0000; // fifth bit of mapper type
        const MAPPER_TYPE6    = 0b0010_0000; // sixth bit of mapper type
        const MAPPER_TYPE7    = 0b0100_0000; // seventh bit of mapper type
        const MAPPER_TYPE8    = 0b1000_0000; // eighth bit of mapper type

        const MAPPER_MASK     = 0b1111_0000;
    }
}

impl ControlByte2 {
    pub fn mapper_bits_hi(&self) -> Byte {
        (*self & Self::MAPPER_MASK).bits()
    }
}

#[derive(Debug)]
struct RomHeader {
    /// Number of 16kB ROM banks (PRG ROM)
    pub prg_rom_banks: usize,
    /// Number o 8kB VROM banks (CHR ROM)
    pub chr_rom_banks: usize,
    pub control_byte1: ControlByte1,
    pub control_byte2: ControlByte2,
    /// Size of PRG RAM in 8kB units
    #[allow(unused)]
    pub prg_ram_units: usize,
}

impl TryFrom<&[Byte]> for RomHeader {
    type Error = anyhow::Error;

    fn try_from(data: &[Byte]) -> Result<Self> {
        Self::validate(data)?;

        Ok(Self {
            prg_rom_banks: data[4].into(),
            chr_rom_banks: data[5].into(),
            control_byte1: ControlByte1::from_bits_truncate(data[6]),
            control_byte2: ControlByte2::from_bits_truncate(data[7]),
            prg_ram_units: data[8].into(),
        })
    }
}

impl RomHeader {
    fn validate(data: &[Byte]) -> Result<()> {
        if data[0..4] != NES_TAG {
            bail!("File is not an iNES format - missing 'NES' tag");
        }

        let is_ines1 = ((data[7] >> 2) & 0b11) == 0;

        if !is_ines1 {
            bail!("Only iNes 1.0 format is currently supported");
        }

        if !data[8..16].iter().all(|&byte| byte == 0) {
            bail!("last 8 bytes of the header are not 0s");
        }

        Ok(())
    }

    fn mapper(&self) -> Result<Box<dyn Mapper>> {
        let ines_mapper_id =
            self.control_byte1.mapper_bits_lo() | self.control_byte2.mapper_bits_hi();
        Ok(match ines_mapper_id {
            0 => {
                if self.prg_rom_banks == 1 {
                    Box::new(Nrom128 {})
                } else {
                    Box::new(Nrom256 {})
                }
            }
            _ => bail!("Unsupported mapper type (ID: {ines_mapper_id})"),
        })
    }
}

pub struct Rom {
    pub prg_rom: Vec<Byte>,
    pub chr_rom: Vec<Byte>,
    pub mapper: Box<dyn Mapper>,
    pub screen_mirroring: MirroringType,
}

impl Rom {
    pub fn new(data: &[Byte]) -> Result<Self> {
        let header: RomHeader = data
            .get(0..16)
            .ok_or_else(|| anyhow!("Failed to parse first 16 bytes for header"))?
            .try_into()?;

        let four_screen = header
            .control_byte1
            .contains(ControlByte1::FOUR_SCREEN_VRAM_LAYOUT);
        let vertical_mirroring = header.control_byte1.contains(ControlByte1::MIRRORING);
        let screen_mirroring = MirroringType::new(four_screen, vertical_mirroring);
        let mapper = header.mapper()?;

        let skip_trainer = header.control_byte1.contains(ControlByte1::HAS_TRAINER);
        let prg_rom_size = header.prg_rom_banks * PRG_ROM_BANK_SIZE;
        let chr_rom_size = header.chr_rom_banks * CHR_ROM_BANK_SIZE;
        let prg_rom_start = 16 + skip_trainer as usize * 512;
        let chr_rom_start = prg_rom_start + prg_rom_size;

        let prg_rom: Vec<_> = data
            .get(prg_rom_start..(prg_rom_start + prg_rom_size))
            .ok_or_else(|| anyhow!("Failed to retrieve PRG ROM data - not enough bytes"))?
            .into();
        let chr_rom: Vec<_> = data
            .get(chr_rom_start..(chr_rom_start + chr_rom_size))
            .ok_or_else(|| anyhow!("Failed to retrieve CHR ROM data - not enough bytes"))?
            .into();

        Ok(Self {
            prg_rom,
            chr_rom,
            mapper,
            screen_mirroring,
        })
    }
}
