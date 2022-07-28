pub mod v1 {
    use crate::{Config, Pallet, ResourceId2Asset, ResourceMap};
    use frame_support::traits::OnRuntimeUpgrade;
    use frame_support::traits::StorageVersion;
    use frame_support::weights::Weight;

    pub struct AddResouceId2Asset<T: Config>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for AddResouceId2Asset<T> {
        fn on_runtime_upgrade() -> Weight {
            let mut weight: Weight = 0;
            let version = StorageVersion::get::<Pallet<T>>();

            if version >= 1 {
                return weight;
            }

            for (asset_id, resource_id) in <ResourceMap<T>>::iter() {
                <ResourceId2Asset<T>>::insert(resource_id, asset_id);
                weight += 1;
            }

            weight
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let version = StorageVersion::get::<Pallet<T>>();
            if version >= 1 {
                return Ok(());
            }

            assert_eq!(
                <ResourceId2Asset<T>>::iter().count(),
                <ResourceMap<T>>::iter().count()
            );
            Ok(())
        }
    }
}
