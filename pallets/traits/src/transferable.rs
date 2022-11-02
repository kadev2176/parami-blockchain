use impl_trait_for_tuples::impl_for_tuples;
use sp_runtime::DispatchResult;

pub trait Transferable<AccountId> {
    fn transfer_all(src: &AccountId, dest: &AccountId) -> DispatchResult;
}

#[impl_for_tuples(10)]
impl<AccountId> Transferable<AccountId> for Tuple {
    fn transfer_all(src: &AccountId, dest: &AccountId) -> DispatchResult {
        for_tuples!(# (Tuple::transfer_all(src, dest)?; )* );
        Ok(())
    }
}
