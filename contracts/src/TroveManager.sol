// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;



// Common interface for the Trove Manager.
interface TroveManager{
    enum Status {
        nonExistent,
        active,
        closedByOwner,
        closedByLiquidation,
        zombie
    }

    function shutdownTime() external view returns (uint256);

    //function boldToken() external view returns (IBoldToken);

    function Troves(uint256 _id)
        external
        view
        returns (
            uint256 debt,
            uint256 coll,
            uint256 stake,
            Status status,
            uint64 arrayIndex,
            uint64 lastDebtUpdateTime,
            uint64 lastInterestRateAdjTime,
            uint256 annualInterestRate,
            address interestBatchManager,
            uint256 batchDebtShares
        );

    function rewardSnapshots(uint256 _id) external view returns (uint256 coll, uint256 boldDebt);

    function getTroveIdsCount() external view returns (uint256);

    function getTroveFromTroveIdsArray(uint256 _index) external view returns (uint256);

    function getCurrentICR(uint256 _troveId, uint256 _price) external view returns (uint256);

    function lastZombieTroveId() external view returns (uint256);

    function batchLiquidateTroves(uint256[] calldata _troveArray) external;

    function redeemCollateral(
        address _sender,
        uint256 _boldAmount,
        uint256 _price,
        uint256 _redemptionRate,
        uint256 _maxIterations
    ) external returns (uint256 _redemeedAmount);

    function shutdown() external;
    function urgentRedemption(uint256 _boldAmount, uint256[] calldata _troveIds, uint256 _minCollateral) external;

    function getUnbackedPortionPriceAndRedeemability() external returns (uint256, uint256, bool);

    function getTroveAnnualInterestRate(uint256 _troveId) external view returns (uint256);

    function getTroveStatus(uint256 _troveId) external view returns (Status);


    // -- permissioned functions called by BorrowerOperations


    // Called from `adjustZombieTrove()`
    function setTroveStatusToActive(uint256 _troveId) external;


    // -- batches --
    function onLowerBatchManagerAnnualFee(
        address _batchAddress,
        uint256 _newColl,
        uint256 _newDebt,
        uint256 _newAnnualManagementFee
    ) external;
    function onSetBatchManagerAnnualInterestRate(
        address _batchAddress,
        uint256 _newColl,
        uint256 _newDebt,
        uint256 _newAnnualInterestRate,
        uint256 _upfrontFee // needed by BatchUpdated event
    ) external;


     enum Operation {
        openTrove,
        closeTrove,
        adjustTrove,
        adjustTroveInterestRate,
        applyPendingDebt,
        liquidate,
        redeemCollateral,
        // batch management
        openTroveAndJoinBatch,
        setInterestBatchManager,
        removeFromBatch
    }

    event Liquidation(
        uint256 _debtOffsetBySP,
        uint256 _debtRedistributed,
        uint256 _boldGasCompensation,
        uint256 _collGasCompensation,
        uint256 _collSentToSP,
        uint256 _collRedistributed,
        uint256 _collSurplus,
        uint256 _L_ETH,
        uint256 _L_boldDebt,
        uint256 _price
    );

    event Redemption(
        uint256 _attemptedBoldAmount,
        uint256 _actualBoldAmount,
        uint256 _ETHSent,
        uint256 _ETHFee,
        uint256 _price,
        uint256 _redemptionPrice
    );

    // A snapshot of the Trove's latest state on-chain
    event TroveUpdated(
        uint256 indexed _troveId,
        uint256 _debt,
        uint256 _coll,
        uint256 _stake,
        uint256 _annualInterestRate,
        uint256 _snapshotOfTotalCollRedist,
        uint256 _snapshotOfTotalDebtRedist
    );

    // Details of an operation that modifies a Trove
    event TroveOperation(
        uint256 indexed _troveId,
        Operation _operation,
        uint256 _annualInterestRate,
        uint256 _debtIncreaseFromRedist,
        uint256 _debtIncreaseFromUpfrontFee,
        int256 _debtChangeFromOperation,
        uint256 _collIncreaseFromRedist,
        int256 _collChangeFromOperation
    );

    event RedemptionFeePaidToTrove(uint256 indexed _troveId, uint256 _ETHFee);

    // Batch management

    enum BatchOperation {
        registerBatchManager,
        lowerBatchManagerAnnualFee,
        setBatchManagerAnnualInterestRate,
        applyBatchInterestAndFee,
        joinBatch,
        exitBatch,
        // used when the batch is updated as a result of a Trove change inside the batch
        troveChange
    }

    event BatchUpdated(
        address indexed _interestBatchManager,
        BatchOperation _operation,
        uint256 _debt,
        uint256 _coll,
        uint256 _annualInterestRate,
        uint256 _annualManagementFee,
        uint256 _totalDebtShares,
        uint256 _debtIncreaseFromUpfrontFee
    );

    event BatchedTroveUpdated(
        uint256 indexed _troveId,
        address _interestBatchManager,
        uint256 _batchDebtShares,
        uint256 _coll,
        uint256 _stake,
        uint256 _snapshotOfTotalCollRedist,
        uint256 _snapshotOfTotalDebtRedist
    );

    // -- end of permissioned functions --
}