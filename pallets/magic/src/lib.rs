#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod migrations;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    traits::{Currency, ExistenceRequirement, IsSubType, IsType, OriginTrait, StorageVersion},
    weights::GetDispatchInfo,
    PalletId,
};
use parami_did::Pallet as Did;
use sp_runtime::{
    traits::{AccountIdConversion, Dispatchable, Saturating},
    DispatchError,
};
use sp_std::boxed::Box;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, HeightOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The balance transfer to magic account automatically
        type AutomaticDeposit: Get<BalanceOf<Self>>;

        /// The overarching call type
        type Call: Parameter
            + Dispatchable<Origin = Self::Origin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsSubType<Call<Self>>
            + IsType<<Self as frame_system::Config>::Call>;

        /// The pallet id, used for deriving stash accounts
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Metadata of a magic-stash account, by controller.
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, MetaOf<T>>;

    /// Controller account of magic account
    #[pallet::storage]
    #[pallet::getter(fn controller)]
    pub(super) type Controller<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, AccountOf<T>>;

    /// Controller account of stash account
    #[pallet::storage]
    #[pallet::getter(fn codoer)]
    pub(super) type Codoer<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, AccountOf<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Stable account created \[stash, controller\]
        Created(AccountOf<T>, AccountOf<T>),
        /// Controller changed \[stash, controller\]
        Changed(AccountOf<T>, AccountOf<T>),
        /// Proxy executed correctly \[ result \]
        Codo(DispatchResult),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        ControllerAccountUsed,
        ControllerEqualToMagic,
        InsufficientBalance,
        MagicAccountUsed,
        NotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::create_stable_account())]
        pub fn create_stable_account(
            origin: OriginFor<T>,
            magic_account: AccountOf<T>,
            #[pallet::compact] stashed: BalanceOf<T>,
        ) -> DispatchResult {
            let controller_account = ensure_signed(origin)?;

            let _ = Self::create(controller_account, magic_account, stashed)?;

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::create_stable_account())]
        pub fn create_accounts_and_did(
            origin: OriginFor<T>,
            magic_account: AccountOf<T>,
            #[pallet::compact] stashed: BalanceOf<T>,
            referrer: Option<T::DecentralizedId>,
        ) -> DispatchResult {
            let controller_account = ensure_signed(origin)?;

            let meta = Self::create(controller_account, magic_account, stashed)?;

            Did::<T>::create(meta.stash_account, referrer)?;

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::change_controller())]
        pub fn change_controller(
            origin: OriginFor<T>,
            new_controller: AccountOf<T>,
        ) -> DispatchResult {
            let magic_account = ensure_signed(origin)?;

            ensure!(
                new_controller != magic_account,
                Error::<T>::ControllerEqualToMagic
            );

            ensure!(
                <Controller<T>>::get(&new_controller).is_none(),
                Error::<T>::ControllerAccountUsed
            );
            ensure!(
                <Metadata<T>>::get(&new_controller).is_none(),
                Error::<T>::ControllerAccountUsed
            );

            let current_controller =
                <Controller<T>>::get(&magic_account).ok_or(Error::<T>::NotExists)?;

            let mut meta = <Metadata<T>>::get(&current_controller).ok_or(Error::<T>::NotExists)?;

            let free = T::Currency::free_balance(&current_controller);
            T::Currency::transfer(
                &current_controller,
                &new_controller,
                free,
                ExistenceRequirement::AllowDeath,
            )?;

            let deposit = T::AutomaticDeposit::get();
            let _ = T::Currency::transfer(
                &meta.stash_account,
                &meta.magic_account,
                deposit,
                ExistenceRequirement::KeepAlive,
            );

            meta.controller_account = new_controller.clone();

            <Metadata<T>>::remove(&current_controller);
            <Metadata<T>>::insert(&meta.controller_account, &meta);

            <Controller<T>>::mutate(&meta.magic_account, |maybe| *maybe = Some(new_controller));

            <Codoer<T>>::insert(&meta.stash_account, &meta.controller_account);

            Self::deposit_event(Event::Changed(meta.stash_account, meta.controller_account));

            Ok(())
        }

        #[pallet::weight({
            let di = call.get_dispatch_info();
            (
                <T as Config>::WeightInfo::codo()
                    .saturating_add(di.weight)
                    .saturating_add(T::DbWeight::get().reads_writes(1, 1)),
                di.class,
            )
        })]
        pub fn codo(origin: OriginFor<T>, call: Box<<T as Config>::Call>) -> DispatchResult {
            let controller_account = ensure_signed(origin)?;
            let meta = <Metadata<T>>::get(&controller_account).ok_or(Error::<T>::NotExists)?;

            let mut origin: T::Origin = frame_system::RawOrigin::Signed(meta.stash_account).into();
            origin.add_filter(move |c: &<T as frame_system::Config>::Call| {
                let c = <T as Config>::Call::from_ref(c);
                match c.is_sub_type() {
                    Some(Call::create_stable_account { .. })
                    | Some(Call::change_controller { .. })
                    | Some(Call::codo { .. }) => false,
                    _ => true,
                }
            });

            let e = call.dispatch(origin);

            Self::deposit_event(Event::Codo(e.map(|_| ()).map_err(|e| e.error)));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// \[magic_account, stash_account, controller_account\]
        pub accounts: Vec<(AccountOf<T>, AccountOf<T>, AccountOf<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                accounts: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let length = self.accounts.len();

            for i in 0..length {
                let meta = types::Metadata {
                    stash_account: self.accounts[i].1.clone(),
                    controller_account: self.accounts[i].2.clone(),
                    magic_account: self.accounts[i].0.clone(),
                    created: Default::default(),
                };

                <Metadata<T>>::insert(&meta.controller_account, &meta);
                <Controller<T>>::insert(&meta.magic_account, &meta.controller_account);
                <Codoer<T>>::insert(&meta.stash_account, &meta.controller_account);
            }
        }
    }
}

