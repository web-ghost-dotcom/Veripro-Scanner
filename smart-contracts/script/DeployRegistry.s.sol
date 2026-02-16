// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/AttestationRegistry.sol";

contract DeployRegistry is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        AttestationRegistry registry = new AttestationRegistry();

        console.log("AttestationRegistry deployed at:", address(registry));

        // The deployer is automatically the owner
        // The prover address should match the CBSE coordinator's signing key
        // By default, we authorize the deployer as the first prover
        address deployer = vm.addr(deployerPrivateKey);
        registry.setProver(deployer, true);
        console.log("Authorized deployer as prover:", deployer);

        vm.stopBroadcast();
    }
}
