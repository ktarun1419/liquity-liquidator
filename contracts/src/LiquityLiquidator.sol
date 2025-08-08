// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract LiquidationExecutor {
    address public owner;
    error NotOwner();
    error InvalidOwner();

    modifier onlyOwner() {
    if (msg.sender != owner) revert NotOwner();
    _;
     }

uint256 private locked;
modifier nonReentrant() {
    require(locked == 0, "REENTRANCY");
    locked = 1;
    _;
    locked = 0;
}

event Executed(address indexed target, uint256 value, bytes data, bytes returnData);
event WithdrawETH(address indexed to, uint256 amount);
event WithdrawERC20(address indexed token, address indexed to, uint256 amount);
event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

receive() external payable {}

constructor() {
    owner = msg.sender;
    emit OwnershipTransferred(address(0), owner);
}

function transferOwnership(address newOwner) external onlyOwner {
    if (newOwner == address(0)) revert InvalidOwner();
    emit OwnershipTransferred(owner, newOwner);
    owner = newOwner;
}

function renounceOwnership() external onlyOwner {
    emit OwnershipTransferred(owner, address(0));
    owner = address(0);
}

function execute(address target, uint256 value, bytes calldata data)
    external
    payable
    onlyOwner
    nonReentrant
    returns (bytes memory ret)
{
    require(target != address(0), "INVALID_TARGET");
    require(address(this).balance >= value, "INSUFFICIENT_ETH");
    (bool ok, bytes memory r) = target.call{value: value}(data);
    if (!ok) {
        if (r.length > 0) {
            assembly {
                revert(add(r, 0x20), mload(r))
            }
        } else {
            revert("CALL_FAILED");
        }
    }
    emit Executed(target, value, data, r);
    return r;
}

function withdrawETH(address payable to, uint256 amount) external onlyOwner nonReentrant {
    require(to != address(0), "INVALID_TO");
    require(address(this).balance >= amount, "INSUFFICIENT_ETH");
    (bool sent, ) = to.call{value: amount}("");
    require(sent, "ETH_TRANSFER_FAILED");
    emit WithdrawETH(to, amount);
}

function withdrawERC20(address token, address to, uint256 amount) external onlyOwner nonReentrant {
    require(token != address(0), "INVALID_TOKEN");
    require(to != address(0), "INVALID_TO");
    (bool ok, bytes memory data) = token.call(abi.encodeWithSelector(bytes4(keccak256("transfer(address,uint256)")), to, amount));
    require(ok && (data.length == 0 || abi.decode(data, (bool))), "TOKEN_TRANSFER_FAILED");
    emit WithdrawERC20(token, to, amount);
}

function rescueAllERC20(address token, address to) external onlyOwner nonReentrant {
    require(token != address(0), "INVALID_TOKEN");
    require(to != address(0), "INVALID_TO");
    (bool s0, bytes memory b0) = token.staticcall(abi.encodeWithSelector(bytes4(keccak256("balanceOf(address)")), address(this)));
    require(s0 && b0.length >= 32, "BALANCE_QUERY_FAILED");
    uint256 bal = abi.decode(b0, (uint256));
    (bool ok, bytes memory data) = token.call(abi.encodeWithSelector(bytes4(keccak256("transfer(address,uint256)")), to, bal));
    require(ok && (data.length == 0 || abi.decode(data, (bool))), "TOKEN_TRANSFER_FAILED");
    emit WithdrawERC20(token, to, bal);
}
}
