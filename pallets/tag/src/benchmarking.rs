use super::*;

#[allow(unused)]
use crate::Pallet as Tag;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use parami_advertiser::Pallet as Advertiser;
use parami_did::Pallet as Did;
use sp_runtime::traits::{Bounded, Saturating};
use sp_std::vec;

benchmarks! {
    where_clause {
        where
        T: parami_advertiser::Config,
        T: parami_did::Config
    }

    create {
        let n in 0 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as crate::Config>::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        let name = vec![0u8; n as usize];
    }: _(RawOrigin::Signed(caller), name.clone())
    verify {
        assert_ne!(<Metadata<T>>::get(&name), None);
    }

    force_create {
        let n in 0 .. 1000;

        let name = vec![0u8; n as usize];
    }: _(RawOrigin::Root, name.clone())
    verify {
        assert_ne!(<Metadata<T>>::get(&name), None);
    }
}

impl_benchmark_test_suite!(Tag, crate::mock::new_test_ext(), crate::mock::Test);
