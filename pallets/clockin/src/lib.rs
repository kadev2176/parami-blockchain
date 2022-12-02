#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{Inspect, Transfer as FungTransfer},
        Currency, EnsureOrigin, Get, StorageVersion,
    },
    Blake2_256, PalletId, StorageHasher,
};
use frame_system::offchain::SendTransactionTypes;
use sp_runtime::{
    traits::{AccountIdConversion, Hash, Saturating},
    RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*};

use parami_traits::{Tag, Tags};

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type NftOf<T> = <T as parami_nft::Config>::AssetId;
type MetaOf<T> =
    types::Metadata<HeightOf<T>, AccountOf<T>, BalanceOf<T>, <T as parami_nft::Config>::AssetId>;
type TagHash = <Blake2_256 as StorageHasher>::Output;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + parami_did::Config
        + parami_nft::Config
        + SendTransactionTypes<Call<Self>>
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The means of storing the tags and tags of advertisement
        type Tags: Tags<TagHash, HashOf<Self>, DidOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type ClockInBucketSize: Get<HeightOf<Self>>;
    }

    /// Metadata
    #[pallet::storage]
    pub(super) type Metadata<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, MetaOf<T>>;

    #[pallet::storage]
    pub(super) type LastClockIn<T: Config> =
        StorageDoubleMap<_, Twox64Concat, NftOf<T>, Twox64Concat, DidOf<T>, u32, ValueQuery>;

    #[pallet::storage]
    pub(super) type TagsOf<T: Config> =
        StorageDoubleMap<_, Twox64Concat, NftOf<T>, Blake2_256, Vec<u8>, bool>;

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
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn enable_clock_in(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            payout_base: BalanceOf<T>,
            payout_min: BalanceOf<T>,
            payout_max: BalanceOf<T>,
            metadata: Vec<u8>,
            tags: Vec<Tag>,
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

            for tag in tags {
                TagsOf::<T>::insert(nft_id, tag, true);
            }

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
            Metadata::<T>::insert(
                nft_id,
                MetaOf::<T> {
                    payout_base,
                    payout_min,
                    payout_max,
                    metadata,
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
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            T::Assets::transfer(nft_meta.token_asset_id, &who, &meta.pot, reward_token, true)?;

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn update_clock_in(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            payout_base: BalanceOf<T>,
            payout_min: BalanceOf<T>,
            payout_max: BalanceOf<T>,
            metadata: Vec<u8>,
            tags: Vec<Tag>,
        ) -> DispatchResult {
            let (did, _who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let nft_meta = parami_nft::Pallet::<T>::meta(nft_id).ok_or(Error::<T>::NftNotExists)?;
            ensure!(nft_meta.owner == did, Error::<T>::NotNftOwner);
            ensure!(nft_meta.minted, Error::<T>::NftNotMinted);
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            TagsOf::<T>::remove_prefix(nft_id, None);

            for tag in tags {
                TagsOf::<T>::insert(nft_id, tag, true);
            }

            Metadata::<T>::insert(
                nft_id,
                MetaOf::<T> {
                    payout_base,
                    payout_min,
                    payout_max,
                    metadata,
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
            let metadata = Metadata::<T>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            Metadata::<T>::remove(nft_id);
            TagsOf::<T>::remove_prefix(nft_id, None);

            let balance = T::Assets::balance(nft_id, &metadata.pot);
            T::Assets::transfer(nft_meta.token_asset_id, &metadata.pot, &who, balance, false)?;

            Self::deposit_event(Event::<T>::ClockInDisabled(nft_id));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn clock_in(origin: OriginFor<T>, nft_id: NftOf<T>) -> DispatchResult {
            let (did, who) = parami_did::EnsureDid::<T>::ensure_origin(origin)?;
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::ClockInNotExists)?;

            let current_height = <frame_system::Pallet<T>>::block_number();
            let clocked_in_height = Self::clocked_in_block_num(nft_id, &did, &meta);
            ensure!(current_height >= clocked_in_height, Error::<T>::ClockedIn);

            let reward: BalanceOf<T> = Self::calculate_reward(nft_id, &did, &meta);
            let free_balance = T::Assets::balance(meta.asset_id, &meta.pot);
            ensure!(free_balance > 0u32.into(), Error::<T>::InsufficientToken);

            let reward = reward.min(free_balance);
            T::Assets::transfer(meta.asset_id, &meta.pot, &who, reward, false)?;
            let clocked_in_bucket = Self::clocked_in_bucket(current_height, &meta)?;
            LastClockIn::<T>::insert(nft_id, did, clocked_in_bucket);

            Self::deposit_event(Event::<T>::ClockIn(nft_id, did));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn generate_reward_pot(nft_id: &NftOf<T>) -> AccountOf<T> {
            return <T as crate::Config>::PalletId::get().into_sub_account_truncating(&nft_id);
        }

        fn clocked_in_block_num(nft_id: NftOf<T>, did: &DidOf<T>, meta: &MetaOf<T>) -> HeightOf<T> {
            let last_clock_in_bucket: HeightOf<T> = LastClockIn::<T>::get(nft_id, did).into();

            meta.start_at + last_clock_in_bucket * meta.bucket_size
        }

        fn clocked_in_bucket(
            current_block: HeightOf<T>,
            meta: &MetaOf<T>,
        ) -> Result<u32, DispatchError> {
            let clock_in_bucket = (current_block - meta.start_at) / meta.bucket_size;
            let clock_in_bucket: u32 = clock_in_bucket
                .try_into()
                .map_err(|_| Error::<T>::NumberConversionError)?;
            Ok(clock_in_bucket + 1)
        }

        fn calculate_reward(nft_id: NftOf<T>, did: &DidOf<T>, meta: &MetaOf<T>) -> BalanceOf<T> {
            let mut scoring = 5i32;
            let tag_hashes = Self::tags_of(nft_id);
            let personas = T::Tags::personas_of(did);

            let length = tag_hashes.len();
            for (tag, score) in personas {
                let delta = if tag_hashes.contains(&tag) {
                    score.saturating_mul(10)
                } else {
                    score
                };
                scoring.saturating_accrue(delta);
            }

            scoring /= length.saturating_mul(10).saturating_add(1) as i32;

            if scoring < 0 {
                return 0u32.into();
            }

            let scoring = scoring as u32;

            meta.payout_base
                .saturating_mul(scoring.into())
                .min(meta.payout_max)
                .max(meta.payout_min)
        }

        fn tags_of(nft_id: NftOf<T>) -> Vec<TagHash> {
            let mut tag_hashes = vec![];
            let mut iter = TagsOf::<T>::iter_prefix_values(nft_id);
            while let Some(_value) = iter.next() {
                let prefix = iter.prefix();
                let raw = iter.last_raw_key();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&raw[prefix.len()..]);
                tag_hashes.push(hash);
            }

            tag_hashes
        }

        pub fn get_clock_in_info(
            nft_id: NftOf<T>,
            did: &DidOf<T>,
        ) -> Result<(bool, bool, BalanceOf<T>), DispatchError> {
            let meta = Metadata::<T>::get(nft_id);
            if let Some(meta) = meta {
                let balance = T::Assets::balance(meta.asset_id, &meta.pot);
                if balance == 0u32.into() {
                    return Ok((false, false, 0u32.into()));
                }

                let current_height = <frame_system::Pallet<T>>::block_number();
                let user_claimble =
                    current_height > Self::clocked_in_block_num(nft_id, &did, &meta);
                let reward = Self::calculate_reward(nft_id, &did, &meta).min(balance);
                return Ok((true, user_claimble, reward));
            } else {
                return Ok((false, false, 0u32.into()));
            };
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
