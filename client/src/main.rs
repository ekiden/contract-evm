#![feature(use_extern_macros)]

#[macro_use]
extern crate clap;
extern crate futures;
extern crate hex;
extern crate rand;
extern crate tokio_core;

#[macro_use]
extern crate client_utils;
extern crate ekiden_core_common;
extern crate ekiden_rpc_client;

extern crate evm_api;

use clap::{App, Arg};
use futures::future::Future;

use ekiden_rpc_client::create_client_rpc;
use evm_api::with_api;

with_api! {
    create_client_rpc!(evm, evm_api, api);
}


/// Initializes the ethtoken scenario.
fn init<Backend>(client: &mut evm::Client<Backend>, _runs: usize, _threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    // Initialize empty state.
    println!(
        "Initializing empty state"
    );

    client
        .init_genesis_state(evm::InitStateRequest::new())
        .wait()
        .unwrap();

    // Create new mixGenes contract.
    println!(
        "Creating mixGenes contract"
    );

    client
        .create(evm::CreateContractRequest::new())
        .wait()
        .unwrap();
}

/// Runs the ethtoken scenario.
fn scenario<Backend>(client: &mut evm::Client<Backend>)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    // Replay inputs from this transaction:
    //   https://etherscan.io/tx/0x2cc763b0856e96bfa3a4d6afe07ab9593c7da7f46b84cde3ca069a2a8f293581
    let matron_genes = "0x00004256931885498634a12e00060210c4318cd1986621ce730ce273de95b5ce";
    let sire_genes = "0x00005a10829087718c67a13700000210c46358b0806a318b3308427a5296b5b6";
    let target_block = 1;

    // Current block number on which this transaction will be mined. The transaction above
    // was mined on block 4976798. This value affects the behavior of mixGenes!
    //
    // IMPORTANT: The mixGenes contract requests the hash of a block relative to the current
    // mined block. If you change this number you'll need to supply the hash for a different
    // block (see handle_fire method in evm.rs, which hard-codes an actual blockhash from
    // Ethereum mainnet).
    let current_block = 4976798;

    // Mix some genes!
    let mixed_genes = client
        .mix_genes({
            let mut req = evm::MixGenesRequest::new();
            req.set_matron_genes(matron_genes.to_string());
            req.set_sire_genes(sire_genes.to_string());
            req.set_target_block(target_block);
            req.set_current_block(current_block);
            req
        })
        .wait()
        .unwrap()
        .get_mixed_genes()
        .to_string();

    // Output of this transaction is 0x000042d0831892718634a51700000210c4330cd08c0a21ce531c464bde96b6ce
    // (see https://etherscan.io/vmtrace?txhash=0x2cc763b0856e96bfa3a4d6afe07ab9593c7da7f46b84cde3ca069a2a8f293581&type=parity)
    println!(
        "Mixed genes = {}",
        mixed_genes
    );

    assert_eq!(mixed_genes, "0x000042d0831892718634a51700000210c4330cd08c0a21ce531c464bde96b6ce", "Unexpected mixed gene result");
}

/// Finalize the ethtoken scenario.
fn finalize<Backend>(_client: &mut evm::Client<Backend>, _runs: usize, _threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
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
