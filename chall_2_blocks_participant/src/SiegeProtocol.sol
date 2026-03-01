// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

/**
 * @title SiegeProtocol
 * @notice A medieval DeFi kingdom. Contenders fight for the throne while
 *         the treasury funds the realm through collateralized loans.
 *
 *  ╔═══════════════════════════════════════════════════════════╗
 *  ║   Can you become the eternal king and break the treasury? ║
 *  ╚═══════════════════════════════════════════════════════════╝
 */
contract SiegeProtocol {

    // ═══════════════════════════════════════════════════════════
    //                      THRONE STATE
    // ═══════════════════════════════════════════════════════════

    address public king;
    uint256 public thronePrice;
    uint256 public crownedBlock;
    bool    public throneSealed;

    uint256 public constant MIN_THRONE_INCREASE = 0.005 ether;
    uint256 public constant REIGN_BLOCKS = 10;

    // ═══════════════════════════════════════════════════════════
    //                     TREASURY STATE
    // ═══════════════════════════════════════════════════════════

    mapping(address => uint256) public treasuryDeposits;
    uint256 public totalTreasuryDeposits;

    // ═══════════════════════════════════════════════════════════
    //                      LENDING STATE
    // ═══════════════════════════════════════════════════════════

    struct Loan {
        address borrower;
        uint256 amount;
        uint256 collateralRequired;
        bool    funded;
        bool    repaid;
    }

    Loan[] public loans;
    IERC20  public immutable collateralToken;
    mapping(address => uint256) public collateralDeposited;
    mapping(address => uint256) public collateralLocked;

    uint256 public constant COLLATERAL_RATIO_BPS = 15000; // 150 %

    // ═══════════════════════════════════════════════════════════
    //                         FLAGS
    // ═══════════════════════════════════════════════════════════

    bool    public siegeComplete;
    address public siegeVictor;
    bytes32 public flagHash;

    // ═══════════════════════════════════════════════════════════
    //                        EVENTS
    // ═══════════════════════════════════════════════════════════

    event Coronation(address indexed newKing, uint256 thronePrice);
    event ThroneSealed(address indexed eternalKing);
    event Deposited(address indexed depositor, uint256 amount);
    event Withdrawn(address indexed depositor, uint256 amount);
    event LoanRequested(uint256 indexed loanId, address indexed borrower, uint256 amount);
    event LoanFunded(uint256 indexed loanId);
    event LoanRepaid(uint256 indexed loanId);
    event SiegeComplete(address indexed victor, bytes32 flagHash);

    // ═══════════════════════════════════════════════════════════
    //                      CONSTRUCTOR
    // ═══════════════════════════════════════════════════════════

    constructor(address _collateralToken) payable {
        require(msg.value > 0, "Throne requires an initial stake");
        collateralToken = IERC20(_collateralToken);
        king        = msg.sender;
        thronePrice = msg.value;
        crownedBlock = block.number;
    }

    // ═══════════════════════════════════════════════════════════
    //                    THRONE  MECHANICS
    // ═══════════════════════════════════════════════════════════

    /**
     * @notice Challenge the current king by offering a larger stake.
     *         The previous king is refunded their entire throne deposit.
     */
    function overthrow() external payable {
        require(!throneSealed, "The throne has been sealed for eternity");
        require(
            msg.value >= thronePrice + MIN_THRONE_INCREASE,
            "Tribute too small - offer more than the current throne price"
        );

        address previousKing = king;
        uint256 refund        = thronePrice;

        king        = msg.sender;
        thronePrice = msg.value;
        crownedBlock = block.number;

        // Refund the deposed king
        (bool sent, ) = payable(previousKing).call{value: refund}("");
        require(sent, "Failed to refund the deposed king");

        emit Coronation(msg.sender, msg.value);
    }

    /**
     * @notice Seal the throne permanently.
     *         Only a king who has reigned for at least REIGN_BLOCKS may seal it.
     */
    function sealThrone() external {
        require(msg.sender == king, "Only the king may seal the throne");
        require(
            block.number >= crownedBlock + REIGN_BLOCKS,
            "You must reign longer before sealing the throne"
        );
        throneSealed = true;
        emit ThroneSealed(king);
    }

    // ═══════════════════════════════════════════════════════════
    //                        TREASURY
    // ═══════════════════════════════════════════════════════════

    /**
     * @notice Deposit ETH into the royal treasury.
     */
    function depositToTreasury() external payable {
        require(msg.value > 0, "Empty tribute");
        treasuryDeposits[msg.sender] += msg.value;
        totalTreasuryDeposits         += msg.value;
        emit Deposited(msg.sender, msg.value);
    }

    /**
     * @notice Withdraw your treasury deposit.
     */
    function withdrawFromTreasury(uint256 amount) external {
        require(treasuryDeposits[msg.sender] >= amount, "Insufficient deposit");
        treasuryDeposits[msg.sender] -= amount;
        totalTreasuryDeposits         -= amount;
        payable(msg.sender).transfer(amount);
        emit Withdrawn(msg.sender, amount);
    }

    // ═══════════════════════════════════════════════════════════
    //                        LENDING
    // ═══════════════════════════════════════════════════════════

    /**
     * @notice Request an ETH loan from the treasury.
     * @param amount The loan amount in wei.
     */
    function requestLoan(uint256 amount) external {
        require(amount > 0, "Zero-amount loan");
        uint256 collateralReq = (amount * COLLATERAL_RATIO_BPS) / 10000;
        loans.push(Loan({
            borrower:           msg.sender,
            amount:             amount,
            collateralRequired: collateralReq,
            funded:             false,
            repaid:             false
        }));
        emit LoanRequested(loans.length - 1, msg.sender, amount);
    }

    /**
     * @notice Deposit ERC-20 collateral tokens to back your loans.
     * @param amount Amount of tokens to deposit.
     */
    function depositCollateral(uint256 amount) external {
        require(
            collateralToken.transferFrom(msg.sender, address(this), amount),
            "Collateral transfer failed"
        );
        collateralDeposited[msg.sender] += amount;
    }

    /**
     * @notice Withdraw unlocked collateral tokens.
     * @param amount Amount of tokens to withdraw.
     */
    function withdrawCollateral(uint256 amount) external {
        uint256 free = collateralDeposited[msg.sender] - collateralLocked[msg.sender];
        require(free >= amount, "Collateral is locked in active loans");
        collateralDeposited[msg.sender] -= amount;
        require(collateralToken.transfer(msg.sender, amount), "Transfer failed");
    }

    /**
     * @notice Fund a single pending loan from the treasury.
     * @dev    Validates that the borrower has sufficient free collateral.
     * @param loanId ID of the loan to fund.
     */
    function fundLoan(uint256 loanId) external {
        require(loanId < loans.length, "Invalid loan ID");
        Loan storage loan = loans[loanId];
        require(!loan.funded && !loan.repaid, "Loan already processed");

        uint256 freeCollateral =
            collateralDeposited[loan.borrower] - collateralLocked[loan.borrower];
        require(freeCollateral >= loan.collateralRequired, "Insufficient collateral");

        collateralLocked[loan.borrower] += loan.collateralRequired;
        loan.funded = true;

        (bool sent, ) = payable(loan.borrower).call{value: loan.amount}("");
        require(sent, "Loan disbursement failed");

        emit LoanFunded(loanId);
    }

    /**
     * @notice Fund multiple pending loans in a single transaction.
     * @dev    Batch validation: the lead loan's collateral coverage establishes
     *         the batch's creditworthiness.  Individual loan state checks are
     *         performed inside the processing loop.
     * @param loanIds Ordered array of loan IDs to fund.
     */
    function batchFundLoans(uint256[] calldata loanIds) external {
        require(loanIds.length > 0, "Empty batch");

        // Validate the batch leader's collateral backing
        _validateCollateral(loanIds[0]);

        for (uint256 i = 0; i < loanIds.length; i++) {
            require(loanIds[i] < loans.length, "Invalid loan ID");
            Loan storage loan = loans[loanIds[i]];
            require(!loan.funded && !loan.repaid, "Loan already processed");

            collateralLocked[loan.borrower] += loan.collateralRequired;
            loan.funded = true;

            (bool sent, ) = payable(loan.borrower).call{value: loan.amount}("");
            require(sent, "Loan disbursement failed");

            emit LoanFunded(loanIds[i]);
        }

        // Prevent draining throne funds
        require(address(this).balance >= thronePrice, "Would compromise throne security");
    }

    /**
     * @dev Verify a loan's borrower has enough free collateral.
     */
    function _validateCollateral(uint256 loanId) internal view {
        require(loanId < loans.length, "Invalid loan ID");
        Loan storage loan = loans[loanId];
        require(!loan.funded && !loan.repaid, "Loan already processed");
        uint256 freeCollateral =
            collateralDeposited[loan.borrower] - collateralLocked[loan.borrower];
        require(freeCollateral >= loan.collateralRequired, "Insufficient collateral");
    }

    /**
     * @notice Repay a funded loan.  Collateral is unlocked upon repayment.
     * @param loanId ID of the loan to repay.
     */
    function repayLoan(uint256 loanId) external payable {
        require(loanId < loans.length, "Invalid loan ID");
        Loan storage loan = loans[loanId];
        require(loan.funded && !loan.repaid, "Invalid loan state");
        require(msg.sender == loan.borrower, "Only the borrower may repay");
        require(msg.value >= loan.amount, "Insufficient repayment");

        loan.repaid = true;
        collateralLocked[loan.borrower] -= loan.collateralRequired;

        // Refund excess
        if (msg.value > loan.amount) {
            payable(msg.sender).transfer(msg.value - loan.amount);
        }
        emit LoanRepaid(loanId);
    }

    // ═══════════════════════════════════════════════════════════
    //                     FLAG  CAPTURE
    // ═══════════════════════════════════════════════════════════

    /**
     * @notice Prove your dominance: seal the throne AND drain the treasury.
     * @return True if the siege is complete.
     */
    function captureFlag() external returns (bool) {
        require(
            msg.sender == king || _isKingOwner(msg.sender),
            "Only the king (or its deployer) may capture the flag"
        );
        require(throneSealed, "Seal the throne first");
        require(
            address(this).balance <= thronePrice,
            "The treasury still holds funds - drain it"
        );

        siegeComplete = true;
        siegeVictor   = tx.origin;
        flagHash      = keccak256(
            abi.encodePacked("TRYST_SIEGE_", address(this))
        );
        emit SiegeComplete(tx.origin, flagHash);
        return true;
    }

    /**
     * @notice Check whether the siege has been completed.
     */
    function isSolved() external view returns (bool) {
        return siegeComplete;
    }

    /**
     * @notice Return the flag string.  Empty until captureFlag() succeeds.
     * @return The flag in the format tryst{0x…}.
     */
    function getFlag() external view returns (string memory) {
        require(siegeComplete, "Siege not complete");
        return string(abi.encodePacked("tryst{", _toHex(flagHash), "}"));
    }

    /// @dev Convert bytes32 to a 0x-prefixed lowercase hex string (66 chars).
    function _toHex(bytes32 value) internal pure returns (string memory) {
        bytes memory alphabet = "0123456789abcdef";
        bytes memory str = new bytes(66);
        str[0] = "0";
        str[1] = "x";
        for (uint256 i = 0; i < 32; i++) {
            str[2 + i * 2]     = alphabet[uint8(value[i] >> 4)];
            str[2 + i * 2 + 1] = alphabet[uint8(value[i] & 0x0f)];
        }
        return string(str);
    }

    // ═══════════════════════════════════════════════════════════
    //                         VIEWS
    // ═══════════════════════════════════════════════════════════

    function getLoanCount() external view returns (uint256) {
        return loans.length;
    }

    function getContractBalance() external view returns (uint256) {
        return address(this).balance;
    }

    /**
     * @dev If the current king is a contract that exposes owner(), return it.
     */
    function _isKingOwner(address account) internal view returns (bool) {
        if (king.code.length == 0) return false;
        (bool ok, bytes memory data) = king.staticcall(
            abi.encodeWithSignature("owner()")
        );
        return ok && data.length >= 32 && abi.decode(data, (address)) == account;
    }

    receive() external payable {}
}
