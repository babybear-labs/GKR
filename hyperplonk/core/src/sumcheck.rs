use crate::{
    challenges::ChallengeGenerator,
    misc::sum_tuple,
    polynomials::{split_and_fold, CommitmentScheme, Evaluations, HyperPoint, MultivariateScheme},
};
use ark_ff::Field;
use ark_poly::{univariate::DensePolynomial, Polynomial, UVPolynomial};
use std::{marker::PhantomData, rc::Rc};

pub struct Sumcheck<F: Field, S: CommitmentScheme<F>> {
    vars: usize,
    commitment_scheme: Rc<S>,
    _field: PhantomData<F>,
}

impl<F: Field, S: CommitmentScheme<F>> Sumcheck<F, S> {
    pub fn new(vars: usize, scheme: Rc<S>) -> Self {
        {
            assert_eq!(vars, scheme.vars());
            Self {
                vars,
                commitment_scheme: scheme,
                _field: PhantomData::default(),
            }
        }
    }
    fn proof_rounds(
        &self,
        evals: Evaluations<F>,
        commitment_bits: Vec<u8>,
    ) -> (Vec<(F, F)>, Vec<F>, F) {
        let mut challenge_generator = ChallengeGenerator::new(commitment_bits);
        let evals = evals.to_vec();
        let nvars = evals.len().ilog2();
        let n = evals.len();
        let z = F::zero();
        let first_message = evals
            .iter()
            .enumerate()
            .fold((z, z), |acc, (i, eval)| accumulate_eval(acc, eval, i, n));
        let sum = sum_tuple(first_message);
        let first_challenge = challenge_generator.next_challenge(first_message);

        let (_, messages, _) = (0..(nvars - 2)).into_iter().fold(
            (first_challenge, vec![first_message], evals),
            |acc, _i| {
                let (challenge, mut messages, evals) = acc;
                let (message, evals) = evaluations(challenge, evals);
                messages.push(message);
                let challenge = challenge_generator.next_challenge(message);
                (challenge, messages, evals)
            },
        );
        let challenges = challenge_generator.challenges();
        (messages, challenges, sum)
    }

    pub fn prove(
        &self,
        evals: Evaluations<F>,
        scheme: &mut S::Multivariate,
    ) -> SumcheckProof<F, S> {
        let commitment = scheme.commit(&evals);
        let commitment_bits = S::Multivariate::commitment_to_bits(commitment);
        let (rounds, challenges, claimed_sum) = self.proof_rounds(evals.clone(), commitment_bits);
        let point = HyperPoint::new(challenges);
        let opening = scheme.open(evals, &point);

        SumcheckProof {
            claimed_sum,
            rounds,
            commitment,
            opening,
        }
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
    pub fn verify(&self, proof: SumcheckProof<F, S>, scheme: &mut S::Multivariate) -> bool {
        let SumcheckProof {
            claimed_sum,
            rounds,
            commitment,
            opening,
        } = proof;
        let commitment_bits = S::Multivariate::commitment_to_bits(commitment);
        let mut challenge_generator = ChallengeGenerator::new(commitment_bits);
        let mut sum = claimed_sum;
        for message in rounds {
            if sum_tuple(message) != sum {
                return false;
            }
            let poly = Self::message_to_poly(message);
            let challenge = challenge_generator.next_challenge(message);
            sum = poly.evaluate(&challenge);
        }
        let point = HyperPoint::new(challenge_generator.challenges());
        scheme.verify(commitment, &point, sum, opening)
    }
}

pub struct SumcheckProof<F: Field, S: CommitmentScheme<F>> {
    claimed_sum: F,
    rounds: Vec<(F, F)>,
    commitment: <S::Multivariate as MultivariateScheme<F>>::Commitment,
    opening: <S::Multivariate as MultivariateScheme<F>>::OpenProof,
}

fn accumulate_eval<F: Field>(acc: (F, F), eval: &F, i: usize, n: usize) -> (F, F) {
    let (l, r) = acc;
    if i < n / 2 {
        (l + eval, r)
    } else {
        (l, r + eval)
    }
}

fn evaluations<F: Field>(challenge: F, evals: Vec<F>) -> ((F, F), Vec<F>) {
    assert!(evals.len().is_power_of_two());
    let n = evals.len();
    let fold = |acc, i, (left, right): (&mut F, &mut F)| {
        let new_eval: F = *left * (F::one() - challenge) + *right * challenge;
        *left = new_eval;
        accumulate_eval(acc, &new_eval, i, n / 2)
    };
    let (message, mut evals) = split_and_fold(evals, (F::zero(), F::zero()), fold);
    evals.truncate(n / 2);
    (message, evals)
}
