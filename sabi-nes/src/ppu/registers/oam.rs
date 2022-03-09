use crate::utils::NthBit;
use crate::Byte;

const OAM_DATA_SIZE: usize = 64;
const SPRITE_DATA_SIZE: Byte = std::mem::size_of::<SpriteData>() as Byte;

/// Internal memory to keep state of sprites (Object Attribute Memory)
#[derive(Debug)]
pub struct Oam {
    sprites: [SpriteData; OAM_DATA_SIZE],
    address: Byte,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SpriteData {
    pub x: Byte,
    pub y: Byte,
    pub index_number: Byte,
    pub attributes: Byte,
}

impl SpriteData {
    pub fn x_pos(&self, x_offset: usize) -> usize {
        if self.flip_horizontally() {
            self.x as usize + 7 - x_offset
        } else {
            self.x as usize + x_offset
        }
    }

    pub fn y_pos(&self, y_offset: usize) -> usize {
        if self.flip_vertically() {
            self.y as usize + 7 - y_offset
        } else {
            self.y as usize + y_offset
        }
    }

    #[inline]
    pub fn palette_index(&self) -> Byte {
        self.attributes & 0b11
    }

    #[inline]
    /// 0 - in front of background
    /// 1 - behind background
    pub fn priority(&self) -> bool {
        self.attributes.nth_bit(5)
    }
    #[inline]
    pub fn flip_horizontally(&self) -> bool {
        self.attributes.nth_bit(6)
    }

    #[inline]
    pub fn flip_vertically(&self) -> bool {
        self.attributes.nth_bit(7)
    }
}

impl Default for Oam {
    fn default() -> Self {
        Self {
            sprites: [SpriteData::default(); OAM_DATA_SIZE],
            address: Default::default(),
        }
    }
}

impl Oam {
    pub fn read(&self) -> Byte {
        let sprite_data = &self.sprites[self.address.div_euclid(SPRITE_DATA_SIZE) as usize];
        let sprite_data_index = self.address % SPRITE_DATA_SIZE;

        match sprite_data_index {
            0 => sprite_data.y,
            1 => sprite_data.index_number,
            2 => sprite_data.attributes,
            3 => sprite_data.x,
            _ => unreachable!(),
        }
    }

    pub fn read_all(&self) -> &[SpriteData] {
        &self.sprites
    }

    pub fn write(&mut self, value: Byte) {
        let sprite_data = &mut self.sprites[self.address.div_euclid(SPRITE_DATA_SIZE) as usize];
        let sprite_data_index = self.address % SPRITE_DATA_SIZE;

        match sprite_data_index {
            0 => sprite_data.y = value,
            1 => sprite_data.index_number = value,
            2 => sprite_data.attributes = value,
            3 => sprite_data.x = value,
            _ => unreachable!(),
        }

        self.address = self.address.wrapping_add(1);
    }

    pub fn write_address(&mut self, address: Byte) {
        self.address = address;
    }

    pub fn write_all(&mut self, buffer: &[Byte]) {
        for byte in buffer {
            self.write(*byte);
        }
    }
}
