1. Change parachain runtime settings
    * Session length: 10 mins
    * Lease Period Length: 14,400 Blocks (1 day)
    * Ending Period: 600 Blocks (60 mins)
    * Current Lease Period Index = Current Block Number / 14400
1. Register as a Parathread
    1. Register Para ID
    1. Generate genesis state
        `./target/release/parami-para export-genesis-state --chain parachain-2000-raw.json > para-2000-genesis`
    1. Generate genesis wasm
        `./target/release/parami-para export-genesis-wasm --chain parachain-2000-raw.json > para-2000-wasm`
    1. Register Parathread
1. Auctions
    1. Network -> Parachains -> Auctions
    1. Or Network -> Parachains -> Crowdloan

ref:
1. https://docs.substrate.io/tutorials/v3/cumulus/connect-parachain/
1. https://docs.substrate.io/tutorials/v3/cumulus/rococo/
