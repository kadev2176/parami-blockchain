use crate::*;

use parami_primitives::Balance;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StableAccount<Moment, AccountId> {
	#[codec(compact)]
	pub created_time: Moment,
	pub stash_account: AccountId,
	pub controller_account: AccountId,
	pub magic_account: AccountId,
}

pub type GlobalId = u64;

pub type StableAccountOf<T> =
	StableAccount<<T as pallet_timestamp::Config>::Moment, <T as frame_system::Config>::AccountId>;

pub type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type ResultPost<T> = sp_std::result::Result<T, DispatchErrorWithPostInfo<PostDispatchInfo>>;

pub const UNIT: Balance = 1_000_000_000_000_000;
pub const FEE: Balance = 10 * UNIT;
