use crate::circuit::Gate;
use crate::field::FieldElem;
use crate::prover::Prover;

#[derive(Debug)]
enum ProtocolError {
    InvalidSum,
    FinalCheckFailed,
}

type Polynomial = Vec<FieldElem>; // Coefficients of a univariate polynomial

// Horner's method. Ref: gkr/core/docs/Terms.md
fn evaluate_polynomial(poly: &Polynomial, x: FieldElem) -> FieldElem {
    poly.iter().rev().fold(FieldElem::new(0), |acc, &coeff| {
        acc * x + coeff
    })
}