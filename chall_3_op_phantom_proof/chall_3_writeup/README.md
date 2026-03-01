# Operation Phantom Proof — Writeup

**Category:** Zero-Knowledge / Cryptography | **Difficulty:** Hard | **Points:** ___

---

## TL;DR

<!-- One-liner: what's the verifier bug and how did you exploit it? -->

---

## Challenge Overview

A custom STARK prover/verifier for Rescue-Prime hash preimage proofs over BabyBear (`p = 2013265921`).

**Goal:** Produce a valid proof for a *false* statement — i.e., a proof that `Rescue(input) = output` where `output` is incorrect. Submit to the oracle to get the flag.

### Key Parameters

| Parameter | Value |
|---|---|
| Field | BabyBear (p = 2³¹ − 2²⁷ + 1) |
| Rescue rounds | 7 |
| State width | 4 (rate=2, capacity=2) |
| Trace rows | 8 |
| Blowup factor | 8 |
| FRI queries | 28 |

---

## Vulnerability Analysis

<!-- Describe what you found in the verifier code -->
<!-- Which file/function? What check is missing or flawed? -->

### Location

```
src/stark/verifier.rs  — line ___
```

### Root Cause

<!-- e.g. missing boundary constraint check, weak Fiat-Shamir binding,
     FRI folding bug, insufficient query sampling, etc. -->

```rust
// TODO: paste the vulnerable code snippet
```

### Why It Breaks Soundness

<!-- Explain why this allows a false proof to pass verification -->

---

## Exploit Construction

### Step 1 — Generate an Honest Proof

```bash
./target/release/phantom-stark hash --input 0,0
# → honest output: <y0>,<y1>

./target/release/phantom-stark prove --input 0,0 --output <y0>,<y1> -o honest.bin
```

### Step 2 — Craft the Forged Proof

```bash
# TODO: describe how you modified the proof or prover
```

### Step 3 — Submit to Oracle

```bash
PROOF_HEX=$(xxd -p proof.bin | tr -d '\n')

curl -s http://localhost:3000/oracle \
  -H 'Content-Type: application/json' \
  -d "{\"input\": [0, 0], \"output\": [1337, 42], \"proof\": \"$PROOF_HEX\"}" | jq .
```

---

## Flag

```
TODO
```

---

## Key Takeaways

- STARK soundness depends on every constraint being checked — a single missing validation breaks the entire proof system.
- Custom crypto implementations are high-risk; production systems use audited libraries for a reason.
- <!-- Add specific lesson from the bug you found -->
