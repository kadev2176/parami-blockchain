pub mod v1 {
    use crate::{AssetOf, Config, NextAssetId, Weight};
    use frame_support::traits::OnRuntimeUpgrade;
    use sp_std::marker::PhantomData;

    pub struct SetInitialAssetId<T>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for SetInitialAssetId<T> {
        fn on_runtime_upgrade() -> Weight {
            // start with 10000 if asset id not set.
            if NextAssetId::<T>::get() == AssetOf::<T>::default() {
                let asset_id: AssetOf<T> = 10000u32.into();
                NextAssetId::<T>::put(asset_id);
            }
            1
        }
    }
}
