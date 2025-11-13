# GKR Recursion Proof Compression (WIP)

A research prototype exploring Plonk + GKR integration for recursive proof compression (open problem).

The goal: design a sensible IR that can be compiled to a Plonk/HyperPlonk backend with deferred subcircuits verified via GKR-style sumchecks.

## Overview

This project experiments with:
- Combining Plonk constraint systems and GKR protocols.
- Exploring *recursive* composition through layered sumchecks.
- Investigating Poseidon and MSM as realistic subcircuit benchmarks.
- Building a foundation for HyperPlonk-based recursive compression.

| Module                                     | Description                                                                  |
| :----------------------------------------- | :--------------------------------------------------------------------------- |
| [`gkr`](./gkr)                             | Core GKR protocol logic, including sumcheck and verifier folding.            |
| [`basic/poseidon2`](./basic/poseidon)       | Poseidon hash circuit example implemented via GKR-style gates.               |
| [`circuit-construct`](./circuit-construct) | Circuit IR and Plonk-style compiler for layered circuit construction.        |
| [`hyperplonk`](./hyperplonk)               | Multilinear Plonk backend (HyperPlonk PIOP) for recursive proof compression. |


## Key Resources

* [**HOW TO: GKR Compression**](https://hackmd.io/@levs57/SJb7-WZFyx) — by Lev Soukhanov, on integrating GKR with Plonk.
* [**Proof Aggregation using GKR**](https://hackmd.io/@soowon/gkr) — overview of GKR-based aggregation.
* [**GKRFold: SumFold-based GKR Proof Compression**](https://ethresear.ch/t/gkrfold-sumfold-based-gkr-proof-compression/21788)
* [**GKRFold: SumFold-based GKR Proof Compression HackMD**](https://hackmd.io/b_9x0pmAQKaoziQ4PjlSUQ)
* [**GKR–MSM Implementation**](https://github.com/morgana-proofs/GKR-MSM) — reference design for MSM via GKR.
  * *Multi-Scalar Multiplication (MSM)*: efficient computation of `[a]P + [b]Q + [c]R` over elliptic curves.
  * *Multi-Exponentiation*: equivalent formulation in multiplicative groups.
  * *Base Field*: field over which elliptic curve points are defined.
* [**PSE Fork of Halo2 v0.3.0**](https://github.com/privacy-scaling-explorations/halo2/tree/v0.3.0/halo2) — baseline for circuit integration.
* [**Halo2 Circuit Construction Repo**](https://github.com/poly-layer/halo2) — reusable components for circuit layering and wiring.
  
## Acknowledgement
This project is part of [Explore expander bounty](https://www.youtube.com/watch?v=eqwpVA3baok&list=PLfEHHr3qexv89-MW6VVMOMZIzrv3YBFIR&index=13), advised by [lev soukhanov](https://x.com/levs57)
