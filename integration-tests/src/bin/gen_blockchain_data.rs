use ethers::{
    abi::Tokenize,
    contract::{Contract, ContractFactory},
    core::types::{TransactionRequest, U256},
    core::utils::WEI_IN_ETHER,
    middleware::SignerMiddleware,
    providers::Middleware,
    signers::Signer,
    solc::Solc,
};
use integration_tests::{
    get_client, get_provider, get_wallet, log_init, CompiledContract, GenDataOutput, CONTRACTS,
    CONTRACTS_PATH,
};
use log::{debug, info};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

async fn deploy<T, M>(prov: Arc<M>, compiled: &CompiledContract, args: T) -> Contract<M>
where
    T: Tokenize,
    M: Middleware,
{
    info!("Deploying {}...", compiled.name);
    let factory = ContractFactory::new(compiled.abi.clone(), compiled.bin.clone(), prov);
    factory
        .deploy(args)
        .expect("cannot deploy")
        .confirmations(0usize)
        .send()
        .await
        .expect("cannot confirm deploy")
}

#[tokio::main]
async fn main() {
    log_init();
    // Compile contracts
    info!("Compiling contracts...");
    let mut contracts = HashMap::new();
    for (name, contract_path) in CONTRACTS {
        let path_sol = Path::new(CONTRACTS_PATH).join(contract_path);
        let compiled = Solc::default()
            .compile_source(&path_sol)
            .expect("solc compile error");
        if !compiled.errors.is_empty() {
            panic!("Errors compiling {:?}:\n{:#?}", &path_sol, compiled.errors)
        }

        let contract = compiled
            .get(
                &path_sol.to_str().expect("path is not str").to_string(),
                name,
            )
            .expect("contract not found");
        let abi = contract.abi.expect("no abi found").clone();
        let bin = contract.bin.expect("no bin found").clone();
        let bin_runtime = contract.bin_runtime.expect("no bin_runtime found").clone();
        let compiled_contract = CompiledContract {
            path: path_sol.to_str().expect("path is not str").to_string(),
            name: name.to_string(),
            abi,
            bin,
            bin_runtime,
        };

        let mut path_json = path_sol.clone();
        path_json.set_extension("json");
        serde_json::to_writer(
            &File::create(&path_json).expect("cannot create file"),
            &compiled_contract,
        )
        .expect("cannot serialize json into file");

        contracts.insert(name.to_string(), compiled_contract);
    }

    let prov = get_provider();

    // Wait for geth to be online.
    loop {
        match prov.client_version().await {
            Ok(version) => {
                info!("Geth online: {}", version);
                break;
            }
            Err(err) => {
                debug!("Geth not available: {:?}", err);
                sleep(Duration::from_millis(500));
            }
        }
    }

    // Make sure the blockchain is in a clean state: block 0 is the last block.
    let block_number = prov
        .get_block_number()
        .await
        .expect("cannot get block number");
    if block_number.as_u64() != 0 {
        panic!(
            "Blockchain is not in a clean state.  Last block number: {}",
            block_number
        );
    }

    let accounts = prov.get_accounts().await.expect("cannot get accounts");
    let wallet0 = get_wallet(0);
    info!("wallet0: {:x}", wallet0.address());

    let mut blocks = HashMap::new();

    // Transfer funds to our account.
    info!("Transferring funds from coinbase...");
    let tx = TransactionRequest::new()
        .to(wallet0.address())
        .value(WEI_IN_ETHER) // send 1 ETH
        .from(accounts[0]);
    prov.send_transaction(tx, None)
        .await
        .expect("cannot send tx")
        .await
        .expect("cannot confirm tx");
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Transfer 0".to_string(), block_num.as_u64());

    // Deploy smart contracts
    let mut deployments = HashMap::new();
    let prov_wallet0 = Arc::new(SignerMiddleware::new(get_provider(), wallet0));
    let contract = deploy(
        prov_wallet0.clone(),
        contracts.get("Greeter").expect("contract not found"),
        U256::from(42),
    )
    .await;
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Deploy Greeter".to_string(), block_num.as_u64());
    deployments.insert(
        "Greeter".to_string(),
        (block_num.as_u64(), contract.address()),
    );

    // Generate a block with multiple transfers
    info!("Generating block with multiple transfers...");
    const NUM_TXS: usize = 4;
    let wallets: Vec<_> = (0..NUM_TXS + 1)
        .map(|i| Arc::new(SignerMiddleware::new(get_provider(), get_wallet(i as u32))))
        .collect();

    let cli = get_client();

    // Fund NUM_TXS wallets from coinbase
    cli.miner_stop().await.expect("cannot stop miner");
    let mut pending_txs = Vec::new();
    for wallet in &wallets[0..NUM_TXS] {
        let tx = TransactionRequest::new()
            .to(wallet.address())
            .value(WEI_IN_ETHER * 2u8) // send 2 ETH
            .from(accounts[0]);
        pending_txs.push(
            prov.send_transaction(tx, None)
                .await
                .expect("cannot send tx"),
        );
    }
    cli.miner_start().await.expect("cannot start miner");
    for tx in pending_txs {
        tx.await.expect("cannot confirm tx");
    }
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Fund wallets".to_string(), block_num.as_u64());

    // Make NUM_TXS transfers in a "chain"
    cli.miner_stop().await.expect("cannot stop miner");
    let mut pending_txs = Vec::new();
    for i in 0..NUM_TXS {
        let tx = TransactionRequest::new()
            .to(wallets[i + 1].address())
            .value(WEI_IN_ETHER / (2 * (i + 1))) // send a fraction of an ETH
            .from(wallets[i].address());
        pending_txs.push(
            wallets[i]
                .send_transaction(tx, None)
                .await
                .expect("cannot send tx"),
        );
    }
    cli.miner_start().await.expect("cannot start miner");
    for tx in pending_txs {
        tx.await.expect("cannot confirm tx");
    }
    let block_num = prov.get_block_number().await.expect("cannot get block_num");
    blocks.insert("Multiple transfers 0".to_string(), block_num.as_u64());

    let gen_data = GenDataOutput {
        coinbase: accounts[0],
        wallets: wallets.iter().map(|w| w.address()).collect(),
        blocks,
        deployments,
    };
    gen_data.store();
}
