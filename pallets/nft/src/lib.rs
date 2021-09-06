#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    ensure, dispatch::{DispatchResult, DispatchResultWithPostInfo},
    traits::{Currency, ExistenceRequirement::{KeepAlive, }, Get, ReservableCurrency},
    pallet_prelude::*, PalletId,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use frame_system::pallet_prelude::*;
use orml_nft::Pallet as NftModule;
use primitives::{AssetId};
use sp_runtime::RuntimeDebug;
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32BitUnsigned, StaticLookup, One},
    DispatchError,
};
use sp_std::{vec::Vec, prelude::*};
use orml_traits::{AssetHandler};

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;

pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ClassData<Balance> {
    // Minimum balance to create a collection of Asset
    pub deposit: Balance,
    // Metadata from ipfs
    pub metadata: Vec<u8>,
    pub token_type: TokenType,
    pub collection_type: CollectionType,
    pub total_supply: u64,
    pub initial_supply: u64,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AdsSlot<Balance, BlockNumber> {
    pub id: u32,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub deposit: Balance,
    pub media: Vec<u8>,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetData<Balance, BlockNumber> {
    pub deposit: Balance,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub slot: AdsSlot<Balance, BlockNumber>,
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenType {
    Transferable,
    BoundToAddress,
}

impl TokenType {
    pub fn is_transferable(&self) -> bool {
        match *self {
            TokenType::Transferable => true,
            _ => false,
        }
    }
}

impl Default for TokenType {
    fn default() -> Self {
        TokenType::Transferable
    }
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CollectionType {
    Collectable,
    Executable,
}

//Collection extension for fast retrieval
impl CollectionType {
    pub fn is_collectable(&self) -> bool {
        match *self {
            CollectionType::Collectable => true,
            _ => false,
        }
    }

    pub fn is_executable(&self) -> bool {
        match *self {
            CollectionType::Executable => true,
            _ => false,
        }
    }
}

impl Default for CollectionType {
    fn default() -> Self {
        CollectionType::Collectable
    }
}

pub use pallet::*;

const MIN_BALANCE: u128 = 1_000;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::sp_runtime::traits::{CheckedSub, CheckedAdd, };

    #[pallet::config]
    pub trait Config:
        frame_system::Config +
        parami_assets::Config +
        orml_nft::Config<
            TokenData=AssetData<BalanceOf<Self>, <Self as frame_system::Config>::BlockNumber>,
            ClassData=ClassData<BalanceOf<Self>>,
        >
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Minimum deposit to create class
        #[pallet::constant]
        type CreateClassDeposit: Get<BalanceOf<Self>>;

        /// Minimum deposit to create a NFT
        #[pallet::constant]
        type CreateAssetDeposit: Get<BalanceOf<Self>>;

        /// The currency trait
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        //NFT Pallet Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Asset handler
        type AssetsHandler: AssetHandler<<Self as parami_assets::Config>::AssetId>;

        // Weight info
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    type ClassIdOf<T> = <T as orml_nft::Config>::ClassId;
    type TokenIdOf<T> = <T as orml_nft::Config>::TokenId;
    type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::storage]
    #[pallet::getter(fn get_asset)]
    pub(super) type Assets<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AssetId, (ClassIdOf<T>, TokenIdOf<T>), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_assets_by_owner)]
    pub(super) type AssetsByOwner<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AccountId, Vec<T::AssetId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_class_type)]
    pub(super) type ClassDataType<T: Config> = StorageMap<_, Blake2_128Concat, ClassIdOf<T>, TokenType, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn next_asset_id)]
    pub(super) type NextAssetId<T: Config> = StorageValue<_, T::AssetId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_asset_supporters)]
    pub(super) type AssetSupporters<T: Config> =
    StorageMap<_, Blake2_128Concat, T::AssetId, Vec<T::AccountId>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    #[pallet::metadata(
    < T as frame_system::Config >::AccountId = "AccountId",
    ClassIdOf <T> = "ClassId",
    TokenIdOf <T> = "TokenId",
    T::AssetId = "AssetId",
    )]
    pub enum Event<T: Config> {
        /// new NFT Class created
        NewNftClassCreated(<T as frame_system::Config>::AccountId, ClassIdOf<T>),

        /// new NFT minted
        NewNftMinted(T::AssetId, T::AssetId, <T as frame_system::Config>::AccountId, ClassIdOf<T>, u32, TokenIdOf<T>),

        /// NFT transferred
        TransferedNft(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::AccountId, TokenIdOf<T>),

        /// NFT signed
        SignedNft(TokenIdOf<T>, <T as frame_system::Config>::AccountId),
    }

    #[pallet::error]
    pub enum Error<T> {
        //Asset Info not found
        AssetInfoNotFound,
        //Asset Id not found
        AssetIdNotFound,
        //No permission
        NoPermission,
        //No available collection id
        NoAvailableCollectionId,
        //Collection id is not exist
        CollectionIsNotExist,
        //Class Id not found
        ClassIdNotFound,
        //Non Transferable
        NonTransferable,
        //Invalid quantity
        InvalidQuantity,
        //No available asset id
        NoAvailableAssetId,
        //Asset Id is already exist
        AssetIdAlreadyExist,
        //Asset Id is currently in an auction
        AssetAlreadyInAuction,
        //Sign your own Asset
        SignOwnAsset,
    }
    
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <<T as pallet::Config>::Currency as frame_support::traits::Currency<
            <T as frame_system::Config>::AccountId,
        >>::Balance: From<u128> + Into<u128>,
        <T as parami_assets::Config>::Balance: From<u128> + Into<u128>,
        <T as parami_assets::Config>::AssetId: AtLeast32BitUnsigned,
    {
        #[pallet::weight(10_000)]
        pub fn create_class(origin: OriginFor<T>, metadata: Vec<u8>, token_type: TokenType, collection_type: CollectionType) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let next_class_id = NftModule::<T>::next_class_id();

            let class_fund: T::AccountId = T::PalletId::get().into_sub_account(next_class_id);
            let class_deposit = T::CreateClassDeposit::get();

            // Put fund to Class pool
            <T as Config>::Currency::transfer(&sender, &class_fund, class_deposit, KeepAlive)?;
            <T as Config>::Currency::reserve(&class_fund, <T as Config>::Currency::free_balance(&class_fund))?;

            let class_data = ClassData
            {
                deposit: class_deposit,
                token_type,
                collection_type,
                metadata: metadata.clone(),
                total_supply: Default::default(),
                initial_supply: Default::default(),
            };

            NftModule::<T>::create_class(&sender, metadata, class_data)?;

            Self::deposit_event(Event::<T>::NewNftClassCreated(sender, next_class_id));

            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::mint(* quantity))]
        pub fn mint(origin: OriginFor<T>, class_id: ClassIdOf<T>, name: Vec<u8>, symbol: Vec<u8>, description: Vec<u8>, metadata: Vec<u8>, quantity: u32) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin.clone())?;

            ensure!(quantity >= 1, Error::<T>::InvalidQuantity);
            let class_info = NftModule::<T>::classes(class_id).ok_or(Error::<T>::ClassIdNotFound)?;
            ensure!(sender == class_info.owner, Error::<T>::NoPermission);

            let deposit = T::CreateAssetDeposit::get();
            let class_fund: T::AccountId = T::PalletId::get().into_sub_account(class_id);
            let total_deposit = deposit * Into::<BalanceOf<T>>::into(quantity);

            <T as Config>::Currency::transfer(&sender, &class_fund, total_deposit, KeepAlive)?;
            <T as Config>::Currency::reserve(&class_fund, total_deposit)?;

            let ads_slot = AdsSlot {
                id: 1u32,
                start_time: <frame_system::Pallet<T>>::block_number(),
                end_time: <frame_system::Pallet<T>>::block_number(),
                deposit,
                media: description.clone(),

            };
            let new_nft_data = AssetData {
                deposit,
                name: name.clone(),
                description,
                slot: ads_slot,
            };

            let mut new_asset_ids: Vec<T::AssetId> = Vec::new();
            let mut last_token_id: TokenIdOf<T> = Default::default();

            for _ in 0..quantity {
                let asset_id = NextAssetId::<T>::try_mutate(|id| -> Result<T::AssetId, DispatchError> {
                    let current_id = *id;
                    *id = id.checked_add(&(Into::into(1u32))).ok_or(Error::<T>::NoAvailableAssetId)?;

                    Ok(current_id)
                })?;

                new_asset_ids.push(asset_id);

                if AssetsByOwner::<T>::contains_key(&sender) {
                    AssetsByOwner::<T>::try_mutate(
                        &sender,
                        |asset_ids| -> DispatchResult {
                            ensure!(!asset_ids.iter().any(|i| asset_id == *i), Error::<T>::AssetIdAlreadyExist);
                            asset_ids.push(asset_id);
                            Ok(())
                        },
                    )?;
                } else {
                    let mut assets = Vec::<T::AssetId>::new();
                    assets.push(asset_id);
                    AssetsByOwner::<T>::insert(&sender, assets)
                }

                let token_id = NftModule::<T>::mint(&sender, class_id, metadata.clone(), new_nft_data.clone())?;
                Assets::<T>::insert(asset_id, (class_id, token_id));
                last_token_id = token_id;

                // create fractional
                // let fractional_id = T::AssetId::from(asset_id as u32);
                <parami_assets::Pallet<T>>::create(
                    origin.clone(),
                    asset_id,
                    <T::Lookup as StaticLookup>::unlookup(sender.clone()),
                    MIN_BALANCE.into(),
                )?;

                // set metadata
                <parami_assets::Pallet<T>>::set_metadata(
                    origin.clone(),
                    asset_id,
                    name.clone(),
                    symbol.clone(),
                    18,
                )?;

                // initial supply
                <parami_assets::Pallet<T>>::mint(
                    origin.clone(),
                    asset_id,
                    <T::Lookup as StaticLookup>::unlookup(sender.clone()),
                    100_000_000u32.into(),
                )?;

                log::info!("fractional_id => {:?}", asset_id);
                log::info!("name => {:?}", name);
                log::info!("symbol => {:?}", symbol);
            }

            Self::deposit_event(Event::<T>::NewNftMinted(*new_asset_ids.first().unwrap(), *new_asset_ids.last().unwrap(), sender, class_id, quantity, last_token_id));

            Ok(().into())
        }

        #[pallet::weight(10_000)]
        pub fn transfer(origin: OriginFor<T>, to: T::AccountId, asset_id: T::AssetId) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            ensure!(!T::AssetsHandler::check_item_in_auction(asset_id),Error::<T>::AssetAlreadyInAuction);

            let token_id = Self::do_transfer(&sender, &to, asset_id)?;

            Self::deposit_event(Event::<T>::TransferedNft(sender, to, token_id));

            Ok(().into())
        }

        #[pallet::weight(10_000)]
        pub fn transfer_batch(origin: OriginFor<T>, tos: Vec<(T::AccountId, T::AssetId)>) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            for (_i, x) in tos.iter().enumerate() {
                let item = &x;
                let owner = &sender.clone();

                let asset = Assets::<T>::get(item.1).ok_or(Error::<T>::AssetIdNotFound)?;

                let class_info = NftModule::<T>::classes(asset.0).ok_or(Error::<T>::ClassIdNotFound)?;
                let data = class_info.data;

                match data.token_type {
                    TokenType::Transferable => {
                        let asset_info = NftModule::<T>::tokens(asset.0, asset.1).ok_or(Error::<T>::AssetInfoNotFound)?;
                        ensure!(owner.clone() == asset_info.owner, Error::<T>::NoPermission);
                        Self::handle_ownership_transfer(&owner, &item.0, item.1)?;
                        NftModule::<T>::transfer(&owner, &item.0, (asset.0, asset.1))?;
                        Self::deposit_event(Event::<T>::TransferedNft(owner.clone(), item.0.clone(), asset.1.clone()));
                    }
                    _ => ()
                };
            }

            Ok(().into())
        }

        #[pallet::weight(10_000)]
        pub fn sign_asset(origin: OriginFor<T>, asset_id: T::AssetId) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let asset_by_owner: Vec<T::AssetId> = Self::get_assets_by_owner(&sender);
            ensure!(!asset_by_owner.contains(&asset_id), Error::<T>::SignOwnAsset);

            if AssetSupporters::<T>::contains_key(&asset_id) {
                AssetSupporters::<T>::try_mutate(asset_id, |supporters| -> DispatchResult{
                    let supporters = supporters.as_mut().ok_or("Empty supporters")?;
                    supporters.push(sender);
                    Ok(())
                });
                Ok(().into())
            } else {
                let mut new_supporters = Vec::new();
                new_supporters.push(sender);
                AssetSupporters::<T>::insert(asset_id, new_supporters);
                Ok(().into())
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    impl<T: Config> Pallet<T> {
        pub fn handle_ownership_transfer(
            sender: &T::AccountId,
            to: &T::AccountId,
            asset_id: T::AssetId,
        ) -> DispatchResult {
            AssetsByOwner::<T>::try_mutate(&sender, |asset_ids| -> DispatchResult {
                let asset_index = asset_ids.iter().position(|x| *x == asset_id).unwrap();
                asset_ids.remove(asset_index);
    
                Ok(())
            })?;
    
            if AssetsByOwner::<T>::contains_key(to) {
                AssetsByOwner::<T>::try_mutate(&to, |asset_ids| -> DispatchResult {
                    ensure!(
                        !asset_ids.iter().any(|i| asset_id == *i),
                        Error::<T>::AssetIdAlreadyExist
                    );
                    asset_ids.push(asset_id);
                    Ok(())
                })?;
            } else {
                let mut asset_ids = Vec::<T::AssetId>::new();
                asset_ids.push(asset_id);
                AssetsByOwner::<T>::insert(&to, asset_ids);
            }
    
            Ok(())
        }

        pub fn do_transfer(
            sender: &T::AccountId,
            to: &T::AccountId,
            asset_id: T::AssetId) -> Result<<T as orml_nft::Config>::TokenId, DispatchError> {
            let asset = Assets::<T>::get(asset_id).ok_or(Error::<T>::AssetIdNotFound)?;
    
            let class_info = NftModule::<T>::classes(asset.0).ok_or(Error::<T>::ClassIdNotFound)?;
            let data = class_info.data;
    
            match data.token_type {
                TokenType::Transferable => {
                    let check_ownership = Self::check_ownership(&sender, &asset_id)?;
                    ensure!(check_ownership, Error::<T>::NoPermission);
    
                    Self::handle_ownership_transfer(&sender, &to, asset_id)?;
    
                    NftModule::<T>::transfer(&sender, &to, asset.clone())?;
                    Ok(asset.1)
                }
                TokenType::BoundToAddress => Err(Error::<T>::NonTransferable.into())
            }
        }

        pub fn check_ownership(
            sender: &T::AccountId,
            asset_id: &T::AssetId) -> Result<bool, DispatchError> {
            let asset = Assets::<T>::get(asset_id).ok_or(Error::<T>::AssetIdNotFound)?;
            let class_info = NftModule::<T>::classes(asset.0).ok_or(Error::<T>::ClassIdNotFound)?;
            let _data = class_info.data;
    
            let asset_info = NftModule::<T>::tokens(asset.0, asset.1).ok_or(Error::<T>::AssetInfoNotFound)?;
            if sender == &asset_info.owner {
                return Ok(true);
            }
    
            return Ok(false);
        }
    }
}
