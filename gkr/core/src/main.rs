mod circuit {
    const P: u64 = 101;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct FieldElem(u64);

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

    #[derive(Clone, Debug)]
    pub enum Gate {
        Add(usize, usize),
        Mul(usize, usize),
    }

    #[derive(Clone, Debug)]
    pub struct Layer {
        gates: Vec<Gate>,
    }

    #[derive(Clone, Debug)]
    pub struct Circuit {
        layers: Vec<Layer>,
        input_size: usize
    }

    #[derive(Debug)]
    pub enum CircuitError {
        InvalidInputSize(usize, usize), // Expected, provided
        InvalidGateIndex(usize, usize), // Gate index, layer size
    }

    impl Circuit {
        pub fn new(input_size: usize) -> Self {
            Circuit { layers: Vec::new(), input_size }
        }

        pub fn add_layer(&mut self, gates: Vec<Gate>) {
            self.layers.push(Layer { gates });
        }

        pub fn evaluate(&self, inputs: &[FieldElem]) -> Result<Vec<FieldElem>, CircuitError> {
            if inputs.len() != self.input_size {
                return Err(CircuitError::InvalidInputSize(self.input_size, inputs.len()));
            }

            let mut values = inputs.to_vec();
            for (layer_idx, layer) in self.layers.iter().enumerate() {
                let mut next_values = Vec::new();
                for gate in &layer.gates {
                    match gate {
                        Gate::Add(i, j) => {
                            if *i >= values.len() || *j >= values.len() {
                                return Err(CircuitError::InvalidGateIndex(
                                    *i.max(j),
                                    values.len(),
                                ));
                            }
                            let sum = values[*i].add(values[*j]);
                            next_values.push(sum);
                        }
                        Gate::Mul(i, j) => {
                            if *i >= values.len() || *j >= values.len() {
                                return Err(CircuitError::InvalidGateIndex(
                                    *i.max(j),
                                    values.len(),
                                ));
                            }
                            let prod = values[*i].mul(values[*j]);
                            next_values.push(prod);
                        }
                    }
                }
                values = next_values;
            }
            Ok(values)
        }
    }
}

use circuit::*;

fn main() {
    // circuit: (1 + 2) * (3 + 4)
    let mut circuit = Circuit::new();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_arithmetic() {
        let a = FieldElem::new(50);
        let b = FieldElem::new(60);

        assert_eq!(a.add(b), FieldElem::new(9)); // 50 + 60 = 110 ≡ 9 (mod 101)
        assert_eq!(a.mul(b), FieldElem::new(95)); // 50 * 60 = 3000 ≡ 95 (mod 101)

    }

    #[test]
    fn test_circuit_evaluation() {
        let mut circuit = Circuit::new(4);
        circuit.add_layer(vec![Gate::Add(0, 1), Gate::Add(2, 3)]);
        circuit.add_layer(vec![Gate::Mul(0, 1)]);

        let inputs = vec![
            FieldElem::new(1),
            FieldElem::new(2),
            FieldElem::new(3),
            FieldElem::new(4),
        ];
        let output = circuit.evaluate(&inputs).unwrap();
        assert_eq!(output, vec![FieldElem::new(21)]); // (1 + 2) * (3 + 4) = 3 * 7 = 21
    }

    #[test]
    fn test_invalid_input_size() {
        let circuit = Circuit::new(4);
        let inputs = vec![FieldElem::new(1), FieldElem::new(2)];
        let result = circuit.evaluate(&inputs);
        assert!(matches!(
            result,
            Err(CircuitError::InvalidInputSize(4, 2))
        ));
    }

    #[test]
    fn test_invalid_gate_index() {
        let mut circuit = Circuit::new(2);
        circuit.add_layer(vec![Gate::Add(0, 2)]); // Index 2 is out of bounds
        let inputs = vec![FieldElem::new(1), FieldElem::new(2)];
        let result = circuit.evaluate(&inputs);
        assert!(matches!(
            result,
            Err(CircuitError::InvalidGateIndex(2, 2))
        ));
    }
}