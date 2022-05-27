#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NmiStatus {
    Active,
    Inactive,
}

impl NmiStatus {
    pub fn activated(before: Self, after: Self) -> bool {
        before == Self::Inactive && after == Self::Active
    }
}
