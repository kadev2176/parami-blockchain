#[macro_export]
macro_rules! s {
	($e: expr) => {
        sp_runtime::SaturatedConversion::saturated_into($e)
	}
}

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub mod test_helper {
    use crate::*;

    #[macro_export]
    macro_rules! d {
        ($who: expr) => {
            parami_did::Pallet::<Runtime>::lookup_account($who.clone()).unwrap()
        }
    }

    pub fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
        frame_system::Pallet::<T>::assert_last_event(generic_event.into());
    }

    pub fn signer<T: Config>(who: T::AccountId) -> sp_runtime::MultiSigner
        where T: frame_system::Config<AccountId = sp_runtime::AccountId32>,
    {
        sp_runtime::MultiSigner::from(
            sp_core::sr25519::Public(
                std::convert::TryInto::<[u8; 32]>::try_into(
                    who.as_ref()
                ).unwrap()))
    }

    pub fn reserved_balance<T: Config>(who: &T::AccountId) -> BalanceOf<T> {
        <T as Config>::Currency::reserved_balance(who)
    }

    pub fn free_balance<T: Config>(who: &T::AccountId) -> BalanceOf<T> {
        <T as Config>::Currency::free_balance(who)
    }

}
