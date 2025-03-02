# GKR

B = # subCircuits of size is 
S = # of gates
n = # variables
d = # layers of the circuit

Prover Time: `O(B*S + S logS)` 
    - Constant factor slower than unverified
Verifier Time: `O(B*n + d log(B*S))` 
    - Sublinear in circuit size
Communication Cost: `d log(B*S)` 
    - Field elements

- GKR layered circuit
  - layers[0] is the input layer; subsequent layers compute gate outputs.