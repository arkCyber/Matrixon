//! Smart Contract Interaction Module
//!
//! Provides functionality for interacting with Ethereum smart contracts.
//! Author: arkSong (arksong2018@gmail.com)
//! Version: 0.1.0
//! Date: 2025-06-15

use web3::{
    types::{Address, H256},
    contract::{Contract, Options},
    Transport,
    Web3,
};
use thiserror::Error;
use tracing::{info, instrument};
use hex;
use web3::ethabi::Contract as EthContract;
use web3::ethabi::Token;
use serde_json;


/// Smart contract manager
#[derive(Debug)]
pub struct ContractManager<T: Transport> {
    web3: Web3<T>,
    contracts: Vec<Contract<T>>,
    abi: EthContract,
}

impl<T: Transport> ContractManager<T> {
    /// Create new contract manager
    #[instrument(level = "debug")]
    pub fn new(web3: Web3<T>, abi: EthContract) -> Self {
        info!("🔧 Initializing ContractManager");
        Self {
            web3,
            contracts: Vec::new(),
            abi,
        }
    }

    /// Deploy new contract
    #[instrument(level = "debug")]
    pub async fn deploy(
        &mut self,
        web3: &web3::Web3<T>,
        abi: &[u8],
        bytecode: &[u8],
        options: Options,
    ) -> std::result::Result<Address, ContractError> {
        let abi = web3::ethabi::Contract::load(abi)?;
        let bytecode_hex = format!("0x{}", hex::encode(bytecode));
        let accounts = web3.eth().accounts().await?;
        let from = accounts[0];
        let abi_json = serde_json::to_vec(&abi).map_err(ContractError::Serialization)?;
        let contract = Contract::deploy(web3.eth(), &abi_json)?
            .options(options)
            .execute(bytecode_hex, (), from)
            .await?;
        let address = contract.address();
        self.contracts.push(contract);
        Ok(address)
    }

    /// Call contract function
    #[instrument(level = "debug")]
    pub async fn call(
        &self,
        contract_index: usize,
        function: &str,
        params: (),
        from: Address,
        options: Options,
    ) -> std::result::Result<(), ContractError> {
        if contract_index >= self.contracts.len() {
            return Err(ContractError::InvalidIndex(contract_index));
        }

        self.contracts[contract_index]
            .call(function, params, from, options)
            .await?;
        Ok(())
    }

    #[instrument(level = "debug")]
    pub async fn deploy_contract(&mut self, bytecode: &[u8], params: Vec<Token>) -> std::result::Result<Address, ContractError> {
        let bytecode_hex = hex::encode(bytecode);
        let accounts = self.web3.eth().accounts().await?;
        let from = accounts[0];

        let abi_json = serde_json::to_vec(&self.abi).map_err(ContractError::Serialization)?;
        let contract = Contract::deploy(self.web3.eth(), &abi_json)?
            .confirmations(1)
            .options(Options::with(|opt| {
                opt.gas = Some(3_000_000.into());
            }))
            .execute(bytecode_hex, params, from)
            .await?;

        let address = contract.address();
        self.contracts.push(contract);
        Ok(address)
    }

    pub async fn call_contract(
        &self,
        contract_index: usize,
        function: &str,
        params: Vec<Token>,
        from: Address,
        options: Options,
    ) -> std::result::Result<H256, ContractError> {
        if contract_index >= self.contracts.len() {
            return Err(ContractError::InvalidIndex(contract_index));
        }

        let tx_hash = self.contracts[contract_index]
            .call(function, params, from, options)
            .await?;

        Ok(tx_hash)
    }

    pub async fn query_contract(
        &self,
        contract_index: usize,
        function: &str,
        params: Vec<Token>,
        from: Address,
        options: Options,
    ) -> std::result::Result<Vec<Token>, ContractError> {
        if contract_index >= self.contracts.len() {
            return Err(ContractError::InvalidIndex(contract_index));
        }

        let result = self.contracts[contract_index]
            .query(function, params, from, options, None)
            .await?;

        Ok(result)
    }

    pub fn get_contract(&self, address: Address) -> Option<&Contract<T>> {
        self.contracts.iter().find(|c| c.address() == address)
    }
}

/// Contract-specific errors
#[derive(Error, Debug)]
pub enum ContractError {
    /// Web3 contract error
    #[error("Web3 error: {0}")]
    Web3(#[from] web3::Error),

    /// Contract deployment error
    #[error("Contract deployment error: {0}")]
    Deploy(#[from] web3::contract::deploy::Error),

    /// Contract error
    #[error("Contract error: {0}")]
    Contract(#[from] web3::contract::Error),

    /// Invalid contract index
    #[error("Invalid contract index: {0}")]
    InvalidIndex(usize),

    /// Invalid address
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Ethabi error
    #[error("Ethabi error: {0}")]
    Ethabi(#[from] web3::ethabi::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;
    use test_log::test;
    use web3::transports::Http;
    use web3::types::TransactionRequest;

    #[test]
    fn test_contract_manager_creation() -> Result<(), Box<dyn std::error::Error>> {
        // Create simple ABI for testing
        let abi_json = r#"[
            {
                "inputs": [],
                "name": "getValue",
                "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
                "stateMutability": "view",
                "type": "function"
            }
        ]"#;
        
        let abi = web3::ethabi::Contract::load(abi_json.as_bytes())?;
        
        // Create manager with dummy web3 instance
        let transport = Http::new("http://localhost:8545").unwrap();
        let web3 = web3::Web3::new(transport);
        let manager = ContractManager::new(web3, abi);
        
        assert_eq!(manager.contracts.len(), 0);
        Ok(())
    }
}
