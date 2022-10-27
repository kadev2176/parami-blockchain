use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StakingActivity<A, AC, H, B> {
    pub asset_id: A,
    pub reward_total_amount: B,
    pub reward_total_remains: B,
    pub reward_pot: AC,
    pub start_block_num: H,
    pub halve_time: H,
    pub lastblock: H,
    pub total_supply: B,
    pub earnings_per_share: B,
    pub daily_output: B,
}
