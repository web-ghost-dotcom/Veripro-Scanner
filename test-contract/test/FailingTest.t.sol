// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test} from "forge-std/Test.sol";

contract FailingTest is Test {
    function test_WillFail() public {
        // This assertion will always fail
        assert(false);
    }

    function test_WillPass() public {
        // This assertion will pass
        assert(true);
    }

    function test_ArithmeticOverflow() public {
        uint256 x = type(uint256).max;
        // This will cause overflow (Panic code 0x11)
        x = x + 1;
    }
}
