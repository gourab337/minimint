use bitcoin::{Address, Transaction};
use bitcoin_hashes::hex::ToHex;
use minimint::config::load_from_file;
use minimint::modules::mint::tiered::coins::Coins;
use minimint::modules::wallet::txoproof::TxOutProof;
use minimint_api::encoding::Decodable;
use minimint_api::Amount;
use mint_client::mint::SpendableCoin;
use mint_client::{ClientAndGatewayConfig, UserClient};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(StructOpt)]
struct Options {
    workdir: PathBuf,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Generate a new peg-in address, funds sent to it can later be claimed
    PegInAddress,

    /// Issue tokens in exchange for a peg-in proof (not yet implemented, just creates coins)
    PegIn {
        #[structopt(parse(try_from_str = from_hex))]
        txout_proof: TxOutProof,
        #[structopt(parse(try_from_str = from_hex))]
        transaction: Transaction,
    },

    /// Reissue tokens received from a third party to avoid double spends
    Reissue {
        #[structopt(parse(from_str = parse_coins))]
        coins: Coins<SpendableCoin>,
    },

    /// Prepare coins to send to a third party as a payment
    Spend { amount: Amount },

    /// Withdraw funds from the federation
    PegOut {
        address: Address,
        amount: bitcoin::Amount,
    },

    /// Pay a lightning invoice via a gateway
    LnPay { bolt11: lightning_invoice::Invoice },

    /// Fetch (re-)issued coins and finalize issuance process
    Fetch,

    /// Display wallet info (holdings, tiers)
    Info,
}

#[derive(Debug, Serialize, Deserialize)]
struct PayRequest {
    coins: Coins<SpendableCoin>,
    invoice: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let opts: Options = StructOpt::from_args();
    let cfg_path = opts.workdir.join("client.json");
    let db_path = opts.workdir.join("client.db");
    let cfg: ClientAndGatewayConfig = load_from_file(&cfg_path);
    let db = sled::open(&db_path)
        .unwrap()
        .open_tree("mint-client")
        .unwrap();

    let mut rng = rand::rngs::OsRng::new().unwrap();

    let client = UserClient::new(cfg.client, Box::new(db), Default::default());

    match opts.command {
        Command::PegInAddress => {
            println!("{}", client.get_new_pegin_address(&mut rng))
        }
        Command::PegIn {
            txout_proof,
            transaction,
        } => {
            let id = client
                .peg_in(txout_proof, transaction, &mut rng)
                .await
                .unwrap();
            info!(
                "Started peg-in {}, please fetch the result later",
                id.to_hex()
            );
        }
        Command::Reissue { coins } => {
            info!("Starting reissuance transaction for {}", coins.amount());
            let id = client.reissue(coins, &mut rng).await.unwrap();
            info!("Started reissuance {}, please fetch the result later", id);
        }
        Command::Spend { amount } => {
            match client.select_and_spend_coins(amount) {
                Ok(outgoing_coins) => {
                    println!("{}", serialize_coins(&outgoing_coins));
                }
                Err(e) => {
                    error!("Error: {:?}", e);
                }
            };
        }
        Command::Fetch => {
            for id in client.fetch_all_coins().await.unwrap() {
                info!("Fetched coins from issuance {}", id.to_hex());
            }
        }
        Command::Info => {
            let coins = client.coins();
            info!(
                "We own {} coins with a total value of {}",
                coins.coin_count(),
                coins.amount()
            );
            for (amount, coins) in coins.coins {
                info!("We own {} coins of denomination {}", coins.len(), amount);
            }
        }
        Command::PegOut { address, amount } => {
            client.peg_out(amount, address, &mut rng).await.unwrap();
        }
        Command::LnPay { bolt11 } => {
            let http = reqwest::Client::new();

            let contract_id = client
                .fund_outgoing_ln_contract(&cfg.gateway, bolt11, &mut rng)
                .await
                .expect("Not enough coins");

            client
                .wait_contract_timeout(contract_id, Duration::from_secs(5))
                .await
                .expect("Contract wasn't accepted in time");

            info!(
                "Funded outgoing contract {}, notifying gateway",
                contract_id
            );

            http.post(&format!("{}/pay_invoice", cfg.gateway.api))
                .json(&contract_id)
                .timeout(Duration::from_secs(15))
                .send()
                .await
                .unwrap();
        }
    }
}

fn parse_coins(s: &str) -> Coins<SpendableCoin> {
    let bytes = base64::decode(s).unwrap();
    bincode::deserialize(&bytes).unwrap()
}

fn serialize_coins(c: &Coins<SpendableCoin>) -> String {
    let bytes = bincode::serialize(&c).unwrap();
    base64::encode(&bytes)
}

fn from_hex<D: Decodable>(s: &str) -> Result<D, Box<dyn Error>> {
    let bytes = hex::decode(s)?;
    Ok(D::consensus_decode(std::io::Cursor::new(bytes))?)
}
