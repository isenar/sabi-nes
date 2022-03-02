use crate::Byte;

const OAM_DATA_SIZE: usize = 256;

/// Internal memory to keep state of sprites (Object Attribute Memory)
#[derive(Debug)]
pub struct Oam {
    data: [Byte; OAM_DATA_SIZE],
    address: Byte,
}

impl Default for Oam {
    fn default() -> Self {
        Self {
            data: [0; OAM_DATA_SIZE],
            address: Default::default(),
        }
    }
}

impl Oam {
    pub fn read(&self) -> Byte {
        self.data[self.address as usize]
    }

    pub fn read_all(&self) -> &[Byte] {
        &self.data
    }

    pub fn write(&mut self, value: Byte) {
        self.data[self.address as usize] = value;
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
