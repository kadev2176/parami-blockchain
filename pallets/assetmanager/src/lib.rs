#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::traits::tokens::AssetId;
use pallet::NextAssetId;
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_runtime::traits::CheckedAdd;

pub use pallet::*;
type AssetOf<T> = <T as pallet::Config>::AssetId;

#[frame_support::pallet]
pub mod pallet {
    use crate::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AssetId: AtLeast32BitUnsigned
            + AssetId
            + Default
            + MaxEncodedLen
            + MaybeSerializeDeserialize;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub(super) type NextAssetId<T: Config> = StorageValue<_, AssetOf<T>, ValueQuery, GetDefault>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub next_asset_id: AssetOf<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                next_asset_id: 1u32.into(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NextAssetId<T>>::put(self.next_asset_id);
        }
    }
}

pub trait AssetIdManager<T: pallet::Config> {
    fn next_id() -> Result<AssetOf<T>, Error>;
}

pub enum Error {
    Overflow,
}

impl<T: pallet::Config> AssetIdManager<T> for pallet::Pallet<T> {
    fn next_id() -> Result<AssetOf<T>, Error> {
        let id = <NextAssetId<T>>::try_mutate(|id| {
            let current_id = *id;
            *id = id.checked_add(&1u32.into()).ok_or(Error::Overflow)?;
            Ok(current_id)
        })?;

        Ok(id)
    }
}
