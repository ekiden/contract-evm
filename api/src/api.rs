rpc_api! {
    metadata {
        name = evm;
        version = "0.1.0";
        client_attestation_required = false;
    }

    rpc init_genesis_state(InitStateRequest) -> InitStateResponse;

    rpc create(CreateContractRequest) -> CreateContractResponse;

    rpc mix_genes(MixGenesRequest) -> MixGenesResponse;
}
