# Operation Phantom Proof

A custom STARK prover/verifier for Rescue-Prime hash preimage proofs.

## Challenge

This binary implements a STARK proof system for the Rescue-Prime hash function over the BabyBear field (p = 2^31 - 2^27 + 1).

The `oracle` command checks whether you can produce a *valid proof* for a *false statement* — specifically, a proof that Rescue-Prime(input) = output, where the claimed output doesn't match the actual hash.

If you can produce such a proof, the oracle reveals the flag.

## Usage

```bash
# Build
cargo build --release

# Compute a hash
./target/release/phantom-stark hash --input 0,0

# Generate an honest proof
./target/release/phantom-stark prove --input 0,0 --output <y0>,<y1> -o proof.bin

# Verify a proof
./target/release/phantom-stark verify --input 0,0 --output <y0>,<y1> --proof proof.bin
```

## Oracle

The oracle runs as a separate webserver in `../flag1-oracle/`. It accepts proof submissions over HTTP.

### Running the oracle

```bash
cd ../flag1-oracle
cargo run --release
# Listening on 0.0.0.0:3000
```

### API

**`POST /oracle`** — Submit a proof for flag checking.

Request body (JSON):

```json
{
  "input": [0, 0],
  "output": [1337, 42],
  "proof": "<hex-encoded proof bytes>"
}
```

- `input` — two u32 values (the Rescue-Prime input)
- `output` — two u32 values (the claimed output)
- `proof` — the proof file contents, hex-encoded

Response:

```json
{
  "success": true,
  "flag": "FLAG{...}",
  "message": "Congratulations! You found the vulnerability!"
}
```

### Example with curl

```bash
# Generate a proof first
./target/release/phantom-stark prove --input 0,0 --output 1337,42 -o proof.bin

# Hex-encode and submit
PROOF_HEX=$(xxd -p proof.bin | tr -d '\n')

curl -s http://localhost:3000/oracle \
  -H 'Content-Type: application/json' \
  -d "{\"input\": [0, 0], \"output\": [1337, 42], \"proof\": \"$PROOF_HEX\"}" | jq .
```

**`GET /health`** — Health check. Returns `phantom-oracle alive`.

## Architecture

The implementation is structured as:

- `field/` — BabyBear prime field arithmetic (Montgomery form)
- `poly/` — Dense polynomials with FFT/IFFT
- `hash/` — Rescue-Prime permutation and hash
- `merkle/` — SHA-256 Merkle tree commitments
- `air/` — Algebraic Intermediate Representation for Rescue-Prime
- `fri/` — FRI (Fast Reed-Solomon IOP) protocol
- `stark/` — STARK prover and verifier orchestration
## Parameters

| Parameter | Value |
|-----------|-------|
| Field | BabyBear (p = 2013265921) |
| Rescue-Prime rounds | 7 |
| S-box exponent | 7 |
| State width | 4 (rate=2, capacity=2) |
| Trace rows | 8 |
| Blowup factor | 8 |
| LDE domain size | 64 |
| FRI queries | 28 |

## Goal

Find a vulnerability in the STARK verifier that allows you to construct a valid proof for a false statement. Submit the proof to the oracle to get the flag.

Good luck.
