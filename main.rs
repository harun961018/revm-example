use anyhow::{Ok, Result};
use bytes::Bytes;
use ethers_contract::BaseContract;
use ethers_core::abi::parse_abi;
use ethers_providers::{Http, Provider};
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{ExecutionResult, Output, TransactTo, B160, U256 as rU256},
    Database, EVM,
};
use std::{str::FromStr, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let http_url = "<HTTPS_RPC_ENDPOINT>";
    let client = Provider::<Http>::try_from(http_url)?;
    let client = Arc::new(client);

    let mut ethersdb = EthersDB::new(client.clone(), None).unwrap();
    let pool_address = B160::from_str("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852")?;
    let acc_info = ethersdb.basic(pool_address).unwrap().unwrap();

    let slot = rU256::from(8);
    let value = ethersdb.storage(pool_address, slot).unwrap();
    println!("{:?}", value); // 0x64ca691b00000000000000001d11899c51780000000003aa5712d4e77e453b6c_U256
    
    let mut cache_db = CacheDB::new(EmptyDB::default());
    cache_db.insert_account_info(pool_address, acc_info);
    cache_db.insert_account_storage(pool_address, slot, value).unwrap();
    let mut evm = EVM::new();
    evm.database(cache_db);

    let pool_contract = BaseContract::from(
        parse_abi(&[
            "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
        ])?
    );

    let encoded = pool_contract.encode("getReserves", ())?;

    evm.env.tx.caller = B160::from_str("0x0000000000000000000000000000000000000000")?;
    evm.env.tx.transact_to = TransactTo::Call(pool_address);
    evm.env.tx.data = encoded.0;
    evm.env.tx.value = rU256::ZERO;

    let ref_tx = evm.transact_ref().unwrap();
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(value) => Some(value),
            _ => None,
        },
        _ => None,
    };
    println!("{:?}", value);
    Ok(())
}