impl<T: Config> parami_traits::Accounts<AccountOf<T>> for Pallet<T> {
    fn fee_account(account: &AccountOf<T>) -> AccountOf<T> {
        if let Some(account) = <Controller<T>>::get(account) {
            account
        } else if let Some(account) = <Codoer<T>>::get(account) {
            account
        } else {
            // <Metadata<T>>::contains_key(account)
            // or not a magic-stash account
            account.clone()
        }
    }
}

impl<T: Config> Pallet<T> {
    fn create(
        controller_account: AccountOf<T>,
        magic_account: AccountOf<T>,
        stashed: BalanceOf<T>,
    ) -> Result<MetaOf<T>, DispatchError> {
        use codec::Encode;
        use frame_support::{ensure, traits::Get};

        ensure!(
            controller_account != magic_account,
            Error::<T>::ControllerEqualToMagic
        );

        ensure!(
            <Controller<T>>::get(&magic_account).is_none(),
            Error::<T>::MagicAccountUsed
        );
        ensure!(
            <Controller<T>>::get(&controller_account).is_none(),
            Error::<T>::ControllerAccountUsed
        );

        ensure!(
            <Metadata<T>>::get(&controller_account).is_none(),
            Error::<T>::ControllerAccountUsed
        );
        ensure!(
            <Metadata<T>>::get(&magic_account).is_none(),
            Error::<T>::MagicAccountUsed
        );

        let deposit = T::AutomaticDeposit::get();
        let minimum = T::Currency::minimum_balance();

        ensure!(
            T::Currency::free_balance(&controller_account) - minimum
                >= deposit.saturating_add(stashed),
            Error::<T>::InsufficientBalance
        );

        let created = <frame_system::Pallet<T>>::block_number();

        // TODO: use a HMAC-based algorithm.
        let mut raw = <AccountOf<T>>::encode(&magic_account);
        let mut ord = T::BlockNumber::encode(&created);
        raw.append(&mut ord);

        let stash_account = <T as Config>::PalletId::get().into_sub_account(raw);

        let meta = types::Metadata {
            stash_account,
            controller_account,
            magic_account,
            created,
        };

        T::Currency::transfer(
            &meta.controller_account,
            &meta.magic_account,
            deposit,
            ExistenceRequirement::KeepAlive,
        )?;
        T::Currency::transfer(
            &meta.controller_account,
            &meta.stash_account,
            stashed,
            ExistenceRequirement::KeepAlive,
        )?;

        <Metadata<T>>::insert(&meta.controller_account, &meta);
        <Controller<T>>::insert(&meta.magic_account, &meta.controller_account);
        <Codoer<T>>::insert(&meta.stash_account, &meta.controller_account);

        Self::deposit_event(Event::Created(
            meta.stash_account.clone(),
            meta.controller_account.clone(),
        ));

        Ok(meta)
    }
}
