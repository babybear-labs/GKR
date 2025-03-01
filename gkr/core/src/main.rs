const P: u64 = 101;

#[derive(Clone, Copy, Debug, PartialEq)]
struct FieldElem(u64);

impl FieldElem {
    fn new(value: u64) -> Self {
        FieldElem(value % P)
    }

    fn add(self, other: Self) -> Self {
        FieldElem((self.0 + other.0) % P)
    }

    fn mul(self, other: Self) -> Self {
        FieldElem((self.0 * other.0) % P)
    }
}

#[derive(Clone, Debug)]
enum Gate {
    Add(usize, usize),
    Mul(usize, usize),
}

#[derive(Clone, Debug)]
struct Layer {
    gates: Vec<Gate>,
}

#[derive(Clone, Debug)]
struct Circuit {
    layers: Vec<Layer>,
}

impl Circuit {
    fn new() -> Self {
        Circuit { layers: Vec::new() }
    }

    fn add_layer(&mut self, gates: Vec<Gate>) {
        self.layers.push(Layer { gates });
    }

    fn evaluate(&self, inputs: &[FieldElem]) -> Vec<FieldElem> {
        let mut values = inputs.to_vec();
        for layer in &self.layers {
            let mut next_values = Vec::new();
            for gate in &layer.gates {
                match gate {
                    Gate::Add(i, j) => {
                        let sum = values[*i].add(values[*j]);
                        next_values.push(sum);
                    }
                    Gate::Mul(i, j) => {
                        let prod = values[*i].mul(values[*j]);
                        next_values.push(prod);
                    }
                }
            }
            values = next_values;
        }
        values
    }
}

fn main() {
    // circuit: (1 + 2) * (3 + 4)
    let mut circuit = Circuit::new();

    // Layer 1: Two addition gates
    circuit.add_layer(vec![Gate::Add(0, 1), Gate::Add(2, 3)]); // w0 = a + b, w1 = c + d
    // Layer 2: One multiplication gate
    circuit.add_layer(vec![Gate::Mul(0, 1)]); // v = w0 * w1

    // Define inputs
    let inputs = vec![
        FieldElem::new(1), // a
        FieldElem::new(2), // b
        FieldElem::new(3), // c
        FieldElem::new(4), // d
    ];

    // Evaluate the circuit
    let output = circuit.evaluate(&inputs);
    println!("Circuit output: {:?}", output); // Expected: [21], since (1+2)*(3+4) = 3*7 = 21
}