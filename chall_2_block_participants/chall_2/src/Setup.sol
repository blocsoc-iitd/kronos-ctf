// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./SiegeProtocol.sol";
import "./SiegeToken.sol";

/**
 * @title Setup
 * @notice Deployment harness for the Siege Protocol CTF challenge.
 *         Deploys both contracts and seeds the initial game state.
 *
 *         Win condition:  isSolved() returns true.
 */
contract Setup {
    SiegeProtocol public immutable siege;
    SiegeToken    public immutable token;

    constructor() payable {
        require(msg.value >= 0.05 ether, "Send >= 0.05 ETH to initialize the challenge");

        // 1. Deploy the collateral token
        token = new SiegeToken();

        // 2. Deploy the main contract - the deployer (Setup) becomes the first king
        //    with a 0.01 ETH throne deposit.
        siege = new SiegeProtocol{value: 0.01 ether}(address(token));

        // 3. Seed the royal treasury with citizen deposits (0.04 ETH).
        siege.depositToTreasury{value: 0.04 ether}();
    }

    /**
     * @notice Check whether the challenge has been solved.
     */
    function isSolved() external view returns (bool) {
        return siege.isSolved();
    }

    /**
     * @notice Return the unique flag. Only available after captureFlag().
     */
    function getFlag() external view returns (string memory) {
        return siege.getFlag();
    }

    /// @dev Accept ETH refunds when the throne is overthrown.
    receive() external payable {}
}
