#![feature(use_extern_macros)]
#![feature(alloc)]

mod evm;

extern crate protobuf;

extern crate alloc;
extern crate bigint;
extern crate hexutil;
extern crate sha3;
extern crate sputnikvm;

extern crate ekiden_core_common;
extern crate ekiden_core_trusted;

extern crate evm_api;

use evm_api::{with_api, CreateTokenRequest, CreateTokenResponse, EthState, GetBalanceRequest,
                   GetBalanceResponse, InitStateRequest, InitStateResponse, TransferTokenRequest,
                   TransferTokenResponse};

use sputnikvm::{TransactionAction, ValidTransaction};

use bigint::{Address, Gas, H256, U256};
use hexutil::{read_hex, to_hex};
use sha3::{Digest, Keccak256};

use std::str::FromStr;
use std::rc::Rc;

use evm::fire_transactions_and_update_state;

use ekiden_core_common::Result;
use ekiden_core_trusted::db::Db;
use ekiden_core_trusted::rpc::create_enclave_rpc;

// Create enclave RPC handlers.
with_api! {
    create_enclave_rpc!(api);
}

fn create(request: &CreateTokenRequest) -> Result<CreateTokenResponse> {
    let state = Db::instance().get("state")?;
    println!("create creator={}", request.get_creator_address());

    let creator_addr = Address::from_str(request.get_creator_address()).unwrap();

    // EVM bytecode for ERC20 token contract (from https://ethereum.org/token) with the following parameters:
    //
    // decimals: 0
    // initialSupply: <filled from request>
    // tokenName: "Test"
    // tokenSymbol: "TST"
    //
    let mut bytecode: Vec<u8> = read_hex(include_str!("../resources/erc20.contract")).unwrap();
    // Add encoded initialSupply parameter.
    bytecode.extend_from_slice(&H256::from(request.get_initial_supply()));
    // Add remaining constructor parameters (tokenName, tokenSymbol).
    bytecode.extend_from_slice(&read_hex("0x000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000004546573740000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000035453540000000000000000000000000000000000000000000000000000000000").unwrap());

    let transactions = [
        ValidTransaction {
            caller: Some(creator_addr),
            action: TransactionAction::Create,
            gas_price: Gas::zero(),
            gas_limit: Gas::max_value(),
            value: U256::zero(),
            input: Rc::new(bytecode),
            nonce: U256::zero(),
        },
    ];

    let (new_state, _) = fire_transactions_and_update_state(&transactions, &state);

    // Compute address of new token contract. In practice, a web3 client handling a "create" action
    // returns a transaction hash, and the caller needs to wait until the next block is mined to
    // retrieve the contract's address. For simplicity, we manually compute the address and return
    // it immediately. The address is a function of the caller and nonce (see https://ethereum.stackexchange.com/questions/760/how-is-the-address-of-an-ethereum-contract-computed)
    //
    let token_contract_addr = {
        let mut vec = read_hex("0xd694").unwrap().to_vec();
        vec.extend_from_slice(&creator_addr);
        vec.extend_from_slice(&[0x80]);
        to_hex(&Keccak256::digest(&vec)[12..])
    };

    let mut response = CreateTokenResponse::new();
    response.set_contract_address(token_contract_addr.clone());

    Db::instance().set("state", new_state)?;
    Ok(response)
}

fn transfer(request: &TransferTokenRequest) -> Result<TransferTokenResponse> {
    let state = Db::instance().get("state")?;

    println!(
        "transfer amount={}, from={}, to={}",
        request.amount, request.from_address, request.to_address
    );

    let to_addr = Address::from_str(request.get_to_address()).unwrap();

    // Construct the EVM payload for this transaction.
    //
    // To call the contract's "transfer" method, we take the first 4 bytes from the Keccak256 hash
    // of the the function's signature, then append the parameters values (destination and amount),
    // encoded and padded according to the Ethereum ABI spec.
    //
    // For more information, see https://github.com/ethereum/wiki/wiki/Ethereum-Contract-ABI.
    //
    let mut payload =
        Keccak256::digest("transfer(address,uint256)".as_bytes()).as_slice()[..4].to_vec();
    payload.extend_from_slice(&H256::from(to_addr));
    payload.extend_from_slice(&H256::from(request.get_amount()));

    let caller = Address::from_str(request.get_from_address()).unwrap();
    let contract_addr = Address::from_str(request.get_contract_address()).unwrap();

    let transactions = [
        ValidTransaction {
            caller: Some(caller),
            action: TransactionAction::Call(contract_addr),
            gas_price: Gas::zero(),
            gas_limit: Gas::max_value(),
            value: U256::zero(),
            input: Rc::new(payload),
            nonce: U256::zero(),
        },
    ];

    let (new_state, _) = fire_transactions_and_update_state(&transactions, &state);
    let response = TransferTokenResponse::new();

    Db::instance().set("state", new_state)?;
    Ok(response)
}

fn get_balance(request: &GetBalanceRequest) -> Result<GetBalanceResponse> {
    let state = Db::instance().get("state")?;
    println!("get_balance addr={}", request.get_address());

    let address = Address::from_str(request.get_address()).unwrap();
    let contract_addr = Address::from_str(request.get_contract_address()).unwrap();

    // Construct the EVM payload for this transaction. See comment in transfer_tokens() for explanation.
    let mut payload = Keccak256::digest("balanceOf(address)".as_bytes()).as_slice()[..4].to_vec();
    payload.extend_from_slice(&H256::from(address));

    let transactions = [
        ValidTransaction {
            caller: Some(Address::default()),
            action: TransactionAction::Call(contract_addr),
            gas_price: Gas::zero(),
            gas_limit: Gas::max_value(),
            value: U256::zero(),
            input: Rc::new(payload),
            nonce: U256::zero(),
        },
    ];

    let (_, result) = fire_transactions_and_update_state(&transactions, &state);

    let mut response = GetBalanceResponse::new();
    let result_as_u64 = U256::from(result.as_slice()).as_u64();
    response.set_balance(result_as_u64);

    Ok(response)
}

fn init_genesis_state(_request: &InitStateRequest) -> Result<InitStateResponse> {
    let response = InitStateResponse::new();
    Db::instance().set("state", EthState::new())?;
    Ok(response)
}
