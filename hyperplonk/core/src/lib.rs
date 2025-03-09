#![feature(array_zip)]
#![feature(array_methods)]
#![feature(generic_const_exprs)]
#![feature(slice_as_chunks)]
#![feature(split_array)]

pub mod builder;
mod challenges;
pub mod commitment;
pub(crate) mod function_sumcheck;
pub mod gates;
pub mod hyperplonk;
pub(crate) mod misc;
pub(crate) mod permutation;
pub(crate) mod polynomials;
// pub mod sumcheck;
//pub mod interactive_sumcheck;
pub use function_sumcheck::Element;

#[cfg(test)]
mod example {
    use crate::{
        builder::{CircuitBuilder, Wire},
        commitment::MultilinearKzgScheme,
        function_sumcheck::Element,
        hyperplonk::{Gates, HyperPlonk},
    };
    use ark_bls12_381::{Bls12_381, Fr};
    use ark_ff::UniformRand;
    use rand::thread_rng;
    use std::time::Instant;

    const COLS: usize = 16;
    type Plonk<G> = HyperPlonk<Fr, MultilinearKzgScheme<Bls12_381>, G, COLS, 2>;

    struct TestGate;
    impl Gates<COLS, 2> for TestGate {
        fn function<F>(vars: &[F; COLS]) -> [F; 2]
        where
            F: Element<Out = F>,
            for<'a> &'a F: Element<Out = F>,
        {
            // let [a] = crate::gates::Sub::function(vars);
            let [b] = crate::gates::Product::function(vars);
            [b.clone(), b]
        }
    }
    const SIZE: usize = 1 << 16;
    #[test]
    fn proof_system() {
        let mut builder = CircuitBuilder::<COLS>::default();
        let mut rng = thread_rng();

        builder.add_row();
        for i in 1..SIZE {
            builder.add_row();
            builder.add_wire(Wire {
                cell1: (i - 1, 4),
                cell2: (i - 1, 5),
            });
            builder.add_wire(Wire {
                cell1: (i - 1, 2),
                cell2: (i, 0),
            });
        }
        // builder.add_wire(Wire {
        // cell1: (0, 4),
        // cell2: (0, 5),
        // });
        let circuit: Plonk<TestGate> = builder.compile();

        let mut witness: Vec<[Fr; COLS]> = vec![];
        let mut last = Fr::rand(&mut rng);
        // let mut last = Fr::zero();
        for _ in 0..SIZE {
            let mut row = [(); COLS].map(|_| Fr::rand(&mut rng));
            let new = Fr::rand(&mut rng);
            row[0] = last;
            row[1] = new;
            row[2] = last * new;
            last = row[2];
            //remove
            row[4] = new;
            row[5] = new;
            witness.push(row);
        }
        // witness[5][4] = Fr::rand(&mut rng);

        println!("meassuring...");
        let now = Instant::now();
        let proof = circuit.prove(witness);
        let time = now.elapsed();
        println!("TIME: {}s {}ms", time.as_secs(), time.as_millis());

        let check = circuit.verify(proof);

        assert!(check);
    }
}
