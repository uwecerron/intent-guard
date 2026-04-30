// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IActionAdapter} from "./IntentGuardModule.sol";

interface IAggregatorV3 {
    function decimals() external view returns (uint8);

    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
}

/// @notice Example adapter for a protocol function shaped like:
/// listCollateral(address token, address oracle, uint256 fairValueUsdE18, uint256 maxDepositUsdE18)
/// @dev Production adapters should be specific to the protocol's real ABI and risk model.
contract CollateralListingAdapter is IActionAdapter {
    bytes4 public constant LIST_COLLATERAL_SELECTOR = bytes4(
        keccak256("listCollateral(address,address,uint256,uint256)")
    );

    uint256 public constant BPS = 10_000;

    struct FeedPolicy {
        address oracle;
        uint64 maxStalenessSecs;
        uint16 toleranceBps;
        uint256 maxBootstrapDepositUsdE18;
        bool allowed;
    }

    address public immutable owner;
    mapping(address => FeedPolicy) public feedPolicyByToken;

    error NotOwner();
    error BadSelector();
    error BadFeed();
    error BadPrice();
    error StaleOracle();
    error DepositCapTooHigh();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(address owner_) {
        owner = owner_;
    }

    function setFeedPolicy(
        address token,
        address oracle,
        uint64 maxStalenessSecs,
        uint16 toleranceBps,
        uint256 maxBootstrapDepositUsdE18,
        bool allowed
    ) external onlyOwner {
        if (toleranceBps > BPS) revert BadFeed();
        feedPolicyByToken[token] = FeedPolicy({
            oracle: oracle,
            maxStalenessSecs: maxStalenessSecs,
            toleranceBps: toleranceBps,
            maxBootstrapDepositUsdE18: maxBootstrapDepositUsdE18,
            allowed: allowed
        });
    }

    function intentHash(address target, uint256 value, bytes calldata data) external view returns (bytes32) {
        (address token, address oracle, uint256 fairValueUsdE18, uint256 maxDepositUsdE18) = _decode(data);
        return keccak256(
            abi.encode(
                keccak256("WhitelistCollateral(address target,uint256 value,address token,address oracle,uint256 fairValueUsdE18,uint256 maxDepositUsdE18)"),
                target,
                value,
                token,
                oracle,
                fairValueUsdE18,
                maxDepositUsdE18
            )
        );
    }

    function validate(address, uint256, bytes calldata data, bytes32) external view {
        (address token, address oracle, uint256 fairValueUsdE18, uint256 maxDepositUsdE18) = _decode(data);
        FeedPolicy memory policy = feedPolicyByToken[token];
        if (!policy.allowed || policy.oracle != oracle) revert BadFeed();
        if (maxDepositUsdE18 > policy.maxBootstrapDepositUsdE18) revert DepositCapTooHigh();

        (, int256 answer,, uint256 updatedAt,) = IAggregatorV3(oracle).latestRoundData();
        if (answer <= 0) revert BadPrice();
        if (block.timestamp - updatedAt > policy.maxStalenessSecs) revert StaleOracle();

        uint256 oraclePriceE18 = _scaleToE18(uint256(answer), IAggregatorV3(oracle).decimals());
        uint256 diff = oraclePriceE18 > fairValueUsdE18
            ? oraclePriceE18 - fairValueUsdE18
            : fairValueUsdE18 - oraclePriceE18;

        if (diff * BPS > oraclePriceE18 * policy.toleranceBps) revert BadPrice();
    }

    function _scaleToE18(uint256 value, uint8 decimals) internal pure returns (uint256) {
        if (decimals == 18) return value;
        if (decimals < 18) return value * (10 ** (18 - decimals));
        return value / (10 ** (decimals - 18));
    }

    function _decode(bytes calldata data)
        internal
        pure
        returns (address token, address oracle, uint256 fairValueUsdE18, uint256 maxDepositUsdE18)
    {
        if (data.length != 4 + 32 * 4) revert BadSelector();
        bytes4 selector;
        assembly {
            selector := calldataload(data.offset)
        }
        if (selector != LIST_COLLATERAL_SELECTOR) revert BadSelector();
        return abi.decode(data[4:], (address, address, uint256, uint256));
    }
}
