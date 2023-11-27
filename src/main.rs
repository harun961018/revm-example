use anyhow::{Ok, Result};
use bytes::Bytes;
use ethers_contract::BaseContract;
use ethers_core::abi::parse_abi;
use ethers_providers::{Http, Provider};
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{address, ExecutionResult, Output, TransactTo, U256},
    Database, EVM,
};
use std::{sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    let http_url = "https://virginia.rpc.blxrbdn.com";
    let client = Provider::<Http>::try_from(http_url)?;
    let client = Arc::new(client);

    let mut ethersdb = EthersDB::new(client.clone(), None).unwrap();
    let pool_address = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");

    let slot = U256::from(8);
    let abi = BaseContract::from(
        parse_abi(&[
            "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
        ])?
    );
    let encoded = abi.encode("getReserves", ())?;
    let mut ethersdb = EthersDB::new(Arc::clone(&client), None).unwrap();
    let acc_info = ethersdb.basic(pool_address).unwrap().unwrap();
    let value = ethersdb.storage(pool_address, slot).unwrap();
    println!("{:?}", acc_info); // 0x64ca691b00000000000000001d11899c51780000000003aa5712d4e77e453b6c_U256
    
    let mut cache_db = CacheDB::new(EmptyDB::default());
    cache_db.insert_account_info(pool_address, acc_info);
    cache_db.insert_account_storage(pool_address, slot, value).unwrap();
    let mut evm = EVM::new();
    evm.database(cache_db);

    // let pool_contract = BaseContract::from(
    //     parse_abi(&[
    //         "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
    //     ])?
    // );

    // let encoded = pool_contract.encode("getReserves", ())?;

    evm.env.tx.caller = address!("0000000000000000000000000000000000000000");
    // account you want to transact with
    evm.env.tx.transact_to = TransactTo::Call(pool_address);
    // calldata formed via abigen
    evm.env.tx.data = encoded.0.into();
    // transaction value in wei
    evm.env.tx.value = U256::from(0);

    // execute transaction without writing to the DB
    let ref_tx = evm.transact_ref().unwrap();
    // select ExecutionResult struct
    let result = ref_tx.result;

    // unpack output call enum into raw bytes
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        result => panic!("Execution failed: {result:?}"),
    };

    // decode bytes to reserves + ts via ethers-rs's abi decode
    let (reserve0, reserve1, ts): (u128, u128, u32) = abi.decode_output("getReserves", value)?;

    // Print emulated getReserves() call output
    println!("Reserve0: {:#?}", reserve0);
    println!("Reserve1: {:#?}", reserve1);
    println!("Timestamp: {:#?}", ts);

    Ok(())
}