// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// --- INTERFACES ---

interface ITroveManager {
    function batchLiquidateTroves(uint256[] calldata _troveArray) external;

    function getCurrentICR(
        uint256 _troveId,
        uint256 _price
    ) external view returns (uint256);
}

interface ISortedTroves {
    function getLast() external view returns (uint256);
    function getFirst() external view returns (uint256);

    function getPrev(uint256 _id) external view returns (uint256);
}

interface IPriceFeed {
    function fetchPrice() external returns (uint256);

    function lastGoodPrice() external view returns (uint256);
}

interface IAddressRegistry {
    function MCR() external view returns (uint256);
}

/// --- CONTRACT ---

contract AutoLiquidator {
    ITroveManager public immutable troveManager;
    ISortedTroves public immutable sortedTroves;
    IPriceFeed public immutable priceFeed;
    IAddressRegistry public immutable addressRegistry;

    constructor(
        address _troveManager,
        address _sortedTroves,
        address _priceFeed,
        address _addressRegistry
    ) {
        troveManager = ITroveManager(_troveManager);
        sortedTroves = ISortedTroves(_sortedTroves);
        priceFeed = IPriceFeed(_priceFeed);
        addressRegistry = IAddressRegistry(_addressRegistry);
    }

    function getLastGoodPrice() external view returns (uint256) {
        return priceFeed.lastGoodPrice();
    }

    function getLast() external view returns (uint256) {
        return sortedTroves.getFirst();
    }

    function checkLiquidate() external view returns (uint256) {
        uint256 price = priceFeed.lastGoodPrice();
        require(price > 0, "Invalid price");

        uint256 mcr = addressRegistry.MCR();

        // Will store addresses to be batch liquidated
        uint256[] memory candidates = new uint256[](100); // 200 is a safe max batch for most chains; adjust for your network
        uint256 count = 0;

        uint256 iterator = sortedTroves.getLast();
        while (iterator != 0) {
            uint256 icr = troveManager.getCurrentICR(iterator, price);
            if (icr < mcr) {
                candidates[count] = iterator;
                unchecked {
                    ++count;
                }
            }
            iterator = sortedTroves.getPrev(iterator);
            // Safety: avoid exceeding block gas limit or memory, break if needed
            if (count == candidates.length) break;
        }

        require(count > 0, "No liquidatable troves found");
        return count;
    }

    // Traverse the entire list and liquidate all undercollateralized troves (within gas limit)
    function liquidateAllUnsafe() external {
        uint256 price = priceFeed.fetchPrice();
        require(price > 0, "Invalid price");

        uint256 mcr = addressRegistry.MCR();

        // Will store addresses to be batch liquidated
        uint256[] memory candidates = new uint256[](5); // 200 is a safe max batch for most chains; adjust for your network
        uint256 count = 0;

        uint256 iterator = sortedTroves.getLast();
        while (iterator != 0) {
            uint256 icr = troveManager.getCurrentICR(iterator, price);
            if (icr < mcr) {
                candidates[count] = iterator;
                unchecked {
                    ++count;
                }
            }
            iterator = sortedTroves.getPrev(iterator);

            if (count == candidates.length) break;
        }

        require(count > 0, "No liquidatable troves found");

        // Prepare the exact size array for liquidation
        uint256[] memory batch = new uint256[](count);
        for (uint256 i = 0; i < count; ++i) batch[i] = candidates[i];

        troveManager.batchLiquidateTroves(batch);
    }
}
