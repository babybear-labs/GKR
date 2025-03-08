# GKR Recursion Proof Compression

- Steps
  - Circuit Construction
    - [PSE fork of Halo2 v0.3.0](https://github.com/privacy-scaling-explorations/halo2/tree/v0.3.0/halo2)
    - [Halo2 Circuit Construction Repo for integration](https://github.com/poly-layer/halo2)
  - Proof System
    - HyperPlonk
    - GKR
      - [Documentation](./gkr/Readme.md)
  
## Resources
- [GKR - MSM](https://github.com/morgana-proofs/GKR-MSM)
    - Multi-scalar multiplication (MSM): The addition of many points multiplied by many scalars `[𝑎]𝑃+[𝑏]𝑄+[𝑐]𝑅`. There are efficient algorithms to do this in a non-naive way.
    - Multi-exponentiation: Equivalent to an [MSM], if you use the multiplicative notation for your group (common in cryptography)
    - Base field: An elliptic curve is an equation defined over some field `𝐹`. Consequently, elliptic curve points have coordinates in this field. We call this field the base field.
    - Scalar field: 