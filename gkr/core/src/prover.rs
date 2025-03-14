use crate::circuit::{Circuit, CircuitError};
use crate::field::FieldElem;

pub struct Prover<'a> {
    pub circuit: &'a Circuit,
    pub(crate) inputs: Vec<FieldElem>,
    pub(crate) witness: Vec<FieldElem>, // Values at each layer
}

impl<'a> Prover<'a> {
    pub fn new(circuit: &'a Circuit, inputs: Vec<FieldElem>) -> Result<Self, CircuitError> {
        let witness = circuit.compute_witness(&inputs)?;
        Ok(Prover { circuit, inputs, witness })
    }
}