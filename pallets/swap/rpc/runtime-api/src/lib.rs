#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use parami_primitives::BalanceWrapper;
use sp_runtime::{
    traits::{MaybeDisplay, MaybeFromStr},
    DispatchError,
};

type ApiResult<T> = Result<T, DispatchError>;

sp_api::decl_runtime_apis! {
    pub trait SwapRuntimeApi<AssetId, Balance>
    where
        AssetId: Codec,
        Balance: Codec + MaybeDisplay + MaybeFromStr,
    {
        /// Get dry-run result of add_liquidity
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be involved in the swap
        /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
        ///
        /// # Results
        ///
        /// tuple of (token_id, tokens, lp_token_id, liquidity)
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be involved in the swap
        /// * `lp_token_id` - The Asset ID of the liquidity provider token
        /// * `liquidity` - The amount of liquidity to be minted
        fn dryly_add_liquidity(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
            max_tokens: BalanceWrapper<Balance>,
        ) -> ApiResult<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )>;

        /// Get dry-run result of remove_liquidity
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `liquidity` - The amount of liquidity to be removed
        ///
        /// # Results
        ///
        /// tuple of (token_id, tokens, lp_token_id, currency)
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be returned
        /// * `lp_token_id` - The Asset ID of the liquidity provider token
        /// * `currency` - The currency to be returned
        fn dryly_remove_liquidity(
            token_id: AssetId,
            liquidity: BalanceWrapper<Balance>,
        ) -> ApiResult<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )>;

        /// Get dry-run result of buy_tokens
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be bought
        ///
        /// # Results
        ///
        /// The currency needed
        fn dryly_buy_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;

        /// Get dry-run result of sell_tokens
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be sold
        ///
        /// # Results
        ///
        /// The currency to be gained
        fn dryly_sell_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;

        /// Get dry-run result of sell_currency
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be sold
        ///
        /// # Results
        ///
        /// The amount of tokens to be gained
        fn dryly_sell_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;

        /// Get dry-run result of buy_currency
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be bought
        ///
        /// # Results
        ///
        /// The amount of tokens needed
        fn dryly_buy_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;
    }
}
