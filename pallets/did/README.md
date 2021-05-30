# parami-did

## design

- use `[u8; 20]` as did method specific id type
- use `"4"` as prefix byte
- `base58encode_check(prefix + did_method_specific_id)`

## weights update

```
cargo run --release --features runtime-benchmarks --  benchmark --chain=dev --steps=50 --repeat=20 --pallet=parami_did --execution=wasm --wasm-execution=compiled --heap-pages=4096 --output=./pallets/did/src/weights.rs --template=./.maintain/frame-weight-template.hbs --extrinsic '*'


./target/release/parami  benchmark --chain=dev --steps=50 --repeat=200 --pallet=parami_did --execution=wasm --wasm-execution=compiled --heap-pages=4096 --output=./pallets/did/src/weights.rs --template=./.maintain/frame-weight-template.hbs --extrinsic='*'
```
