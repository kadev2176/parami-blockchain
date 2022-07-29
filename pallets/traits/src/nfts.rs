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
}
