use crate::{Config, Pallet};
use crate::{Date, StorageVersion};
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::{traits::PalletInfoAccess, weights::Weight};
use sp_runtime::traits::Saturating;

pub fn migrate<T: Config>() -> Weight {
    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 2 {
        weight.saturating_accrue(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
    }

    weight
}

mod v2 {
    use super::*;

    use frame_support::storage::{
        migration::{move_prefix, remove_storage_prefix},
        storage_prefix,
    };

    pub fn migrate<T: Config>() -> Weight {
        let module = <Pallet<T>>::name().as_bytes();
        remove_storage_prefix(module, b"Account", b"");

        move_prefix(
            &storage_prefix(module, b"NftMetaStore"),
            &storage_prefix(module, b"Metadata"),
        );

        move_prefix(
            &storage_prefix(module, b"NextNftId"),
            &storage_prefix(module, b"NextClassId"),
        );

        Weight::max_value()
    }
}

pub mod v3 {
    use crate::HeightOf;

    use super::*;

    pub struct ResetHeight<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> OnRuntimeUpgrade for ResetHeight<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 2 {
                return 0;
            }

            Date::<T>::translate_values(|_d: HeightOf<T>| Some(0u32.into()));

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(3));

            1
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;
            let count: u32 = Date::<T>::iter_values()
                .filter(|m| *m != 0u32.into())
                .map(|_| 1u32)
                .sum();
            info!("non zero date count = {:?}", count);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;
            let count: u32 = Date::<T>::iter_values()
                .filter(|m| *m == 0u32.into())
                .map(|_| 1u32)
                .sum();
            info!("zero date count = {:?}", count);

            Ok(())
        }
    }
}
