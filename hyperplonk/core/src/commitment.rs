use crate::{
    misc::index_to_point,
    polynomials::{
        split_and_fold, CommitmentScheme, Evaluations, HyperPoint, MultivariateScheme,
        UnivariateScheme,
    },
};
use ark_ec::{
    msm::{FixedBaseMSM, VariableBaseMSM},
    AffineCurve, PairingEngine, ProjectiveCurve,
};
use ark_ff::{Field, One, PrimeField, ToBytes, UniformRand};
use ark_poly::univariate::DensePolynomial;
use rand::{thread_rng, Rng};
use std::ops::{Add, Mul};

pub struct MultilinearKzgScheme<P: PairingEngine> {
    pub lagrange_commitments: Vec<Vec<P::G1Affine>>,
    pub vars: Vec<P::G2Projective>,
}

impl<P: PairingEngine> MultilinearKzgScheme<P> {
    pub fn new<R: Rng>(rng: &mut R, vars: usize, secret: Option<Vec<P::Fr>>) -> Self {
        let evals = 1 << vars;
        let secret_point: Vec<_> = (0..vars).map(|_| <P::Fr>::rand(rng)).collect();
        let secret_point = match secret {
            Some(p) => {
                assert_eq!(vars, p.len());
                p
            }
            None => secret_point,
        };
        let g1_generator = <P::G1Projective as ProjectiveCurve>::prime_subgroup_generator();
        let scalar_size = P::Fr::size_in_bits();
        let window = FixedBaseMSM::get_mul_window_size(evals);
        let table = FixedBaseMSM::get_window_table(scalar_size, window, g1_generator);

        let lagrange_commitments: Vec<Vec<_>> = (0..=vars)
            .rev()
            .map(|i| lagrange_poly_evals(i, &secret_point))
            .map(|evals| FixedBaseMSM::multi_scalar_mul(scalar_size, window, &table, &evals))
            .map(|evals| P::G1Projective::batch_normalization_into_affine(&evals))
            .collect();
        println!("len:{}", lagrange_commitments[0].len());

        let g2_generator = <P::G2Affine as AffineCurve>::prime_subgroup_generator();
        let vars = secret_point
            .into_iter()
            .map(|v| g2_generator.mul(v))
            .collect();

        Self {
            lagrange_commitments,
            vars,
        }
    }
    pub fn commit(&self, evaluations: &Evaluations<P::Fr>) -> P::G1Affine {
        let bases = &self.lagrange_commitments[0];
        let scalars: Vec<_> = evaluations
            .evaluations()
            .iter()
            .map(|e| e.into_repr())
            .collect();
        VariableBaseMSM::multi_scalar_mul(bases, &scalars).into_affine()
    }
    #[cfg(test)]
    fn commit2(&self, evaluations: &Evaluations<P::Fr>, i: usize) -> P::G1Affine {
        let bases = &self.lagrange_commitments[i];
        let scalars: Vec<_> = evaluations
            .evaluations()
            .iter()
            .map(|e| e.into_repr())
            .collect();
        VariableBaseMSM::multi_scalar_mul(bases, &scalars).into_affine()
    }
    pub fn open(
        &self,
        evaluations: Evaluations<P::Fr>,
        point: Vec<P::Fr>,
    ) -> (Vec<P::G1Affine>, P::Fr) {
        assert_eq!(evaluations.vars(), point.len());
        let quotient_commitments = Vec::with_capacity(evaluations.vars());
        let evals: Vec<_> = evaluations.to_vec();
        let (quotient_commitments, remainder) =
            point
                .into_iter()
                .enumerate()
                .fold((quotient_commitments, evals), |state, (i, p)| {
                    let (mut quotient_commitments, remainder) = state;
                    let (_, mut evals) = split_and_fold(remainder, (), |_, _, (l, r)| {
                        let quotient = *r - *l;
                        let remainder = *l * (P::Fr::one() - p) + *r * p;
                        *l = remainder;
                        *r = quotient;
                    });
                    let (_, quotient) = evals.split_at(evals.len() / 2);

                    let quotient: Vec<_> = quotient.into_iter().map(|e| e.into_repr()).collect();
                    evals.truncate(evals.len() / 2);
                    let remainder = evals;

                    let quotient_commit = VariableBaseMSM::multi_scalar_mul(
                        &self.lagrange_commitments[i + 1],
                        &quotient,
                    )
                    .into_affine();
                    quotient_commitments.push(quotient_commit);
                    (quotient_commitments, remainder)
                });
        (quotient_commitments, remainder[0])
    }
    pub fn check(
        &self,
        commitment: P::G1Affine,
        point: Vec<P::Fr>,
        eval: P::Fr,
        proof: Vec<P::G1Affine>,
    ) -> bool {
        let g1_generator = <P::G1Affine as AffineCurve>::prime_subgroup_generator();
        let g2_generator = <P::G2Affine as AffineCurve>::prime_subgroup_generator();
        if point.len() != proof.len() {
            return false;
        }
        let left_pairing = {
            let eval = g1_generator.mul(eval);
            let g1 = commitment.into_projective() - eval;
            P::pairing(g1, g2_generator)
        };
        let right_pairing = {
            let pairings: Vec<_> = self
                .vars
                .iter()
                .zip(point.into_iter())
                .zip(proof)
                .map(|e| {
                    let ((var, point), commit) = e;
                    let point = g2_generator.mul(-point);
                    let right = *var + point;
                    (
                        P::G1Prepared::from(commit),
                        P::G2Prepared::from(right.into_affine()),
                    )
                })
                .collect();
            let product = P::product_of_pairings(&pairings);
            product
        };
        left_pairing == right_pairing
    }
}

