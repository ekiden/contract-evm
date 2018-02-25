#![feature(use_extern_macros)]

extern crate protobuf;

extern crate ekiden_core_common;
extern crate ekiden_core_trusted;

extern crate dummy_api;

use dummy_api::{with_api, HelloWorldRequest, HelloWorldResponse};

use ekiden_core_common::Result;
use ekiden_core_trusted::rpc::create_enclave_rpc;

// Create enclave RPC handlers.
with_api! {
    create_enclave_rpc!(api);
}

fn hello_world(request: &HelloWorldRequest) -> Result<HelloWorldResponse> {
    let mut response = HelloWorldResponse::new();
    response.set_world(format!("enclave says {}", request.hello));

    Ok(response)
}
