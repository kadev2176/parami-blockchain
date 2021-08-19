#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin, ExistenceRequirement::KeepAlive, ReservableCurrency},
	transactional,
	weights::PostDispatchInfo,
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use parami_primitives::Balance;
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating, Verify},
	DispatchErrorWithPostInfo, FixedPointNumber, PerU16,
};
use sp_std::vec::Vec;

mod mock;
mod tests;

pub type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub const UNIT: Balance = 1_000_000_000_000_000;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: pallet_timestamp::Config<AccountId = parami_primitives::AccountId> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Required `origin` for updating configuration
		type ConfigOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CreatedAdvertiser(T::AccountId, u8, u8),
	}

	#[pallet::error]
	pub enum Error<T> {
		SomethingTerribleHappened,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			0
		}
		fn integrity_test() {}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {}
	}

	/// an index for rewards. The secondary key: `(user_did, media_did)`
	#[pallet::storage]
	pub type Rewards<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u8, Blake2_128Concat, (u8, u8), ()>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000_000_000)]
		#[transactional]
		pub fn create_advertiser(
			origin: OriginFor<T>,
			#[pallet::compact] reward_pool: Balance,
		) -> DispatchResultWithPostInfo {
			let who: T::AccountId = ensure_signed(origin)?;
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {}
