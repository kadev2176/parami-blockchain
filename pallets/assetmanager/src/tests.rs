use crate::mock::*;
use crate::*;

#[test]
fn should_get_next_asset_id() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let initial_asset_id = NextAssetId::<MockRuntime>::get();
            let next_asset_id = Pallet::<MockRuntime>::next_id().unwrap();

            let current_asset_id = NextAssetId::<MockRuntime>::get();

            assert_eq!(initial_asset_id, next_asset_id);
            assert_eq!(initial_asset_id + 1, current_asset_id);
        });
}
