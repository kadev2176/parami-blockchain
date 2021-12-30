use super::*;

#[allow(unused)]
use crate::Pallet as Did;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Saturating;

benchmarks! {
    register {
        let caller: T::AccountId = whitelisted_caller();

        let referer: T::AccountId = account("referer", 1, 1);

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        T::Currency::make_free_balance_be(&referer, pot);

        Did::<T>::register(RawOrigin::Signed(referer.clone()).into(), None)?;

        let referer = <DidOf<T>>::get(&referer);
    }: _(RawOrigin::Signed(caller), referer)
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_ne!(<DidOf<T>>::get(&caller), None);
    }

    transfer {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let receiver: T::AccountId = account("receiver", 1, 1);
    }: _(RawOrigin::Signed(caller), receiver)
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_eq!(<DidOf<T>>::get(&caller), None);

        let receiver: T::AccountId = account("receiver", 1, 1);
        assert_ne!(<DidOf<T>>::get(&receiver), None);
    }

    revoke {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller))
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_eq!(<DidOf<T>>::get(&caller), None);
    }

    set_metadata {
        let k in 0 .. 100;
        let v in 0 .. 1000;

        let key = vec![0u8; k as usize];
        let value = vec![0u8; v as usize];

        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller), key.clone(), value.clone())
    verify {
        // TODO: verify metadata
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
