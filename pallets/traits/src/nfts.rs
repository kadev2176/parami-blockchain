use sp_runtime::DispatchError;

pub trait Nfts<AccountId> {
    type DecentralizedId;
    type NftId;
    type Balance: From<u32>;

    // force transfer all assets of account src to account dest
    fn force_transfer_all_fractions(src: &AccountId, dest: &AccountId)
        -> Result<(), DispatchError>;

    fn get_claim_info(
        nft_id: Self::NftId,
        claimer: &Self::DecentralizedId,
    ) -> Result<(Self::Balance, Self::Balance, Self::Balance), DispatchError>;

    fn get_nft_pot(nft_id: Self::NftId) -> Option<AccountId>;
}

impl<AccountId> Nfts<AccountId> for ()
where
    AccountId: TryFrom<&'static [u8]>,
{
    type DecentralizedId = u32;
    type NftId = u32;
    type Balance = u128;

    fn force_transfer_all_fractions(
        _src: &AccountId,
        _dest: &AccountId,
    ) -> Result<(), DispatchError> {
        Ok(())
    }
    fn get_claim_info(
        _nft_id: Self::NftId,
        _claimer: &Self::DecentralizedId,
    ) -> Result<(Self::Balance, Self::Balance, Self::Balance), DispatchError> {
        Ok((0u32.into(), 0u32.into(), 0u32.into()))
    }

    fn get_nft_pot(_nft_id: Self::NftId) -> Option<AccountId> {
        //Alice
        let account_bytes = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".as_bytes();
        Some(AccountId::try_from(account_bytes).map_err(|_| "").unwrap())
    }
}
