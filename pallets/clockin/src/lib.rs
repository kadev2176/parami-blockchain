#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod migrations;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{Inspect, Transfer as FungTransfer},
        Currency, EnsureOrigin, Get, StorageVersion,
    },
    PalletId,
};
use frame_system::offchain::SendTransactionTypes;
use sp_runtime::{traits::AccountIdConversion, RuntimeDebug};
use sp_std::{convert::TryInto, prelude::*};

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type NftOf<T> = <T as parami_nft::Config>::AssetId;
type LotteryMetaOf<T> = types::LotteryMetadata<
    HeightOf<T>,
    AccountOf<T>,
    BalanceOf<T>,
    <T as parami_nft::Config>::AssetId,
>;
const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + parami_did::Config
        + parami_nft::Config
        + SendTransactionTypes<Call<Self>>
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type ClockInBucketSize: Get<HeightOf<Self>>;
    }

    /// Lottery Metadata
    #[pallet::storage]
    pub(super) type LotteryMetadataStore<T: Config> =
        StorageMap<_, Twox64Concat, NftOf<T>, LotteryMetaOf<T>>;

    #[pallet::storage]
    pub(super) type LastClockIn<T: Config> =
        StorageDoubleMap<_, Twox64Concat, NftOf<T>, Twox64Concat, DidOf<T>, u32, ValueQuery>;

    #[pallet::storage]
    pub(super) type BucketClaimedSharesStore<T: Config> =
        StorageDoubleMap<_, Twox64Concat, NftOf<T>, Twox64Concat, u32, u32, ValueQuery>;

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ClockInEnabled(NftOf<T>),
        ClockInDisabled(NftOf<T>),
        ClockIn(NftOf<T>, DidOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        NftNotExists,
        NotNftOwner,
        NftNotMinted,
        InsufficientToken,
        ClockInNotExists,
        ClockedIn,
        NumberConversionError,
        MetaParamInvalid,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        ///
        #[pallet::weight(0)]
        pub fn enable_clock_in(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            level_probability: Vec<u32>,
            level_upper_bounds: Vec<BalanceOf<T>>,
            shares_per_bucket: u32,
            award_per_share: BalanceOf<T>,
            total_reward_token: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let nft_meta = parami_nft::Pallet::<T>::meta(nft_id).ok_or(Error::<T>::NftNotExists)?;
            ensure!(nft_meta.owner == did, Error::<T>::NotNftOwner);
            ensure!(nft_meta.minted, Error::<T>::NftNotMinted);
            ensure!(
                T::Assets::balance(nft_meta.token_asset_id, &who) >= total_reward_token,
                Error::<T>::InsufficientToken
            );
            //validate level_probability and level_endpoints
            ensure!(!level_probability.is_empty(), Error::<T>::MetaParamInvalid);

            let mut level_probability = level_probability;
            let mut level_upper_bounds = level_upper_bounds;

            level_probability.sort();
            level_upper_bounds.sort();

            ensure!(
                level_upper_bounds.len() == level_probability.len(),
                Error::<T>::MetaParamInvalid
            );

            let pot = Self::generate_reward_pot(&nft_id);
            T::Assets::transfer(
                nft_meta.token_asset_id,
                &who,
                &pot,
                total_reward_token,
                true,
            )?;

            let bucket_size = T::ClockInBucketSize::get();

            let start_at = <frame_system::Pallet<T>>::block_number();
            <LotteryMetadataStore<T>>::insert(
                nft_id,
                LotteryMetaOf::<T> {
                    level_probability,
                    level_upper_bounds: level_upper_bounds,
                    shares_per_bucket,
                    award_per_share,
                    start_at,
                    pot,
                    bucket_size,
                    asset_id: nft_meta.token_asset_id,
                },
            );

            Self::deposit_event(Event::<T>::ClockInEnabled(nft_id));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn add_token_reward(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            reward_token: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let nft_meta = parami_nft::Pallet::<T>::meta(nft_id).ok_or(Error::<T>::NftNotExists)?;
            ensure!(nft_meta.owner == did, Error::<T>::NotNftOwner);
            ensure!(nft_meta.minted, Error::<T>::NftNotMinted);
            let meta =
                LotteryMetadataStore::<T>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            T::Assets::transfer(nft_meta.token_asset_id, &who, &meta.pot, reward_token, true)?;

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn update_clock_in(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            level_probability: Vec<u32>,
            level_upper_bounds: Vec<BalanceOf<T>>,
            shares_per_bucket: u32,
            award_per_share: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, _who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let nft_meta = parami_nft::Pallet::<T>::meta(nft_id).ok_or(Error::<T>::NftNotExists)?;
            ensure!(nft_meta.owner == did, Error::<T>::NotNftOwner);
            ensure!(nft_meta.minted, Error::<T>::NftNotMinted);
            ensure!(
                level_upper_bounds.len() == level_probability.len(),
                Error::<T>::MetaParamInvalid
            );
            let meta =
                <LotteryMetadataStore<T>>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            <LotteryMetadataStore<T>>::insert(
                nft_id,
                LotteryMetaOf::<T> {
                    level_probability,
                    level_upper_bounds,
                    shares_per_bucket,
                    award_per_share,
                    ..meta
                },
            );

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn disable_clock_in(origin: OriginFor<T>, nft_id: NftOf<T>) -> DispatchResult {
            let (did, who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let nft_meta = parami_nft::Pallet::<T>::meta(nft_id).ok_or(Error::<T>::NftNotExists)?;
            ensure!(nft_meta.owner == did, Error::<T>::NotNftOwner);
            let metadata =
                <LotteryMetadataStore<T>>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            <LotteryMetadataStore<T>>::remove(nft_id);

            let balance = T::Assets::balance(nft_id, &metadata.pot);
            T::Assets::transfer(nft_meta.token_asset_id, &metadata.pot, &who, balance, false)?;

            Self::deposit_event(Event::<T>::ClockInDisabled(nft_id));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn clock_in(origin: OriginFor<T>, nft_id: NftOf<T>) -> DispatchResult {
            let (did, who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let meta =
                <LotteryMetadataStore<T>>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            let current_height = <frame_system::Pallet<T>>::block_number();
            let clocked_in_height = Self::clocked_in_block_num(nft_id, &did, &meta);
            ensure!(current_height >= clocked_in_height, Error::<T>::ClockedIn);

            let free_balance = T::Assets::balance(meta.asset_id, &meta.pot);
            ensure!(free_balance > 0u32.into(), Error::<T>::InsufficientToken);

            let clocked_in_bucket = Self::clocked_in_bucket(current_height, &meta)?;

            let reward: BalanceOf<T> =
                Self::calculate_reward(nft_id, &did, &who, &meta, clocked_in_bucket);
            if reward > 0u32.into() {
                <BucketClaimedSharesStore<T>>::mutate(nft_id, clocked_in_bucket, |v| *v += 1);
            }

            let reward = reward.min(free_balance);

            T::Assets::transfer(meta.asset_id, &meta.pot, &who, reward, false)?;

            LastClockIn::<T>::insert(nft_id, did, clocked_in_bucket);

            Self::deposit_event(Event::<T>::ClockIn(nft_id, did));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn generate_reward_pot(nft_id: &NftOf<T>) -> AccountOf<T> {
            return <T as crate::Config>::PalletId::get().into_sub_account_truncating(&nft_id);
        }

        fn clocked_in_block_num(
            nft_id: NftOf<T>,
            did: &DidOf<T>,
            meta: &LotteryMetaOf<T>,
        ) -> HeightOf<T> {
            let last_clock_in_bucket: HeightOf<T> = LastClockIn::<T>::get(nft_id, did).into();
            meta.start_at + last_clock_in_bucket * meta.bucket_size
        }

        fn clocked_in_bucket(
            current_block: HeightOf<T>,
            meta: &LotteryMetaOf<T>,
        ) -> Result<u32, DispatchError> {
            let clock_in_bucket = (current_block - meta.start_at) / meta.bucket_size;
            let clock_in_bucket: u32 = clock_in_bucket
                .try_into()
                .map_err(|_| Error::<T>::NumberConversionError)?;
            Ok(clock_in_bucket + 1)
        }

        fn calculate_reward(
            nft_id: NftOf<T>,
            _did: &DidOf<T>,
            account: &AccountOf<T>,
            meta: &LotteryMetaOf<T>,
            current_bucket: u32,
        ) -> BalanceOf<T> {
            let parent_hash = <frame_system::Pallet<T>>::parent_hash();
            let parent_hash: &[u8] = parent_hash.as_ref();
            let random_number = parent_hash[parent_hash.len() - 1];

            // guard shares_per_bucket invariant.
            let claimed_shares_in_cur_bucket =
                <BucketClaimedSharesStore<T>>::get(nft_id, current_bucket);
            if claimed_shares_in_cur_bucket >= meta.shares_per_bucket {
                return 0u32.into();
            }

            let user_level = Self::get_user_level_in_lottery(nft_id, &meta, account);

            let hit = u32::from(random_number % 100)
                < meta.level_probability.get(user_level).unwrap().clone();

            return if hit {
                meta.award_per_share
            } else {
                Zero::zero()
            };
        }

        pub fn get_clock_in_info(
            nft_id: NftOf<T>,
            did: &DidOf<T>,
        ) -> Result<(bool, bool, BalanceOf<T>), DispatchError> {
            let meta = <LotteryMetadataStore<T>>::get(nft_id);
            if let Some(meta) = meta {
                let balance = T::Assets::balance(meta.asset_id, &meta.pot);
                if balance == 0u32.into() {
                    return Ok((false, false, 0u32.into()));
                }

                let current_height = <frame_system::Pallet<T>>::block_number();
                let user_claimble =
                    current_height > Self::clocked_in_block_num(nft_id, &did, &meta);
                return Ok((true, user_claimble, 0u32.into()));
            } else {
                return Ok((false, false, 0u32.into()));
            };
        }

        pub fn get_user_level_in_lottery(
            nft_id: NftOf<T>,
            meta: &LotteryMetaOf<T>,
            account: &AccountOf<T>,
        ) -> usize {
            let user_balance = T::Assets::balance(nft_id, account);
            let user_level: usize = {
                let mut res = meta.level_upper_bounds.len() - 1;
                for i in 0..meta.level_upper_bounds.len() {
                    if meta.level_upper_bounds.get(i).unwrap() >= &user_balance {
                        res = i;
                        break;
                    }
                }
                res
            };
            user_level.into()
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        _marker: sp_std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                _marker: sp_std::marker::PhantomData::<T>,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {}
    }
}
