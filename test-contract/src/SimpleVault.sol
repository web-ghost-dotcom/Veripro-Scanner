// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/// @notice A simple vault contract for testing symbolic execution
contract SimpleVault {
    mapping(address => uint256) public balances;
    uint256 public totalDeposits;

    event Deposit(address indexed user, uint256 amount);
    event Withdraw(address indexed user, uint256 amount);

    /// @notice Deposit funds into the vault
    function deposit(uint256 amount) external {
        require(amount > 0, "Amount must be greater than 0");
        balances[msg.sender] += amount;
        totalDeposits += amount;
        emit Deposit(msg.sender, amount);
    }

    /// @notice Withdraw funds from the vault
    function withdraw(uint256 amount) external {
        require(amount > 0, "Amount must be greater than 0");
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;
        totalDeposits -= amount;
        emit Withdraw(msg.sender, amount);
    }

    /// @notice Get balance of an address
    function getBalance(address user) external view returns (uint256) {
        return balances[user];
    }
}
