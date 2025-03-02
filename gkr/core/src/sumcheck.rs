use crate::field::FieldElem;

#[derive(Debug)]
enum ProtocolError {
    InvalidSum,
    FinalCheckFailed,
}

type Polynomial = Vec<FieldElem>; // Coefficients of a univariate polynomial

fn evaluate_polynomial(poly: &Polynomial, x: FieldElem) -> FieldElem {
    poly.iter().rev().fold(FieldElem::new(0), |acc, &coeff| {
        acc * x + coeff
    })
}

