1. Change parachain runtime settings
    - Session length: 10 mins
    - Lease Period Length: 14,400 Blocks (1 day)
    - Ending Period: 600 Blocks (60 mins)
    - Current Lease Period Index = Current Block Number / 14400
1. Register as a Parathread
    1. Register Para ID
    1. Generate chain spec
       `./target/release/parami-collator build-spec --disable-default-bootnode > parami-rococo.json`
       `./target/release/parami-collator build-spec --chain parami-rococo.json --raw --disable-default-bootnode > parami-rococo-2001-raw.json`
    1. Generate genesis state
       `./target/release/parami-collator export-genesis-state --chain parami-rococo-2001-raw.json > para-2001-genesis`
    1. Generate genesis wasm
       `./target/release/parami-collator export-genesis-wasm --chain parami-rococo-2001-raw.json > para-2001-wasm`
    1. Register Parathread
1. Auctions
    1. Network -> Parachains -> Auctions
    1. Or Network -> Parachains -> Crowdloan

ref:

1. https://docs.substrate.io/tutorials/v3/cumulus/connect-parachain/
1. https://docs.substrate.io/tutorials/v3/cumulus/rococo/
