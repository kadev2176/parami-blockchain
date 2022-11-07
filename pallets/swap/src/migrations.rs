use frame_support::traits::OnRuntimeUpgrade;

pub mod v4 {
    use frame_support::Twox64Concat;
    use frame_support::{traits::StorageVersion, weights::Weight};

    use crate::{types, AccountOf, AssetOf, BalanceOf, Config, HeightOf, Pallet};

    use codec::{Decode, Encode, MaxEncodedLen};
    use frame_support::log::info;
    use frame_support::traits::tokens::fungibles::Inspect;
    use parami_traits::Stakes;
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_runtime::traits::Zero;
    use sp_runtime::RuntimeDebug;

    use super::*;

    mod old {
        use super::*;

        #[derive(
            Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        )]
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
        pub struct V3Swap<T: Config> {
            pub created: HeightOf<T>,
            pub liquidity: BalanceOf<T>,
        }

        #[frame_support::storage_alias]
        pub(super) type Metadata<T: Config> =
            StorageMap<crate::Pallet<T>, Twox64Concat, AssetOf<T>, V3Swap<T>>;
    }

    pub struct ReInitStakingRewardOfOldNfts<T>(sp_std::marker::PhantomData<T>);
    pub const DOLLARS: u128 = 1_000_000_000_000_000_000;

    impl<T: crate::Config> ReInitStakingRewardOfOldNfts<T> {
        fn cal_remain_tokens(asset_id: AssetOf<T>) -> BalanceOf<T> {
            let already_insurance = <T::Assets as Inspect<AccountOf<T>>>::total_issuance(asset_id);
            let max_10_million = (10_000_000 * DOLLARS)
                .try_into()
                .map_err(|_| "Type Cast Error")
                .unwrap();

            // staking_reward_amount initially set as 1KW - already_issurance
            let staking_reward_amount: BalanceOf<T> = if already_insurance < max_10_million {
                max_10_million - already_insurance
            } else {
                Zero::zero()
            };
            staking_reward_amount
        }
    }

    impl<T: crate::Config> OnRuntimeUpgrade for ReInitStakingRewardOfOldNfts<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();

            if version != 3 {
                return 0;
            }

            info!("begin update metadata");

            for (asset_id, meta) in <old::Metadata<T>>::iter() {
                let is_asset_before_batch_mint = if asset_id < 10023u32.into() {
                    true
                } else {
                    false
                };

                let staking_reward_amount = Self::cal_remain_tokens(asset_id);

                let enable_staking =
                    is_asset_before_batch_mint && staking_reward_amount > Zero::zero();
                if enable_staking {
                    info!("enable staking for asset_id {:?}, because it mint before batch mint and remain_tokens G.T. zero.", asset_id);
                }

                let new_swap = types::Swap {
                    created: meta.created,
                    liquidity: meta.liquidity,
                    enable_staking,
                };
                <crate::Metadata<T>>::insert(asset_id, new_swap);
            }

            info!("begin start staking");

            for (asset_id, meta) in <crate::Metadata<T>>::iter() {
                if meta.enable_staking {
                    let staking_reward_amount = Self::cal_remain_tokens(asset_id);
                    assert!(
                        staking_reward_amount != Zero::zero(),
                        "staking_reward_amount should not be zero!"
                    );
                    info!(
                        "begin staking for asset_id {:?} with reward_amount {:?}",
                        asset_id, staking_reward_amount
                    );
                    T::Stakes::start(asset_id, staking_reward_amount).unwrap();
                }
            }
            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(4));

            1
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert!(storage_version == 3, "current storage version should be 3");
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert!(storage_version == 4, "current storage version should be 4");

            let mut staking_count = 0;
            for (_asset_id, meta) in <crate::Metadata<T>>::iter() {
                if meta.enable_staking {
                    staking_count += 1;
                }
            }

            assert_eq!(staking_count, 3);
            Ok(())
        }
    }
}
