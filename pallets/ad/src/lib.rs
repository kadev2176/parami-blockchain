#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    transactional,
    traits::{Currency, ReservableCurrency}
};
use frame_system::pallet_prelude::*;
use parami_did::DidMethodSpecId;

mod mock;
mod tests;

pub type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type ResultPost<T> = sp_std::result::Result<T, sp_runtime::DispatchErrorWithPostInfo<frame_support::weights::PostDispatchInfo>>;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// BBB
        BBBB(T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The DID does not exist.
        DIDNotExists,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            0
        }

        fn integrity_test () {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        #[pallet::weight(100_000)]
        #[transactional]
        pub fn create_advertiser(
            origin: OriginFor<T>,
        ) -> DispatchResultWithPostInfo {
            let who: T::AccountId = ensure_signed(origin)?;
            let did: DidMethodSpecId = Self::ensure_did(&who)?;

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn ensure_did(who: &T::AccountId) -> ResultPost<DidMethodSpecId> {
        let did: Option<DidMethodSpecId> = parami_did::Pallet::<T>::lookup_account(who.clone());
        ensure!(did.is_some(), Error::<T>::DIDNotExists);
        Ok(did.expect("Must be Some"))
    }
}