fn lagrange_poly_evals<F: Field>(vars: usize, secret_point: &Vec<F>) -> Vec<F> {
    let evals = 1 << vars;
    let n = secret_point.len();
    let point = &secret_point[(n - vars)..];
    (0..evals)
        .map(|i| lagrange_poly_eval(vars, i, &point))
        .collect()
}

fn lagrange_poly_eval<F: Field>(vars: usize, point: usize, eval_point: &[F]) -> F {
    if vars == 0 {
        println!("p: {:#?}", eval_point);
        return F::one();
    }
    let point = index_to_point::<F>(vars, point);
    point
        .into_iter()
        .rev()
        .zip(eval_point.iter())
        .map(|(x, y)| {
            let one = F::one();
            x * y + (one - x) * (one - y)
        })
        .reduce(Mul::mul)
        .unwrap()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::polynomials::HyperPoint;
    use ark_bls12_381::{Bls12_381, Fr};
    use std::ops::Add;

    #[test]
    fn lagrange() {
        let vars = 4;
        let evals = 1 << vars;
        let mut rng = make_rng();
        let random_point: Vec<Fr> = (0..vars + 1).map(|_| Fr::rand(&mut rng)).collect();
        let point = HyperPoint::new(random_point[1..].to_vec());

        let poly = Evaluations::new(vec![Fr::one(); evals]);
        let eval = poly.evaluate(&point).unwrap();
        let eval2 = lagrange_poly_evals(vars, &random_point)
            .into_iter()
            .reduce(Add::add)
            .unwrap();
        assert_eq!(eval, eval2);
    }

    #[test]
    fn reval() {
        let mut rng = make_rng();
        let vars = 3;
        let evals = 1 << vars;
        let mut random_point: Vec<Fr> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();
        for i in 0..evals {
            let val = lagrange_poly_eval(vars, i, &random_point);
            println!("{}", val);
        }

        let vars = 2;
        let evals = 1 << vars;
        random_point.pop();
        println!();

        for i in 0..evals {
            let val = lagrange_poly_eval(vars, i, &random_point);
            println!("{}", val);
        }
    }

    #[cfg(test)]
    fn make_rng() -> rand::rngs::StdRng {
        use rand::SeedableRng;
        let rng = rand::rngs::StdRng::seed_from_u64(1234);
        rng
    }
    #[test]
    fn equal() {
        let vars = 4;
        let n_evals = 1 << vars;
        let mut rng = make_rng();

        let eval_point: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();
        let random_point: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();

        let evals: Vec<_> = (0..n_evals).map(|_| Fr::rand(&mut rng)).collect();
        let poly = Evaluations::new(evals.clone());

        let scheme =
            MultilinearKzgScheme::<Bls12_381>::new(&mut rng, vars, Some(eval_point.clone()));
        let full_commit = scheme.commit(&poly);

        let random_eval = poly
            .clone()
            .evaluate(&HyperPoint::new(random_point.clone()))
            .unwrap();
        let original_eval = poly.evaluate(&HyperPoint::new(eval_point.clone())).unwrap();
        println!("good eval: {:?}", original_eval);
        println!("random eval: {:?}", random_eval);

        let mut quotient_commits = vec![];
        let g1 =
            <<Bls12_381 as PairingEngine>::G1Affine as AffineCurve>::prime_subgroup_generator();
        let g2 =
            <<Bls12_381 as PairingEngine>::G2Affine as AffineCurve>::prime_subgroup_generator();

        let (quotients, remainder) =
            random_point
                .clone()
                .into_iter()
                .fold((vec![], evals), |state, p| {
                    //let mut sub_poly = vec![];
                    let (mut quotients, remainder) = state;
                    let evals = remainder;
                    let (_, mut evals) = split_and_fold(evals, (), |_, _, e| {
                        let (l, r) = e;
                        let quotient = *r - *l;
                        let remainder = *l * (Fr::one() - p) + *r * p;
                        *l = quotient;
                        *r = remainder;
                        ()
                    });
                    let remainder = {
                        let (_, remainder) = evals.split_at(evals.len() / 2);
                        remainder.to_vec()
                    };

                    println!("rlen: {}", remainder.len());
                    evals.truncate(evals.len() / 2);
                    println!("len: {}", evals.len());
                    let quotient = Evaluations::new(evals);

                    {
                        let i = vars - quotient.vars();
                        let quotient = scheme.commit2(&quotient, i);
                        quotient_commits.push(quotient);
                    }

                    quotients.push(quotient);
                    (quotients, remainder)
                });

        let mut quotient_commits2 = vec![];
        let alternative_eval = random_point
            .iter()
            .zip(eval_point.clone().into_iter())
            .map(|(t, x)| x - t)
            .zip(quotients.into_iter().enumerate())
            .map(|(small_term, (i, quotient))| {
                let point = &eval_point[(i + 1)..];
                let point = HyperPoint::new(point.to_vec());
                let eval = quotient.evaluate(&point).expect("eval");
                quotient_commits2.push(g1.mul(eval));
                small_term * eval
            })
            .reduce(Add::add)
            .unwrap();
        assert_eq!(original_eval - remainder[0], alternative_eval);

        println!("checking..\n");
        for (a, b) in quotient_commits.iter().zip(quotient_commits2.into_iter()) {
            assert_eq!(*a, b);
            println!("equal");
        }
        for (a, b) in eval_point.iter().zip(scheme.vars.iter()) {
            assert_eq!(g2.mul(*a).into_affine(), *b);
        }

        //check product
        let pairs = random_point
            .iter()
            .zip(eval_point)
            .map(|(p, x)| g2.mul(x) - g2.mul(*p))
            .zip(quotient_commits.into_iter())
            .map(|(m, quotient)| (quotient.into(), m.into_affine().into()))
            .collect::<Vec<_>>();
        let product = Bls12_381::product_of_pairings(pairs.iter());
        let left = Bls12_381::pairing(full_commit.into_projective() - g1.mul(remainder[0]), g2);
        assert_eq!(left, product);
    }

    #[test]
    fn commit_test() {
        let mut rng = make_rng();
        let vars = 4;
        let n_evals = 1 << vars;
        let scheme = MultilinearKzgScheme::<Bls12_381>::new(&mut rng, vars, None);

        let evals: Vec<_> = (0..n_evals).map(|_| Fr::rand(&mut rng)).collect();
        let evaluations = Evaluations::new(evals.clone());
        let random_point: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();
        let point = HyperPoint::new(random_point.clone());

        let normal_eval = evaluations.clone().evaluate(&point).unwrap();

        let commitment = scheme.commit(&evaluations);

        let open = scheme.open(evaluations, random_point.clone());
        let (proof, eval) = open;
        assert_eq!(eval, normal_eval);

        let check = scheme.check(commitment, random_point, eval, proof);

        assert!(check)
    }

    #[test]
    fn srs() {
        let mut rng = make_rng();
        let vars = 4;
        let secret: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();
        let scheme = MultilinearKzgScheme::<Bls12_381>::new(&mut rng, vars, Some(secret.clone()));

        let lagranges = scheme.lagrange_commitments;
        for lag in lagranges {
            let vars = lag.len().ilog2() as usize;
            let evals = 1 << vars;
            let one_poly = Evaluations::new(vec![Fr::one(); evals]);
            let point = HyperPoint::new(secret[(secret.len() - vars)..].to_vec());
            let eval = one_poly.evaluate(&point).unwrap();

            let left = lag.iter().cloned().reduce(Add::add).unwrap();
            let g1 =
                <<Bls12_381 as PairingEngine>::G1Affine as AffineCurve>::prime_subgroup_generator();
            let right = g1.mul(eval);
            assert_eq!(left, right);
        }
    }

    #[test]
    fn lagrange_pairing() {
        let mut rng = make_rng();
        let vars = 4;
        let n_evals = 1 << vars;
        let secret_point: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();
        let scheme =
            MultilinearKzgScheme::<Bls12_381>::new(&mut rng, vars, Some(secret_point.clone()));

        let evaluations: Vec<_> = (0..(n_evals / 2)).map(|_| Fr::rand(&mut rng)).collect();
        let poly = Evaluations::new(evaluations.clone());

        let left_commit = scheme.commit2(&poly, 1);

        let g1_generator =
            <<Bls12_381 as PairingEngine>::G1Affine as AffineCurve>::prime_subgroup_generator();
        let g2_generator =
            <<Bls12_381 as PairingEngine>::G2Affine as AffineCurve>::prime_subgroup_generator();

        let point = HyperPoint::new(secret_point[1..].to_vec());
        let eval = poly.evaluate(&point).unwrap();
        let commit = left_commit;

        let l = Bls12_381::pairing(commit, g2_generator);
        let r = Bls12_381::pairing(g1_generator.mul(eval), g2_generator);
        assert_eq!(l, r);
    }
}
#[derive(Clone, Copy)]
pub struct Commitment<P: PairingEngine>(P::G1Affine);

