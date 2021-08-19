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
	traits::{AccountIdConversion, One, Saturating},
	DispatchErrorWithPostInfo,
};

macro_rules! s {
	($e: expr) => {
		sp_runtime::SaturatedConversion::saturated_into($e)
	};
}

mod mock;
mod tests;

mod types;
pub use types::*;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: pallet_timestamp::Config {
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
        /// StableAccount changed \[stash, controller, magic\]
        StableAccount(T::AccountId, T::AccountId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
        NoAvailableId,
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

	/// Next available ID.
	#[pallet::storage]
	pub type NextId<T: Config> = StorageValue<_, GlobalId, ValueQuery>;

    /// map from controller account to `StableAccount`
    #[pallet::storage]
    pub type StableAccounts<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, StableAccountOf<T>>;

    /// map from magic account to controller account
    #[pallet::storage]
    pub type StableAccountByMagic<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

	/// an index for rewards. The secondary key: `(user_did, media_did)`
	#[pallet::storage]
	pub type Rewards<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u8, Blake2_128Concat, (u8, u8), ()>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000_000_000)]
		#[transactional]
		pub fn create_stable_account(
			origin: OriginFor<T>,
			magic_account: T::AccountId,
			#[pallet::compact] stash_deposit: Balance,
		) -> DispatchResultWithPostInfo {
            let controller_account = ensure_signed(origin)?;
			let sa: StableAccountOf<T> = StableAccount {
				created_time: now::<T>(),
				stash_account: Self::create_stash_account(Self::inc_id()?),
                controller_account,
				magic_account,
			};

            <T as Config>::Currency::transfer(&sa.controller_account, &sa.magic_account, s!(FEE), KeepAlive)?;
            <T as Config>::Currency::transfer(&sa.controller_account, &sa.stash_account, s!(stash_deposit), KeepAlive)?;

            Self::update_stable_account(&sa);
			Ok(().into())
		}
	}
}

/// now is duration since unix epoch in millisecond
pub fn now<T: Config>() -> T::Moment {
	pallet_timestamp::Pallet::<T>::now()
}

impl<T: Config> Pallet<T> {
    fn update_stable_account(sa: &StableAccountOf<T>) {
        StableAccounts::<T>::insert(&sa.controller_account, sa);
        StableAccountByMagic::<T>::insert(&sa.magic_account, &sa.controller_account);
        Self::deposit_event(Event::StableAccount(sa.stash_account.clone(), sa.controller_account.clone(), sa.magic_account.clone()));
    }

	fn create_stash_account(id: GlobalId) -> T::AccountId {
		let stab_acc = PalletId(*b"prm/stab");
		stab_acc.into_sub_account(id)
	}

	fn inc_id() -> Result<GlobalId, DispatchError> {
		NextId::<T>::try_mutate(|id| -> Result<GlobalId, DispatchError> {
			let current_id = *id;
			*id = id.checked_add(GlobalId::one()).ok_or(Error::<T>::NoAvailableId)?;
			Ok(current_id)
		})
	}
}
