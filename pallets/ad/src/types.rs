use crate::{
    AccountOf, AssetsOf, BalanceOf, Config, Currency, DispatchResult, FungInspect, FungTransfer,
};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::ExistenceRequirement;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Metadata<B, D, H, N> {
    pub id: H,
    pub creator: D,
    pub metadata: Vec<u8>,
    pub reward_rate: u16,
    pub created: N,
    pub payout_base: B,
    pub payout_min: B,
    pub payout_max: B,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Slot<Hash, Height, NftId, TokenId, AccountId, AdAsset> {
    pub ad_id: Hash,
    pub nft_id: NftId,
    pub ad_asset: AdAsset,
    pub fungible_id: Option<TokenId>,
    // budget pot is specifically for locking budget.
    pub budget_pot: AccountId,
    pub created: Height,
}

pub struct RewardInfo<Balance> {
    pub total: Balance,
    pub for_visitor: Balance,
    pub for_referrer: Balance,
    pub fungibles: Balance,
}

#[derive(Clone, Decode, Encode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum CurrencyOrAsset<AssetOf> {
    Currency,
    Asset(AssetOf),
}

pub trait AdAsset<T: Config> {
    fn reduciable_balance(&self, account: &AccountOf<T>) -> BalanceOf<T>;
    fn transfer(
        &self,
        from: &AccountOf<T>,
        to: &AccountOf<T>,
        amount: BalanceOf<T>,
        keep_alive: bool,
    ) -> DispatchResult;
}

impl<T: Config> AdAsset<T> for CurrencyOrAsset<AssetsOf<T>> {
    fn reduciable_balance(&self, account: &AccountOf<T>) -> BalanceOf<T> {
        match self {
            CurrencyOrAsset::Currency => T::Currency::free_balance(account),
            CurrencyOrAsset::Asset(asset_id) => {
                T::Assets::reducible_balance(*asset_id, account, false)
            }
        }
    }

    fn transfer(
        &self,
        from: &AccountOf<T>,
        to: &AccountOf<T>,
        amount: BalanceOf<T>,
        keep_alive: bool,
    ) -> DispatchResult {
        match self {
            CurrencyOrAsset::Currency => {
                let requirement = if keep_alive {
                    ExistenceRequirement::KeepAlive
                } else {
                    ExistenceRequirement::AllowDeath
                };
                T::Currency::transfer(from, to, amount, requirement)?;
            }
            CurrencyOrAsset::Asset(asset_id) => {
                T::Assets::transfer(*asset_id, from, to, amount, keep_alive)?;
            }
        }

        Ok(())
    }
}
