mod nrom;

use crate::{Address, Result};

pub use nrom::{Nrom128, Nrom256};

pub trait MapperId {
    const ID: u32;
}

pub trait Mapper {
    fn map_address(&self, address: Address) -> Result<Address>;
}
