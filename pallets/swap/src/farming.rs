use crate::{
    Account, AssetOf, BalanceOf, Config, Error, HeightOf, Liquidity, LiquidityOf, Metadata, Pallet,
};

use frame_support::traits::tokens::fungibles::Inspect;
use sp_core::U512;
use sp_runtime::{traits::Zero, DispatchError};

pub trait FarmingCurve<T: Config> {
    /// Calculate the farming value for a given block height
    ///
    /// # Arguments
    ///
    /// * `created_height` - The block number at which the swap was created
    /// * `staked_height` - The block number at which the liquidity was staked
    /// * `current_height` - the block number of current block
    /// * `total_supply` - the tokens issued
    fn calculate_farming_reward(
        created_height: HeightOf<T>,
        staked_height: HeightOf<T>,
        current_height: HeightOf<T>,
        total_supply: BalanceOf<T>,
    ) -> BalanceOf<T>;
}

impl<T: Config> FarmingCurve<T> for () {
    fn calculate_farming_reward(
        _created_height: HeightOf<T>,
        _staked_height: HeightOf<T>,
        _current_height: HeightOf<T>,
        _total_supply: BalanceOf<T>,
    ) -> BalanceOf<T> {
        Zero::zero()
    }
}

impl<T: Config> Pallet<T> {
    pub fn calculate_reward(
        lp_token_id: AssetOf<T>,
    ) -> Result<(LiquidityOf<T>, BalanceOf<T>), DispatchError> {
        let liquidity = <Liquidity<T>>::get(lp_token_id).ok_or(Error::<T>::NotExists)?;

        let meta = <Metadata<T>>::get(liquidity.token_id).ok_or(Error::<T>::NotExists)?;

        let height = <frame_system::Pallet<T>>::block_number();
        let supply = T::Assets::total_issuance(liquidity.token_id);

        let reward = T::FarmingCurve::calculate_farming_reward(
            meta.created,
            liquidity.minted,
            height,
            supply,
        );

        let reward: U512 = Self::try_into(reward)?;
        let numerator: U512 = Self::try_into(liquidity.amount)?;
        let denominator: U512 = Self::try_into(meta.liquidity)?;

        let reward = reward * numerator / denominator;

        let reward: BalanceOf<T> = Self::try_into(reward)?;

        let minted = <Account<T>>::get(&liquidity.owner, lp_token_id).unwrap_or_default();

        Ok((liquidity, reward - minted))
    }
}
