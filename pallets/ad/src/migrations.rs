pub mod v5 {
    use codec::{Decode, Encode, MaxEncodedLen};
    use frame_support::traits::OnRuntimeUpgrade;
    use frame_support::weights::Weight;
    use scale_info::TypeInfo;

    use crate::Pallet;
    use crate::StorageVersion;
    use crate::{Config, CurrencyOrAsset, SlotMetaOf, SlotOf};
    use sp_runtime::RuntimeDebug;

    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};

    mod old {
        use super::*;
        use crate::{AccountOf, AssetsOf, HashOf, HeightOf, NftOf};
        use frame_support::Twox64Concat;

        #[derive(
            Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        )]
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
        pub struct V4Slot<Hash, Height, NftId, TokenId, AccountId> {
            pub ad_id: Hash,
            pub nft_id: NftId,
            pub fraction_id: TokenId,
            pub fungible_id: Option<TokenId>,
            // budget pot is specifically for locking budget.
            pub budget_pot: AccountId,
            pub created: Height,
        }

        type V4SlotMetaOf<T> = V4Slot<HashOf<T>, HeightOf<T>, NftOf<T>, AssetsOf<T>, AccountOf<T>>;

        #[frame_support::storage_alias]
        pub(super) type SlotOf<T: Config> =
            StorageMap<crate::Pallet<T>, Twox64Concat, NftOf<T>, V4SlotMetaOf<T>>;
    }

    pub struct BidWithCurrencyOrAsset<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for BidWithCurrencyOrAsset<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version > 4 {
                return 0;
            }

            for (nft_id, slot) in old::SlotOf::<T>::iter() {
                let new_slot = SlotMetaOf::<T> {
                    ad_id: slot.ad_id,
                    nft_id: slot.nft_id,
                    ad_asset: CurrencyOrAsset::Asset(slot.fraction_id),
                    fungible_id: slot.fungible_id,
                    budget_pot: slot.budget_pot,
                    created: slot.created,
                };
                SlotOf::<T>::insert(nft_id, new_slot);
            }

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(5));
            0
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use log::info;
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert!(
                // for staging
                storage_version <= 4,
                "current storage version should be less than 5"
            );

            let mut counter = 0;
            for (_, slot) in old::SlotOf::<T>::iter() {
                info!("Migrating slot {:?} ", slot);
                counter += 1;
            }
            info!("total slot num = {:?}", counter);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use log::info;
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert!(storage_version == 5, "current storage version should be 5");

            let mut counter = 0;
            for (_, slot) in SlotOf::<T>::iter() {
                info!("new slot {:?} ", slot);
                counter += 1;
            }
            info!("total slot num = {:?}", counter);

            Ok(())
        }
    }
}
