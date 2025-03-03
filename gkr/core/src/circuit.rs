use crate::field::FieldElem;

#[derive(Clone, Debug)]
pub enum Gate {
    Add(usize, usize),
    Mul(usize, usize),
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub gates: Vec<Gate>,
}


#[derive(Clone, Debug)]
pub struct Circuit {
    pub(crate) layers: Vec<Layer>,
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

    pub fn compute_witness(&self, inputs: &[FieldElem]) -> Result<Vec<FieldElem>, CircuitError> {
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
                        let sum = values[*i] + values[*j];
                        next_values.push(sum);
                    }
                    Gate::Mul(i, j) => {
                        if *i >= values.len() || *j >= values.len() {
                            return Err(CircuitError::InvalidGateIndex(
                                *i.max(j),
                                values.len(),
                            ));
                        }
                        let prod = values[*i] * values[*j];
                        next_values.push(prod);
                    }
                }
            }
            values = next_values;
        }
        Ok(values)
    }
}