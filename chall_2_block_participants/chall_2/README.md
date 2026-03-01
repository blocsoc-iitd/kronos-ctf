# Siege Protocol

> _"The chain remembers every king — but only one rules forever."_

**Category:** Smart Contract / Blockchain  
**Difficulty:** Hard  
**Points:** 500  
**Network:** Sepolia Testnet

---

## Lore

In the decentralized kingdom of **Etheria**, a smart contract governs who sits on the throne. Kings
rise and fall as challengers stake their ETH to claim the crown. The deposed king receives a full
refund — a graceful exile.

The kingdom's **Royal Treasury** accepts citizen deposits and issues collateralised loans to fund the
realm's endeavours. The system works well... or does it?

A true conqueror does not merely seize the throne — they **seal it for eternity** and **break the
treasury** to prove the realm's weakness.

---

## Objective

**Call `captureFlag()` on the SiegeProtocol contract.**

For `captureFlag()` to succeed, you must satisfy **all three** conditions simultaneously:

1. **You are the king** (or the king's deployer).
2. **The throne is sealed** — you have held the throne for >= 10 blocks and called `sealThrone()`.
3. **The treasury is drained** — `address(siege).balance <= thronePrice`.

When `captureFlag()` succeeds, `Setup.isSolved()` returns `true` and a unique flag hash is generated on-chain.
Read your flag directly:

```solidity
SiegeProtocol.getFlag()   // returns "tryst{0x...}"  — your unique flag
Setup.getFlag()           // convenience wrapper
```

---

## Deployed Contracts (Sepolia)

| Contract                           | Address                                                                                                                         |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Setup**                          | [`0x6Dd65981aeA0DFa96A17ac97Ea29BEf8BEca3290`](https://sepolia.etherscan.io/address/0x6Dd65981aeA0DFa96A17ac97Ea29BEf8BEca3290) |
| **SiegeProtocol**                  | [`0x204722Db42eF118C3C0E55AE1B55608DAD61fc6b`](https://sepolia.etherscan.io/address/0x204722Db42eF118C3C0E55AE1B55608DAD61fc6b) |
| **SiegeToken** (ERC-20 collateral) | [`0x4E232DE8a6ABB22372E0cB76DBF1C268b8647a08`](https://sepolia.etherscan.io/address/0x4E232DE8a6ABB22372E0cB76DBF1C268b8647a08) |

Read addresses from the Setup contract:

```solidity
Setup.siege()  // SiegeProtocol address
Setup.token()  // SiegeToken address
```

---

## Contract Overview

### SiegeProtocol

| Function                       | Description                                                                                    |
| ------------------------------ | ---------------------------------------------------------------------------------------------- |
| `overthrow()`                  | Become king by sending more ETH than `thronePrice + 0.005 ETH`. The previous king is refunded. |
| `sealThrone()`                 | After reigning for 10+ blocks, permanently seal the throne.                                    |
| `depositToTreasury()`          | Deposit ETH as a citizen.                                                                      |
| `withdrawFromTreasury(amount)` | Withdraw your deposit.                                                                         |
| `requestLoan(amount)`          | Request an ETH loan (requires 150% collateral in SIEGE tokens).                                |
| `depositCollateral(amount)`    | Deposit SIEGE tokens as collateral.                                                            |
| `fundLoan(loanId)`             | Fund a single loan (checks borrower's collateral).                                             |
| `batchFundLoans(loanIds)`      | Fund multiple loans at once.                                                                   |
| `repayLoan(loanId)`            | Repay a funded loan.                                                                           |
| `captureFlag()`                | **Win condition** — call this when you're the sealed king of a drained kingdom.                || `getFlag()`                    | Returns the flag string (only after `captureFlag()` succeeds).                                  |
### SiegeToken

An ERC-20 token used as collateral. Anyone can mint, but total supply is capped:

```solidity
SiegeToken.MINT_CAP()            // 0.01 ether — max total supply
SiegeToken.mint(yourAddress, amount)
```

### Setup

Deploys everything. Check your progress:

```solidity
Setup.isSolved() // returns bool
```

---

## Getting Started

```bash
# 1. Get Sepolia ETH from a faucet (you need ~0.03 ETH)
#    https://sepoliafaucet.com  or  https://faucets.chain.link

# 2. Install Foundry (if you haven't):
curl -L https://foundry.paradigm.xyz | bash
foundryup

# 3. Set up the project:
cd chall_2_blocks_participant
forge install OpenZeppelin/openzeppelin-contracts --no-commit
forge install foundry-rs/forge-std --no-commit
cp .env.example .env
# Edit .env with your private key and RPC URL

# 4. Build:
forge build

# 5. Check on-chain state:
source .env
cast call 0x6Dd65981aeA0DFa96A17ac97Ea29BEf8BEca3290 "isSolved()" --rpc-url $SEPOLIA_RPC_URL

# 6. Write and run your exploit:
forge script script/Exploit.s.sol --rpc-url $SEPOLIA_RPC_URL \
    --private-key $PRIVATE_KEY --broadcast -vvvv
```

---

## Rules

- You **may** deploy your own contracts to Sepolia.
- You **may** use any tooling (Foundry, Remix, ethers.js, cast, etc.).
- You may **not** modify the deployed contracts.
- You may **not** use the deployer's private key.
- The challenge is solved when `Setup.isSolved()` returns `true`.
- Once solved, report to the organizers to claim your flag.

---

## Hints

<details>
<summary>Hint 1 (mild)</summary>

What happens when a king _can't_ accept their refund?

</details>

<details>
<summary>Hint 2 (moderate)</summary>

Compare `fundLoan()` and `batchFundLoans()`. Are they really equivalent?

</details>

<details>
<summary>Hint 3 (strong)</summary>

The batch function only validates collateral for the first loan in the array.
A contract without a `receive()` function will cause the overthrow refund to revert.

</details>

---

## Source Code

The full Solidity source for all three contracts is provided in `src/`.
Read them carefully — the vulnerabilities are hiding in plain sight.

---

_Good luck, challenger. The throne awaits._
