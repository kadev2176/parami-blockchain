use sp_std::marker::PhantomData;

pub trait WeightInfo {}

impl WeightInfo for () {}

pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {}
