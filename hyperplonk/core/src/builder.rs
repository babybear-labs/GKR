use crate::{
    hyperplonk::{Gates, HyperPlonk},
    polynomials::CommitmentScheme,
};
use ark_ff::{FftField, Field};
use std::{
    cmp::min,
    ops::{Index, IndexMut},
    rc::Rc,
};

/// as (row,col)
#[derive(Default)]
pub struct Wire {
    pub cell1: (usize, usize),
    pub cell2: (usize, usize),
}
#[derive(Default)]
pub struct CircuitBuilder<const COLS: usize> {
    //only wiring for now
    wiring: Vec<Wire>,
    size: usize,
}

impl<const COLS: usize> CircuitBuilder<COLS>
where
    [(); COLS * 2]:,
{
    pub fn new() -> CircuitBuilder<COLS> {
        CircuitBuilder {
            wiring: vec![],
            size: 0,
        }
    }
    pub fn add_wire(&mut self, wire: Wire) {
        for cell in [wire.cell1, wire.cell2] {
            assert!(cell.0 < self.size);
            assert!(cell.1 < COLS);
        }
        self.wiring.push(wire);
    }
    pub fn add_row(&mut self) {
        self.size += 1;
    }
    fn identity_row(row: usize) -> [(usize, usize); COLS] {
        (0..COLS)
            .into_iter()
            .map(|col| (row, col))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
    fn to_permutation<F: Field>(self, size: usize) -> Vec<[u64; COLS]> {
        let mapping: Vec<[(usize, usize); COLS]> = (0..size)
            .into_iter()
            .map(|row| Self::identity_row(row))
            .collect();
        let aux = mapping.clone();
        let sizes = vec![[1; COLS]; size];

        let mut mapping = Matrix { inner: mapping };
        let mut aux = Matrix { inner: aux };
        let mut sizes = Matrix { inner: sizes };

        let Self { wiring, .. } = self;
        for wire in wiring {
            let Wire {
                mut cell1,
                mut cell2,
            } = wire;
            if aux[cell1] == aux[cell2] {
                continue;
            }
            let size1 = sizes[aux[cell1]];
            let size2 = sizes[aux[cell2]];
            if size1 < size2 {
                std::mem::swap(&mut cell1, &mut cell2);
            }
            sizes[aux[cell1]] = size1 + size2;

            let mut next = cell2;
            let cycle = aux[cell1];
            for _ in 0..min(size1, size2) {
                aux[next] = cycle;
                next = mapping[next];
            }
            let map = mapping[cell1];
            mapping[cell1] = mapping[cell2];
            mapping[cell2] = map;
        }
        let mut mapping = mapping.inner;
        let filler = (mapping.len()..size).map(|row| Self::identity_row(row));
        mapping.extend(filler);

        mapping
            .into_iter()
            .map(|row| row.map(|(row, col)| (row + col * size + 1) as u64))
            .collect()
    }

    pub fn compile<F, S, G, const GATES: usize>(self) -> HyperPlonk<F, S, G, COLS, GATES>
    where
        F: FftField,
        S: CommitmentScheme<F>,
        G: Gates<COLS, GATES>,
        [F; 3 * COLS + 2 + 2 + 1]:,
        [F; GATES + 2]:,
    {
        let size = self.size.next_power_of_two();
        let vars = size.ilog2();
        let permutation = self.to_permutation::<F>(size);

        let scheme = S::init(vars as usize);
        println!("vars: {}", vars);

        let hyperplonk = HyperPlonk::<F, S, G, COLS, GATES>::new(Rc::new(scheme), permutation);
        hyperplonk
    }
}
struct Matrix<T, const C: usize> {
    inner: Vec<[T; C]>,
}

impl<T, const C: usize> Index<(usize, usize)> for Matrix<T, C> {
    type Output = T;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        let (row, col) = index;
        &self.inner[row][col]
    }
}
impl<T, const C: usize> IndexMut<(usize, usize)> for Matrix<T, C> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        let (row, col) = index;
        &mut self.inner[row][col]
    }
}
