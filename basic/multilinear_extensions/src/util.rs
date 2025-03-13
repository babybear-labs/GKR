/// Decompose an integer into a binary vector in little endian.
/// Input (u64)	num_var	Binary (little-endian)	Output (Vec<bool>)
/// 5	3	101	[true, false, true]
/// 13	3	101 (from 1101)	[true, false, true]
pub fn bit_decompose(input: u64, num_var: usize) -> Vec<bool> {
    let mut res = Vec::with_capacity(num_var);
    let mut i = input;
    for _ in 0..num_var {
        res.push(i & 1 == 1);
        i >>= 1;
    }
    res
}
