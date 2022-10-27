use crate::Account;
use crate::Pallet;
use crate::StorageVersion;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::weights::Weight;

pub mod v1 {
    use frame_support::assert_ok;
    use parami_traits::Swaps;

    use crate::AccountOf;

    use super::*;

    pub struct ResetHeight<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> OnRuntimeUpgrade for ResetHeight<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 0 {
                return 0;
            }

            for (account, asset, _claimed_at) in Account::<T>::iter() {
                let result = <Pallet<T> as Swaps<AccountOf<T>>>::burn(
                    account,
                    asset,
                    0u32.into(),
                    0u32.into(),
                );
                assert_ok!(result);
            }

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(1));
            1
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;

            let count = Account::<T>::iter().count();
            info!("accounts: {:?}", count);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let count = Account::<T>::iter().count();
            assert_eq!(count, 0);

            Ok(())
        }
    }
}

pub mod v2 {

    use crate::{Metadata, SwapOf};

    use super::*;

    pub struct ResetHeight<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> OnRuntimeUpgrade for ResetHeight<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 1 {
                return 0;
            }

            Metadata::<T>::translate_values(|m| {
                Some(SwapOf::<T> {
                    created: 0u32.into(),
                    ..m
                })
            });

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(2));
            1
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;

            let count: u32 = Metadata::<T>::iter_values()
                .filter(|m| m.created != 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("non zero count: {:?}", count);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let count: u32 = Metadata::<T>::iter_values()
                .filter(|m| m.created != 0u32.into())
                .map(|_| 1u32)
                .sum();
            assert_eq!(count, 0);

            Ok(())
        }
    }
}

pub mod v3 {
    use parami_traits::Nfts;
    use parami_traits::Swaps;

    use crate::AccountOf;
    use crate::AssetOf;
    use crate::Liquidity;
    use crate::Metadata;
    use crate::Pallet as SwapPallet;
    use frame_support::log::info;
    use sp_std::vec;
    use sp_std::vec::Vec;

    use super::*;
    pub struct RemoveAllLiquidityExceptInitialLiquidity<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> RemoveAllLiquidityExceptInitialLiquidity<T> {
        fn initial_liquidity_provider(asset_id: AssetOf<T>) -> Option<AccountOf<T>> {
            T::Nfts::get_nft_pot(asset_id)
        }
    }

    impl<T: crate::Config> OnRuntimeUpgrade for RemoveAllLiquidityExceptInitialLiquidity<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 2 {
                return 0;
            }

            let mut lp_token_ids: Vec<AssetOf<T>> = vec![];
            for (lp_token_id, _liquidity) in <Liquidity<T>>::iter() {
                lp_token_ids.push(lp_token_id.clone());
            }

            let mut lp_token_burned = 0;

            for lp_token_id in lp_token_ids {
                let liquidity = <Liquidity<T>>::get(lp_token_id).unwrap();

                let asset_id = liquidity.token_id;
                let liquidity_owner = liquidity.owner.clone();

                let initial_liquidity_provider =
                    Self::initial_liquidity_provider(asset_id).unwrap();
                if liquidity_owner != initial_liquidity_provider {
                    // SwapPallet::<T>::acquire_reward_inner(
                    //     liquidity_owner.clone(),
                    //     lp_token_id.clone(),
                    // )
                    // .unwrap();

                    SwapPallet::<T>::burn(
                        liquidity_owner.clone(),
                        lp_token_id.clone(),
                        0u32.into(),
                        0u32.into(),
                    )
                    .unwrap();

                    lp_token_burned += 1;
                    info!("remove liquidity {:?}", liquidity);
                }
            }

            info!("lp_token_burned is {:}", lp_token_burned);
            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(3));
            1
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            assert_eq!(StorageVersion::get::<Pallet<T>>(), 2u16);
            let mut lp_token_count_to_burn = 0;
            for (lp_token_id, liquidity) in <Liquidity<T>>::iter() {
                info!("got liquidity {:?}", &liquidity);
                let liquidity_owner = liquidity.owner;
                let asset_id = liquidity.token_id;

                let initial_liquidity_provider = Self::initial_liquidity_provider(asset_id);

                if let None = initial_liquidity_provider {
                    info!(
                        "initial liquidity provider of lp_token_id {:?} not exists",
                        lp_token_id
                    )
                }

                let initial_liquidity_provider = initial_liquidity_provider
                    .expect("initial liquidity provider of asset_id not exists");

                if liquidity_owner != initial_liquidity_provider {
                    lp_token_count_to_burn += 1;
                }
            }

            info!("there are {:} lp_token to burn", lp_token_count_to_burn);
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let mut remain_liquidity_count = 0;
            for (_lp_token_id, liquidity) in <Liquidity<T>>::iter() {
                assert_eq!(
                    liquidity.owner,
                    Self::initial_liquidity_provider(liquidity.token_id).unwrap()
                );
                info!(
                    "post upgrade owner is {:?}, liquidity is {:?}",
                    liquidity.owner, liquidity
                );
                remain_liquidity_count += 1;
            }

            let swap_count = <Metadata<T>>::iter().count();

            info!("swap_count is {:}", swap_count);
            info!("remain_liquidity_count is {:}", remain_liquidity_count);
            assert_eq!(swap_count, remain_liquidity_count);

            Ok(())
        }
    }
}
