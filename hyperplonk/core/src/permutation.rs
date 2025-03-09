use ark_ff::Field;
use std::{collections::VecDeque, marker::PhantomData};

pub struct PermutationCheck<F: Field, const COLS: usize>
where
    //anti-overflow constraint
    [F; 3 * COLS + 2 + 2 + 1]:,
    [F; COLS * 2]:,
{
    perm: Vec<[u64; COLS]>,
    _field: PhantomData<F>,
}

impl<const COLS: usize, F: Field> PermutationCheck<F, COLS>
where
    //anti-overflow constraint
    [F; 3 * COLS + 2 + 2 + 1]:,
    [F; COLS * 2]:,
{
    ///creates new and return the permutations to commit
    pub fn new(perm: Vec<[u64; COLS]>) -> (Self, [[Vec<F>; COLS]; 2]) {
        let len = perm.len() as u64;
        let mut identity_extensions = [(); COLS].map(|_| Vec::with_capacity(len as usize));
        let mut permutation_extensions: [Vec<F>; COLS] = identity_extensions.clone();
        for (i, row) in perm.iter().enumerate() {
            for (col, p) in identity_extensions.iter_mut().enumerate() {
                p.push(F::from(i as u64 + col as u64 * len + 1));
            }
            for (a, b) in permutation_extensions.iter_mut().zip(row.iter()) {
                a.push(F::from(*b));
            }
        }

        (
            Self {
                perm,
                _field: PhantomData,
            },
            [identity_extensions, permutation_extensions],
        )
    }
    //extends the witness extensions with permutation related columns
    pub fn prepare_extension(
        &self,
        witness: Vec<[F; COLS]>,
        beta: F,
        gamma: F,
    ) -> (
        Vec<[F; 3 * COLS + 2 + 2 + 1]>,
        [Vec<F>; COLS * 2],
        [Vec<F>; 2],
    )
    where
        [(); COLS * 2]:,
    {
        assert_eq!(self.perm.len(), witness.len());
        let len = self.perm.len();

        let zero = F::zero();
        let mut extensions = witness
            .into_iter()
            .map(|row| {
                let mut new = [zero; 3 * COLS + 2 + 2 + 1];
                for (old, new) in row.into_iter().zip(new.iter_mut()) {
                    *new = old;
                }
                new
            })
            .collect::<Vec<_>>();

        let mut aux = Vec::with_capacity(len);
        let mut pending = None;
        let mut add_to_aux = |e| match pending.take() {
            Some(prev) => {
                aux.push([prev, e]);
            }
            None => pending = Some(e),
        };
        let mut product_check_members = [(); COLS * 2].map(|_| Vec::with_capacity(len));

        let mut total = (F::one(), F::one());
        let mut for_quot = (Vec::new(), Vec::new());
        let iter = extensions.iter_mut().zip(self.perm.iter());
        for (i, (extensions, perm)) in iter.enumerate() {
            {
                let (witness, rest) = extensions.split_array_mut::<COLS>();
                for (i2, (w, p)) in witness.iter().zip(perm.iter()).enumerate() {
                    let num = *w + beta * F::from((i + (len * i2) + 1) as u64) + gamma;
                    total.0 *= num;
                    for_quot.0.push(num);
                    rest[i2] = num;
                    product_check_members[i2].push(num);
                    let den = *w + beta * F::from(*p) + gamma;
                    total.1 *= den;
                    for_quot.1.push(den);
                    rest[i2 + COLS] = den;
                    product_check_members[i2 + COLS].push(den);
                }
            }
            let product = |v: &[F]| (v[0..COLS]).iter().fold(F::one(), |a, b| a * b);
            let num: F = product(&extensions[COLS..(COLS * 2)]);
            let den = product(&extensions[(COLS * 2)..(COLS * 3)]);
            let rel = num / den;
            add_to_aux(rel);
            extensions[COLS * 3] = rel;
        }

        let mut grand_product_poly = Vec::with_capacity(len);
        let first_half = extensions.iter().map(|elems| elems[COLS * 3 + 0]);
        let mut temp = first_half.collect::<VecDeque<_>>();
        loop {
            if temp.len() > 1 {
                let a = temp.pop_front();
                let b = temp.pop_front();
                let (a, b) = a.zip(b).unwrap();
                let p = a * b;
                grand_product_poly.push(p);
                add_to_aux(p);
                temp.push_back(p);
            } else {
                grand_product_poly.push(F::zero());
                add_to_aux(F::zero());
                break;
            }
        }
        debug_assert_eq!(grand_product_poly.len(), len);
        for ((elems, produc), aux) in extensions
            .iter_mut()
            .zip(grand_product_poly.into_iter())
            .zip(aux)
        {
            elems[COLS * 3 + 1] = produc;
            let [a1, a2] = aux;
            elems[COLS * 3 + 2 + 0] = a1;
            elems[COLS * 3 + 2 + 1] = a2;
        }
        let polys_to_commit = extensions
            .iter()
            .map(|row| (row[COLS * 3 + 0], row[COLS * 3 + 1]))
            .unzip();
        let (a, b) = polys_to_commit;

        (extensions, product_check_members, [a, b])
    }
}

