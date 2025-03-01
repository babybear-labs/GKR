use crate::circuit::circuit::{Circuit, FieldElem, Gate};
mod circuit;
mod test;

fn main() {
    // circuit: (1 + 2) * (3 + 4)
    let mut circuit = Circuit::new(4);
    circuit.add_layer(vec![Gate::Add(0, 1), Gate::Add(2, 3)]);
    circuit.add_layer(vec![Gate::Mul(0, 1)]);

    // Evaluate with inputs [1, 2, 3, 4]
    let inputs = vec![
        FieldElem::new(1),
        FieldElem::new(2),
        FieldElem::new(3),
        FieldElem::new(4),
    ];
    match circuit.evaluate(&inputs) {
        Ok(output) => println!("Circuit output: {:?}", output), // Should be [21]
        Err(e) => println!("Error: {:?}", e),
    }
}