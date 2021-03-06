#![feature(use_extern_macros)]

#[macro_use]
extern crate clap;
extern crate futures;
extern crate hex;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate tokio_core;

#[macro_use]
extern crate client_utils;
extern crate ekiden_core_common;
extern crate ekiden_rpc_client;

extern crate evm_api;

use clap::{App, Arg};
use futures::future::Future;

use rand::{thread_rng, Rng};

use ekiden_rpc_client::create_client_rpc;
use evm_api::with_api;

with_api! {
    create_client_rpc!(evm, evm_api, api);
}

// Initial supply of tokens.
const INITIAL_SUPPLY: u64 = 1_000_000;

// Address of token creator. Can be anything but must parse to a valid Ethereum address (160-bit).
const TOKEN_CREATOR: &str = "0x4e4f41484e4f41484e4f41484e4f41484e4f4148";

// Amount to transfer from this client.
const TRANSFER_AMOUNT: u64 = 3;

// Address to transfer tokens to.
const TRANSFER_TO_ADDR: &str = "0x57415252454e57415252454e57415252454e0000";

// Address of created contract (set by init method).
static mut CONTRACT_ADDR: Option<String> = None;

const OTHER_ACCOUNT_COUNT: usize = 100;
lazy_static! {
    static ref OTHER_ACCOUNTS: Vec<String> = {
        // Generate some random account names.
        let mut accounts = vec![];

        for _ in 0..OTHER_ACCOUNT_COUNT {
            let mut buf = [0; 20];
            thread_rng().fill_bytes(&mut buf);
            accounts.push(String::from("0x") + &hex::encode(buf));
        }

        accounts
    };
}

/// Initializes the ethtoken scenario.
fn init<Backend>(client: &mut evm::Client<Backend>, _runs: usize, _threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    // Initialize empty state.
    client
        .init_genesis_state(evm::InitStateRequest::new())
        .wait()
        .unwrap();

    // Create new ERC20 token contract. Returns the address of the newly created contract.
    // When instantiated, the contract automatically assigns all initial tokens to the contract's
    // creator (i.e. the caller). The token name and symbol are hardcoded in the contract bytecode
    // so they aren't specified here.
    println!(
        "Creating token contract with {} initial tokens (creator address {})",
        INITIAL_SUPPLY, TOKEN_CREATOR
    );
    let contract_addr = client
        .create({
            let mut req = evm::CreateTokenRequest::new();
            req.set_creator_address(TOKEN_CREATOR.to_string());
            req.set_initial_supply(INITIAL_SUPPLY);
            req
        })
        .wait()
        .unwrap()
        .get_contract_address()
        .to_string();

    unsafe {
        CONTRACT_ADDR = Some(contract_addr.clone());
    }
    println!("Token contract address: {}", contract_addr);

    // Request the token balance of the creator's address. Should equal the initial supply.
    let balance = client
        .get_balance({
            let mut req = evm::GetBalanceRequest::new();
            req.set_contract_address(contract_addr.clone());
            req.set_address(TOKEN_CREATOR.to_string());
            req
        })
        .wait()
        .unwrap()
        .get_balance();

    println!("\nBalance of address {} = {}", TOKEN_CREATOR, balance);
    assert_eq!(
        balance, INITIAL_SUPPLY,
        "Creator did not receive initial tokens"
    );

    // Populate the other accounts.
    for other_account in OTHER_ACCOUNTS.iter() {
        // Transfer tokens from the creator to a given address.
        println!("Populating other account {}", other_account);

        client
            .transfer({
                let mut req = evm::TransferTokenRequest::new();
                unsafe {
                    req.set_contract_address(CONTRACT_ADDR.as_ref().unwrap().clone());
                }
                req.set_from_address(TOKEN_CREATOR.to_string());
                req.set_to_address(other_account.clone());
                req.set_amount(1);
                req
            })
            .wait()
            .unwrap();
    }
}

/// Runs the ethtoken scenario.
fn scenario<Backend>(client: &mut evm::Client<Backend>)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    #[cfg(feature = "benchmark_transfer")]
    {
        // Transfer tokens from the creator to a given address.
        println!(
            "Transferring {} tokens from {} to {}",
            TRANSFER_AMOUNT, TOKEN_CREATOR, TRANSFER_TO_ADDR
        );

        client
            .transfer({
                let mut req = evm::TransferTokenRequest::new();
                unsafe {
                    req.set_contract_address(CONTRACT_ADDR.as_ref().unwrap().clone());
                }
                req.set_from_address(TOKEN_CREATOR.to_string());
                req.set_to_address(TRANSFER_TO_ADDR.to_string());
                req.set_amount(TRANSFER_AMOUNT);
                req
            })
            .wait()
            .unwrap();
    }
    #[cfg(feature = "benchmark_get_balance")]
    {
        // Check the balance of the creator's address.
        let creator_balance = client
            .get_balance({
                let mut req = evm::GetBalanceRequest::new();
                unsafe {
                    req.set_contract_address(CONTRACT_ADDR.as_ref().unwrap().clone());
                }
                req.set_address(TOKEN_CREATOR.to_string());
                req
            })
            .wait()
            .unwrap()
            .get_balance();

        println!(
            "\nBalance of address {} = {}",
            TOKEN_CREATOR, creator_balance
        );
    }
}

/// Finalize the ethtoken scenario.
fn finalize<Backend>(client: &mut evm::Client<Backend>, runs: usize, threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    // Check the final balance of the creator's address.
    let creator_balance = client
        .get_balance({
            let mut req = evm::GetBalanceRequest::new();
            unsafe {
                req.set_contract_address(CONTRACT_ADDR.as_ref().unwrap().clone());
            }
            req.set_address(TOKEN_CREATOR.to_string());
            req
        })
        .wait()
        .unwrap()
        .get_balance();

    println!(
        "\nBalance of address {} = {}",
        TOKEN_CREATOR, creator_balance
    );
    #[cfg(feature = "benchmark_transfer")]
    assert_eq!(
        creator_balance,
        INITIAL_SUPPLY - OTHER_ACCOUNT_COUNT as u64
            - TRANSFER_AMOUNT * runs as u64 * threads as u64,
        "Tokens not debited from sender"
    );

    // Check the balance for the destination address.
    let dest_balance = client
        .get_balance({
            let mut req = evm::GetBalanceRequest::new();
            unsafe {
                req.set_contract_address(CONTRACT_ADDR.as_ref().unwrap().clone());
            }
            req.set_address(TRANSFER_TO_ADDR.to_string());
            req
        })
        .wait()
        .unwrap()
        .get_balance();

    println!("Balance of address {} = {}", TRANSFER_TO_ADDR, dest_balance);
    #[cfg(feature = "benchmark_transfer")]
    assert_eq!(
        dest_balance,
        TRANSFER_AMOUNT * runs as u64 * threads as u64,
        "Tokens not transferred"
    );
}

#[cfg(feature = "benchmark")]
fn main() {
    let results = benchmark_client!(evm, init, scenario, finalize);
    results.show();
}

#[cfg(not(feature = "benchmark"))]
fn main() {
    let mut client = contract_client!(evm);
    init(&mut client, 1, 1);
    scenario(&mut client);
    finalize(&mut client, 1, 1);
}
