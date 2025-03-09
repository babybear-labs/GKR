//#![feature(generic_const_exprs)]
use ark_ff::{FftField, Field};
use ark_poly::{
    univariate::DensePolynomial, DenseMultilinearExtension, MultilinearExtension, Polynomial,
    UVPolynomial,
};
use rand::thread_rng;
use std::{cmp::Ordering, ops::Add};

fn accumulate_eval<F: Field>(acc: (F, F), eval: &F, i: usize, n: usize) -> (F, F) {
    let (l, r) = acc;
    if i < n / 2 {
        (l + eval, r)
    } else {
        (l, r + eval)
    }
}

fn evaluations<F: Field>(challenge: F, mut evals: Vec<F>) -> ((F, F), Vec<F>) {
    assert!(evals.len().is_power_of_two());
    let n = evals.len();
    let message = {
        let evals = evals.as_mut_slice();
        let (left, right) = evals.split_at_mut(n / 2);
        left.iter_mut().zip(right.iter_mut()).enumerate().fold(
            (F::zero(), F::zero()),
            |acc, (i, (left, right))| {
                let new_eval = *left * (F::one() - challenge) + *right * challenge;
                *left = new_eval;
                accumulate_eval(acc, &new_eval, i, n / 2)
            },
        )
    };
    evals.truncate(n / 2);
    (message, evals)
}
/// to evaluate multilinear extension at single point, because the arkworks implementation
/// uses different endianness and is probably slower
fn evaluate_extension<F: Field>(mut evals: Vec<F>, point: &[F]) -> Result<F, &'static str> {
    let n = evals.len();
    n.is_power_of_two()
        .then_some(())
        .ok_or("must be power of two")?;
    match 2_usize.pow(point.len() as u32).cmp(&n) {
        Ordering::Less => Err("too few variables"),
        Ordering::Greater => Err("to many variables"),
        _ => Ok(()),
    }?;

    let mut evals = evals.as_mut_slice();
    for p in point {
        let n = evals.len();
        let (left, right) = evals.split_at_mut(n / 2);
        for e in left.iter_mut().zip(right.iter_mut()) {
            let (left, right) = e;
            let new_eval = *left * (F::one() - p) + *right * p;
            *left = new_eval;
        }
        evals = left;
    }
    Ok(evals[0])
}
//-------------prover--------------
struct Prover<F: FftField> {
    evals: Vec<F>,
    //ideally an oracle, for the last check of the verifier
    poly: DenseMultilinearExtension<F>,
    challenges: Vec<F>,
}

fn sum_tuple<T: Add<Output = T>>(t: (T, T)) -> T {
    let (a, b) = t;
    a + b
}

impl<F: FftField> Prover<F> {
    fn new(evals: Vec<F>) -> (Self, F, (F, F)) {
        assert!(evals.len().is_power_of_two());
        let first_message = Self::first_pass(&evals);
        let num_vars = evals.len().ilog2() as usize;
        let poly = DenseMultilinearExtension::from_evaluations_slice(num_vars, &evals);
        let claimed_sum = sum_tuple(first_message);
        let challenges = Vec::with_capacity(evals.len().ilog2() as usize);
        (
            Self {
                evals,
                poly,
                challenges,
            },
            claimed_sum,
            first_message,
        )
    }
    fn first_pass(evals: &Vec<F>) -> (F, F) {
        let n = evals.len();
        evals
            .iter()
            .enumerate()
            .fold((F::zero(), F::zero()), |sum, (i, eval)| {
                accumulate_eval(sum, eval, i, n)
            })
    }
    fn next_message(&mut self, challenge: F) -> (F, F) {
        let mut message: (F, F) = Default::default();
        take_mut::take_or_recover(&mut self.evals, Vec::new, |evals| {
            let (m, evals) = evaluations(challenge, evals);
            message = m;
            evals
        });
        self.challenges.push(challenge);
        message
    }
    pub fn last_message(mut self, challenge: F) -> ((F, F), DenseMultilinearExtension<F>) {
        let message = self.next_message(challenge);
        (message, self.poly)
    }
}

//--------------verifier------------
#[derive(Default)]
struct Verifier<F: FftField> {
    //univariate degree 1 polys, messages from the prover
    last_poly: DensePolynomial<F>,
    challenges: Vec<F>,
}

impl<F: FftField> Verifier<F> {
    pub fn new(claimed_sum: F, first_message: (F, F)) -> Result<(Self, F), &'static str> {
        let (l, f) = first_message;
        (claimed_sum == l + f)
            .then_some(claimed_sum)
            .ok_or("sum doesn't hold")?;

        let challenge = Self::challenge(&first_message);
        let last_poly = Self::message_to_poly(first_message);

