# Fractured Lattices — Writeup

**Category:** Cryptography / Forensics | **Difficulty:** Medium-Hard | **Points:** ___

---

## TL;DR

<!-- One-liner summary of the solve path -->

---

## Challenge Overview

Given `cipher.txt` containing a prime `q` and a ciphertext vector `c` of length 25.

The encryption is LWE (Learning With Errors):

```
c[i] = (A[i] · s + e[i]) mod q
```

where `s` is the flag (ASCII ordinals), `e` is small noise (±3), and `A` is derived from a secret prime `p` via `A[i][j] = SHA256(p || i || j) mod q`.

**Without `p`, the matrix `A` is unknown → the ciphertext is unbreakable.**

---

## Step 1 — Forensics: Recover the Prime

<!-- Describe how you found the 4 fragments of p -->
<!-- e.g. EXIF metadata extraction, steganography, etc. -->

```bash
# TODO: commands used to extract fragments
```

Fragments reassembled → prime `p = ___`

---

## Step 2 — Reconstruct the Matrix

Once `p` is known, reconstruct `A` deterministically:

```python
import hashlib

def build_A(p, q, n=25):
    A = []
    for i in range(n):
        row = []
        for j in range(n):
            h = hashlib.sha256(f"{p}{i}{j}".encode()).hexdigest()
            row.append(int(h, 16) % q)
        A.append(row)
    return A
```

---

## Step 3 — Lattice Attack (LLL)

With `n=25`, `q ≈ 2³²`, and noise `|e| ≤ 3`, this is a toy-sized LWE instance broken trivially by LLL:

```python
# TODO: SageMath / fpylll script
# Build the lattice, run LLL, recover s
```

---

## Step 4 — Extract Flag

```python
flag = ''.join(chr(x) for x in s)
print(flag)
```

---

## Flag

```
TODO
```

---

## Key Takeaways

- LWE security depends entirely on parameter selection — `n=25` with tiny noise is trivially broken.
- The real difficulty was the forensics step: locating and reassembling the prime fragments.
- SHA-256 deterministic derivation of `A` from `p` means the problem collapses once `p` is recovered.
