#[macro_export]
macro_rules! s {
	($e: expr) => {
        sp_runtime::SaturatedConversion::saturated_into($e)
	}
}

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub mod test_helper {
    use crate::*;

    pub fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
        frame_system::Pallet::<T>::assert_last_event(generic_event.into());
    }

    #[macro_export]
    macro_rules! d {
        ($who: expr) => {
            parami_did::Pallet::<Runtime>::lookup_account($who.clone()).unwrap()
        }
    }
}
