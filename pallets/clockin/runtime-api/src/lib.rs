#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use parami_primitives::{Balance, BalanceWrapper};
use sp_runtime::DispatchError;

pub type ApiResult<T> = Result<T, DispatchError>;

sp_api::decl_runtime_apis! {
    pub trait ClockInRuntimeApi<NftId, Did>
    where
        NftId: Codec,
        Did:Codec
    {
        /// ClockIn Info, returns (is_clock_in_enabled, user claimable, reward token)
        fn get_clock_in_info(nft_id: NftId, did: Did) -> ApiResult<(bool, bool, BalanceWrapper<Balance>)>;
    }
}
