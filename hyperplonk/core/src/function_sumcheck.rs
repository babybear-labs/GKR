use crate::{challenges::ChallengeGenerator, polynomials::HyperPoint};
use ark_ff::{FftField, Field};
use ark_poly::{
    univariate::DensePolynomial, EvaluationDomain, Evaluations, Polynomial, Radix2EvaluationDomain,
    UVPolynomial,
};
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefIterator},
    prelude::{IntoParallelRefMutIterator, ParallelIterator},
};
use std::{
    cmp::max,
    marker::PhantomData,
    ops::{Add, Mul, Sub},
};

pub trait Function<const LEN: usize, const AUX: usize = 0> {
    fn function<F>(vars: [F; LEN], aux: &[F; AUX]) -> F
    where
        F: Element<Out = F>,
        for<'a> &'a F: Element<Out = F>;
}

// trait Variable: Sized + Add + Mul {}

pub struct SumcheckFunc<F: FftField, C, const COLS: usize, const AUX: usize = 0>
where
    // C: Fn(&[F; COLS]) -> F,
    C: Function<COLS, AUX>,
{
    vars: usize,
    aux: [F; AUX],
    _func: PhantomData<C>,
    _field: PhantomData<F>,
}

pub struct Proof<F: FftField, const C: usize> {
    claimed_sum: F,
    //rounds: Vec<(F, F)>,
    rounds: Vec<UniPoly<F>>,
}

#[inline]
fn reduce_evals<F: Field, const C: usize>(left: &mut [F; C], right: &[F; C], var: F) {
    for (l, r) in left.iter_mut().zip(right.iter()) {
        let new_eval: F = *l * (F::one() - var) + *r * var;
        *l = new_eval;
    }
}

