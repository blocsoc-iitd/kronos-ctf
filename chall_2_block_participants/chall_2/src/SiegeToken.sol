// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/**
 * @title SiegeToken
 * @notice Collateral token for the Siege Protocol lending system.
 *         Anyone may mint tokens — their value lies solely in backing loans.
 *         Total supply is capped to prevent unlimited collateralisation.
 */
contract SiegeToken is ERC20 {
    uint256 public constant MINT_CAP = 0.01 ether; // max total supply

    constructor() ERC20("Siege Collateral Token", "SIEGE") {}

    /**
     * @notice Mint tokens to any address (capped at MINT_CAP total supply).
     * @param to        Recipient address.
     * @param amount    Amount of tokens (18 decimals).
     */
    function mint(address to, uint256 amount) external {
        require(totalSupply() + amount <= MINT_CAP, "Mint cap exceeded");
        _mint(to, amount);
    }
}
