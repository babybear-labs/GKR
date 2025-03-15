// field.rs
use ark_ff::{Field, PrimeField};
use ark_bn254::Fr; // BN254 scalar field
use std::ops::{Add, Mul, Sub, Neg};

/// A wrapper around the field element for HyperPlonk.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldElement(Fr);

impl FieldElement {
    pub fn new(value: Fr) -> Self {
        Self(value)
    }

    pub fn zero() -> Self {
        Self(Fr::zero())
    }

    pub fn one() -> Self {
        Self(Fr::one())
    }

    pub fn from_u64(value: u64) -> Self {
        Self(Fr::from(value))
    }

    pub fn add(&self, other: &Self) -> Self {
        Self(self.0 + other.0)
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self(self.0 * other.0)
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self(self.0 - other.0)
    }

    pub fn neg(&self) -> Self {
        Self(-self.0)
    }

    pub fn inv(&self) -> Option<Self> {
        self.0.inverse().map(Self)
    }
}

// Operator overloads for convenience
impl Add for FieldElement {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.add(&rhs)
    }
}

impl Mul for FieldElement {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.mul(&rhs)
    }
}

impl Sub for FieldElement {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.sub(&rhs)
    }
}

impl Neg for FieldElement {
    type Output = Self;
    fn neg(self) -> Self {
        self.neg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_ops() {
        let a = FieldElement::from_u64(2);
        let b = FieldElement::from_u64(3);
        assert_eq!(a.add(&b), FieldElement::from_u64(5));
        assert_eq!(a.mul(&b), FieldElement::from_u64(6));
        assert_eq!(a.sub(&b), FieldElement::from_u64(2 - 3));
        assert_eq!(a.inv().unwrap().mul(&a), FieldElement::one());
    }
}