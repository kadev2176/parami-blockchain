use super::*;

#[allow(unused)]
use crate::Pallet as Nft;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::fungibles::Inspect;
use frame_system::RawOrigin;
use parami_did::Pallet as Did;
use sp_runtime::traits::{Bounded, Saturating, Zero};

benchmarks! {
    back {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account::<T::AccountId>("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        T::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        let kol = Did::<T>::did_of(&kol).unwrap();
    }: _(RawOrigin::Signed(caller.clone()), kol, pot)
    verify {
        let meta = Did::<T>::meta(&kol).unwrap();
        assert_eq!(T::Currency::free_balance(&meta.pot), pot);
    }

    mint {
        let n in 1 .. 1000 - 4;
        let s in 1 .. 1000 - 4;

        let name = vec![b'x'; n as usize];
        let symbol = vec![b'x'; n as usize];

        let caller: T::AccountId = whitelisted_caller();

        let supporter: T::AccountId = account::<T::AccountId>("supporter", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&supporter, max);
        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(supporter.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let kol = Did::<T>::did_of(&caller).unwrap();

        Nft::<T>::back(RawOrigin::Signed(supporter).into(), kol, pot)?;
    }: _(RawOrigin::Signed(caller.clone()), name, symbol)
    verify {
        let meta = Did::<T>::meta(&kol).unwrap();
        assert_ne!(meta.nft, None);
    }

    claim {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account::<T::AccountId>("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        T::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        let did = Did::<T>::did_of(&kol).unwrap();

        Nft::<T>::back(RawOrigin::Signed(caller.clone()).into(), did, pot)?;

        Nft::<T>::mint(RawOrigin::Signed(kol).into(), b"Test Token".to_vec(), b"XTT".to_vec())?;
    }: _(RawOrigin::Signed(caller.clone()), did)
    verify {
        let meta = Did::<T>::meta(&did).unwrap();
        let token_id = meta.nft.unwrap();
        assert!(T::Assets::balance(token_id, &caller) > Zero::zero());
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
