#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use parami_primitives::BalanceWrapper;
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};
use sp_runtime::DispatchError;

pub type ApiResult<T> = Result<T, DispatchError>;

sp_api::decl_runtime_apis! {
    pub trait AdRuntimeApi<AdvertisementId, NftId, Did, Balance>
    where
        AdvertisementId: Codec,
        NftId: Codec,
        Did: Codec,
        Balance: Codec + MaybeDisplay + MaybeFromStr,
    {
        // calculate Lp staking reward for given lp_token_id, result format is <token_amount>
        fn cal_reward(ad_id: AdvertisementId, nft_id: NftId, visitor: Did, referrer: Option<Did>) -> ApiResult<BalanceWrapper<Balance>>;
    }
}
