use log::{debug, info};
use crate::circuit::Gate;
use crate::field::{FieldElem, P};
use crate::prover::Prover;
use crate::verfier::Verifier;

#[derive(Debug)]
pub enum SumCheckError {
    IndexOutOfBounds(String),
    InvalidSum(String),
    FinalCheckFailed,
}
type Polynomial = Vec<FieldElem>; // Coefficients of a uni variate polynomial

// Horner's method. Ref: gkr/core/docs/Terms.md
fn evaluate_polynomial(poly: &Polynomial, x: FieldElem) -> FieldElem {
    poly.iter().rev().fold(FieldElem::new(0), |acc, &coeff| {
        acc * x + coeff
    })
}

/// Computes the uni variate polynomial g_i(x_i) for a layer.
fn compute_layer_polynomial(
    prover: &Prover,
    layer_index: usize,
    fixed_vars: &[FieldElem],
    x: FieldElem,
) -> Result<FieldElem, String> {
    let layer = &prover.circuit.layers[layer_index];
    let num_gates = layer.gates.len();
    let num_vars = (num_gates as f64).log2().ceil() as usize;

    // Ensure the layer size is a power of 2 (simplification for GKR)
    if num_gates != 1 << num_vars {
        return Err(format!(
            "Layer {} has {} gates, expected a power of 2",
            layer_index, num_gates
        ));
    }

    // Check that fixed_vars length is valid
    if fixed_vars.len() >= num_vars {
        return Err(format!(
            "Too many fixed variables: {} >= {}",
            fixed_vars.len(),
            num_vars
        ));
    }

    let prev_layer = &prover.witness[layer_index - 1];
    let mut sum = FieldElem::new(0);

    // Iterate over all possible assignments to the remaining variables
    let remaining_vars = num_vars - fixed_vars.len() - 1; // -1 for x_j
    let num_iterations = 1 << remaining_vars;

    for i in 0..num_iterations {
        let mut point = fixed_vars.to_vec();
        point.push(x); // Current variable x_j

        // Fill in remaining variables based on binary representation of i
        for bit in 0..remaining_vars {
            let bit_value = (i >> bit) & 1;
            point.push(FieldElem::new(bit_value));
        }

        // Compute gate index from the point
        let gate_idx = point.iter().enumerate().fold(0, |acc, (j, &p)| {
            acc + if p.0 == 1 { 1 << (num_vars - 1 - j) } else { 0 }
        });

        if gate_idx >= num_gates {
            return Err(format!(
                "Computed gate index {} exceeds layer size {}",
                gate_idx, num_gates
            ));
        }

        // Evaluate the gate
        let gate = &layer.gates[gate_idx];
        match gate {
            Gate::Add(a, b) => {
                if a >= prev_layer.len() || b >= prev_layer.len() {
                    return Err(format!(
                        "Invalid indices {} or {} for prev_layer size {}",
                        a, b, prev_layer.len()
                    ));
                }
                sum = sum + (prev_layer[a] + prev_layer[b]);
            }
            Gate::Mul(a, b) => {
                if a >= prev_layer.len() || b >= prev_layer.len() {
                    return Err(format!(
                        "Invalid indices {} or {} for prev_layer size {}",
                        a, b, prev_layer.len()
                    ));
                }
                sum = sum + (prev_layer[a] * prev_layer[b]);
            }
        }
    }

    Ok(sum)
}

/// Generates the polynomial g_i(x) for the current round.
fn generate_sum_check_polynomial(
    prover: &Prover,
    layer_index: usize,
    fixed_vars: &[FieldElem],
) -> Result<Polynomial, SumCheckError> {
    let modulus = prover.witness[0][0].modulus;
    // Evaluate at x = 0 and x = 1 to construct a linear polynomial
    let g0 = compute_layer_polynomial(prover, layer_index, fixed_vars, FieldElem::new(0))?;
    let g1 = compute_layer_polynomial(prover, layer_index, fixed_vars, FieldElem::new(1))?;
    // g(x) = g0 + (g1 - g0)x
    Ok(vec![g0, g1 - g0])
}

fn sum_check_protocol(
    prover: &mut Prover,
    verifier: &mut Verifier,
    layer_index: usize,
    claimed_sum: FieldElem,
) -> Result<bool, SumCheckError> {
    use rand::Rng;
    let mut rng = rand::rng();
    let num_gates = verifier.circuit.layers[layer_index].len();
    let num_vars = (num_gates as f64).log2().ceil() as usize;
    let mut current_claim = claimed_sum;
    let mut fixed_vars = Vec::new();

    info!("Starting sum-check for layer {} with {} gates, {} vars", layer_index, num_gates, num_vars);

    for round in 0..num_vars {
        let g_i = generate_sum_check_polynomial(prover, layer_index, &fixed_vars)?;
        let sum = evaluate_polynomial(&g_i, FieldElem::new(0)) +
            evaluate_polynomial(&g_i, FieldElem::new(1));
        if sum != current_claim {
            return Err(SumCheckError::InvalidSum(format!(
                "Round {}: Expected sum {}, got {}", round, current_claim, sum
            )));
        }
        let r_i = FieldElem::new(rng.gen_range(0..P));
        debug!("Round {}: Challenge r_{} = {}", round, round + 1, r_i);
        fixed_vars.push(r_i);
        current_claim = evaluate_polynomial(&g_i, r_i);
    }

    let final_value = compute_layer_polynomial(prover, layer_index, &fixed_vars, fixed_vars.last().unwrap().clone())?;
    if current_claim != final_value {
        return Err(SumCheckError::FinalCheckFailed);
    }
    Ok(true)
}