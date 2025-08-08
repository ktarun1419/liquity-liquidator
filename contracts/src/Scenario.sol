// SPDX-License-Identifier: AGPL-3.0
pragma solidity ^0.8.0;

import {IPool} from "./IPool.sol";
import {IPoolAddressesProvider} from "./IPoolAddressesProvider.sol";
import {ERC20} from "../lib/solady/src/tokens/ERC20.sol";
import {Ownable} from "../lib/solady/src/auth/Ownable.sol";
import {AggregatorV3Interface} from "./AggregatorV3Interface.sol";

interface IWETHGateway {
    function depositETH(
        address pool,
        address onBehalfOf,
        uint16 referralCode
    ) external payable;
}

/// @notice Minimal Chainlink mock to override price in tests
contract MockV3Aggregator is AggregatorV3Interface {
    uint8 public override decimals;
    string public override description = "mock";
    uint256 public override version = 0;
    int256 private _price;

    constructor(uint8 _decimals, int256 initialPrice) {
        decimals = _decimals;
        _price = initialPrice;
    }

    function getRoundData(
        uint80
    )
        external
        pure
        override
        returns (uint80, int256, uint256, uint256, uint80)
    {
        revert("no data");
    }

    function latestRoundData()
        external
        view
        override
        returns (uint80, int256, uint256, uint256, uint80)
    {
        return (0, _price, block.timestamp, block.timestamp, 0);
    }
}

contract Scenario is Ownable {
    IWETHGateway public constant WETH_GATEWAY =
        IWETHGateway(0xa0d9C1E9E48Ca30c8d8C3B5D69FF5dc1f6DFfC24);

    IPool public constant POOL =
        IPool(0xA238Dd80C259a72e81d7e4664a9801593F98d1c5);

    address public constant USDC = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913;

    function createPosition(uint256 borrowAmount) external payable{
        require(msg.value > 0, "Send ETH");

        WETH_GATEWAY.depositETH{value: msg.value}(
            address(POOL),
            address(this),
            0
        );

        POOL.borrow(USDC, borrowAmount, 2, 0, address(this));
    }
}
