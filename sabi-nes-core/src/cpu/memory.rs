use crate::{Address, Byte, Word};

pub trait Memory {
    fn read_byte(&mut self, addr: Address) -> Byte;
    fn write_byte(&mut self, addr: Address, value: Byte);

    fn read_word(&mut self, addr: Address) -> Word {
        let low = self.read_byte(addr);
        let high = self.read_byte(addr + 1);

        Word::from_le_bytes(low, high)
    }

    fn write_word(&mut self, addr: Address, word: Word) {
        let [low, high] = word.to_le_bytes();

        self.write_byte(addr, low);
        self.write_byte(addr + 1, high);
    }
}
