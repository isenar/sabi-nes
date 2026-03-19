use derive_more::{
    Add, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, Display, Div, From, LowerHex, Shl,
    ShlAssign, Shr, ShrAssign, Sub, UpperHex,
};
use std::ops::{Add, AddAssign, BitAnd, BitAndAssign, BitOrAssign, Shl, Sub, SubAssign};

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
    Default,
    From,
    Add,
    BitOr,
    BitOrAssign,
    BitAnd,
    BitAndAssign,
    BitXor,
    Shr,
    ShrAssign,
    Shl,
    ShlAssign,
    Display,
    LowerHex,
    UpperHex,
)]
pub struct Byte(u8);

impl Byte {
    pub const fn new(byte: u8) -> Self {
        Self(byte)
    }

    pub const fn from_word_lossy(word: Word) -> Self {
        Self::new(word.0 as u8)
    }

    pub const fn value(self) -> u8 {
        self.0
    }

    pub const fn as_word(self) -> Word {
        Word::new(self.0 as u16)
    }

    pub const fn as_address(self) -> Address {
        Address::new(self.0 as u16)
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub const fn wrapping_add(&self, value: u8) -> Self {
        Self::new(self.0.wrapping_add(value))
    }

    pub const fn wrapping_sub(&self, value: u8) -> Self {
        Self::new(self.0.wrapping_sub(value))
    }
}

impl PartialEq<Address> for Byte {
    fn eq(&self, other: &Address) -> bool {
        self.0 as u16 == other.0
    }
}

impl PartialEq<u8> for Byte {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for Byte {
    fn partial_cmp(&self, other: &u8) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl BitAnd<u8> for Byte {
    type Output = Self;

    fn bitand(self, rhs: u8) -> Self::Output {
        Self::new(self.0 & rhs)
    }
}

impl BitOrAssign<u8> for Byte {
    fn bitor_assign(&mut self, rhs: u8) {
        self.0 |= rhs;
    }
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
    From,
    Sub,
    Add,
    Display,
    LowerHex,
    UpperHex,
)]
pub struct Word(u16);

impl Word {
    pub const fn new(word: u16) -> Self {
        Self(word)
    }

    pub const fn from_le_bytes(low: Byte, high: Byte) -> Self {
        Self::new(u16::from_le_bytes([low.value(), high.value()]))
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub const fn as_address(self) -> Address {
        Address::new(self.0)
    }

    pub fn to_le_bytes(self) -> [Byte; 2] {
        let [low, high] = self.0.to_le_bytes();
        [Byte::new(low), Byte::new(high)]
    }

    pub fn wrapping_add(&self, value: impl Into<u16>) -> Self {
        Self::new(self.0.wrapping_add(value.into()))
    }
}

impl PartialEq<u16> for Word {
    fn eq(&self, other: &u16) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u16> for Word {
    fn partial_cmp(&self, other: &u16) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl Add<u16> for Word {
    type Output = Self;

    fn add(self, rhs: u16) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u16> for Word {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs;
    }
}

impl SubAssign<u16> for Word {
    fn sub_assign(&mut self, rhs: u16) {
        self.0 -= rhs;
    }
}

impl Shl<u16> for Word {
    type Output = Self;

    fn shl(self, rhs: u16) -> Self::Output {
        Self::new(self.0 << rhs)
    }
}

#[repr(transparent)]
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    From,
    Div,
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

    pub const fn as_word(self) -> Word {
        Word::new(self.0)
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

impl From<Byte> for Address {
    fn from(byte: Byte) -> Self {
        Self(byte.0 as u16)
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

impl Add<Byte> for Address {
    type Output = Self;

    fn add(self, rhs: Byte) -> Self::Output {
        Self(self.0 + rhs.0 as u16)
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
        self.0.partial_cmp(other)
    }
}

impl BitAnd<u16> for Address {
    type Output = Self;

    fn bitand(self, rhs: u16) -> Self::Output {
        Self::new(self.0 & rhs)
    }
}

impl BitAndAssign<u16> for Address {
    fn bitand_assign(&mut self, rhs: u16) {
        self.0 &= rhs;
    }
}

impl AddAssign<u16> for Address {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs;
    }
}
