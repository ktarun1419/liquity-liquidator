// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface ITroveManager {
    function batchLiquidate(address[] calldata _troveArray) external;
}

interface IERC20 {
    function transfer(address recipient, uint256 amount) external returns (bool);
}

contract Liquidator {
    address public owner;
    ITroveManager public immutable troveManager;

    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }

    constructor(address _troveManager) {
        require(_troveManager != address(0), "Invalid troveManager");
        owner = msg.sender;
        troveManager = ITroveManager(_troveManager);
    }

    /// @notice Batch liquidate multiple troves using TroveManager's native method
    function batchLiquidate(address[] calldata _troveArray) external {
        troveManager.batchLiquidate(_troveArray);
    }

    /// @notice Withdraw any ERC20 tokens (owner only)
    function withdrawToken(address token, uint256 amount) external onlyOwner {
        require(token != address(0), "Invalid token address");
        bool success = IERC20(token).transfer(owner, amount);
        require(success, "Token transfer failed");
    }

    /// @notice Withdraw native ETH (owner only)
    function withdrawETH(uint256 amount) external onlyOwner {
        (bool sent, ) = owner.call{value: amount}("");
        require(sent, "ETH transfer failed");
    }

    /// @notice Accept ETH
    receive() external payable {}
}
