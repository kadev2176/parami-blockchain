use codec::MaxEncodedLen;
use frame_support::{traits::tokens::Balance, Parameter};
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};

pub trait Accounts {
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Ord + Default + MaxEncodedLen;
    type Balance: Balance + MaybeSerializeDeserialize + MaxEncodedLen;

    fn fee_account(account: &Self::AccountId) -> Self::AccountId;

    fn fee_account_balance(account: &Self::AccountId) -> Self::Balance;
}
