# ZKP Age Proof

A zero-knowledge proof system that proves a person is 18 or older **without revealing their date of birth**. Built with [Plonky3](https://github.com/Plonky3/Plonky3) STARKs.

## Context

Identity verification often requires sharing personal data (date of birth, address, documents) with third parties. This project demonstrates a different model: **sharing proofs instead of data**.

A prover who knows someone's year of birth can generate a cryptographic STARK proof that the person is at least 18. Anyone can verify this proof without learning the actual year of birth.

## How it works

```
                PROVER (backend)                          VERIFIER (anyone)
               ==================                        ==================

  Secret input:  year_of_birth = 1995
  Public input:  current_year  = 2026       ------>   Knows: age_ok=1, current_year=2026
                                                      Does NOT know: year_of_birth
         |
         v
  +------------------+
  | AIR constraints  |    Polynomial constraints enforce:
  |                  |      1. age_ok is 0 or 1
  |  age = 2026 -   |      2. if age_ok=1 then age >= 18
  |        1995 = 31 |      3. if age_ok=0 then age < 18
  |  31 >= 18  -->   |      4. range check via bit decomposition
  |  age_ok = 1      |
  +------------------+
         |
         v
  STARK proof (~18 KB)  ---------------------->   verify(proof, public_values)
                                                        |
                                                        v
                                                  OK / FAIL
```

### Constraint system

The AIR (Algebraic Intermediate Representation) enforces correctness using two conditional constraints and a range check:

- **Adult case** (`age_ok = 1`): the prover provides `diff` such that `year_of_birth + diff + 18 = current_year`. Since `diff` is decomposed into 7 boolean bits, it must be in `[0, 127]`, which guarantees `age - 18 >= 0`.

- **Minor case** (`age_ok = 0`): the prover provides `diff` such that `current_year + diff + 1 = year_of_birth + 18`. This means `diff = 17 - age >= 0`, again range-checked via bit decomposition.

A malicious prover cannot fake the result because the constraints are verified over the entire polynomial domain.

## Project structure

```
src/
  main.rs        -- Entry point: runs 4 demo proofs (adult, minor, edge cases)
  air.rs         -- AIR definition: polynomial constraints for age >= 18
  columns.rs     -- Trace column layout (year_of_birth, current_year, age_ok, diff_bits)
  config.rs      -- STARK configuration (KoalaBear field, Poseidon2 hash, FRI PCS)
  generation.rs  -- Trace generation: fills the execution trace from private inputs
```

## Requirements

- Rust 1.85+ (edition 2024)
- All dependencies are pulled from crates.io (Plonky3 v0.5)

## Build

```bash
cargo build
```

For optimized builds (recommended for benchmarking):

```bash
RUSTFLAGS="-Ctarget-cpu=native" cargo build --release
```

## Run the demo

The `main` binary runs 4 proof scenarios end-to-end (generate + verify):

```bash
cargo run
```

This produces output like:

```
=== ZKP Age Proof (Plonky3 STARK) ===

--- Test 1: Adulte (ne en 1995, annee 2026) ---
  Year of birth: 1995 (SECRET - non revele)
  Current year:  2026 (PUBLIC)
  Public values:  age_ok=1, current_year=2026
  Generating STARK proof...
  Proof size: 18678 bytes
  Verifying proof...
  VERIFIED: age >= 18 = true (without revealing year of birth!)
```

### Test cases

| # | Year of birth | Current year | Age | age_ok | Description |
|---|---------------|-------------|-----|--------|-------------|
| 1 | 1995 | 2026 | 31 | 1 | Adult |
| 2 | 2015 | 2026 | 11 | 0 | Minor |
| 3 | 2008 | 2026 | 18 | 1 | Exactly 18 (edge case) |
| 4 | 2009 | 2026 | 17 | 0 | Exactly 17 (edge case) |

## Run in release mode

Release mode is significantly faster for both proof generation and verification:

```bash
RUSTFLAGS="-Ctarget-cpu=native" cargo run --release
```

## Performance

Measured in debug mode on a single core:

| Operation | Time | Size |
|-----------|------|------|
| Proof generation | ~8 ms | ~18 KB |
| Verification | ~16 ms | - |

Release mode with `target-cpu=native` will be faster.

## STARK configuration

| Parameter | Value |
|-----------|-------|
| Field | KoalaBear (~31-bit prime) |
| Hash | Poseidon2 (width 16) |
| PCS | FRI (log_blowup=2, 28 queries) |
| Extension field | BinomialExtensionField degree 4 |
| Trace height | 8 rows (1 data row + 7 padding) |
| Trace width | 10 columns |

These parameters are tuned for development/testing. For production, increase `num_queries` and add proof-of-work bits for higher security.

## Deployment: who runs what

In a real system, the code is split across three roles:

### Prover (backend server)

The prover holds the secret data and generates proofs. It needs:

| File | Why |
|------|-----|
| `generation.rs` | Builds the execution trace from the secret `year_of_birth` |
| `air.rs` + `columns.rs` | AIR definition (shared with verifier) |
| `config.rs` | STARK configuration (shared with verifier) |

The prover calls `generate_age_proof_trace()` then `prove()`, serializes the resulting proof with `postcard::to_allocvec(&proof)`, and sends the bytes along with the public values.

```rust
let (trace, pis) = generate_age_proof_trace::<Val>(year_of_birth, current_year, 8);
let proof = prove(&config, &AgeProofAir, trace, &pis);
let proof_bytes = postcard::to_allocvec(&proof).unwrap();
// Send (proof_bytes, pis) to blockchain or verifier
```

### Blockchain (on-chain storage)

The chain stores only the proof and the public values. No secret data ever touches it.

| Data | Size | Content |
|------|------|---------|
| `proof_bytes` | ~18 KB | Serialized STARK proof |
| `public_values` | 8 bytes | `[age_ok, current_year]` as field elements |

Optionally, the STARK configuration parameters can be hardcoded in a smart contract or published once and referenced by ID.

### Verifier (any client / smart contract / auditor)

The verifier has no access to the secret year of birth. It only needs:

| File | Why |
|------|-----|
| `air.rs` + `columns.rs` | Same AIR definition as the prover |
| `config.rs` | Same STARK configuration as the prover |

It deserializes the proof, rebuilds the config, and calls `verify()`:

```rust
let proof: Proof<MyStarkConfig> = postcard::from_bytes(&proof_bytes).unwrap();
let config = stark_config();
verify(&config, &AgeProofAir, &proof, &pis).expect("invalid proof");
// pis[0] = age_ok (1 or 0), pis[1] = current_year
```

The verifier does **not** need `generation.rs` at all -- it never sees the trace or the year of birth.

## License

MIT
