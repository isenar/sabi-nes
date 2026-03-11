use derive_more::{Add, Display, Div, From, Index, LowerHex, Mul, Sub, UpperHex};
use std::ops::{Add, Sub};

#[repr(transparent)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    From,
    Div,
    Index,
    Mul,
    Sub,
    Add,
    Display,
    LowerHex,
    UpperHex,
)]
pub struct Byte(u8);

impl Byte {
    pub const fn new(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}

impl PartialEq<Address> for Byte {
    fn eq(&self, other: &Address) -> bool {
        self.0 as u16 == other.0
    }
}

#[repr(transparent)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    From,
    Div,
    Index,
    Mul,
    Sub,
    Add,
    Display,
    LowerHex,
    UpperHex,
)]
pub struct Address(u16);

impl Address {
    pub const fn new(address: u16) -> Self {
        Self(address)
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub fn wrapping_add(&self, value: impl Into<u16>) -> Self {
        Self::new(self.0.wrapping_add(value.into()))
    }

    pub fn wrapping_sub(&self, value: impl Into<u16>) -> Self {
        Self::new(self.0.wrapping_sub(value.into()))
    }
}

impl PartialEq<Byte> for Address {
    fn eq(&self, other: &Byte) -> bool {
        self.0 == other.0 as u16
    }
}

impl From<u8> for Address {
    fn from(byte: u8) -> Self {
        Self(byte as u16)
    }
}

impl From<Address> for usize {
    fn from(address: Address) -> usize {
        address.0 as usize
    }
}

impl Add<u16> for Address {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<u16> for Address {
    type Output = Self;

    fn sub(self, rhs: u16) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub<Address> for u16 {
    type Output = Address;

    fn sub(self, rhs: Address) -> Self::Output {
        Address::new(self) - rhs
    }
}

impl PartialEq<u16> for Address {
    fn eq(&self, other: &u16) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u16> for Address {
    fn partial_cmp(&self, other: &u16) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other)
    }
}
