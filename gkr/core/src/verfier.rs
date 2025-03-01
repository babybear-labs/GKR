use crate::circuit::circuit::{Circuit, FieldElem};

pub struct Verifier<'a> {
    circuit: &'a Circuit,
    pub claimed_output: FieldElem,
}

impl<'a> Verifier<'a> {
    pub fn new(circuit: &'a Circuit, claimed_output: FieldElem) -> Self {
        Verifier { circuit, claimed_output }
    }
}