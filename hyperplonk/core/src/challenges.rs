use ark_ff::Field;
use rand::{rngs::StdRng, SeedableRng};

#[derive(Default)]
pub(crate) struct ChallengeGenerator<F: Field> {
    hasher: blake3::Hasher,
    challenges: Vec<F>,
}

impl<F: Field> ChallengeGenerator<F> {
    pub fn new(commitment_bits: Vec<u8>) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&commitment_bits);
        Self {
            hasher,
            challenges: Vec::with_capacity(8),
        }
    }
    pub fn new_for_hyper(commitment_bits: Vec<u8>) -> (Self, (F, F)) {
        let mut sel = Self::new(commitment_bits);
        let challenges = sel.permutation_challenges();
        (sel, challenges)
    }
    fn new_challenge(&mut self) -> F {
        let seed = self.hasher.finalize();
        let mut rng = StdRng::from_seed(*seed.as_bytes());

        let challenge = F::rand(&mut rng);

        let mut bytes = vec![];
        challenge.write(&mut bytes).unwrap();
        self.hasher.update(&bytes);

        challenge
    }

    ///alpha to combine gates and delta for the linear combination of commitments
    pub fn alpha_and_delta(mut self, permutation_bits: [Vec<u8>; 2]) -> (Self, F, F) {
        let [a, b] = permutation_bits;
        self.hasher.update(&a);
        self.hasher.update(&b);
        let alpha = self.new_challenge();
        let delta = self.new_challenge();
        (self, alpha, delta)
    }
    pub fn zero_point(&mut self, vars: usize) -> Vec<F> {
        (0..vars).map(|_| self.new_challenge()).collect()
    }

    //beta and gamma
    fn permutation_challenges(&mut self) -> (F, F) {
        let [a, b] = [(), ()].map(|_| self.new_challenge());
        (a, b)
    }

    pub fn next_challenge(&mut self, message: (F, F)) -> F {
        //ideally replace all of this with a poseidon hash or similar
        let mut bytes = vec![];
        message.0.write(&mut bytes).unwrap();
        message.1.write(&mut bytes).unwrap();
        self.hasher.update(&bytes);

        let seed = self.hasher.finalize();
        let mut rng = StdRng::from_seed(*seed.as_bytes());

        let challenge = F::rand(&mut rng);

        bytes.clear();
        challenge.write(&mut bytes).unwrap();

        self.hasher.update(&bytes);

        self.challenges.push(challenge);
        challenge
    }
    pub fn challenges(self) -> Vec<F> {
        self.challenges
    }
}
