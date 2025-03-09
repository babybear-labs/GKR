use crate::hyperplonk::Gates;
use ark_ff::Field;

///sample gate to enforce third element to product of first two
pub fn product<F: Field, const COLS: usize>(input: &[F; COLS]) -> F {
    assert!(COLS > 2);
    let u = input[0] * input[1] - input[2];
    if !u.is_zero() {
        println!("WRONG");
    }
    u
}

pub struct Product;
impl<const COLS: usize> Gates<COLS, 1> for Product {
    fn function<F>(vars: &[F; COLS]) -> [F; 1]
    where
        F: crate::function_sumcheck::Element<Out = F>,
        for<'a> &'a F: crate::function_sumcheck::Element<Out = F>,
    {
        assert!(COLS > 2);
        let u = &(&vars[0] * &vars[1]) - &vars[2];
        [u]
    }
}

pub struct Sum;
impl<const COLS: usize> Gates<COLS, 1> for Sum {
    fn function<F>(vars: &[F; COLS]) -> [F; 1]
    where
        F: crate::function_sumcheck::Element<Out = F>,
        for<'a> &'a F: crate::function_sumcheck::Element<Out = F>,
    {
        assert!(COLS > 2);
        let u = &(&vars[0] + &vars[1]) - &vars[2];
        [u]
    }
}

pub struct Sub;
impl<const COLS: usize> Gates<COLS, 1> for Sub {
    fn function<F>(vars: &[F; COLS]) -> [F; 1]
    where
        F: crate::function_sumcheck::Element<Out = F>,
        for<'a> &'a F: crate::function_sumcheck::Element<Out = F>,
    {
        assert!(COLS > 2);
        let u = &(&vars[0] - &vars[1]) - &vars[2];
        [u]
    }
}
