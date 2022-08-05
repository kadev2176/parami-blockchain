use frame_support::{pallet_prelude::PhantomData, weights::Weight};

pub trait WeightInfo {
    fn transfer_hash() -> Weight;

    fn transfer_native() -> Weight;

    fn transfer_token() -> Weight;

    fn transfer() -> Weight;

    fn remark() -> Weight;

    fn force_set_resource() -> Weight;

    fn update_native_fee() -> Weight;

    fn update_transfer_token_fee() -> Weight;

    fn create_xasset() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn transfer_hash() -> Weight {
        195_000_000 as Weight
    }

    fn transfer_native() -> Weight {
        195_000_000 as Weight
    }

    fn transfer() -> Weight {
        195_000_000 as Weight
    }

    fn transfer_token() -> Weight {
        195_000_000 as Weight
    }

    fn remark() -> Weight {
        195_000_000 as Weight
    }

    fn force_set_resource() -> Weight {
        195_000_000 as Weight
    }

    fn update_native_fee() -> Weight {
        195_000_000 as Weight
    }

    fn create_xasset() -> Weight {
        195_000_000 as Weight
    }

    fn update_transfer_token_fee() -> Weight {
        195_000_000 as Weight
    }
}
