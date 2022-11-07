use sp_runtime::DispatchError;

pub trait Stakes<AccountId> {
    type AssetId;
    type Balance;

    fn start(
        asset_id: Self::AssetId,
        reward_total_amount: Self::Balance,
    ) -> Result<(), DispatchError>;

    fn make_profit(asset_id: Self::AssetId) -> Result<(), DispatchError>;

    fn stake(
        asset_id: Self::AssetId,
        account: &AccountId,
        amount: Self::Balance,
    ) -> Result<(), sp_runtime::DispatchError>;

    fn withdraw(
        asset_id: Self::AssetId,
        account: &AccountId,
        amount: Self::Balance,
    ) -> Result<(), sp_runtime::DispatchError>;

    fn exit(asset_id: Self::AssetId, account: &AccountId) -> Result<(), DispatchError>;

    fn earned(asset_id: Self::AssetId, account: &AccountId)
        -> Result<Self::Balance, DispatchError>;

    fn get_reward(
        asset_id: Self::AssetId,
        account: &AccountId,
    ) -> Result<Self::Balance, DispatchError>;
}

impl<AccountId> Stakes<AccountId> for ()
where
    AccountId: TryFrom<&'static [u8]>,
{
    type AssetId = u32;
    type Balance = u128;

    fn start(
        _asset_id: Self::AssetId,
        _reward_total_amount: Self::Balance,
    ) -> Result<(), DispatchError> {
        Ok(())
    }

    fn make_profit(_asset_id: Self::AssetId) -> Result<(), DispatchError> {
        Ok(())
    }

    fn stake(
        _asset_id: Self::AssetId,
        _account: &AccountId,
        _amount: Self::Balance,
    ) -> Result<(), sp_runtime::DispatchError> {
        Ok(())
    }

    fn earned(
        _asset_id: Self::AssetId,
        _account: &AccountId,
    ) -> Result<Self::Balance, DispatchError> {
        Ok(0u32.into())
    }

    fn get_reward(
        _asset_id: Self::AssetId,
        _account: &AccountId,
    ) -> Result<Self::Balance, DispatchError> {
        Ok(0u32.into())
    }

    fn withdraw(
        _asset_id: Self::AssetId,
        _account: &AccountId,
        _amount: Self::Balance,
    ) -> Result<(), sp_runtime::DispatchError> {
        Ok(())
    }

    fn exit(_asset_id: Self::AssetId, _account: &AccountId) -> Result<(), DispatchError> {
        Ok(())
    }
}
