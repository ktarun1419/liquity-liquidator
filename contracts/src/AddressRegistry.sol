// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import './IPriceFeed.sol';
import "./TroveManager.sol";
interface AddressesRegistry {
    

    function CCR() external returns (uint256);
    function SCR() external returns (uint256);
    function MCR() external returns (uint256);
    function BCR() external returns (uint256);
    function LIQUIDATION_PENALTY_SP() external returns (uint256);
    function LIQUIDATION_PENALTY_REDISTRIBUTION() external returns (uint256);


    function troveManager() external view returns (TroveManager);

    function priceFeed() external view returns (IPriceFeed);
    function gasPoolAddress() external view returns (address);
}