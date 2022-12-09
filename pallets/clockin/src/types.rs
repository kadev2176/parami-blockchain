//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct Metadata<Height, Account, Balance, AssetId> {
    pub payout_base: Balance,
    pub payout_min: Balance,
    pub payout_max: Balance,
    pub pot: Account,
    pub metadata: Vec<u8>,
    pub asset_id: AssetId,
    pub start_at: Height,
    pub bucket_size: Height,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct LotteryMetadata<Height, Account, Balance, AssetId> {
    pub level_probability: Vec<u32>,
    pub level_upper_bounds: Vec<Balance>,
    pub shares_per_bucket: u32,
    pub award_per_share: Balance,
    pub pot: Account,
    pub asset_id: AssetId,
    pub start_at: Height,
    pub bucket_size: Height,
}