impl<P: PairingEngine> Mul<P::Fr> for Commitment<P> {
    type Output = Self;

    fn mul(self, rhs: P::Fr) -> Self::Output {
        Commitment(self.0.mul(rhs).into_affine())
    }
}

impl<P: PairingEngine> Add for Commitment<P> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Commitment(self.0 + rhs.0)
    }
}

impl<P> MultivariateScheme<P::Fr> for MultilinearKzgScheme<P>
where
    P: PairingEngine,
{
    type Commitment = Commitment<P>;

    type OpenProof = Vec<P::G1Affine>;

    fn commitment_to_bits(commitment: Self::Commitment) -> Vec<u8> {
        let mut bytes = Vec::new();
        commitment.0.write(&mut bytes).unwrap();
        bytes
    }

    fn setup(vars: usize) -> Self {
        let mut rng = thread_rng();
        Self::new(&mut rng, vars, None)
    }

    fn commit(&self, evals: &Evaluations<P::Fr>) -> Self::Commitment {
        Commitment(self.commit(evals))
    }

    fn open(&self, evals: Evaluations<P::Fr>, point: &HyperPoint<P::Fr>) -> Self::OpenProof {
        MultilinearKzgScheme::open(&self, evals, point.inner().to_vec()).0
    }

    fn verify(
        &self,
        commitment: Self::Commitment,
        point: &HyperPoint<P::Fr>,
        eval: P::Fr,
        proof: Self::OpenProof,
    ) -> bool {
        MultilinearKzgScheme::check(&self, commitment.0, point.inner().to_vec(), eval, proof)
    }
}
pub struct MockUnivariate;
impl<F: Field> UnivariateScheme<F> for MockUnivariate {
    type Commitment = F;

    type OpenProof = ();

    fn setup(_vars: usize) -> Self {
        todo!()
    }

    fn commit(&self, _poly: DensePolynomial<F>) -> Self::Commitment {
        todo!()
    }

    fn open(&self, _evals: DensePolynomial<F>, _point: F) -> Self::OpenProof {
        todo!()
    }

    fn verify(
        &self,
        _commitment: Self::Commitment,
        _point: F,
        _eval: F,
        _proof: Self::OpenProof,
    ) -> bool {
        todo!()
    }
}

impl<P> CommitmentScheme<P::Fr> for MultilinearKzgScheme<P>
where
    P: PairingEngine,
{
    type Multivariate = Self;

    type Univariate = MockUnivariate;

    fn vars(&self) -> usize {
        self.vars.len()
    }

    fn multi_scheme(&self) -> &Self::Multivariate {
        &self
    }

    fn init(vars: usize) -> Self {
        let mut rng = thread_rng();
        Self::new(&mut rng, vars, None)
    }
}
