#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::traits::{Currency, ReservableCurrency};
use sp_io::hashing::keccak_256;
use sp_runtime::traits::{IdentifyAccount, LookupError, Member, StaticLookup, Verify};
use sp_runtime::MultiAddress;
use sp_std::prelude::*;
// pub use weights::WeightInfo;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Use 0x34(b'4') as prefix, so you will get a `N-did` after base58encode_check.
type DidMethodSpecId = [u8; 20];

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The deposit needed for reserving an index.
        type Deposit: Get<BalanceOf<Self>>;

        type Public: IdentifyAccount<AccountId = Self::AccountId> + AsRef<[u8]> + Member + Codec;

        type Signature: Verify<Signer = Self::Public> + Member + Codec;

        // /// Weight information for extrinsics in this pallet.
        // type WeightInfo: WeightInfo;
    }

    // 4. Runtime Storage
    // Use to declare storage items.
    #[pallet::storage]
    pub(super) type Metadata<T: Config> =
        StorageMap<_, Blake2_128Concat, DidMethodSpecId, (T::AccountId, BalanceOf<T>, bool)>;

    #[pallet::storage]
    #[pallet::getter(fn did_of)]
    pub(super) type DidOf<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, DidMethodSpecId>;

    #[pallet::storage]
    #[pallet::getter(fn referrer_of)]
    pub(super) type ReferrerOf<T: Config> =
        StorageMap<_, Identity, DidMethodSpecId, DidMethodSpecId>;

    // 5. Runtime Events
    // Can stringify event types to metadata.
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A account index was assigned. \[index, who, referrer\]
        Assigned(DidMethodSpecId, T::AccountId, Option<DidMethodSpecId>),
        /// A account index has been freed up (unassigned). \[index\]
        Revoked(DidMethodSpecId),
        /// A account index has been frozen to its current account ID. \[index, who\]
        Frozen(DidMethodSpecId, T::AccountId),
    }

    /// Error for the nicks module.
    #[pallet::error]
    pub enum Error<T> {
        /// DID does not exist
        NotExists,
        /// did already exists
        DidExists,
        /// The index is assigned to another account.
        NotOwner,
        /// The DID was not available or revoked.
        InUse,
        /// DID is revoked.
        Revoked,
        /// Only accepts account ID.
        AccountIdRequired,
        /// Referrer does not exist.
        ReferrerNotExists,
    }

    // 6. Hooks
    // Define some logic that should be executed
    // regularly in some context, for e.g. on_initialize.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // 7. Extrinsics
    // Functions that are callable from outside the runtime.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(50_000_000)]
        pub(super) fn register(
            origin: OriginFor<T>,
            public: T::Public,
            referrer: Option<DidMethodSpecId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(!<DidOf<T>>::contains_key(&who), Error::<T>::DidExists);
            if let Some(r) = referrer.as_ref() {
                ensure!(<Metadata<T>>::contains_key(r), Error::<T>::ReferrerNotExists);
            }

            let hash = keccak_256(public.as_ref());

            let acct = public.into_account();
            ensure!(who == acct, Error::<T>::NotOwner);

            let mut id = [0u8; 20];
            id.copy_from_slice(&hash[12..]);

            Metadata::<T>::try_mutate(id, |maybe_value| {
                ensure!(maybe_value.is_none(), Error::<T>::InUse);
                *maybe_value = Some((who.clone(), T::Deposit::get(), false));
                T::Currency::reserve(&who, T::Deposit::get())
            })?;
            DidOf::<T>::insert(who.clone(), id);

            if let Some(referrer) = referrer {
                ReferrerOf::<T>::insert(id, referrer);
                Self::deposit_event(Event::Assigned(id, who, Some(referrer)));
            } else {
                Self::deposit_event(Event::Assigned(id, who, None));
            }

            Ok(().into())
        }
    }

    // PUBLIC IMMUTABLES
    impl<T: Config> Pallet<T> {
        /// Lookup an T::AccountIndex to get an Id, if there's one there.
        pub fn lookup_index(index: DidMethodSpecId) -> Option<T::AccountId> {
            Metadata::<T>::get(index).map(|x| x.0)
        }

        pub fn lookup_account(a: T::AccountId) -> Option<DidMethodSpecId> {
            DidOf::<T>::get(a)
        }

        /// Lookup an address to get an Id, if there's one there.
        pub fn lookup_address(a: MultiAddress<T::AccountId, ()>) -> Option<T::AccountId> {
            match a {
                MultiAddress::Id(i) => Some(i),
                MultiAddress::Address20(i) => Self::lookup_index(i),
                _ => None,
            }
        }

        pub fn is_did(a: MultiAddress<T::AccountId, ()>) -> bool {
            match a {
                MultiAddress::Address20(_) => true,
                _ => false,
            }
        }

        pub fn is_account_id(a: MultiAddress<T::AccountId, ()>) -> bool {
            match a {
                MultiAddress::Id(_) => true,
                _ => false,
            }
        }
    }
}

impl<T: Config> StaticLookup for Pallet<T> {
    type Source = MultiAddress<T::AccountId, ()>;
    type Target = T::AccountId;

    fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
        Self::lookup_address(a).ok_or(LookupError)
    }

    fn unlookup(a: Self::Target) -> Self::Source {
        MultiAddress::Id(a)
    }
}

