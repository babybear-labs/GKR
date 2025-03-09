use crate::{
    challenges::ChallengeGenerator,
    function_sumcheck::{self, CheckPoint, Element, Function, SumcheckFunc},
    permutation::PermutationCheck,
    polynomials::{
        eval_sparse_poly, random_poly_evals_slow, CommitmentScheme, Evaluations, HyperPoint,
        MultivariateScheme,
    },
};
use ark_ff::{FftField, Field};
use std::{
    iter::successors,
    marker::PhantomData,
    ops::{Add, Mul},
    rc::Rc,
};

pub struct HyperPlonk<F, S, G, const COLS: usize = 16, const GATES: usize = 0>
where
    F: Field,
    S: CommitmentScheme<F>,
    G: Gates<COLS, GATES>,
    [F; 3 * COLS + 2 + 2 + 1]:,
    [F; GATES + 2]:,
    [F; COLS * 2]:,
{
    permutation: PermutationCheck<F, COLS>,
    permutation_commitments: [[Commit<F, S>; COLS]; 2],
    scheme: Rc<S>,
    ///commitment to a polynomial evaluating to 1 at all points
    one_commitment: Commit<F, S>,
    _gates: PhantomData<G>,
}
type Open<F, S> = <<S as CommitmentScheme<F>>::Multivariate as MultivariateScheme<F>>::OpenProof;
type Commit<F, S> = <<S as CommitmentScheme<F>>::Multivariate as MultivariateScheme<F>>::Commitment;

struct OpenMultiProof<F, S, const N: usize>
where
    F: Field,
    S: CommitmentScheme<F>,
{
    opens_in_random_point: (Open<F, S>, [[F; N]; 3], [F; 2]),
    permutation_in_zero: (Open<F, S>, F),
    permutation_in_one: (Open<F, S>, F),
    ///supposed to be one, no value needed
    product_eval: Open<F, S>,
}

pub struct HyperProof<F, S, const COLS: usize>
where
    F: FftField,
    S: CommitmentScheme<F>,
    [(); 3 * COLS + 2 + 2 + 1]:,
{
    sumcheck_proof: function_sumcheck::Proof<F, { 3 * COLS + 2 + 2 + 1 }>,
    witness_commitments: [Commit<F, S>; COLS],
    grand_product_commitments: [Commit<F, S>; 2],
    multi_proof: OpenMultiProof<F, S, COLS>,
}

