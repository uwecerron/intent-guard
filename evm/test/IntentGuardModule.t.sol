// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "../src/IntentGuardModule.sol";

/// @notice Smoke-test scaffolding for the Foundry layout.
/// @dev This file intentionally avoids external Safe dependencies. Add protocol-
/// specific tests before any deployment.
contract IntentGuardModuleSmokeTest {
    function testModuleHasBytecode() public pure {
        require(type(IntentGuardModule).creationCode.length > 0, "missing module bytecode");
    }
}
