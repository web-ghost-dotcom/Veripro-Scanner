// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../src/SimpleVault.sol";

/// @notice Symbolic tests for SimpleVault
contract SimpleVaultTest is Test {
    SimpleVault public vault;

    function setUp() public {
        vault = new SimpleVault();
    }

    /// @notice Test that deposit increases balance correctly
    function testDeposit(uint256 amount) public {
        // Assume valid input range
        vm.assume(amount > 0);
        vm.assume(amount < type(uint128).max);

        address user = address(0x1234);
        vm.prank(user);

        uint256 balanceBefore = vault.balances(user);
        uint256 totalBefore = vault.totalDeposits();

        vault.deposit(amount);

        // Assertions that should always hold
        assert(vault.balances(user) == balanceBefore + amount);
        assert(vault.totalDeposits() == totalBefore + amount);
    }

    /// @notice Test that withdraw decreases balance correctly
    function testWithdraw(
        uint256 depositAmount,
        uint256 withdrawAmount
    ) public {
        // Assume valid input ranges
        vm.assume(depositAmount > 0);
        vm.assume(depositAmount < type(uint128).max);
        vm.assume(withdrawAmount > 0);
        vm.assume(withdrawAmount <= depositAmount);

        address user = address(0x1234);
        vm.prank(user);
        vault.deposit(depositAmount);

        uint256 balanceBefore = vault.balances(user);
        uint256 totalBefore = vault.totalDeposits();

        vm.prank(user);
        vault.withdraw(withdrawAmount);

        // Assertions that should always hold
        assert(vault.balances(user) == balanceBefore - withdrawAmount);
        assert(vault.totalDeposits() == totalBefore - withdrawAmount);
    }

    /// @notice Test that you cannot withdraw more than your balance
    function testWithdrawOverflow(
        uint256 depositAmount,
        uint256 withdrawAmount
    ) public {
        // Assume deposit is less than withdraw (should fail)
        vm.assume(depositAmount > 0);
        vm.assume(depositAmount < type(uint128).max);
        vm.assume(withdrawAmount > depositAmount);
        vm.assume(withdrawAmount < type(uint128).max);

        address user = address(0x1234);
        vm.prank(user);
        vault.deposit(depositAmount);

        vm.prank(user);
        vm.expectRevert("Insufficient balance");
        vault.withdraw(withdrawAmount);
    }

    /// @notice Test balance consistency - balance should never exceed total deposits
    function testBalanceInvariant(uint256 amount) public {
        vm.assume(amount > 0);
        vm.assume(amount < type(uint128).max);

        address user = address(0x1234);
        vm.prank(user);
        vault.deposit(amount);

        // Invariant: user balance should never exceed total deposits
        assert(vault.balances(user) <= vault.totalDeposits());
    }

    /// @notice Test that zero deposits are rejected
    function testZeroDeposit() public {
        address user = address(0x1234);
        vm.prank(user);
        vm.expectRevert("Amount must be greater than 0");
        vault.deposit(0);
    }

    /// @notice Test multiple deposits from same user
    function testMultipleDeposits(uint256 amount1, uint256 amount2) public {
        vm.assume(amount1 > 0);
        vm.assume(amount2 > 0);
        vm.assume(amount1 < type(uint64).max);
        vm.assume(amount2 < type(uint64).max);

        address user = address(0x1234);

        vm.prank(user);
        vault.deposit(amount1);

        vm.prank(user);
        vault.deposit(amount2);

        // Balance should be sum of both deposits
        assert(vault.balances(user) == amount1 + amount2);
        assert(vault.totalDeposits() == amount1 + amount2);
    }
}
