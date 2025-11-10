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
  - GKR allows us to commit only the circuit’s input and output, whereas in Plonkish-based solutions, the prover needs to commit to all the witnesses.

## Resources
- [The GKR Protocol](https://65610.csail.mit.edu/2024/lec/l12-gkr.pdf)
- [A Note on the GKR Protocol](https://people.cs.georgetown.edu/jthaler/GKRNote.pdf)
- [Understanding GKR](https://hackmd.io/@timofey/rJW--amO0)
- [GKR Protocol](https://docs.sotazk.org/terms/gkr_protocol)
- [The GKR Protocol - Proof arguments ZK](https://montekki.github.io/thaler-ch4-4)
- [Understanding GKR](https://research.chainsafe.io/blog/gkr)
- [The GKR Protocol and Its Efficient Implementation](https://people.cs.georgetown.edu/jthaler/gkrnotes.pdf)
- [GKR](https://github.com/amit0365/gkr)
