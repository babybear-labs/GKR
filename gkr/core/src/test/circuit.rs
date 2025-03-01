
#[cfg(test)]
pub mod tests {
    use crate::circuit::circuit::{Circuit, CircuitError, FieldElem, Gate};
    use super::*;

    #[test]
    fn test_field_arithmetic() {
        let a = FieldElem::new(5);
        let b = FieldElem::new(6);

        assert_eq!(a.add(b), FieldElem::new(11));
        assert_eq!(a.mul(b), FieldElem::new(30));
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