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
  "ChainId": "u8",
  "ResourceId": "[u8; 32]",
  "DepositNonce": "u64",
  "ClassId": "u32",
  "TokenId": "u64",
  "TAssetBalance": "u128",
  "NativeBalance": "Balance",
  "SwapAssetBalance": "TAssetBalance",
  "SwapPair": {
    "account": "AccountId",
    "nativeReserve": "Balance",
    "assetReserve": "TAssetBalance"
  },
  "ProposalStatus": {
    "_enum": [
      "Initiated",
      "Approved",
      "Rejected"
    ]
  },
  "ProposalVotes": {
    "votesFor": "Vec<AccountId>",
    "votesAgainst": "Vec<AccountId>",
    "status": "ProposalStatus",
    "expiry": "BlockNumber"
  },
  "TagType": "u8",
  "TagCoefficient": "u8",
  "TagScore": "i8",
  "GlobalId": "u64",
  "AdId": "GlobalId",
  "AdvertiserId": "GlobalId",
  "AdvertiserOf": {
    "createdTime": "Compact<Moment>",
    "advertiserId": "Compact<AdvertiserId>",
    "deposit": "Compact<Balance>",
    "depositAccount": "AccountId",
    "rewardPoolAccount": "AccountId"
  },
  "AdvertisementOf": {
    "createdTime": "Compact<Moment>",
    "deposit": "Compact<Balance>",
    "tagCoefficients": "Vec<(TagType, TagCoefficient)>",
    "signer": "AccountId",
    "mediaRewardRate": "Compact<PerU16>"
  },
  "AssetId": "u64",
  "ClassIdOf": "ClassId",
  "CollectionType": {
    "_enum": [
      "Collectable",
      "Executable"
    ]
  },
  "GroupCollectionId": "u64",
  "TokenType": {
    "_enum": [
      "Transferable",
      "BoundToAddress"
    ]
  },
  "ClassData": {
    "deposit": "Balance",
    "metadata": "Vec<u8>",
    "tokenType": "TokenType",
    "collectionType": "CollectionType",
    "totalSupply": "u64",
    "initialSupply": "u64"
  },
  "ClassInfoOf": {
    "metadata": "Vec<u8>",
    "totalIssuance": "TokenId",
    "owner": "AccountId",
    "data": "ClassData"
  },
  "TokenIdOf": "TokenId",
  "AssetData": {
    "deposit": "Balance",
    "name": "Vec<u8>",
    "description": "Vec<u8>",
    "properties": "Vec<u8>"
  },
  "TokenInfoOf": {
    "metadata": "Vec<u8>",
    "owner": "AccountId",
    "data": "AssetData"
  },
  "Erc20Event": {
    "_enum": {
      "Transfer": {
        "value": "Compact<Balance>",
        "from": "Vec<u8>"
      },
      "Withdraw": {
        "value": "Compact<Balance>",
        "who": "Vec<u8>",
        "status": "bool"
      },
      "Redeem": {
        "value": "Compact<Balance>",
        "from": "Vec<u8>",
        "to": "AccountId"
      },
      "Despoit": {
          "value": "Compact<Balance>",
          "to": "Vec<u8>",
          "from": "AccountId"
      }
    }
  }
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

### Test
```
cargo test -p package-name -- --nocapture
```
