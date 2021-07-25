#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*, PalletId,
    transactional,
    traits::{Currency, ReservableCurrency, ExistenceRequirement::KeepAlive},
    weights::PostDispatchInfo
};
use sp_runtime::{traits::{AccountIdConversion, One}, DispatchErrorWithPostInfo};
use frame_system::pallet_prelude::*;
use parami_did::DidMethodSpecId;
use parami_primitives::{Balance};

mod mock;
mod tests;
mod utils;
mod types;
pub use types::*;

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
    pub trait Config: pallet_timestamp::Config + parami_did::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// an advertiser was created. \[who, did, advertiser id\]
        CreatedAdvertiser(T::AccountId, DidMethodSpecId, AdvertiserId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The DID does not exist.
        DIDNotExists,
        /// id overflow.
        NoAvailableId,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight { 0 }
        fn integrity_test () {}
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub advertiser_deposit: Balance,
        pub ad_deposit: Balance,
        pub _phantom: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                advertiser_deposit: UNIT.saturating_mul(500),
                ad_deposit: UNIT.saturating_mul(20),
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            AdvertiserDeposit::<T>::put(self.advertiser_deposit);
            AdDeposit::<T>::put(self.ad_deposit);
        }
    }

    /// ad deposit
    #[pallet::storage]
    pub type AdDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// advertiser deposit
    #[pallet::storage]
    pub type AdvertiserDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

    /// Next available ID.
    #[pallet::storage]
    pub type NextId<T: Config> = StorageValue<_, GlobalId, ValueQuery>;

    /// an index for advertisers
    #[pallet::storage]
    pub type Advertisers<T: Config> = StorageMap<_, Blake2_128Concat, DidMethodSpecId, AdvertiserOf<T>>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn create_advertiser(
            origin: OriginFor<T>,
            #[pallet::compact] reward_pool: Balance,
        ) -> DispatchResultWithPostInfo {
            let who: T::AccountId = ensure_signed(origin)?;
            let did: DidMethodSpecId = Self::ensure_did(&who)?;

            let advertiser_id = Self::inc_id()?;
            let (deposit_account, reward_pool_account) = Self::ad_accounts(advertiser_id);

            let deposit = AdvertiserDeposit::<T>::get();
            <T as Config>::Currency::transfer(&who, &deposit_account, s!(deposit), KeepAlive)?;
            <T as Config>::Currency::transfer(&who, &reward_pool_account, s!(reward_pool), KeepAlive)?;

            let a = Advertiser {
                did,
                created_time: Self::now(),
                advertiser_id,
                deposit,
                deposit_account,
                reward_pool_account,
            };
            Advertisers::<T>::insert(did, a);
            Self::deposit_event(Event::CreatedAdvertiser(who, did, advertiser_id));
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

    fn now() -> T::Moment {
        pallet_timestamp::Pallet::<T>::now()
    }

    fn ad_accounts(id: AdvertiserId) -> (T::AccountId, T::AccountId) {
        let deposit = PalletId(*b"prm/ad/d");
        let reward_pool = PalletId(*b"prm/ad/r");
        (deposit.into_sub_account(id), reward_pool.into_sub_account(id))
    }

    fn inc_id() -> Result<GlobalId, DispatchError> {
        NextId::<T>::try_mutate(|id| -> Result<GlobalId, DispatchError> {
            let current_id = *id;
            *id = id.checked_add(GlobalId::one()).ok_or(Error::<T>::NoAvailableId)?;
            Ok(current_id)
        })
    }
}
