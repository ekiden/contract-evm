#![feature(use_extern_macros)]

#[macro_use]
extern crate clap;
extern crate futures;
extern crate rand;
extern crate tokio_core;

#[macro_use]
extern crate client_utils;
extern crate ekiden_core_common;
extern crate ekiden_rpc_client;

extern crate dummy_api;

use clap::{App, Arg};
use futures::future::Future;

use ekiden_rpc_client::create_client_rpc;
use dummy_api::with_api;

with_api! {
    create_client_rpc!(dummy, dummy_api, api);
}

fn main() {
    let mut client = contract_client!(dummy);

    // Send some text.
    let mut request = dummy::HelloWorldRequest::new();
    request.set_hello("hello from client".to_string());

    // Call contract method and check the response.
    let response = client.hello_world(request).wait().unwrap();

    assert_eq!(response.get_world(), "enclave says hello from client");
}
