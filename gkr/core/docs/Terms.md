# Terms

## Sumcheck

It allows a prover to convince a verifier that the sum of a multivariate polynomial `𝑓 ( 𝑥_1 , 𝑥_2 , … , 𝑥_𝑣 ) ` over all points in the hypercube `{ 0 , 1 } 𝑣 {0,1}` v equals a claimed value `𝑆`.

In GKR, the sum-check protocol verifies the computation of a circuit layer-by-layer. For a layer 𝐿_𝑖 with 𝑆_𝑖 gates, each gate’s output depends on inputs from the previous layer 𝐿_(𝑖 − 1) with 𝑆_(𝑖 − 1) values. We represent the layer’s computation as a multi-linear polynomial, where 𝑣 is log of the number of variables needed to index all gates.

## Horner's Method
![](./imgs/horner.png