rpc_api! {
    metadata {
        name = dummy;
        version = "0.1.0";
        client_attestation_required = false;
    }

    rpc hello_world(HelloWorldRequest) -> HelloWorldResponse;
}