#[cfg(test)]
mod test {
    use super::VecDeque;
    use crate::polynomials::{Evaluations, HyperPoint};
    use ark_ff::{One, UniformRand, Zero};
    use rand::thread_rng;
    use std::ops::{Add, Mul};

    #[test]
    fn perm() {
        use ark_bls12_381::Fr;
        let len = 1 << 4;
        let mut rng = thread_rng();
        let mut make_vec = || (0..len).map(|_| Fr::rand(&mut rng)).collect::<Vec<_>>();

        let w1 = make_vec();
        let mut w1p = w1.clone();
        w1p.sort();
        let w2 = make_vec();
        let mut w2p = w2.clone();
        w2p.sort();

        let div = |a: Vec<_>, b: Vec<_>| {
            a.into_iter()
                .zip(b.into_iter())
                .map(|(a, b)| a * b)
                .collect::<Vec<_>>()
        };
        let w = div(w1, w2);
        let wp = div(w1p, w2p);
        let div = w
            .iter()
            .zip(wp.iter())
            .map(|(a, b)| *a / *b)
            .collect::<Vec<_>>();
        let gran_product_relation = div.clone().into_iter().reduce(Mul::mul).unwrap();
        println!("{}", gran_product_relation);
        assert_eq!(gran_product_relation, Fr::one());

        let div_check = wp
            .into_iter()
            .zip(div.into_iter())
            .map(|(a, b)| a * b)
            .zip(w.into_iter())
            .map(|(a, b)| a - b)
            .reduce(Add::add)
            .unwrap();
        assert_eq!(div_check, Fr::zero());
    }

    #[test]
    fn spliting() {
        use ark_bls12_381::Fr;
        let vars = 4;
        let n_evals = 1 << vars;
        let mut rng = thread_rng();

        let point: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();

        let evals: Vec<_> = (0..n_evals).map(|_| Fr::rand(&mut rng)).collect();
        let (left, right) = evals.split_at(n_evals / 2);
        let [left, right] = [left, right].map(|p| Evaluations::new(p.to_vec()));
        let poly = Evaluations::new(evals.clone());

        let first_var = point[0];
        let [left, right] = {
            let point = point[1..].to_vec();
            let point = HyperPoint::new(point);
            [left, right].map(|p| p.evaluate(&point).unwrap())
        };
        let full_eval = {
            let point = HyperPoint::new(point);
            poly.evaluate(&point).unwrap()
        };
        let combined_eval = first_var * right + (Fr::one() - first_var) * left;

        assert_eq!(combined_eval, full_eval);
    }

    #[test]
    fn deq() {
        let mut product = ["a", "b", "c", "d", "e", "f", "g", "h"]
            .map(String::from)
            .to_vec();
        let mut temp = VecDeque::from(product.clone());
        loop {
            if temp.len() > 1 {
                let a = temp.pop_front();
                let b = temp.pop_front();
                let (a, b) = a.zip(b).unwrap();
                let p = format!("{a}{b}");
                product.push(p.clone());
                temp.push_back(p);
            } else {
                product.push(String::from("zero"));
                break;
            }
        }
        println!("{:?}", product);
    }

    #[test]
    fn grand_product() {
        use ark_bls12_381::Fr;
        let vars = 4;
        let n_evals = 1 << vars;
        let mut rng = thread_rng();

        //let point: Vec<_> = (0..vars).map(|_| Fr::rand(&mut rng)).collect();
        let evals: Vec<_> = (0..n_evals).map(|_| Fr::rand(&mut rng)).collect();

        //let print = |a: &Vec<_>| {
        //println!("----");
        //for e in a.iter() {
        //println!("{}", e);
        //}
        //};

        let true_product = evals.iter().copied().reduce(Mul::mul).unwrap();

        let mut grand_product = vec![];
        let mut temp = evals.iter().copied().collect::<VecDeque<_>>();
        loop {
            if temp.len() > 1 {
                let a = temp.pop_front();
                let b = temp.pop_front();
                let (a, b) = a.zip(b).unwrap();
                let p = a * b;
                grand_product.push(p);
                temp.push_back(p);
            } else {
                grand_product.push(Fr::zero());
                break;
            }
        }
        assert_eq!(true_product, grand_product[n_evals - 2]);
    }
}
