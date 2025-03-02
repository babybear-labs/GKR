use std::fmt;
use std::ops::{Add, Mul};

const P: u64 = 101;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldElem(pub(crate) u64);

impl FieldElem {
    pub fn new(value: u64) -> Self {
        FieldElem(value % P)
    }

    pub fn add(self, other: Self) -> Self {
        FieldElem((self.0 + other.0) % P)
    }

    pub fn mul(self, other: Self) -> Self {
        FieldElem((self.0 * other.0) % P)
    }
}

impl Add for FieldElem {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.add(other)
    }
}

impl Mul for FieldElem {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        self.mul(other)
    }
}

impl fmt::Display for FieldElem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}