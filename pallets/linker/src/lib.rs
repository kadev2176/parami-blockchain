#![cfg_attr(not(feature = "std"), no_std)]

pub use btc::hashing;
pub use ocw::images;
pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod btc;
mod did;
mod impls;
mod ocw;
mod types;
mod witness;

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    ensure,
    traits::{Currency, NamedReservableCurrency, OnUnbalanced},
    PalletId,
};
use frame_system::offchain::CreateSignedTransaction;
use parami_did::{EnsureDid, Pallet as Did};
use parami_traits::Tags;
use sp_runtime::traits::Hash;
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <CurrencyOf<T> as Currency<AccountOf<T>>>::Balance;
type CurrencyOf<T> = <T as parami_did::Config>::Currency;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type NegativeImbOf<T> = <CurrencyOf<T> as Currency<AccountOf<T>>>::NegativeImbalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + parami_did::Config //
        + CreateSignedTransaction<Call<Self>>
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Minimum deposit to become a registrar
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Lifetime of a pending account
        #[pallet::constant]
        type PendingLifetime: Get<Self::BlockNumber>;

        /// Handler for the unbalanced reduction when slashing an registrar
        type Slash: OnUnbalanced<NegativeImbOf<Self>>;

        /// The means of storing the tags and personas of a DID.
        type Tags: Tags<DecentralizedId = DidOf<Self>, Hash = HashOf<Self>>;

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// The origin which may forcibly trust or block a registrar
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Linked accounts of a DID
    #[pallet::storage]
    #[pallet::getter(fn links_of)]
    pub(super) type LinksOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        DidOf<T>,
        Twox64Concat,
        types::AccountType,
        Vec<u8>, //
    >;

    /// Accounts pending to be checked with the offchain worker
    #[pallet::storage]
    #[pallet::getter(fn pendings_of)]
    pub(super) type PendingOf<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        types::AccountType,
        Identity,
        DidOf<T>,
        types::Pending<T::BlockNumber>,
    >;

    /// Linked accounts
    #[pallet::storage]
    #[pallet::getter(fn linked)]
    pub(super) type Linked<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        types::AccountType,
        Blake2_256,
        Vec<u8>,
        bool,
        ValueQuery,
    >;

    /// DID of a registrar
    #[pallet::storage]
    #[pallet::getter(fn registrar)]
    pub(super) type Registrar<T: Config> = StorageMap<_, Identity, DidOf<T>, bool>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account linked \[did, type, account, by\]
        AccountLinked(DidOf<T>, types::AccountType, Vec<u8>, DidOf<T>),
        /// Account unlinked \[did, type, by\]
        AccountUnlinked(DidOf<T>, types::AccountType, DidOf<T>),
        /// Registrar was blocked \[id\]
        Blocked(DidOf<T>),
        /// Registrar deposited \[id, value\]
        Deposited(DidOf<T>, BalanceOf<T>),
        /// Registrar was trusted \[id\]
        Trusted(DidOf<T>),
        /// Pending link failed \[did, type, account\]
        ValidationFailed(DidOf<T>, types::AccountType, Vec<u8>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(block_number: T::BlockNumber) {
            match Self::ocw_begin_block(block_number) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("An error occurred in OCW: {:?}", e);
                }
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Blocked,
        Deadline,
        ExistentialDeposit,
        Exists,
        HttpFetchingError,
        InvalidAddress,
        InvalidSignature,
        NotExists,
        UnexpectedAddress,
        UnsupportedSite,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Link a sociality account to a DID
        ///
        /// Link will become pending, and will be checked with the offchain worker or a registrar
        ///
        /// # Arguments
        ///
        /// * `site` - Account type
        /// * `profile` - Profile URL
        #[pallet::weight(<T as Config>::WeightInfo::link_sociality(profile.len() as u32))]
        pub fn link_sociality(
            origin: OriginFor<T>,
            site: types::AccountType,
            profile: Vec<u8>,
        ) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            Self::insert_pending(did, site, profile)
        }

        /// Link a cryptographic account to a DID
        ///
        ///
        /// # Arguments
        ///
        /// * `crypto` - Account type
        /// * `address` - Account address
        ///   * When dealing with BTC, DOT, SOL, TRX, the address should in the format of base58
        ///   * When dealing with ETH, the address should in the format of binary or hex
        /// * `signature` - Account signature
        ///   * When dealing with DOT, SOL, the signature should have a prefix of `0x00`
        #[pallet::weight(<T as Config>::WeightInfo::link_crypto())]
        pub fn link_crypto(
            origin: OriginFor<T>,
            crypto: types::AccountType,
            address: Vec<u8>,
            signature: types::Signature,
        ) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(address.len() >= 2, Error::<T>::InvalidAddress);

            let bytes = Self::generate_message(&did);

            let recovered = Self::recover_address(crypto, address.clone(), signature, bytes)?;

            ensure!(recovered == address, Error::<T>::UnexpectedAddress);

            Self::insert_link(did, crypto, address, did)
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_link(profile.len() as u32))]
        pub fn submit_link(
            origin: OriginFor<T>,
            did: DidOf<T>,
            site: types::AccountType,
            profile: Vec<u8>,
            validated: bool,
        ) -> DispatchResultWithPostInfo {
            let registrar = if let Err(_) = ensure_none(origin.clone()) {
                let (registrar, _) = EnsureDid::<T>::ensure_origin(origin)?;

                ensure!(
                    <Registrar<T>>::get(&registrar) == Some(true),
                    Error::<T>::Blocked
                );

                registrar
            } else {
                Did::<T>::zero()
            };

            if validated {
                Self::insert_link(did, site, profile, registrar)?;
            } else {
                <PendingOf<T>>::remove(&site, &did);

                Self::deposit_event(Event::<T>::ValidationFailed(did, site, profile));
            }

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_score(tag.len() as u32))]
        pub fn submit_score(
            origin: OriginFor<T>,
            did: DidOf<T>,
            tag: Vec<u8>,
            score: i32,
        ) -> DispatchResultWithPostInfo {
            let (registrar, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                <Registrar<T>>::get(&registrar) == Some(true),
                Error::<T>::Blocked
            );

            ensure!(T::Tags::get_score(&did, &tag) == 0, Error::<T>::Exists);

            T::Tags::influence(&did, &tag, score)?;

            // Self::deposit_event(Event::<T>::Scored(did, tag, score));

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::deposit())]
        pub fn deposit(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                <Registrar<T>>::get(&did) != Some(false),
                Error::<T>::Blocked
            );

            let id = <T as Config>::PalletId::get();

            T::Currency::reserve_named(&id.0, &who, value)?;

            Self::deposit_event(Event::Deposited(did, value));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::force_unlink())]
        pub fn force_unlink(
            origin: OriginFor<T>,
            did: DidOf<T>,
            site: types::AccountType,
        ) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let link = <LinksOf<T>>::get(&did, &site).ok_or(Error::<T>::NotExists)?;

            <LinksOf<T>>::remove(&did, &site);
            <Linked<T>>::remove(&site, &link);

            Self::deposit_event(Event::<T>::AccountUnlinked(did, site, Did::<T>::zero()));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::force_trust())]
        pub fn force_trust(origin: OriginFor<T>, did: DidOf<T>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let minimum = T::MinimumDeposit::get();

            let meta = Did::<T>::meta(&did).ok_or(Error::<T>::NotExists)?;

            let id = <T as Config>::PalletId::get();

            let reserved = T::Currency::reserved_balance_named(&id.0, &meta.account);

            ensure!(reserved >= minimum, Error::<T>::ExistentialDeposit);

            <Registrar<T>>::insert(&did, true);

            Self::deposit_event(Event::Trusted(did));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::force_block())]
        pub fn force_block(origin: OriginFor<T>, registrar: DidOf<T>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let meta = Did::<T>::meta(&registrar).ok_or(Error::<T>::NotExists)?;

            let id = <T as Config>::PalletId::get();

            let imb = T::Currency::slash_all_reserved_named(&id.0, &meta.account);

            T::Slash::on_unbalanced(imb);

            <Registrar<T>>::insert(&registrar, false);

            Self::deposit_event(Event::Blocked(registrar));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub links: Vec<(DidOf<T>, types::AccountType, Vec<u8>)>,
        pub registrars: Vec<DidOf<T>>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                links: Default::default(),
                registrars: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (did, typ, dat) in &self.links {
                <LinksOf<T>>::insert(did, typ, dat);
                <Linked<T>>::insert(typ, dat, true);
            }

            for registrar in &self.registrars {
                <Registrar<T>>::insert(registrar, true);
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("linker")
                    .priority(T::UnsignedPriority::get())
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(true)
                    .build()
            };

            match call {
                Call::submit_link { .. } => valid_tx(b"submit_link".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}
