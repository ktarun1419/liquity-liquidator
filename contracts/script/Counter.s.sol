// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Counter} from "../src/Counter.sol";
import {Liquidator} from "../src/Liquidator.sol";
import {Swapper} from "../src/Swapper.sol";
import {Scenario} from "../src/Scenario.sol";
import {IPool} from "../src/IPool.sol";
import {AggregatorV3Interface} from "../src/AggregatorV3Interface.sol";
import {IPriceOracle} from "../src/IPriceOracle.sol";
import {ERC20} from "../lib/solady/src/tokens/ERC20.sol";
import {LiquidationExecutor} from "../src/LiquityLiquidator.sol";

//trove manager: 0x3100F4e7BDA2ED2452d9A57EB30260ab071BBe62;
//sortedTroves: 0xD1CaA4218808EB94d36e1Df7247f7406F43F2Ef6;
//price Feed: 0x12a1868b89789900e413a6241CA9032dD1873a51;
//address registry: 0x7201Fb5C3BA06f10A858819F62221AE2f473815D

interface IWETH {
    function deposit() external payable;

    function withdraw(uint256) external;

    function transfer(address dst, uint wad) external;
}

contract CounterScript is Script {
  
    // Aave V3 Pool on Base
   LiquidationExecutor internal liquidationExecutor;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        
        liquidationExecutor = new LiquidationExecutor();
        // uint256 price = autorLiquidator.getLastGoodPrice();
        // autorLiquidator.liquidateAllUnsafe();
        
        

        vm.stopBroadcast();
    }
}

// for testing the base liquidation
// scenario = new Scenario();
// usdc = ERC20(0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913);
// weth = ERC20(0x4200000000000000000000000000000000000006);

// 4) Check health factor
// (
//     uint256 totalCollateralBase,
//     uint256 totalDebtBase,
//     uint256 availableBorrowsBase,
//     uint256 currentLiquidationThreshold,
//     uint256 ltv,
//     uint256 healthFactor
// ) = IPool(_aavePool).getUserAccountData(
//         address(0x714c7dC00bd2f82222449F26281F44c650dB3824)
//     );

// console.log("User health factor (18-dec):", healthFactor);
// console.log("totalCollateralBase):", totalCollateralBase);
// console.log("totalDebtBase):", totalDebtBase);
// console.log(
//     "currentLiquidationThreshold):",
//     currentLiquidationThreshold
// );

// if (healthFactor < 1e18) {
//     console.log("Position is liquidatable!");
//     // ─── 1) Pack your calldata into a bytes array ─────────────────────────────────────────
//     bytes memory liquidationCalldata = hex"90df90780000000000000000000000004200000000000000000000000000000000000006000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913000000000000000000000000714c7dc00bd2f82222449f26281f44c650db382400000000000000000000000000000000000000000000000000000000000877cd00000000000000000000000000000000000000000000000000000000000000c00000000000000000000000007d42db93ce739640f4b6e6daf4805c6eff29562000000000000000000000000000000000000000000000000000000000000000a42e44c39100000000000000000000000042000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000002b42000000000000000000000000000000000000060001f4833589fcd6edb6e08f4c7c32d4f71b54bda0291300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

//     // ─── 2) Send it as a contract call (will be signed & broadcast by Foundry) ─────────────
//     (bool ok, bytes memory resp) = address(liquidator).call(
//         liquidationCalldata
//     );
//     require(ok, "Liquidation tx failed");
//     console.log("Liquidation tx broadcast:", ok);
//    uint256 final_usdc_balance =  usdc.balanceOf(address(liquidator));
//     uint256 final_weth_balance =  weth.balanceOf(address(liquidator));
//    console.log("final usdc balance" , final_usdc_balance);
//     console.log("final weth balance" , final_weth_balance);

// } else {
//     console.log("Position NOT yet liquidatable.");
// }
