
// SPDX-License-Identifier: GPL-2.0-or-later
pragma solidity 0.8.28;

import {ERC20} from "../lib/solady/src/tokens/ERC20.sol";
import {SafeTransferLib} from "../lib/solady/src/utils/SafeTransferLib.sol";

interface IUniswapV3Router {
    struct ExactInputParams {
        bytes path;
        address recipient;
        uint256 deadline;
        uint256 amountIn;
        uint256 amountOutMinimum;
    }

    function exactInput(ExactInputParams memory params) external payable returns (uint256 amountOut);
}

contract Swapper {
    using SafeTransferLib for address;

    IUniswapV3Router public constant kittenRouter = IUniswapV3Router(0x8fFDB06039B1b8188c2C721Dc3C435B5773D7346);
    IUniswapV3Router public constant laminarRouter = IUniswapV3Router(0x7d39aE50f97012C5d550240267dbC28355F625A0);
    IUniswapV3Router public constant hyperswapRouter = IUniswapV3Router(0x6D99e7f6747AF2cDbB5164b6DD50e40D4fDe1e77);

    function kittenRouterSwap(address token, bytes memory path) external {
        uint256 amount = ERC20(token).balanceOf(address(this));
        token.safeApproveWithRetry(address(kittenRouter), amount);

        kittenRouter.exactInput(
            IUniswapV3Router.ExactInputParams({
                path: path,
                recipient: msg.sender,
                deadline: block.timestamp,
                amountIn: amount,
                amountOutMinimum: 0
            })
        );
    }

    function laminarRouterSwap(address token, bytes memory path) external {
        uint256 amount = ERC20(token).balanceOf(address(this));
        token.safeApproveWithRetry(address(laminarRouter), amount);

        laminarRouter.exactInput(
            IUniswapV3Router.ExactInputParams({
                path: path,
                recipient: msg.sender,
                deadline: block.timestamp,
                amountIn: amount,
                amountOutMinimum: 0
            })
        );
    }

    function hyperswapRouterSwap(address token, bytes memory path) external {
        uint256 amount = ERC20(token).balanceOf(address(this));
        token.safeApproveWithRetry(address(hyperswapRouter), amount);

        hyperswapRouter.exactInput(
            IUniswapV3Router.ExactInputParams({
                path: path,
                recipient: msg.sender,
                deadline: block.timestamp,
                amountIn: amount,
                amountOutMinimum: 0
            })
        );
    }

}