impl<F: FftField, U, const C: usize, const AUX: usize> SumcheckFunc<F, U, C, AUX>
where
    U: Function<C, AUX>,
{
    //pub fn new(vars: usize, func: fn(&[F; C], &A) -> F, aux: A) -> Self {
    // pub fn new(vars: usize, func: U) -> Self {
    pub fn new(vars: usize, aux: [F; AUX]) -> Self {
        Self {
            vars,
            aux,
            _func: PhantomData,
            _field: PhantomData,
        }
    }
    fn degree() -> usize {
        let vars = [DegreeMeassure::new(1); C];
        let aux = [DegreeMeassure::new(0); AUX];
        let degree = U::function::<DegreeMeassure>(vars, &aux);
        println!("degree: {:?}", degree);
        degree.0
    }

    fn proof_rounds(
        &self,
        evals: Vec<[F; C]>,
        mut challenge_generator: ChallengeGenerator<F>,
        //func: impl Fn(&[F; C]) -> F,
    ) -> (Vec<UniPoly<F>>, Vec<F>, F) {
        let nvars = evals.len().ilog2();
        let degree = Self::degree();
        let domain = Radix2EvaluationDomain::new(degree + 1).unwrap();
        let aux = self.aux.map(|elem| {
            let poly = DensePolynomial::from_coefficients_vec(vec![elem]);
            let evals = poly.evaluate_over_domain(domain);
            UniPoly { evals }
        });
        let first_message = self.message(&evals, aux.clone(), domain);

        //let sum = sum_tuple(first_message);
        let sum = first_message.sum();
        ///taking first two elements for now, should take the commitment to the polynomial instead
        let first_challenge = challenge_generator.next_challenge(first_message.for_challenge());

        let (_, messages, _) = (0..(nvars - 1)).into_iter().fold(
            (first_challenge, vec![first_message], evals),
            |acc, _i| {
                println!("round");
                let (challenge, mut messages, evals) = acc;
                let evals = Self::reduce_evals(evals, challenge);
                //let (message, evals) = evaluations(challenge, evals, &func);
                let message = self.message(&evals, aux.clone(), domain);
                ///taking first two elements for now, should take the commitment to the polynomial instead
                let challenge = challenge_generator.next_challenge(message.for_challenge());
                messages.push(message);
                (challenge, messages, evals)
            },
        );
        let challenges = challenge_generator.challenges();
        (messages, challenges, sum)
    }
    fn reduce_evals(evals: Vec<[F; C]>, val: F) -> Vec<[F; C]> {
        assert!(evals.len().is_power_of_two());
        let n = evals.len();
        let mut evals = evals;
        let fold = |(left, right): (&mut [F; C], &mut [F; C])| {
            reduce_evals(left, right, val);
        };

        let (left, right) = evals.split_at_mut(n / 2);
        left.par_iter_mut().zip(right).for_each(fold);
        evals.truncate(n / 2);
        evals
    }
    fn message(
        &self,
        evals: &Vec<[F; C]>,
        aux: [UniPoly<F>; AUX],
        domain: Radix2EvaluationDomain<F>,
    ) -> UniPoly<F>
    where
        F: FftField,
    {
        let len = evals.len();
        let (left, right) = evals.split_at(len / 2);
        let init = || UniPoly::identity(domain);
        let message = left
            .par_iter()
            .zip(right)
            .fold(init, |state, (left, right)| {
                let a = left.clone().zip(right.clone());
                let vars = a.map(|(zero, one)| Self::make_poly_for_function2(zero, one, &domain));
                let poly: UniPoly<F> = U::function(vars, &aux);
                &state + &poly
            })
            .reduce(init, |a, b| a + b);

        message
    }
    pub(crate) fn prove(
        &self,
        evals: Vec<[F; C]>,
        challenge_generator: ChallengeGenerator<F>,
    ) -> (Proof<F, C>, HyperPoint<F>) {
        println!("evals: {}", evals.len());
        assert_eq!(evals.len().ilog2() as usize, self.vars);
        let (rounds, challenges, claimed_sum) = self.proof_rounds(evals, challenge_generator);
        //self.proof_rounds(evals, challenge_generator, &self.func);
        println!("challs {}", challenges.len());
        let point = HyperPoint::new(challenges);

        (
            Proof {
                claimed_sum,
                rounds,
            },
            point,
        )
    }
    ///checks everithing but the opening, if nothing wrong returns the point where to check commitment opening
    pub(crate) fn verify(
        &self,
        proof: Proof<F, C>,
        mut challenge_generator: ChallengeGenerator<F>,
    ) -> Result<CheckPoint<F>, ()> {
        let Proof {
            claimed_sum,
            rounds,
        } = proof;

        //assert_eq!(claimed_sum, F::zero());
        let mut sum = claimed_sum;
        for message in rounds {
            // if sum_tuple(message) != sum {
            if message.sum() != sum {
                return Err(());
            }
            //let poly = Self::message_to_poly(message);
            // let challenge = challenge_generator.next_challenge(message);
            let challenge = challenge_generator.next_challenge(message.for_challenge());
            println!("vchallenge: {}", challenge);
            // sum = poly.evaluate(&challenge);
            sum = message.eval(&challenge);
        }
        let point = HyperPoint::new(challenge_generator.challenges());

        Ok(CheckPoint { point, sum })
    }
    ///applies the function to the evals in a random point
    pub fn eval_in_random_point(&self, evals: [F; C]) -> F {
        let vars = evals.map(|e| FieldWrapper::new(e));
        let aux = self.aux.map(|e| FieldWrapper::new(e));
        let ev = U::function(vars, &aux);
        ev.0
    }

    /// evals at (0,1)
    fn message_to_poly(message: (F, F)) -> DensePolynomial<F> {
        let (l, f) = message;
        // -x + 1 or 1 at 0
        let l0 = DensePolynomial::from_coefficients_vec(vec![F::one(), -F::one()]);
        // x or 1 at 1
        let l1 = DensePolynomial::from_coefficients_vec(vec![F::zero(), F::one()]);

        let poly = &l0 * l + &l1 * f;
        debug_assert_eq!(l + f, poly.evaluate(&F::zero()) + poly.evaluate(&F::one()));
        poly
    }
    /*#[inline]
    fn reduce_evals2(left: &mut [F; C], right: &mut [F; C], var: F) {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let new_eval: F = *l * (F::one() - var) + *r * var;
            *l = new_eval;
            // l at 0 and r at 1
            let deg_1_poly = Self::message_to_poly((l, r));
            deg_1_poly
        }
    }*/
    #[allow(dead_code)]
    fn make_poly_for_function(
        eval_in_zero: F,
        eval_in_one: F,
        domain: &Radix2EvaluationDomain<F>,
    ) -> UniPoly<F>
    where
        F: FftField,
    {
        let poly = Self::message_to_poly((eval_in_zero, eval_in_one));
        let evals = poly.evaluate_over_domain(*domain);
        UniPoly { evals }
    }
    ///lineal and faster than the original, also could work without fft field
    fn make_poly_for_function2(
        eval_in_zero: F,
        eval_in_one: F,
        domain: &Radix2EvaluationDomain<F>,
    ) -> UniPoly<F>
    where
        F: FftField,
    {
        let poly = Self::message_to_poly((eval_in_zero, eval_in_one));
        let c0 = poly.coeffs[0];
        let c1 = poly.coeffs[1];
        let mut evals = Vec::with_capacity(domain.size());
        evals.extend(domain.elements());
        for e in evals.iter_mut() {
            *e = *e * c1 + c0;
        }
        let evals = Evaluations::from_vec_and_domain(evals, *domain);
        UniPoly { evals }
    }
}
///point where to check openings
pub(crate) struct CheckPoint<F: Field> {
    pub point: HyperPoint<F>,
    pub sum: F,
}

