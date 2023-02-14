//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct External<Did> {
    pub owner: Did,
    pub network: Network,
    pub namespace: Vec<u8>,
    pub token: Vec<u8>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo, MaxEncodedLen)]
pub struct Metadata<Did, AccountId, NftClassId, AssetId> {
    pub owner: Did,
    pub pot: AccountId,
    pub class_id: NftClassId,
    pub minted: bool,
    pub token_asset_id: AssetId,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct ImportTask<Did> {
    pub owner: Did,
    pub network: Network,
    pub namespace: Vec<u8>,
    pub token: Vec<u8>,
    pub owner_address: Vec<u8>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct IcoMeta<Balance, Account> {
    pub expected_currency: Balance,
    pub offered_tokens: Balance,
    pub done: bool,
    pub pot: Account,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct InfluenceMiningMeta<Balance, Account> {
    pub budget_in_tokens: Balance,
    pub pot: Account,
    pub done: bool,
}
