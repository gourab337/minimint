# Minimint

**This is an experimental implementation of a federated Chaumian bank. DO NOT USE IT WITH REAL MONEY, THERE ARE MULTIPLE KNOWN SECURITY ISSUES.**

## Running MiniMint locally
MiniMint is tested and developed using rust `stable`, you can get it through your package manager or from [rustup.rs](https://rustup.rs/).

MiniMint consists of three kinds of services:
* federation member nodes (`server` binary) which make up the federation and run the consensus protocol among each other.
* a Lightning gateway (`ln_gateway` binary) which acts as a bridge between the federation and the Lightning network allowing users to pay LN invoices with e-cash tokens.
* user clients (`mint-client` binary) that interact with the federation nodes and the gateway

In the following we will set up all three.

### Generating config
You first need to generate some config. All scripts assume config to be located in a folder called `cfg`. Then you can generate the necessary configuration files as follows:

```shell
mkdir -p cfg
cargo run --bin configgen cfg <num_nodes> <federation_ports> <api_ports> <tier1> <tier2> …
```

The placeholders can be filled in as follows:
* **`<num_nodes>`:** number of nodes to generate config for. Should be >= 4 and not too big as the cryptography of the BFT protocol is rather intense and you should ideally have 1 core per node.
* **`<federation_ports>`:** base port for federation internal connections. If it is set to 5000 for example and there are 4 nodes they will use ports 5000, 5001, 5002 and 5003.
* **`<api_ports>`:** base port for the federation node API server which user clients connect to. If it is set to 6000 for example and there are 4 nodes they will use ports 6000, 6001, 6002 and 6003.
* **`<tier1> … <tier n>`:** E-cash token denominations/amount tiers in milli sat. There are different token denominations to increase efficiency so that instead of issuing 10 1sat tokens 1 10sat token can be issued. Generally powers of a base are a decent choice, e.g. powers of 10: 1 10 100 1000 10000 100000 1000000 10000000 100000000 1000000000 

An example with concrete parameters could look as follows:
```shell
cargo run --bin configgen cfg 4 5000 6000 1 10 100 1000 10000 100000 1000000 10000000 100000000 1000000000
```

This will both create all the `server-n.json` config files and one `federation_client.json`. The server configs are already complete and can be used to run the nodes. The client config on the other hand needs to be amended with some information about a lightning gateway it can use. For that we run

```shell
cargo run --bin gw_configgen -- cfg <ln_rpc>
```

The **`<ln_rpc>`** placeholder should be replaced with the absolute path to a c-lightning `lightning-rpc` socket, typically located at `/home/<user>/.lightning/regtest/lightning-rpc` for regtest nodes. If you do not intend to use the LN feature this path does not have to be correct and you will not even have to start the gateway. But for the client to start we need to add at least a dummy gateway.

An example with concrete parameters could look as follows:
```shell
cargo run --bin gw_configgen -- cfg /home/user/.lightning/regtest/lightning-rpc
```

`gw_configgen` will both generate the final `client.json` for clients as well as `gateway.json` which will be used by the Lightning gateway.

### Running the mints
A script for running all mints and a regtest `bitcoind` at once is provided at `scripts/startfed.sh`. Run it as follows:

```shell
bash scripts/startfed.sh <num_nodes> 0
```

The `0` in the end specifies how many nodes to leave out. E.g. changing it to one would skip the first node. This is useful to run a single node with a debugger attached.

Log output can be adjusted using the `RUST_LOG` environment variable and is set to `info` by default. Logging can be adjusted per module, see the [`env_logger` documentation](https://docs.rs/env_logger/0.8.4/env_logger/#enabling-logging) for details.

### Using the client
First you need to make sure that your regtest `bitcoind` has some coins that are mature. For that you can generate a few hundred blocks to your own wallet:

```shell
ADDRESS="$(bitcoin-cli -regtest -rpcuser=bitcoin -rpcpassword=bitcoin getnewaddress)"
bitcoin-cli -regtest -rpcuser=bitcoin -rpcpassword=bitcoin generatetoaddress 200 "$ADDRESS"
```

Then you can use the peg-in script to deposit funds. It contains comments that explain the deposit process.

```shell
bash scripts/pegin.sh <amount in BTC>
```

Take care to not request too big of a peg-in (depending on your amount tier the smallest representations as tokens might be too big to finish signing in reasonable time) or too small (there is a 500sat fee that needs to be paid). After about 20s your default client in `cfg` should have newly issued coins.

You can view your client's holdings using the `info` command:

```
minimint $ cargo run --bin mint-client --release -- cfg info
    Finished release [optimized] target(s) in 0.11s
     Running `target/release/mint-client cfg info`
Jun 15 14:57:22.066  INFO mint_client: We own 14 coins with a total value of 9500000 msat
Jun 15 14:57:22.066  INFO mint_client: We own 5 coins of denomination 100000 msat
Jun 15 14:57:22.066  INFO mint_client: We own 9 coins of denomination 1000000 msat
```

The `spend` subcommand allows to send tokens to another client. This will select the smallest possible set of the client's coins that represents a given amount. The coins are base64 encoded and printed to stdout.

```
minimint $ cargo run --bin mint-client --release -- cfg spend 400000
    Finished release [optimized] target(s) in 0.12s
     Running `target/release/mint-client cfg spend 400000`
AQAAAAAAAACghgEAAAAAAAQAAAAAAAAAA7mGus9L4ojsIsctFK7oNz5s4ozZe3pVa0S1jZ3XvSnMMAAAAAAAAACFjOG3a4vlxBOCa9fYD6qWIM2JhH9vitG0
DXQhd9KhGYKheKADLOVXgZwOQDX0NheGP5fFEMYOfidY1FPXB1qRhrEiZKh3YVb2i922uUHoggOsZhrLpk4EGCJjuT1QUWpO8HZ9WOxD4oUv6nPNVQnKvDAA
AAAAAAAAoXKhtm/0w8pFz7CN6xcEQUnukrNcfhc/NtRita1vvZDyX/NBiSmHZVyWx8WEloclIw0A8ljJhp+b517c1LsLJ5Z6Issf9QcV/hwAgY/RJo4DRGWD
IDyyyBYXxRbFuTZoDaTR3TM/49m41Bl7/CPVz98wAAAAAAAAAJOjsZSwWrBUXt+OsojEkxRbqn8KAJrz1TTQkNrdlEiaSRqjx+YCfET3HwL3j26s2clhRugM
rRj6oMC6wKoZz0jCuS5i8faLRHGZp3AMR1/xAvMglQZ9zMEDdDd7dcxwp9WpR6JfdAUJku3EGQ/FUXaQMAAAAAAAAACUXn9s935ruZ5jA5o5aNf1u/smH4TN
+qO8jMHVf6Zzh22P5jJvhWdX62s7kftXTa9AKeiC0I4QxWdWVK4JTLnE62GzGLQqQyEkne3Pn/Pm1g==
```

A receiving client can now reissue these coins to claim them and avoid double spends:

```
minimint $ cargo run --bin mint-client --release -- cfg reissue AQAAAAAAAACghgE…
    Finished release [optimized] target(s) in 0.13s
     Running `target/release/mint-client cfg reissue AQAAAAAAAACghgE…
Jun 15 15:01:47.027  INFO mint_client: Starting reissuance transaction for 400000 msat
Jun 15 15:01:47.040  INFO mint_client: Started reissuance 47d8f08710423c1e300854ecb6463ca6185e4b3890bbbb90fd1ff70c72e1ed18, please fetch the result later
minimint $ cargo run --bin mint-client --release -- cfg fetch
    Finished release [optimized] target(s) in 0.12s
     Running `target/release/mint-client cfg fetch`
Jun 15 15:02:06.264  INFO mint_client: Fetched coins from issuance 47d8f08710423c1e300854ecb6463ca6185e4b3890bbbb90fd1ff70c72e1ed18
```

There also exist some other, more experimental commands that can be explored using the `--help` flag:

```
minimint $ cargo run --bin mint-client --release -- --help
    Finished release [optimized] target(s) in 0.12s
     Running `target/release/mint-client --help`
mint-client 0.1.0

USAGE:
    mint-client <workdir> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <workdir>    

SUBCOMMANDS:
    fetch             Fetch (re-)issued coins and finalize issuance process
    help              Prints this message or the help of the given subcommand(s)
    info              Display wallet info (holdings, tiers)
    ln-pay            Pay a lightning invoice via a gateway
    peg-in            Issue tokens in exchange for a peg-in proof (not yet implemented, just creates coins)
    peg-in-address    Generate a new peg-in address, funds sent to it can later be claimed
    peg-out           Withdraw funds from the federation
    reissue           Reissue tokens received from a third party to avoid double spends
    spend             Prepare coins to send to a third party as a payment
```
