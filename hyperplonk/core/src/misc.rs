use ark_ff::Field;

pub fn index_to_point<F: Field>(vars: usize, point: usize) -> Vec<F> {
    assert!(vars > 0);
    let mut p = Vec::with_capacity(vars);
    for i in 0..vars {
        let var = ((0b1 << i) & point).count_ones();
        p.push(F::from(var))
    }
    p
}

#[allow(dead_code)]
pub fn print_row<F: Field, const C: usize>(row: &[F; C]) {
    println!("\n printing rows:");
    for (i, e) in row.iter().enumerate() {
        println!("w{i}:{e}");
    }
}
