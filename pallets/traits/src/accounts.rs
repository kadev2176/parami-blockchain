pub trait Accounts<AccountId> {
    fn fee_account(account: &AccountId) -> AccountId;
}

impl<AccountId: Clone> Accounts<AccountId> for () {
    fn fee_account(account: &AccountId) -> AccountId {
        account.clone()
    }
}
