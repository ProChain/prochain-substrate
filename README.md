# Prochain

A new SRML-based Substrate node, ready for hacking.

# Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build Prochain:

```
cargo build --release
```

Ensure you have a fresh start if updating from another version:
```
./target/release/prochain purge-chain --dev
```

To start up the Prochain node, run:
```
./target/release/prochain \
  --chain ./customSpecRaw.json \
  --key "your key" \
  --name NodeName \
  --bootnodes /ip4/47.91.247.187/tcp/30333/p2p/QmbKYUXW4V3vBxzXBGAEBRhh2Fm21jq6XWKbzS4i43yGn6 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

# Settings

1) Open [Polkadot UI](https://polkadot.js.org/apps/#/explorer) . 

2) Go to *Settings*, open *Developer* tab. Insert in textbox description of types (copy&paste from here) and Save it.


```bash

{
  "ExternalAddress": {
    "btc": "Vec<u8>",
    "eth": "Vec<u8>",
    "eos": "Vec<u8>"
  },
  "LockedRecords": {
    "locked_time": "Moment",
    "locked_period": "Moment",
    "locked_funds": "Balance",
    "rewards_ratio": "u64",
    "max_quota": "u64"
  },
  "UnlockRecords": {
    "unlock_time": "Moment",
    "unlock_funds": "Balance"
  },
  "MetadataRecord": {
    "address": "AccountId",
    "superior": "Hash",
    "creator": "AccountId",
    "did_ele": "Vec<u8>",
    "locked_records": "Option<LockedRecords<Balance, Moment>>",
    "unlock_records": "Option<UnlockRecords<Balance, Moment>>",
    "social_account": "Option<Hash>",
    "subordinate_count": "u64",
    "external_address": "ExternalAddress"
  },
  "Value": "u32",
  "BTCValue": {
    "price": "u32",
    "block_number": "u32"
  },
  "AdsMetadata": {
    "advertiser": "Vec<u8>",
    "topic": "Vec<u8>",
    "total_amount": "Balance",
    "surplus": "Balance",
    "gas_fee_used": "Balance",
    "single_click_fee": "Balance",
    "create_time": "Moment",
    "period": "Moment"
  },
  "HTLC": {
    "block_number": "BlockNumber",
    "out_amount": "Balance",
    "expire_height": "BlockNumber",
    "random_number_hash": "Hash",
    "swap_id": "Hash",
    "timestamp": "Moment",
    "sender_addr": "Vec<u8>",
    "sender_chain_type": "u64",
    "receiver_addr": "AccountId",
    "receiver_chain_type": "u64",
    "recipient_addr": "Vec<u8>"
  },
  "States": {
    "_enum": [
      "INVALID",
      "OPEN",
      "COMPLETED",
      "EXPIRED"
    ]
  }
}
```

# Development

You can start a development chain with:

```bash
cargo run -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units. Give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet). You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmQZ8TjTqeDj3ciwr93EJ95hxfDsb9pEYDizUAbWpigtQN \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.