#[cfg(test)]
mod test {
    use super::{Function, SumcheckFunc};
    use crate::{
        challenges::ChallengeGenerator, function_sumcheck::CheckPoint, polynomials::Evaluations,
    };
    use ark_ff::{Field, UniformRand, Zero};
    use ark_pallas::Fr;
    use rand::thread_rng;

    fn func<F: Field>(input: &[F; 2]) -> F {
        (input[0] * input[1]) - input[1]
    }
    struct Func;
    impl Function<2> for Func {
        fn function<F>(vars: [F; 2], _aux: &[F; 0]) -> F
        where
            F: super::Element<Out = F>,
            for<'a> &'a F: super::Element<Out = F>,
        {
            let [a, b] = &vars;
            &(a * b) - b
        }
    }
    #[test]
    fn function() {
        let vars = 8;
        let sumcheck = SumcheckFunc::<Fr, Func, 2>::new(vars, []);
        let mut rng = thread_rng();
        let evals = (0..(1 << vars))
            .into_iter()
            .map(|_| [Fr::rand(&mut rng), Fr::rand(&mut rng)])
            .collect::<Vec<_>>();
        let mut extensions = [Vec::new(), Vec::new()];
        let sum = evals.iter().fold(Fr::zero(), |s, e| {
            extensions[0].push(e[0]);
            extensions[1].push(e[1]);
            //s.zip(*e).map(|(a, b)| a * b)
            //let [a, b] = e;
            // s + (*a * b) * b
            s + func(e)
        });
        assert_eq!(sum, Fr::zero());

        let challenge_generator = ChallengeGenerator::new(vec![3, 34, 5]);
        let (proof, _point) = sumcheck.prove(evals, challenge_generator);
        println!("claimed sum: {}", proof.claimed_sum);
        // assert_eq!(proof.claimed_sum, sum[0] * sum[1]);
        assert_eq!(proof.claimed_sum, sum);
        let challenge_generator = ChallengeGenerator::new(vec![3, 34, 5]);
        let verif = sumcheck.verify(proof, challenge_generator).unwrap();
        let CheckPoint { point, sum } = verif;
        println!("check sum: {}", sum);
        let evals = extensions
            .map(|e| Evaluations::new(e))
            .map(|e| e.evaluate(&point).unwrap());
        assert_eq!(sum, func(&evals));
    }
    #[test]
    fn simple_gate() {
        fn func<F: Field>(input: &[F; 3]) -> F {
            (input[0] * input[1]) - input[2]
        }
        struct Func;
        impl Function<3> for Func {
            fn function<F>(vars: [F; 3], _aux: &[F; 0]) -> F
            where
                F: super::Element<Out = F>,
                for<'a> &'a F: super::Element<Out = F>,
            {
                let [a, b, c] = &vars;
                &(a * b) - c
            }
        }

        let vars = 8;
        let sumcheck = SumcheckFunc::<Fr, Func, 3>::new(vars, []);
        let mut rng = thread_rng();
        let evals = (0..(1 << vars))
            .into_iter()
            .map(|_| {
                let [a, b] = [Fr::rand(&mut rng), Fr::rand(&mut rng)];
                [a, b, a * b]
            })
            .collect::<Vec<_>>();
        let mut extensions = [Vec::new(), Vec::new(), Vec::new()];
        let sum = evals.iter().fold(Fr::zero(), |s, e| {
            extensions[0].push(e[0]);
            extensions[1].push(e[1]);
            extensions[2].push(e[2]);
            s + func(e)
        });

        let challenge_generator = ChallengeGenerator::new(vec![3, 34, 5]);
        let (proof, _point) = sumcheck.prove(evals, challenge_generator);
        println!("claimed sum: {}", proof.claimed_sum);
        // assert_eq!(proof.claimed_sum, sum[0] * sum[1]);
        assert_eq!(proof.claimed_sum, sum);
        let challenge_generator = ChallengeGenerator::new(vec![3, 34, 5]);
        let verif = sumcheck.verify(proof, challenge_generator).unwrap();
        let CheckPoint { point, sum } = verif;
        println!("check sum: {}", sum);
        let evals = extensions
            .map(|e| Evaluations::new(e))
            .map(|e| e.evaluate(&point).unwrap());
        assert_eq!(sum, func(&evals));
    }
}

