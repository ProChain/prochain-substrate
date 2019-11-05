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

Build the WebAssembly binary:

```bash
./scripts/build.sh
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
