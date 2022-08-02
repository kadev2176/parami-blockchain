#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod migrations;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;
use codec::MaxEncodedLen;
use frame_support::{
    dispatch::DispatchResultWithPostInfo,
    ensure,
    traits::{
        tokens::fungibles::{Inspect, Mutate, Transfer as FungTransfer},
        Currency, EnsureOrigin,
        ExistenceRequirement::AllowDeath,
        Get,
    },
    PalletId,
};
use frame_system::ensure_signed;
use parami_chainbridge::{ChainId, ResourceId};
use sp_core::U256;
use sp_runtime::traits::{AccountIdConversion, SaturatedConversion};
use sp_std::prelude::*;

use weights::WeightInfo;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetOf<T> = <T as Config>::AssetId;

#[frame_support::pallet]
pub mod pallet {

    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{tokens::WithdrawConsequence, ExistenceRequirement, WithdrawReasons},
    };
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_chainbridge::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
        type BridgeOrigin: EnsureOrigin<
            <Self as frame_system::Config>::Origin,
            Success = <Self as frame_system::Config>::AccountId,
        >;

        /// The currency mechanism.
        type Currency: Currency<<Self as frame_system::Config>::AccountId>;

        type Assets: FungTransfer<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>
            + Mutate<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        type AssetId: Parameter + Member + Default + Copy + MaxEncodedLen;

        /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
        type HashResourceId: Get<ResourceId>;

        type NativeTokenResourceId: Get<ResourceId>;

        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;

        type ForceOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Remark(<T as frame_system::Config>::Hash),
    }

    #[pallet::storage]
    #[pallet::getter(fn resource)]
    pub(super) type ResourceMap<T: Config> = StorageMap<_, Identity, AssetOf<T>, ResourceId>;

    #[pallet::storage]
    #[pallet::getter(fn bridge_fee)]
    pub type NativeFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    pub(super) type ResourceId2Asset<T: Config> =
        StorageMap<_, Twox64Concat, ResourceId, AssetOf<T>>;

    #[pallet::storage]
    pub type TransferTokenFee<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        ChainId,
        Twox64Concat,
        AssetOf<T>, // Provider Account
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        InvalidTransfer,
        NotExists,
        InsufficientFund,
        InsufficientTransferFee,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::transfer_hash())]
        pub fn transfer_hash(
            origin: OriginFor<T>,
            hash: <T as frame_system::Config>::Hash,
            dest_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let resource_id = T::HashResourceId::get();
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <parami_chainbridge::Pallet<T>>::transfer_generic(dest_id, resource_id, metadata)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::transfer_native())]
        pub fn transfer_native(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(
                <parami_chainbridge::Pallet<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );

            let free_balance = T::Currency::free_balance(&source);
            ensure!(free_balance >= amount, Error::<T>::InsufficientFund);

            let fee = <NativeFee<T>>::get();
            ensure!(amount > fee, Error::<T>::InsufficientTransferFee);

            let resource_id = T::NativeTokenResourceId::get();
            let pot = Self::generate_fee_pot();

            T::Currency::withdraw(&source, amount - fee, WithdrawReasons::TRANSFER, AllowDeath)?;
            T::Currency::transfer(&source, &pot, fee.into(), AllowDeath)?;

            <parami_chainbridge::Pallet<T>>::transfer_fungible(
                dest_id,
                resource_id,
                recipient,
                U256::from((amount - fee).saturated_into::<u128>()),
            )?;

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::update_native_fee())]
        pub fn update_native_fee(origin: OriginFor<T>, fee: BalanceOf<T>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            <NativeFee<T>>::put(fee);
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::update_transfer_token_fee())]
        pub fn update_transfer_token_fee(
            origin: OriginFor<T>,
            dest_id: ChainId,
            asset_id: AssetOf<T>,
            fee: BalanceOf<T>,
        ) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            <TransferTokenFee<T>>::insert(dest_id, asset_id, fee);
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::transfer_token())]
        pub fn transfer_token(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: ChainId,
            asset: AssetOf<T>,
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;

            ensure!(
                <parami_chainbridge::Pallet<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );

            let resource_id = <ResourceMap<T>>::get(asset).ok_or(Error::<T>::NotExists)?;
            let fee = <TransferTokenFee<T>>::get(dest_id, asset);
            let currency_balance = T::Currency::free_balance(&source);
            let asset_balance = T::Assets::balance(asset, &source);
            ensure!(fee <= currency_balance, Error::<T>::InsufficientTransferFee);
            ensure!(amount <= asset_balance, Error::<T>::InsufficientFund);

            let pot = Self::generate_fee_pot();
            T::Currency::transfer(&source, &pot, fee, ExistenceRequirement::KeepAlive)?;
            T::Assets::burn_from(asset, &source, amount)?;

            <parami_chainbridge::Pallet<T>>::transfer_fungible(
                dest_id,
                resource_id,
                recipient,
                U256::from(amount.saturated_into::<u128>()),
            )?;

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::force_set_resource())]
        pub fn force_set_resource(
            origin: OriginFor<T>,
            resource_id: ResourceId,
            asset_id: AssetOf<T>,
        ) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            <ResourceMap<T>>::insert(asset_id, resource_id);
            <ResourceId2Asset<T>>::insert(resource_id, asset_id);
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::transfer())]
        pub fn handle_transfer_fungibles(
            origin: OriginFor<T>,
            to: <T as frame_system::Config>::AccountId,
            amount: BalanceOf<T>,
            resource_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            let _bridge = T::BridgeOrigin::ensure_origin(origin)?;
            if resource_id == T::NativeTokenResourceId::get() {
                <T as Config>::Currency::deposit_creating(&to, amount.into());
                return Ok(().into());
            }

            let asset_id = <ResourceId2Asset<T>>::get(resource_id).ok_or(<Error<T>>::NotExists)?;
            T::Assets::mint_into(asset_id, &to, amount.into())?;

            return Ok(().into());
        }

        #[pallet::weight(<T as Config>::WeightInfo::remark())]
        pub fn remark(
            origin: OriginFor<T>,
            hash: <T as frame_system::Config>::Hash,
            _r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(Event::Remark(hash));
            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {}

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {}
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {}
    }
}

impl<T: Config> Pallet<T> {
    fn generate_fee_pot() -> AccountOf<T> {
        <T as Config>::PalletId::get().into_account_truncating()
    }
}
