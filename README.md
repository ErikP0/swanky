# **swanky**: A suite of rust libraries for secure multi-party computation

**swanky** provides a suite of rust libraries for doing secure multi-party
computation (MPC).

This fork extends **fancy-garbling** by adding support for arithmetic operations in GF(2^k). See below for concrete changes.

![library diagram](diagram.png)

* **fancy-garbling**: Boolean and arithmetic garbled circuits.
  * **twopac**: Two-party garbled-circuit-based secure computation.
* **ocelot**: Oblivious transfer and oblivious PRFs.
* **popsicle**: Private-set intersection.
* **scuttlebutt**: Core MPC-related primitives used by various **swanky**
  libraries.

# A Note on Security

**swanky** is currently considered **prototype** software. Do not deploy it in
production, or trust it with sensitive data.

# License

**swanky** from Galois: MIT License

extension: MIT License

# Authors

- Brent Carmer <bcarmer@galois.com>
- Alex J. Malozemoff <amaloz@galois.com>
- Marc Rosen <marc@galois.com>

Extension
- Erik Pohle <erik.pohle@esat.kuleuven.be>
- Robbe Vermeiren
- Elias Wils

# Extension
Arithmetic garbled circuits now support addition, constant-multiplication and projection in the field GF(2^k) for k ≤ 8.

Most of the changes in the library are the introduction of a proper `Modulus` type (instead of the old `u16`)
```rust
pub enum Modulus {
    /// Integer modulus for Zq.
    Zq { q: u16 },
    /// Irreducible polynomial for GF(2^4).
    GF4 { p: u8 },
    /// Irreducible polynomial for GF(2^8).
    GF8 { p: u16 },
    /// Irreducible polynomial for GF(2^k).
    GFk { k: u8, p: u16 }
}
```
which specifies the ring/field and the modulus (respectively the Irreducible polynomial).
All existing tests, bundles and the CRT code have been adapted to work and output `Modulus::Zq` moduli.

Consequently, the `Wire` type was extended as well to allow dedicated representation of a wire label in GF(2^k)^l.
```rust
pub enum Wire {
    ...
    /// An element in the field GF(2^k) is represented by the coefficients of the polynomial
    /// EXAMPLE: x^3 + x + 1 in GF(2^4)
    ///   ===>   Elt: u16 = (0000...0 1 0 1 1) = 11
    ///
    ///
    /// Representation of a wire in GF(2^4)
    GF4 {
        /// Irreducible polynomial.
        p: u8,
        /// A list of GF(2^4) elements.
        elts: Vec<u16>,
    },
    /// Representation of a wire in GF(2^8)
    GF8 {
        /// Irreducible polynomial
        p: u16,
        /// A list of GF(2^4) elements
        elts: Vec<u16>,
    },
    /// Representation of a wire in GF(2^k)
    GFk {
        /// k
        k: u8,
        /// Irreducible polynomial.
        p: u16,
        /// A list of GF(2^4) elements.
        elts: Vec<u16>,
    },
}
```

Thus, wires and gates can perform either arithmetic in `Z_q` or `GF(2^k)`, and, as usual, projection gates translate between moduli.

# Acknowledgments

This material is based upon work supported by the ARO and DARPA under Contract
No. W911NF-15-C-0227 and by DARPA and SSC Pacific under Contract No.
N66001-15-C-4070.

Any opinions, findings and conclusions or recommendations expressed in this
material are those of the author(s) and do not necessarily reflect the views of
the ARO, SSC Pacific, and DARPA.

Copyright © 2019 Galois, Inc.
