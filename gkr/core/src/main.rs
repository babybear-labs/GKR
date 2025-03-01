use crate::circuit::circuit::{Circuit, FieldElem, Gate};
use crate::prover::Prover;
use crate::verfier::Verifier;

mod circuit;
mod test;
mod prover;
mod verfier;

fn main() {
    // circuit: (1 + 2) * (3 + 4)
    let mut circuit = Circuit::new(4);
    circuit.add_layer(vec![Gate::Add(0, 1), Gate::Add(2, 3)]);
    circuit.add_layer(vec![Gate::Mul(0, 1)]);

    // Define inputs
    let inputs = vec![
        FieldElem::new(1),
        FieldElem::new(2),
        FieldElem::new(3),
        FieldElem::new(4),
    ];

    // Create prover
    let prover = Prover::new(&circuit, inputs).unwrap();
    println!("Prover witness:");
    for (i, layer) in prover.witness.iter().enumerate() {
        println!("Layer {}: {:?}", i, layer);
    }

    // Create verifier with claimed output
    let claimed_output = FieldElem::new(21); // (1+2)*(3+4) = 3*7 = 21
    let verifier = Verifier::new(&circuit, claimed_output);
    println!("Verifier claimed output: {}", verifier.claimed_output.0);
}