        let verif = Self {
            last_poly,
            challenges: vec![challenge],
        };
        Ok((verif, challenge))
    }
    ///takes a message, verifies it and produces challenge
    pub fn round(&mut self, message: (F, F)) -> Result<F, &'static str> {
        //check message is degree less than n, in this fixed to one only supporting multilinear polynomials
        //nothing to do, the representation of message already enforces it
        let new_challenge = Self::challenge(&message);
        let eval_at_challenge = self.last_poly.evaluate(self.challenges.last().unwrap());
        let (l, r) = message;
        let message = (eval_at_challenge == l + r)
            .then_some(message)
            .ok_or("sum doesn't hold")?;
        let poly = Self::message_to_poly(message);
        self.last_poly = poly;
        self.challenges.push(new_challenge);
        Ok(new_challenge)
    }
    pub fn last_check(
        self,
        message: (F, F),
        poly: DenseMultilinearExtension<F>,
    ) -> Result<(), &'static str> {
        let eval_at_challenge = self.last_poly.evaluate(self.challenges.last().unwrap());
        let (l, f) = message;
        (eval_at_challenge == l + f)
            .then_some(())
            .ok_or("sum doesn't hold2")?;
        //maybe it should also be based on poly
        let last_challenge = Self::challenge(&message);
        let last_poly = Self::message_to_poly(message);
        let last_poly_eval = last_poly.evaluate(&last_challenge);
        let mut point = self.challenges;
        point.push(last_challenge);
        //let full_poly_eval = poly.evaluate(&*point).unwrap();
        let full_poly_eval = evaluate_extension(poly.to_evaluations(), &*point).unwrap();
        (full_poly_eval == last_poly_eval)
            .then_some(())
            .ok_or("last check failed")
    }
    fn message_to_poly(message: (F, F)) -> DensePolynomial<F> {
        let (l, f) = message;
        // -x + 1
        let l0 = DensePolynomial::from_coefficients_vec(vec![F::one(), -F::one()]);
        // x
        let l1 = DensePolynomial::from_coefficients_vec(vec![F::zero(), F::one()]);

        let poly = &l0 * l + &l1 * f;
        debug_assert_eq!(l + f, poly.evaluate(&F::zero()) + poly.evaluate(&F::one()));
        poly
    }
    fn challenge(_message: &(F, F)) -> F {
        //should use message as seed
        let mut rng = thread_rng();
        F::rand(&mut rng)
    }
}

//----------protocol----------
struct Protocol<F: FftField> {
    evals: Vec<F>,
    vars: usize,
}

impl<F: FftField> Protocol<F> {
    pub fn new(evals: Vec<F>) -> Self {
        assert!(evals.len().is_power_of_two());
        let vars = evals.len().ilog2() as usize;
        Self { evals, vars }
    }
    pub fn new_random(n: usize) -> Self {
        let mut rng = thread_rng();
        let evals = (0..n).into_iter().map(|_| F::rand(&mut rng)).collect();
        Self::new(evals)
    }
    pub fn run(self) -> Result<(), &'static str> {
        let Self { evals, vars } = self;

        let (mut prover, claimed_sum, first_message) = Prover::new(evals);
        let (mut verifier, first_challenge) = Verifier::new(claimed_sum, first_message)?;

        let mut last_challenge = first_challenge;
        for _ in 0..(vars - 2) {
            let message = prover.next_message(last_challenge);
            let challenge = verifier.round(message)?;
            last_challenge = challenge;
        }
        let (last_message, full_poly) = prover.last_message(last_challenge);
        verifier.last_check(last_message, full_poly)
    }
}

#[test]
fn test1() {
    use ark_pallas::Fr;
    use std::time::Instant;
    // let vals: Vec<Fr> = (1_i32..9).map(|x| Fr::from(x)).collect();
    let now = Instant::now();
    let prot = Protocol::<Fr>::new_random(256 * 256 * 256);
    prot.run().unwrap();
    let time = now.elapsed();
    println!("took: {}ms", time.as_millis());
}

#[test]
fn test2() {
    use ark_pallas::Fr;
    let vals: Vec<Fr> = (0_i32..8).map(|x| Fr::from(x)).collect();
    let points = [
        [0, 0, 0],
        [0, 0, 1],
        [0, 1, 0],
        [0, 1, 1],
        [1, 0, 0],
        [1, 0, 1],
        [1, 1, 0],
        [1, 1, 1],
    ];
    let points = points.map(|p| p.map(Fr::from));
    for point in points {
        let e = evaluate_extension(vals.clone(), &point).unwrap();
        println!("{}", e);
    }
}
