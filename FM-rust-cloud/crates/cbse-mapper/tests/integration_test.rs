// SPDX-License-Identifier: AGPL-3.0

#[cfg(test)]
mod tests {
    use cbse_mapper::{AstNode, ContractMappingInfo, DeployAddressMapper, Mapper};
    use serde_json::json;

    #[test]
    fn test_ast_node_creation() {
        let node = AstNode::new(
            "FunctionDefinition".to_string(),
            "transfer".to_string(),
            "0x12345678".to_string(),
        );

        assert_eq!(node.node_type, "FunctionDefinition");
        assert_eq!(node.name, "transfer");
        assert_eq!(node.selector, "0x12345678");
    }

    #[test]
    fn test_ast_node_from_dict() {
        let json_data = json!({
            "nodeType": "FunctionDefinition",
            "name": "balanceOf",
            "functionSelector": "70a08231"
        });

        let node = AstNode::from_dict(&json_data).unwrap();
        assert_eq!(node.node_type, "FunctionDefinition");
        assert_eq!(node.name, "balanceOf");
        assert_eq!(node.selector, "0x70a08231");
    }

    #[test]
    fn test_ast_node_from_dict_event() {
        let json_data = json!({
            "nodeType": "EventDefinition",
            "name": "Transfer",
            "eventSelector": "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        });

        let node = AstNode::from_dict(&json_data).unwrap();
        assert_eq!(node.node_type, "EventDefinition");
        assert_eq!(node.name, "Transfer");
        assert!(node.selector.starts_with("0x"));
    }

    #[test]
    fn test_ast_node_from_dict_error() {
        let json_data = json!({
            "nodeType": "ErrorDefinition",
            "name": "InsufficientBalance",
            "errorSelector": "f4d678b8"
        });

        let node = AstNode::from_dict(&json_data).unwrap();
        assert_eq!(node.node_type, "ErrorDefinition");
        assert_eq!(node.name, "InsufficientBalance");
        assert_eq!(node.selector, "0xf4d678b8");
    }

    #[test]
    fn test_ast_node_from_dict_invalid() {
        let json_data = json!({
            "nodeType": "UnsupportedNode",
            "name": "something"
        });

        let node = AstNode::from_dict(&json_data);
        assert!(node.is_none());
    }

    #[test]
    fn test_contract_mapping_info_creation() {
        let info = ContractMappingInfo::new("MyContract".to_string());
        assert_eq!(info.contract_name, "MyContract");
        assert!(info.bytecode.is_none());
        assert!(info.nodes.is_empty());
    }

    #[test]
    fn test_contract_mapping_info_with_bytecode() {
        let info = ContractMappingInfo::new("MyContract".to_string())
            .with_bytecode("0x6080604052...".to_string());

        assert_eq!(info.contract_name, "MyContract");
        assert!(info.bytecode.is_some());
        assert_eq!(info.bytecode.unwrap(), "0x6080604052...");
    }

    #[test]
    fn test_contract_mapping_info_add_node() {
        let mut info = ContractMappingInfo::new("MyContract".to_string());
        let node = AstNode::new(
            "FunctionDefinition".to_string(),
            "transfer".to_string(),
            "0xa9059cbb".to_string(),
        );

        info.add_node(node);
        assert_eq!(info.nodes.len(), 1);
        assert!(info.get_node("0xa9059cbb").is_some());
    }

    #[test]
    fn test_contract_mapping_info_with_nodes() {
        let nodes = vec![
            AstNode::new(
                "FunctionDefinition".to_string(),
                "transfer".to_string(),
                "0xa9059cbb".to_string(),
            ),
            AstNode::new(
                "FunctionDefinition".to_string(),
                "approve".to_string(),
                "0x095ea7b3".to_string(),
            ),
        ];

        let info = ContractMappingInfo::new("ERC20".to_string()).with_nodes(nodes);

        assert_eq!(info.nodes.len(), 2);
        assert!(info.get_node("0xa9059cbb").is_some());
        assert!(info.get_node("0x095ea7b3").is_some());
    }

    #[test]
    fn test_contract_mapping_info_get_function_name() {
        let nodes = vec![AstNode::new(
            "FunctionDefinition".to_string(),
            "transfer".to_string(),
            "0xa9059cbb".to_string(),
        )];

        let info = ContractMappingInfo::new("ERC20".to_string()).with_nodes(nodes);

        assert_eq!(
            info.get_function_name("0xa9059cbb"),
            Some("transfer".to_string())
        );
        assert_eq!(info.get_function_name("0xdeadbeef"), None);
    }

    #[test]
    fn test_deploy_address_mapper() {
        let mut mapper = DeployAddressMapper::new();
        let addr1 = vec![0x12, 0x34, 0x56, 0x78];
        let addr2 = vec![0xab, 0xcd, 0xef, 0x90];

        mapper.add_mapping(addr1.clone(), "ContractA".to_string());
        mapper.add_mapping(addr2.clone(), "ContractB".to_string());

        assert_eq!(mapper.get_name(&addr1), Some(&"ContractA".to_string()));
        assert_eq!(mapper.get_name(&addr2), Some(&"ContractB".to_string()));

        assert_eq!(mapper.get_address("ContractA"), Some(&addr1));
        assert_eq!(mapper.get_address("ContractB"), Some(&addr2));
        assert_eq!(mapper.get_address("ContractC"), None);
    }

    #[test]
    fn test_deploy_address_mapper_overwrite() {
        let mut mapper = DeployAddressMapper::new();
        let addr = vec![0x12, 0x34, 0x56, 0x78];

        mapper.add_mapping(addr.clone(), "ContractA".to_string());
        mapper.add_mapping(addr.clone(), "ContractB".to_string());

        // Should overwrite
        assert_eq!(mapper.get_name(&addr), Some(&"ContractB".to_string()));
    }

    #[test]
    fn test_mapper_creation() {
        let mapper = Mapper::new();
        assert!(mapper.contracts().is_empty());
    }

    #[test]
    fn test_mapper_add_contract() {
        let mut mapper = Mapper::new();
        let info = ContractMappingInfo::new("MyContract".to_string());

        mapper.add_contract(info);
        assert_eq!(mapper.contracts().len(), 1);
        assert!(mapper.get_contract("MyContract").is_some());
    }

    #[test]
    fn test_mapper_get_function_name() {
        let mut mapper = Mapper::new();

        let nodes = vec![AstNode::new(
            "FunctionDefinition".to_string(),
            "transfer".to_string(),
            "0xa9059cbb".to_string(),
        )];

        let info = ContractMappingInfo::new("ERC20".to_string()).with_nodes(nodes);

        mapper.add_contract(info);

        assert_eq!(
            mapper.get_function_name("ERC20", "0xa9059cbb"),
            Some("transfer".to_string())
        );
        assert_eq!(mapper.get_function_name("ERC20", "0xdeadbeef"), None);
        assert_eq!(mapper.get_function_name("NonExistent", "0xa9059cbb"), None);
    }

    #[test]
    fn test_mapper_multiple_contracts() {
        let mut mapper = Mapper::new();

        // Add ERC20 contract
        let erc20_nodes = vec![
            AstNode::new(
                "FunctionDefinition".to_string(),
                "transfer".to_string(),
                "0xa9059cbb".to_string(),
            ),
            AstNode::new(
                "FunctionDefinition".to_string(),
                "balanceOf".to_string(),
                "0x70a08231".to_string(),
            ),
        ];

        let erc20_info = ContractMappingInfo::new("ERC20".to_string()).with_nodes(erc20_nodes);

        // Add ERC721 contract
        let erc721_nodes = vec![AstNode::new(
            "FunctionDefinition".to_string(),
            "ownerOf".to_string(),
            "0x6352211e".to_string(),
        )];

        let erc721_info = ContractMappingInfo::new("ERC721".to_string()).with_nodes(erc721_nodes);

        mapper.add_contract(erc20_info);
        mapper.add_contract(erc721_info);

        assert_eq!(mapper.contracts().len(), 2);
        assert_eq!(
            mapper.get_function_name("ERC20", "0xa9059cbb"),
            Some("transfer".to_string())
        );
        assert_eq!(
            mapper.get_function_name("ERC721", "0x6352211e"),
            Some("ownerOf".to_string())
        );
    }

    #[test]
    fn test_mapper_with_deploy_addresses() {
        let mut mapper = Mapper::new();

        let addr = vec![0x12, 0x34, 0x56, 0x78];
        mapper
            .deploy_addresses
            .add_mapping(addr.clone(), "MyContract".to_string());

        let info = ContractMappingInfo::new("MyContract".to_string());
        mapper.add_contract(info);

        assert!(mapper.get_contract("MyContract").is_some());
        assert_eq!(
            mapper.deploy_addresses.get_name(&addr),
            Some(&"MyContract".to_string())
        );
    }

    #[test]
    fn test_selector_fields_constants() {
        use cbse_mapper::SELECTOR_FIELDS;

        let expected = vec![
            ("VariableDeclaration", "functionSelector"),
            ("FunctionDefinition", "functionSelector"),
            ("EventDefinition", "eventSelector"),
            ("ErrorDefinition", "errorSelector"),
        ];

        assert_eq!(SELECTOR_FIELDS, &expected[..]);
    }
}
