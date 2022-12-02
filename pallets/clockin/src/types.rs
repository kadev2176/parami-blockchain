//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
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