/*
decl_storage! {
    trait Store for Module<T: Config> as Indices {
        /// The lookup from index to account.
        pub Accounts build(|config: &GenesisConfig<T>|
            config.indices.iter()
                .cloned()
                .map(|(a, b)| (a, (b, Zero::zero(), false)))
                .collect::<Vec<_>>()
        ): map hasher(blake2_128_concat) DidMethodSpecId => Option<(T::AccountId, BalanceOf<T>, bool)>;

        pub DidOf get(fn did_of): map hasher(twox_64_concat) T::AccountId => DidMethodSpecId;
    }
    add_extra_genesis {
        config(indices): Vec<(DidMethodSpecId, T::AccountId)>;
    }
}

decl_event!(
    pub enum Event<T> where
        <T as frame_system::Config>::AccountId,
    {

    }
);

decl_error! {
    pub enum Error for Module<T: Config> {

    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin, system = frame_system {
        /// The deposit needed for reserving an index.
        const Deposit: BalanceOf<T> = T::Deposit::get();

        fn deposit_event() = default;

        #[weight = 10000]
        fn register(origin, public: T::Public, referrer: Option<DidMethodSpecId>) {
            let who = ensure_signed(origin)?;

            ensure!(!<DidOf<T>>::contains_key(&who), Error::<T>::DidExists);

            let hash = keccak_256(public.as_ref());

            let acct = public.into_account();
            ensure!(who == acct, Error::<T>::NotOwner);

            let mut id = [0u8; 20];
            id.copy_from_slice(&hash[12..]);

            Accounts::<T>::try_mutate(id, |maybe_value| {
                ensure!(maybe_value.is_none(), Error::<T>::InUse);
                *maybe_value = Some((who.clone(), T::Deposit::get(), false));
                T::Currency::reserve(&who, T::Deposit::get())
            })?;

            Self::deposit_event(RawEvent::Assigned(id, who));
        }

        /// Assign an previously unassigned index.
        ///
        /// Payment: `Deposit` is reserved from the sender account.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// - `index`: the index to be claimed. This must not be in use.
        ///
        /// Emits `DidAssigned` if successful.
        ///
        /// # <weight>
        /// - `O(1)`.
        /// - One storage mutation (codec `O(1)`).
        /// - One reserve operation.
        /// - One event.
        /// -------------------
        /// - DB Weight: 1 Read/Write (Accounts)
        /// # </weight>
        #[weight = 0]
        fn claim(origin, index: DidMethodSpecId) {
            let who = ensure_signed(origin)?;

            Accounts::<T>::try_mutate(index, |maybe_value| {
                ensure!(maybe_value.is_none(), Error::<T>::InUse);
                *maybe_value = Some((who.clone(), T::Deposit::get(), false));
                T::Currency::reserve(&who, T::Deposit::get())
            })?;
            Self::deposit_event(RawEvent::Assigned(index, who));
        }



        /// Free up an index owned by the sender.
        ///
        /// Payment: Any previous deposit placed for the index is unreserved in the sender account.
        ///
        /// The dispatch origin for this call must be _Signed_ and the sender must own the index.
        ///
        /// - `index`: the index to be freed. This must be owned by the sender.
        ///
        /// Emits `IndexFreed` if successful.
        ///
        /// # <weight>
        /// - `O(1)`.
        /// - One storage mutation (codec `O(1)`).
        /// - One reserve operation.
        /// - One event.
        /// -------------------
        /// - DB Weight: 1 Read/Write (Accounts)
        /// # </weight>
        #[weight = 20]
        fn revoke(origin, index: DidMethodSpecId) {
            let who = ensure_signed(origin)?;

            Accounts::<T>::try_mutate(index, |maybe_value| -> DispatchResult {
                let (account, amount, revoked) = maybe_value.take().ok_or(Error::<T>::NotExists)?;
                ensure!(!revoked, Error::<T>::Revoked);
                ensure!(&account == &who, Error::<T>::NotOwner);
                T::Currency::unreserve(&who, amount);
                Ok(())
            })?;
            Self::deposit_event(Event::<T>::Revoked(index));
        }
    }
}

impl<T: Config> Module<T> {
    // PUBLIC IMMUTABLES

    /// Lookup an T::AccountIndex to get an Id, if there's one there.
    pub fn lookup_index(index: DidMethodSpecId) -> Option<T::AccountId> {
        Accounts::<T>::get(index).map(|x| x.0)
    }

    pub fn lookup_account(a: T::AccountId) -> Option<DidMethodSpecId> {
        unimplemented!()
    }

    /// Lookup an address to get an Id, if there's one there.
    pub fn lookup_address(a: MultiAddress<T::AccountId, ()>) -> Option<T::AccountId> {
        match a {
            MultiAddress::Id(i) => Some(i),
            MultiAddress::Address20(i) => Self::lookup_index(i),
            _ => None,
        }
    }

    pub fn is_did(a: MultiAddress<T::AccountId, ()>) -> bool {
        match a {
            MultiAddress::Address20(_) => true,
            _ => false,
        }
    }

    pub fn is_account_id(a: MultiAddress<T::AccountId, ()>) -> bool {
        match a {
            MultiAddress::Id(_) => true,
            _ => false,
        }
    }
}

 */
