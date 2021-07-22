#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    transactional,
    traits::{Currency, ReservableCurrency}
};
use frame_system::pallet_prelude::*;

mod mock;
mod tests;

pub type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
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
        /// AAAA
        AAAA,
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
        pub fn create_ad_publisher(
            origin: OriginFor<T>,
        ) -> DispatchResultWithPostInfo {
            let _who = ensure_signed(origin)?;
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {

}
