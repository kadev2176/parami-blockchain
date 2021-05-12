#![cfg_attr(not(feature = "std"), no_std)]

/*
mod benchmarking;
mod tests;
#[rustfmt::skip]
pub mod weights;
*/
use frame_support::traits::{Currency, ExistenceRequirement};
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<() as pallet_balances::weights::WeightInfo>::transfer() * dests.len() as u64)]
        pub(super) fn airdrop(
            origin: OriginFor<T>,
            dests: Vec<<T::Lookup as StaticLookup>::Source>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::admin(), Error::<T>::RequireAdmin);

            ensure!(!dests.is_empty(), Error::<T>::EmptyDest);
            // TODO: find a reasonable default limit
            ensure!(dests.len() <= 2_000, Error::<T>::TooManyDests);

            frame_support::debug::native::info!("airdrop to {} dests", dests.len());
            frame_support::debug::native::info!("airdrop {:?}", amount);

            let total_amount = amount * <BalanceOf<T>>::from(dests.len() as u32);
            let free_balance = T::Currency::free_balance(&sender);

            ensure!(free_balance > total_amount, Error::<T>::InsufficientBalance);

            for dest in &dests {
                let who = T::Lookup::lookup(dest.clone())?;
                frame_support::debug::native::info!("airdrop to {:?}", dest);
                T::Currency::transfer(&sender, &who, amount, ExistenceRequirement::AllowDeath)?;
            }

            Self::deposit_event(Event::<T>::Airdropped(dests, amount));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub(crate) fn set_admin(
            origin: OriginFor<T>,
            new: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            // This is a public call, so we ensure that the origin is some signed account.
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::admin(), Error::<T>::RequireAdmin);
            let new = T::Lookup::lookup(new)?;

            Self::deposit_event(Event::AdminChanged(Self::admin()));
            <Admin<T>>::put(new);
            // Admin user does not pay a fee.
            Ok(Pays::No.into())
        }

        #[pallet::weight(0)]
        pub fn force_set_admin(
            origin: OriginFor<T>,
            new: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            let new = T::Lookup::lookup(new)?;

            Self::deposit_event(Event::AdminChanged(Self::admin()));
            <Admin<T>>::put(new);
            Ok(Pays::No.into())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
    pub enum Event<T: Config> {
        Airdropped(Vec<<T::Lookup as StaticLookup>::Source>, BalanceOf<T>),
        /// The \[admin\] just switched identity; the old key is supplied.
        AdminChanged(T::AccountId),
    }

    #[pallet::error]
    /// Error for the Sudo pallet
    pub enum Error<T> {
        /// Sender must be the admin account
        RequireAdmin,
        /// Airdrop dest is empty
        EmptyDest,
        /// Too many airdrop dests
        TooManyDests,
        /// Balance too low to do airdrop
        InsufficientBalance,
    }

    #[pallet::storage]
    #[pallet::getter(fn admin)]
    pub(super) type Admin<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// The `AccountId` of the airdrop admin.
        pub admin: T::AccountId,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                admin: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <Admin<T>>::put(&self.admin);
        }
    }
}
