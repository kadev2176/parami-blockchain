use sp_std::convert::TryFrom;
use sp_runtime::{DispatchError};
use parami_primitives::Signature;

#[macro_export]
macro_rules! s {
	($e: expr) => {
        sp_runtime::SaturatedConversion::saturated_into($e)
	}
}

pub fn sr25519_signature(sign: &[u8]) -> Result<Signature, DispatchError> {
    if let Ok(signature) = sp_core::sr25519::Signature::try_from(sign) {
        Ok(signature.into())
    } else {
        Err(DispatchError::Other("Not a sr25519 signature"))
    }
}

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub mod test_helper {
    use crate::*;
    use std::iter::FromIterator;
    use sp_std::vec::Vec;
    use sp_core::sr25519::Pair as SrPair;
    use sp_core::Pair;

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

    pub fn sign<Runtime: Config>(
        signer_pair: SrPair, user: Runtime::AccountId,
        media: Runtime::AccountId, advertiser_id: AdvertiserId, ad_id: AdId,
    ) -> (Vec<u8>, Vec<u8>) {
        let user_did = d!(user);
        let media_did = d!(media);
        let now = crate::Pallet::<Runtime>::now();
        let data = codec::Encode::encode(&(user_did, media_did, advertiser_id, now, ad_id));
        let data_sign = Vec::from_iter(signer_pair.sign(data.as_slice()).0);
        (data, data_sign)
    }
}