pub trait Element:
    Sized + Clone + Add<Output = Self::Out> + Mul<Output = Self::Out> + Sub<Output = Self::Out>
{
    type Out;
}

#[derive(Clone)]
pub struct FieldWrapper<F: Field>(F);

impl<F: Field> FieldWrapper<F> {
    pub fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<F: Field> Add for &FieldWrapper<F> {
    type Output = FieldWrapper<F>;

    fn add(self, rhs: Self) -> Self::Output {
        FieldWrapper(self.0 + rhs.0)
    }
}
impl<F: Field> Mul for &FieldWrapper<F> {
    type Output = FieldWrapper<F>;

    fn mul(self, rhs: Self) -> Self::Output {
        FieldWrapper(self.0 * rhs.0)
    }
}
impl<F: Field> Sub for &FieldWrapper<F> {
    type Output = FieldWrapper<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        FieldWrapper(self.0 - rhs.0)
    }
}
impl<F: Field> Element for &FieldWrapper<F> {
    type Out = FieldWrapper<F>;
}
impl<F: Field> Add for FieldWrapper<F> {
    type Output = FieldWrapper<F>;

    fn add(self, rhs: Self) -> Self::Output {
        FieldWrapper(self.0 + rhs.0)
    }
}
impl<F: Field> Mul for FieldWrapper<F> {
    type Output = FieldWrapper<F>;

    fn mul(self, rhs: Self) -> Self::Output {
        FieldWrapper(self.0 * rhs.0)
    }
}
impl<F: Field> Sub for FieldWrapper<F> {
    type Output = FieldWrapper<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        FieldWrapper(self.0 - rhs.0)
    }
}
impl<F: Field> Element for FieldWrapper<F> {
    type Out = FieldWrapper<F>;
}

