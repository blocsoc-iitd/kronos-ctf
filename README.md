<div align="center">

```
 ██╗  ██╗██████╗  ██████╗ ███╗   ██╗ ██████╗ ███████╗
 ██║ ██╔╝██╔══██╗██╔═══██╗████╗  ██║██╔═══██╗██╔════╝
 █████╔╝ ██████╔╝██║   ██║██╔██╗ ██║██║   ██║███████╗
 ██╔═██╗ ██╔══██╗██║   ██║██║╚██╗██║██║   ██║╚════██║
 ██║  ██╗██║  ██║╚██████╔╝██║ ╚████║╚██████╔╝███████║
 ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝
                     C  T  F
```

**Cryptography · Smart Contracts · Zero-Knowledge Proofs**

[![Challenges](https://img.shields.io/badge/challenges-3-blue)]()
[![Difficulty](https://img.shields.io/badge/difficulty-medium-orange)]()
[![Organisation](https://img.shields.io/badge/Organisation-BlocSoc_IITD-purple)]()

</div>

---

## Challenges

| #   | Name                                                 | Category           | Difficulty  | Points | # of solves |
| --- | ---------------------------------------------------- | ------------------ | ----------- | ------ | ----------- |
| 1   | [Fractured Lattices](chall_1_fractured_lattices/)    | Crypto / Forensics | Hard        | 500    |  02         |
| 2   | [Siege Protocol](chall_2_block_participants/)        | Smart Contracts    | Medium      | 250    |  08         |
| 3   | [Operation Phantom Proof](chall_3_op_phantom_proof/) | ZK / STARK         | Hard        | 500    |  07         |

---

## Challenge Summaries

### 1 — Fractured Lattices

> _The lattice remembers what the cipher forgot._

An LWE encryption scheme protects a flag. The matrix is derived from a secret prime — but the prime was shattered into fragments and hidden in plain sight. Recover the pieces, reconstruct the lattice, and break the cipher.

**Skills:** forensics, lattice cryptography, LLL reduction

### 2 — Siege Protocol

> _The chain remembers every king — but only one rules forever._

A DeFi kingdom on Sepolia where challengers fight for the throne and the treasury funds loans. Two vulnerabilities must be chained: lock the throne permanently via a refund DoS, then drain the treasury through a batch collateral bypass.

**Skills:** Solidity, smart contract exploitation, Foundry

### 3 — Operation Phantom Proof

> _Trust nothing. Verify everything. Unless the verifier is broken._

A custom STARK prover/verifier for Rescue-Prime hashes over BabyBear. Find the soundness bug in the verifier and forge a valid proof for a false statement. Submit it to the oracle for the flag.

**Skills:** zero-knowledge proofs, STARK internals, Rust

---

## Structure

```
kronos-ctf/
├── chall_1_fractured_lattices/
│   ├── chall_1/              # participant files (cipher.txt)
│   └── chall_1_writeup/      # solution writeup
├── chall_2_block_participants/
│   ├── chall_2/              # participant Foundry project
│   └── chall_2_writeup/      # solution writeup
├── chall_3_op_phantom_proof/
│   ├── chall_3/              # participant Rust project
│   └── chall_3_writeup/      # solution writeup
└── README.md
```

Each challenge directory contains:

- **`chall_N/`** — files distributed to participants
- **`chall_N_writeup/`** — organizer solution writeup

---

## Rules

- You may use any tools, languages, or frameworks.
- You may deploy your own contracts (challenge 2).
- You may **not** brute-force oracle endpoints.
- You may **not** attack infrastructure.
- Flag format: `tryst{...}` or as specified per challenge.

---

## Getting Started

```bash
git clone <repo-url>
cd kronos-ctf

# Challenge 1 — just needs Python + SageMath
cat chall_1_fractured_lattices/chall_1/cipher.txt

# Challenge 2 — needs Foundry + Sepolia ETH
cd chall_2_block_participants/chall_2
forge build

# Challenge 3 — needs Rust
cd chall_3_op_phantom_proof/chall_3
cargo build --release
```

---

<div align="center">

_Good luck. The clock is ticking._

</div>
