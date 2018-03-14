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

use evm_api::{with_api, CreateContractRequest, CreateContractResponse, EthState, InitStateRequest,
              InitStateResponse, MixGenesRequest, MixGenesResponse};

use sputnikvm::{TransactionAction, ValidTransaction};

use bigint::{Address, Gas, H256, U256};
use hexutil::{read_hex, to_hex};

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

fn create(_request: &CreateContractRequest) -> Result<CreateContractResponse> {
    println!("*** Creating mixGenes contract");

    let creator_addr = Address::from_str("0x4e4f41484e4f41484e4f41484e4f41484e4f4148").unwrap();
    let bytecode: Vec<u8> = read_hex(include_str!("../resources/cryptokitties.contract")).unwrap();

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

    let (new_state, _) = fire_transactions_and_update_state(&transactions, &EthState::new(), 1);
    let response = CreateContractResponse::new();

    Db::instance().set("state", new_state)?;
    Ok(response)
}

fn mix_genes(request: &MixGenesRequest) -> Result<MixGenesResponse> {
    let state = Db::instance().get("state")?;

    println!(
        "*** Mixing genes {} and {} on block {}",
        request.matron_genes, request.sire_genes, request.current_block
    );

    // The contract address is a fixed function of the creator's address (defined above) and the
    // nonce. We hardcode the contract address to avoid exposing it to the client.
    let contract_addr = Address::from_str("bb68efec2bf97899407b9887f8b5a6a68dd59f7b").unwrap();

    // Construct the transaction payload, which includes the hash of the mixGenes signature and
    // parameter values.
    let mut payload: Vec<u8> = read_hex("0x0d9f5aed").unwrap();
    payload.extend_from_slice(&H256::from_str(request.get_matron_genes()).unwrap());
    payload.extend_from_slice(&H256::from_str(request.get_sire_genes()).unwrap());
    payload.extend_from_slice(&H256::from(request.get_target_block()));

    let transactions = [
        ValidTransaction {
            // Caller address can be anything (the mixGenes contract doesn't care)
            caller: Some(Address::from_str("0x0371678bd6734b85c3f35a0cb233ebb1477c6284").unwrap()),
            action: TransactionAction::Call(contract_addr),
            gas_price: Gas::zero(),
            gas_limit: Gas::max_value(),
            value: U256::zero(),
            input: Rc::new(payload),
            nonce: U256::zero(),
        },
    ];

    let current_block = request.get_current_block();

    let (_, result) = fire_transactions_and_update_state(&transactions, &state, current_block);

    let mut response = MixGenesResponse::new();
    response.set_mixed_genes(to_hex(&result));

    Ok(response)
}

fn init_genesis_state(_request: &InitStateRequest) -> Result<InitStateResponse> {
    let response = InitStateResponse::new();
    Db::instance().set("state", EthState::new())?;
    Ok(response)
}
