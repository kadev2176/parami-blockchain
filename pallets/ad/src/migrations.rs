use crate::{Config, Pallet};
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;

pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 1 {
        weight.saturating_accrue(v1::migrate::<T>());
        StorageVersion::new(1).put::<Pallet<T>>();
    }

    weight
}

mod v1 {
    use super::*;
    use crate::{types, BalanceOf, Config, HashOf, HeightOf, NftOf, SlotOf};
    use codec::{Decode, Encode};
    use frame_support::RuntimeDebug;
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_runtime::traits::Zero;

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct Slot<B, H, N, T> {
        pub nft: T,
        #[codec(compact)]
        pub budget: B,
        #[codec(compact)]
        pub remain: B,
        #[codec(compact)]
        pub tokens: B,
        pub created: N,
        pub ad: H,
    }

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        <SlotOf<T>>::translate_values(
            |slot: Slot<BalanceOf<T>, HashOf<T>, HeightOf<T>, NftOf<T>>| {
                weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                Some(types::Slot {
                    ad_id: slot.ad,
                    nft_id: slot.nft,
                    fungible_id: None,
                    budget: slot.budget,
                    remain: slot.remain,
                    fractions_remain: slot.tokens,
                    fungibles_budget: Zero::zero(),
                    fungibles_remain: Zero::zero(),
                    created: slot.created,
                })
            },
        );

        weight
    }
}