impl<F, S, G, const COLS: usize, const GATES: usize> HyperPlonk<F, S, G, COLS, GATES>
where
    F: FftField,
    S: CommitmentScheme<F>,
    G: Gates<COLS, GATES>,
    [(); 3 * COLS + 2 + 2 + 1]:,
    [(); GATES + 2]:,
    [(); COLS * 2]:,
{
    pub fn new(scheme: Rc<S>, permutation: Vec<[u64; COLS]>) -> Self {
        let (permutation, permutation_extensions) = PermutationCheck::new(permutation);
        let permutation_commitments =
            Self::permutation_commitments(&*scheme, permutation_extensions);
        let one_extension = vec![F::one(); 1 << scheme.vars()];
        let one_commitment = scheme
            .multi_scheme()
            .commit(&Evaluations::new(one_extension));

        Self {
            permutation,
            permutation_commitments,
            scheme,
            one_commitment,
            _gates: PhantomData,
        }
    }
    fn permutation_commitments(scheme: &S, perm: [[Vec<F>; COLS]; 2]) -> [[Commit<F, S>; COLS]; 2] {
        let scheme = scheme.multi_scheme();
        perm.map(|half| half.map(|extension| scheme.commit(&Evaluations::new(extension))))
    }

    fn bits_of_commitments(commitments: &[Commit<F, S>]) -> Vec<u8> {
        commitments
            .iter()
            .cloned()
            .map(|c| S::Multivariate::commitment_to_bits(c))
            .flatten()
            .collect()
    }
    fn powers<const N: usize>(challenge: F) -> [F; N] {
        successors(Some(challenge), |prev| Some(*prev * challenge))
            .take(N)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
    fn powers_of_alpha(alpha: F) -> [F; GATES + 2] {
        Self::powers::<{ GATES + 2 }>(alpha)
    }
    fn powers_of_delta(delta: F) -> [F; 3 * COLS + 2] {
        Self::powers::<{ 3 * COLS + 2 }>(delta)
    }
    ///opens several polys in the same point through linear combination
    fn multi_open_in_point<const N: usize>(
        &self,
        witness: [Evaluations<F>; N],
        multisets: [Evaluations<F>; COLS * 2],
        grand_product: [Evaluations<F>; 2],
        deltas: [F; 3 * COLS + 2],
        eval_point: &HyperPoint<F>,
    ) -> (Open<F, S>, [[F; N]; 3], [F; 2]) {
        assert!(deltas.len() >= 3 * N);
        let mut evals = Vec::with_capacity(N);
        let polys = witness
            .into_iter()
            .chain(multisets.into_iter())
            .chain(grand_product.into_iter());
        let combination = polys
            .into_iter()
            .zip(deltas.into_iter())
            .map(|(poly, delta)| {
                let eval = poly.clone().evaluate(eval_point).unwrap();
                evals.push(eval);
                let ex = poly * delta;
                ex
            })
            .reduce(Add::add)
            .unwrap();
        let proof = self.scheme.multi_scheme().open(combination, eval_point);
        let (slices, rest) = evals.as_chunks::<N>();
        let evals: [[F; N]; 3] = slices.try_into().unwrap();
        let product: [F; 2] = rest.try_into().unwrap();
        (proof, evals, product)
    }
    fn open_proofs(
        &self,
        witness: [Evaluations<F>; COLS],
        multisets: [Evaluations<F>; COLS * 2],
        permutation: [Evaluations<F>; 2],
        eval_point: &HyperPoint<F>,
        deltas: [F; 3 * COLS + 2],
    ) -> OpenMultiProof<F, S, COLS> {
        let partial_point = eval_point.inner()[1..].to_vec();
        let opens_in_random_point = self.multi_open_in_point::<COLS>(
            witness,
            multisets,
            permutation.clone(),
            deltas,
            &eval_point,
        );

        let product_eval = {
            let mut product_eval_point = vec![F::one(); eval_point.inner().len() - 1];
            product_eval_point.push(F::zero());
            let point = HyperPoint::new(product_eval_point);
            let evals = permutation[1].clone();
            let product_open = self.scheme.multi_scheme().open(evals, &point);
            product_open
        };
        let joint_grand_product_poly = {
            let first_var = eval_point.inner()[0];
            let p = permutation[0].clone() * (F::one() - first_var)
                + permutation[1].clone() * first_var;
            p
        };

        let [zero_point, one_point] = [F::zero(), F::one()].map(|last_var| {
            let mut p = partial_point.clone();
            p.push(last_var);
            HyperPoint::new(p)
        });
        let scheme = self.scheme.multi_scheme();
        let permutation = {
            [zero_point, one_point].map(|p| {
                let eval = joint_grand_product_poly.clone().evaluate(&p).unwrap();
                let open = scheme.open(joint_grand_product_poly.clone(), &p);
                (open, eval)
            })
        };
        let [permutation_in_zero, permutation_in_one] = permutation;

        OpenMultiProof {
            opens_in_random_point,
            permutation_in_zero,
            permutation_in_one,
            product_eval,
        }
    }
    pub fn prove(&self, witness: Vec<[F; COLS]>) -> HyperProof<F, S, COLS>
    where
        [(); 3 * COLS + 2 + 2 + 1]:,
        [(); COLS * 3]:,
    {
        println!("prover starts..");
        let vars = self.scheme.vars();
        let len = 1 << vars;
        assert_eq!(witness.len(), len);
        let mut extensions_to_commit = [(); COLS].map(|_| Vec::with_capacity(len));
        //maybe only one extension at once is better
        for row in witness.iter() {
            for t in extensions_to_commit.iter_mut().zip(row.iter()) {
                let (col, v) = t;
                col.push(*v);
            }
        }
        let extensions_to_commit = extensions_to_commit.map(|e| Evaluations::<F>::new(e));

        let scheme = self.scheme.multi_scheme();
        let witness_commitments = extensions_to_commit.each_ref().map(|ex| scheme.commit(ex));
        let bits = Self::bits_of_commitments(&witness_commitments);
        let (challenge_generator, (beta, gamma)) = ChallengeGenerator::new_for_hyper(bits);

        let time = std::time::Instant::now();
        let (mut full_extension, multisets, permutation_polys) =
            self.permutation.prepare_extension(witness, beta, gamma);
        println!("{}ms extension prepared", time.elapsed().as_millis());
        let permutation_polys = permutation_polys.map(|ex| Evaluations::new(ex));
        let permutation_commitments = permutation_polys.each_ref().map(|ex| scheme.commit(ex));
        let permutation_bits = permutation_commitments
            .clone()
            .map(|c| S::Multivariate::commitment_to_bits(c));
        let (mut challenge_generator, alpha, delta) =
            challenge_generator.alpha_and_delta(permutation_bits);

        let powers_of_alpha = Self::powers_of_alpha(alpha);
        let powers_of_delta = Self::powers_of_delta(delta);
        let zero_check_point = challenge_generator.zero_point(vars);
        let time = std::time::Instant::now();
        let zero_check_extension = random_poly_evals_slow(zero_check_point);
        println!("{}ms zero poly evaluated", time.elapsed().as_millis());
        let zero_poly = Evaluations::new(zero_check_extension.clone());

        for (ex, z) in full_extension
            .iter_mut()
            .zip(zero_check_extension.into_iter())
        {
            ex[3 * COLS + 2 + 2 + 0] = z;
        }

        let sumcheck = SumcheckFunc::<
            F,
            UnifiedFunction<G, PermutationCheck<F, COLS>, COLS, GATES>,
            { 3 * COLS + 2 + 2 + 1 },
            { GATES + 2 },
        >::new(vars, powers_of_alpha);
        //dbg!();
        //misc::print_row(&full_extension[0]);
        let time = std::time::Instant::now();
        let (sumcheck_proof, point) = sumcheck.prove(full_extension, challenge_generator);

        println!("{}ms proving sumcheck", time.elapsed().as_millis());
        println!("ZERO: {}", zero_poly.evaluate(&point).unwrap());
        let multisets = multisets.map(|s| Evaluations::new(s));
        let time = std::time::Instant::now();
        let multi_proof = self.open_proofs(
            extensions_to_commit,
            multisets,
            permutation_polys,
            &point,
            powers_of_delta,
        );

        println!("{}ms open proofs", time.elapsed().as_millis());
        HyperProof {
            sumcheck_proof,
            witness_commitments,
            grand_product_commitments: permutation_commitments,
            multi_proof,
        }
    }
    fn shifted_permutation_commitments(
        &self,
        witness: &[Commit<F, S>; COLS],
        beta: F,
        gamma: F,
    ) -> [[Commit<F, S>; COLS]; 2] {
        let t = self.permutation_commitments;
        let gamma = self.one_commitment * gamma;
        t.clone().map(|commits| {
            commits
                .zip(witness.clone())
                .map(|(p, w)| w + p * beta + gamma)
        })
    }

    pub fn verify(&self, proof: HyperProof<F, S, COLS>) -> bool
    where
        [(); COLS * 3]:,
    {
        let vars = self.scheme.vars();
        let HyperProof {
            sumcheck_proof,
            witness_commitments,
            grand_product_commitments,
            multi_proof,
        } = proof;
        let bits = Self::bits_of_commitments(&witness_commitments);
        let (challenge_generator, (beta, gamma)) = ChallengeGenerator::new_for_hyper(bits);
        let permutation_bits = grand_product_commitments
            .clone()
            .map(|c| S::Multivariate::commitment_to_bits(c));
        let (mut challenge_generator, alpha, delta) =
            challenge_generator.alpha_and_delta(permutation_bits);
        let powers_of_alpha = Self::powers_of_alpha(alpha);
        let powers_of_delta = Self::powers_of_delta(delta);
        let zero_check_point = challenge_generator.zero_point(vars);
        let sumcheck = SumcheckFunc::<
            F,
            UnifiedFunction<G, PermutationCheck<F, COLS>, COLS, GATES>,
            { 3 * COLS + 2 + 2 + 1 },
            { GATES + 2 },
        >::new(vars, powers_of_alpha);
        let point = sumcheck.verify(sumcheck_proof, challenge_generator);
        let point = match point {
            Ok(v) => v,
            Err(_) => {
                return false;
            }
        };
        let CheckPoint { point, sum } = point;
        //have to reverse it, [eval_sparse_poly] works in the oposite order
        let zero_point: Vec<_> = zero_check_point.into_iter().rev().collect();
        let zero_check_eval = eval_sparse_poly(&zero_point, point.inner().to_vec());
        println!("ZERO {}", zero_check_eval);
        let permutation_commitments =
            self.shifted_permutation_commitments(&witness_commitments, beta, gamma);
        let vals_in_point = multi_proof.verify(
            &self.scheme,
            &powers_of_delta,
            witness_commitments,
            permutation_commitments,
            grand_product_commitments,
            point,
        );
        let (witness_and_permutations, grand_product, unified_grand_product) = match vals_in_point {
            Some(v) => v,
            None => return false,
        };
        let mut evals = [F::one(); { 3 * COLS + 2 + 2 + 1 }];
        let vals = witness_and_permutations
            .into_iter()
            .map(|e| e.into_iter())
            .flatten()
            .chain(grand_product.into_iter())
            .chain(unified_grand_product.into_iter());
        for (a, b) in evals.iter_mut().zip(vals) {
            *a = b;
        }
        evals[3 * COLS + 2 + 2 + 0] = zero_check_eval;
        sum == sumcheck.eval_in_random_point(evals)
    }
}

impl<F, S, const N: usize> OpenMultiProof<F, S, N>
where
    F: Field,
    S: CommitmentScheme<F>,
{
    fn verify(
        self,
        scheme: &S,
        powers_of_delta: &[F],
        witness_commitments: [Commit<F, S>; N],
        multiset: [[Commit<F, S>; N]; 2],
        permutation_commitments: [Commit<F, S>; 2],
        point: HyperPoint<F>,
    ) -> Option<([[F; N]; 3], [F; 2], [F; 2])>
    where
        [(); 3 * N + 2 + 2 + 1]:,
    {
        assert!(powers_of_delta.len() >= N * 3);
        let Self {
            opens_in_random_point,
            permutation_in_zero,
            permutation_in_one,
            product_eval,
        } = self;
        let scheme = scheme.multi_scheme();

        let evals_in_random_point = {
            let (open, evals, grand_product) = opens_in_random_point;
            let (deltas, rest) = powers_of_delta.as_chunks::<N>();
            let deltas: [[F; N]; 3] = deltas.try_into().unwrap();
            let deltas_for_prod: [F; 2] = rest.try_into().unwrap();
            let eval_comb = evals
                .into_iter()
                .zip(deltas.iter())
                .map(|(evals, deltas)| linear_combination(evals, deltas))
                .reduce(Add::add)
                .unwrap();
            let prod_eval = linear_combination(grand_product, &deltas_for_prod);
            let eval_comb = eval_comb + prod_eval;

            let [num, den] = multiset;
            let commit_comb = [witness_commitments, num, den]
                .into_iter()
                .zip(deltas.iter())
                .map(|(evals, deltas)| linear_combination(evals, deltas))
                .reduce(Add::add)
                .unwrap();
            let prod_commit = linear_combination(permutation_commitments, &deltas_for_prod);
            let commit_comb = commit_comb + prod_commit;

            let check = scheme.verify(commit_comb, &point, eval_comb, open);
            (check.then_some(evals)?, grand_product)
        };

        //check grand product to be 1
        {
            let mut product_eval_point = vec![F::one(); point.vars() - 1];
            product_eval_point.push(F::zero());
            let point = HyperPoint::new(product_eval_point);
            let check = scheme.verify(permutation_commitments[1], &point, F::one(), product_eval);
            if !check {
                return None;
            }
        }

        let permutation_points = {
            let partial_point = point.inner()[1..].to_vec();
            let mut zero_point = partial_point.clone();
            zero_point.push(F::zero());
            let mut one_point = partial_point;
            one_point.push(F::one());
            [zero_point, one_point].map(|p| HyperPoint::new(p))
        };
        let product_commitment = {
            let first_var = point.inner()[0];
            let [left, right] = permutation_commitments;
            let c = left * (F::one() - first_var) + right * first_var;
            c
        };
        let mut valid_open = true;
        let unified_grand_product = permutation_points
            .zip([permutation_in_zero, permutation_in_one])
            .map(|(point, (proof, eval))| {
                let ok = scheme.verify(product_commitment, &point, eval, proof);
                valid_open = valid_open & ok;
                eval
            });
        if !valid_open {
            return None;
        }
        let (witness_and_permutations, grand_product) = evals_in_random_point;
        Some((
            witness_and_permutations,
            grand_product,
            unified_grand_product,
        ))
    }
}

fn linear_combination<A, F, const N: usize>(elements: [A; N], powers: &[F]) -> A
where
    F: Field,
    A: Add<A, Output = A> + Mul<F, Output = A>,
{
    assert!(powers.len() >= N);
    elements
        .into_iter()
        .zip(powers.iter())
        .map(|(a, b)| a * *b)
        .reduce(Add::add)
        .unwrap()
}

///all the N gates, they should evaluate to zero when satisfied
pub trait Gates<const LEN: usize, const N: usize> {
    fn function<F>(vars: &[F; LEN]) -> [F; N]
    where
        F: Element<Out = F>,
        for<'a> &'a F: Element<Out = F>;
}

struct UnifiedFunction<G, P, const COLS: usize, const GATES: usize>
where
    G: Gates<COLS, GATES>,
    P: Gates<{ 3 * COLS + 2 + 2 + 1 }, 2>,
{
    _gates: PhantomData<G>,
    _permutation: PhantomData<P>,
}

impl<G, P, const COLS: usize, const GATES: usize> Function<{ 3 * COLS + 2 + 2 + 1 }, { GATES + 2 }>
    for UnifiedFunction<G, P, COLS, GATES>
where
    G: Gates<COLS, GATES>,
    P: Gates<{ 3 * COLS + 2 + 2 + 1 }, 2>,
{
    fn function<F>(input: [F; 3 * COLS + 2 + 2 + 1], powers_of_alpha: &[F; GATES + 2]) -> F
    where
        F: Element<Out = F>,
        for<'a> &'a F: Element<Out = F>,
    {
        let gate_func = G::function;
        let [p1, p2] = P::function(&input);
        let [a1, a2] = {
            let (v, _) = powers_of_alpha.split_array_ref::<2>();
            v
        };

        let combination = &(&p1 * a1) + &(&p2 * a2);
        let gates_input = input.split_array_ref::<COLS>().0;
        let zero = input[3 * COLS + 2 + 2 + 0].clone();
        let gates = (gate_func)(gates_input);
        let (_, remaining_powers) = powers_of_alpha.rsplit_array_ref::<GATES>();
        let combination = gates
            .into_iter()
            .zip(remaining_powers.iter())
            .fold(combination, |comb, (a, b)| &comb + &(&a * b));
        &combination * &zero
    }
}

impl<Fi: Field, const COLS: usize> Gates<{ 3 * COLS + 2 + 2 + 1 }, 2> for PermutationCheck<Fi, COLS>
where
    [(); 3 * COLS + 2 + 2 + 1]:,
    [(); COLS * 2]:,
{
    fn function<F>(vars: &[F; 3 * COLS + 2 + 2 + 1]) -> [F; 2]
    where
        F: Element<Out = F>,
        for<'a> &'a F: Element<Out = F>,
    {
        let w = vars;
        let num = &w[COLS] * &w[COLS + 1];
        let num = w[(COLS + 2)..(COLS * 2)].iter().fold(num, |s, e| &s * e);
        let den = &w[COLS * 2] * &w[COLS * 2 + 1];
        let den = w[(COLS * 2 + 2)..(COLS * 3)]
            .iter()
            .fold(den, |s, e| &s * e);

        let quot = &w[COLS * 3 + 0];
        //this proves that w[COLS * 2 + 1] is relation
        let rel = &den * quot - num;
        let grand_product = &w[COLS * 3 + 1];
        //this proves that w[COLS * 2 + 2] is grand product
        let grand = grand_product - &(&w[COLS * 3 + 2 + 0] * &w[COLS * 3 + 2 + 1]);
        [rel, grand]
    }
}
