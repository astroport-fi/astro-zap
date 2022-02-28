# AstroZap

Enter Astroport XYK pools with any combination of the two assets

For an overview of the math behind zapping, see [this explainer](./docs/astrozap.pdf).

## Development

### Dependencies

* Rust 1.57.0
* `wasm32-unknown-unknown` target
* Docker and [`rust-optimizer`](https://github.com/CosmWasm/rust-optimizer)
* Node.js v16

### Testing

In `./contracts/astrozap` directory:

```bash
cargo test
```

### Compilation

In `./contracts/astrozap` directory:

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.4
```

### Deployment

In `./scripts` directory:

```bash
npm install
ts-node 4_deploy.ts --network {mainnet|testnet} [--code-id codeId]
```

## License

Contents of this repository are open-sourced under [GNU Public License v3](./LICENSE)
