use crate::misc::index_to_point;
use ark_ff::Field;
use ark_poly::univariate::DensePolynomial;
use std::{
    cmp::Ordering,
    ops::{Add, Mul, Sub},
};

#[derive(Clone)]
pub struct Evaluations<F: Field> {
    evals: Vec<F>,
}
pub struct HyperPoint<F> {
    point: Vec<F>,
}

impl<F> HyperPoint<F> {
    pub fn new(point: Vec<F>) -> Self {
        Self { point }
    }
    pub fn inner(&self) -> &[F] {
        &self.point
    }
    pub fn vars(&self) -> usize {
        self.point.len()
    }
}
impl<F: Field> Sub<F> for Evaluations<F> {
    type Output = Self;

    fn sub(self, rhs: F) -> Self::Output {
        let Self { mut evals } = self;
        evals.iter_mut().for_each(|e| *e -= rhs);
        Self { evals }
    }
}
impl<F: Field> Mul<F> for Evaluations<F> {
    type Output = Self;

    fn mul(self, rhs: F) -> Self::Output {
        let Self { mut evals } = self;
        evals.iter_mut().for_each(|e| *e *= rhs);
        Self { evals }
    }
}
impl<F: Field> Add for Evaluations<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let Self { mut evals } = self;
        for (l, r) in evals.iter_mut().zip(rhs.evals.into_iter()) {
            *l = *l + r;
        }
        Evaluations { evals }
    }
}

impl<F: Field> Evaluations<F> {
    pub fn new(evals: Vec<F>) -> Self {
        assert!(evals.len().is_power_of_two());
        Self { evals }
    }
    pub fn to_vec(self) -> Vec<F> {
        assert!(self.evals.len().is_power_of_two());
        self.evals
    }
    pub fn evaluations(&self) -> &Vec<F> {
        &self.evals
    }
    pub fn vars(&self) -> usize {
        self.evals.len().ilog2() as usize
    }
    pub fn evaluate(self, point: &HyperPoint<F>) -> Result<F, &'static str> {
        let mut evals = self.evals;
        let n = evals.len();
        n.is_power_of_two()
            .then_some(())
            .ok_or("must be power of two")?;
        match 2_usize.pow(point.point.len() as u32).cmp(&n) {
            Ordering::Less => Err("too few variables"),
            Ordering::Greater => Err("to many variables"),
            _ => Ok(()),
        }?;

        let mut evals = evals.as_mut_slice();
        for p in &point.point {
            let n = evals.len();
            let (left, right) = evals.split_at_mut(n / 2);

            //println!();
            //println!("remainder2:");
            for e in left.iter_mut().zip(right.iter_mut()) {
                let (left, right) = e;
                let new_eval = *left * (F::one() - p) + *right * p;

                //println!("eval: {}", new_eval);
                *left = new_eval;
            }
            evals = left;
        }
        Ok(evals[0])
    }
}

/// unoptimized O(n log n) way to get a random polinomial for zero check
/// can be made faster and O(n)
pub fn random_poly_evals_slow<F: Field>(vars: Vec<F>) -> Vec<F> {
    assert!(vars.len() > 0);
    let evals = 1 << vars.len();
    (0..evals)
        .into_iter()
        .map(|i| {
            let p = index_to_point::<F>(vars.len(), i);
            eval_sparse_poly(&vars, p)
        })
        .collect()
}
pub fn eval_sparse_poly<F: Field>(vars: &[F], point: Vec<F>) -> F {
    assert_eq!(vars.len(), point.len());
    point
        .into_iter()
        .zip(vars.into_iter())
        .map(|(x, y)| {
            let one = F::one();
            x * y + (one - x) * (one - y)
        })
        .reduce(Mul::mul)
        .unwrap()
}

pub trait CommitmentScheme<F: Field> {
    type Multivariate: MultivariateScheme<F>;
    type Univariate: UnivariateScheme<F>;
    fn vars(&self) -> usize;
    fn multi_scheme(&self) -> &Self::Multivariate;
    fn init(vars: usize) -> Self;
}

pub trait MultivariateScheme<F: Field> {
    type Commitment: Add<Output = Self::Commitment> + Mul<F, Output = Self::Commitment> + Copy;
    type OpenProof;
    fn commitment_to_bits(commitment: Self::Commitment) -> Vec<u8>;
    fn setup(vars: usize) -> Self;
    fn commit(&self, evals: &Evaluations<F>) -> Self::Commitment;
    fn open(&self, evals: Evaluations<F>, point: &HyperPoint<F>) -> Self::OpenProof;
    fn verify(
        &self,
        commitment: Self::Commitment,
        point: &HyperPoint<F>,
        eval: F,
        proof: Self::OpenProof,
    ) -> bool;
}
pub trait UnivariateScheme<F: Field> {
    type Commitment: Add<Output = Self::Commitment> + Mul<F, Output = Self::Commitment> + Copy;
    type OpenProof;
    fn setup(vars: usize) -> Self;
    fn commit(&self, poly: DensePolynomial<F>) -> Self::Commitment;
    fn open(&self, evals: DensePolynomial<F>, point: F) -> Self::OpenProof;
    fn verify(
        &self,
        commitment: Self::Commitment,
        point: F,
        eval: F,
        proof: Self::OpenProof,
    ) -> bool;
}

pub(crate) fn split_and_fold<T, S, F>(mut vec: Vec<T>, state: S, fold: F) -> (S, Vec<T>)
where
    //for state, index and item
    F: Fn(S, usize, (&mut T, &mut T)) -> S,
{
    let n = vec.len();
    let (left, right) = vec.split_at_mut(n / 2);
    assert_eq!(left.len(), right.len());
    let v = left
        .iter_mut()
        .zip(right.iter_mut())
        .enumerate()
        .fold(state, |acc, (i, v)| {
            let new_state = fold(acc, i, v);
            new_state
        });
    (v, vec)
}
