use crate::StorageVersion;
use crate::{Config, Metadata, Pallet};
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;

pub fn migrate<T: Config>() -> Weight {
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

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;
        <Metadata<T>>::translate(|_did, meta| {
            weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
            Some(meta)
        });

        weight
    }
}

pub mod v2 {
    use super::*;
    use crate::MetaOf;

    pub struct ResetHeight<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> OnRuntimeUpgrade for ResetHeight<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 1 {
                return 0;
            }

            Metadata::<T>::translate_values(|m| {
                Some(MetaOf::<T> {
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
            let meta_count: u32 = Metadata::<T>::iter_values()
                .filter(|m| m.created != 0u32.into())
                .map(|_| 1u32)
                .sum();
            info!("non zero meta count = {:?}", meta_count);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;
            let meta_count: u32 = Metadata::<T>::iter_values()
                .filter(|m| m.created == 0u32.into())
                .map(|_| 1u32)
                .sum();
            info!("zero meta count = {:?}", meta_count);

            Ok(())
        }
    }
}