#[derive(Clone)]
pub(crate) struct UniPoly<F: FftField> {
    evals: Evaluations<F, Radix2EvaluationDomain<F>>,
}

impl<F: FftField> UniPoly<F> {
    fn identity(domain: Radix2EvaluationDomain<F>) -> Self {
        let zero_poly = DensePolynomial::from_coefficients_vec(vec![F::zero()]);
        let evals = zero_poly.evaluate_over_domain(domain);
        UniPoly { evals }
    }
    ///to use with challenge generator temporally
    fn for_challenge(&self) -> (F, F) {
        ///this function should be removed
        (self.evals[0], self.evals[1])
    }
    ///the sum over the 0 and 1 points
    fn sum(&self) -> F {
        let poly = self.evals.interpolate_by_ref();
        let in_zero = poly.evaluate(&F::zero());
        let in_one = poly.evaluate(&F::one());
        in_zero + in_one
    }
    fn eval(&self, point: &F) -> F {
        let poly = self.evals.interpolate_by_ref();
        poly.evaluate(point)
    }
}

impl<F: FftField> Add for &UniPoly<F> {
    type Output = UniPoly<F>;

    fn add(self, rhs: Self) -> Self::Output {
        let evals = &self.evals + &rhs.evals;
        UniPoly { evals }
    }
}
impl<F: FftField> Mul for &UniPoly<F> {
    type Output = UniPoly<F>;

    fn mul(self, rhs: Self) -> Self::Output {
        let evals = &self.evals * &rhs.evals;
        UniPoly { evals }
    }
}
impl<F: FftField> Sub for &UniPoly<F> {
    type Output = UniPoly<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        let evals = &self.evals - &rhs.evals;
        UniPoly { evals }
    }
}

impl<F: FftField> Element for &UniPoly<F> {
    type Out = UniPoly<F>;
}

impl<F: FftField> Add for UniPoly<F> {
    type Output = UniPoly<F>;

    fn add(self, rhs: Self) -> Self::Output {
        &self + &rhs
    }
}
impl<F: FftField> Mul for UniPoly<F> {
    type Output = UniPoly<F>;

    fn mul(self, rhs: Self) -> Self::Output {
        &self * &rhs
    }
}
impl<F: FftField> Sub for UniPoly<F> {
    type Output = UniPoly<F>;

    fn sub(self, rhs: Self) -> Self::Output {
        &self - &rhs
    }
}
impl<F: FftField> Element for UniPoly<F> {
    type Out = UniPoly<F>;
}

#[derive(Clone, Copy, Debug)]
pub struct DegreeMeassure(usize);

impl DegreeMeassure {
    fn new(degree: usize) -> Self {
        Self(degree)
    }
}

impl Add for &DegreeMeassure {
    type Output = DegreeMeassure;

    fn add(self, rhs: Self) -> Self::Output {
        DegreeMeassure(max(self.0, rhs.0))
    }
}
impl Mul for &DegreeMeassure {
    type Output = DegreeMeassure;

    fn mul(self, rhs: Self) -> Self::Output {
        DegreeMeassure(self.0 + rhs.0)
    }
}
impl Sub for &DegreeMeassure {
    type Output = DegreeMeassure;

    fn sub(self, rhs: Self) -> Self::Output {
        DegreeMeassure(max(self.0, rhs.0))
    }
}

impl Element for &DegreeMeassure {
    type Out = DegreeMeassure;
}

impl Add for DegreeMeassure {
    type Output = DegreeMeassure;

    fn add(self, rhs: Self) -> Self::Output {
        &self + &rhs
    }
}
impl Mul for DegreeMeassure {
    type Output = DegreeMeassure;

    fn mul(self, rhs: Self) -> Self::Output {
        &self * &rhs
    }
}
impl Sub for DegreeMeassure {
    type Output = DegreeMeassure;

    fn sub(self, rhs: Self) -> Self::Output {
        &self - &rhs
    }
}
impl Element for DegreeMeassure {
    type Out = DegreeMeassure;
}
