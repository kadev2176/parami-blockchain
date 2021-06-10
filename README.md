# Parami Blockchain

Dana-v3 testnet.

## types

NOTE: The current `types.json` is at [parami-types.json](./parami-types.json).

```json
{
  "DidMethodSpecId": "[u8; 20]",
  "Public": "MultiSigner",
  "LookupSource": "MultiAddress",
  "Address": "MultiAddress",
  "AccountInfo": "AccountInfoWithProviders"
}
```

## Pallets

### Airdrop

5EYCAe5ijiYdQH8WgewkjxMti9fCfnSbCuFsGtn4wkHeYGDJ

### ChainBridge

```
const MODULE_ID: ModuleId = ModuleId(*b"cb/bridg");
```

5EYCAe5g7bGpFHagwe26HiRHdHdE3hobrwV6hq1UD2BPAiZb

All cross assets should under this account.

### CrossAssets

Cross-chain assets. Current implementation only handles native AD3 token.

### Assets

Mirrored from `pallet-assets`, with fn visiblity modifications. Thus it can be used as a dependent pallet.

```text
create(id: AssetId, admin: LookupSource, min_balance: Balance)
set_metadata(id: T::AssetId, name: Vec<u8>, symbol: Vec<u8>, decimals: u8)
mint(id: T::AssetId, beneficiary: LookupSource, amount: T::Balance)
transfer(id: T::AssetId, target: LookupSource, amount: T::Balance)
```

### Swap

```text
create(asset_id: AssetId)
add_liquidity(asset_id: AssetId, native_amount: Balance, maybe_asset_amount: Option<AssetBalance>)
remove_liquidity(asset_id: AssetId, liquidity_amount: Balance)
buy(asset_id: AssetId, native_amount: Balance)
sell(asset_id: AssetId, asset_amount: AssetBalance)
```

