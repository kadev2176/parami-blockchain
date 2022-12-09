pub mod v0 {
    use crate::Config;
    use crate::Pallet;
    use crate::StorageVersion;
    use frame_support::dispatch::Weight;
    use frame_support::migration::remove_storage_prefix;
    use frame_support::traits::{OnRuntimeUpgrade, PalletInfoAccess};

    pub struct RemoveRedundantStorage<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for RemoveRedundantStorage<T> {
        fn on_runtime_upgrade() -> Weight {
            remove_storage_prefix(<Pallet<T>>::name().as_bytes(), b"Metadata", b"");
            remove_storage_prefix(<Pallet<T>>::name().as_bytes(), b"TagsOf", b"");
            StorageVersion::new(1).put::<Pallet<T>>();
            3
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert_eq!(storage_version, 0, "current storage version should be 0");
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert_eq!(storage_version, 1, "current storage version should be 1");
            Ok(())
        }
    }
}
