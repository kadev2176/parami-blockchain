use crate::StorageVersion;
use crate::{Config, Pallet};
use crate::{DeadlineOf, EndtimeOf, HeightOf, Metadata, Payout, SlotOf};
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::weights::Weight;

pub fn migrate<T: Config>() -> Weight {
    let _version = StorageVersion::get::<Pallet<T>>();
    let weight: Weight = 0;

    weight
}

pub mod v4 {
    use crate::MetaOf;
    use crate::SlotMetaOf;

    use super::*;

    pub struct ResetHeight<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> OnRuntimeUpgrade for ResetHeight<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 3 {
                return 0;
            }

            Payout::<T>::translate_values(|_h: HeightOf<T>| Some(0u32.into()));
            DeadlineOf::<T>::translate_values(|_h: HeightOf<T>| Some(0u32.into()));
            EndtimeOf::<T>::translate_values(|_h: HeightOf<T>| Some(0u32.into()));
            Metadata::<T>::translate_values(|m| {
                Some(MetaOf::<T> {
                    created: 0u32.into(),
                    ..m
                })
            });
            SlotOf::<T>::translate_values(|s| {
                Some(SlotMetaOf::<T> {
                    created: 0u32.into(),
                    ..s
                })
            });

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(4));

            1
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;

            let count: u32 = Payout::<T>::iter_values()
                .filter(|m| *m != 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("non zero payout count = {:?}", count);

            let count: u32 = DeadlineOf::<T>::iter_values()
                .filter(|m| *m != 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("non zero deadline count = {:?}", count);
            let count: u32 = EndtimeOf::<T>::iter_values()
                .filter(|m| *m != 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("non zero endtime count = {:?}", count);
            let count: u32 = Metadata::<T>::iter_values()
                .filter(|m| m.created != 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("non zero meta count = {:?}", count);
            let count: u32 = SlotOf::<T>::iter_values()
                .filter(|m| m.created != 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("non zero slot count = {:?}", count);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use frame_support::log::info;

            let count: u32 = Payout::<T>::iter_values()
                .filter(|m| *m == 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("zero payout count = {:?}", count);

            let count: u32 = DeadlineOf::<T>::iter_values()
                .filter(|m| *m == 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("zero deadline count = {:?}", count);
            let count: u32 = EndtimeOf::<T>::iter_values()
                .filter(|m| *m == 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("zero endtime count = {:?}", count);
            let count: u32 = Metadata::<T>::iter_values()
                .filter(|m| m.created == 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("zero meta count = {:?}", count);
            let count: u32 = SlotOf::<T>::iter_values()
                .filter(|m| m.created == 0u32.into())
                .map(|_| 1u32)
                .sum();

            info!("zero slot count = {:?}", count);

            Ok(())
        }
    }
}
