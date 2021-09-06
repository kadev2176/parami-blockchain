#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    ensure, dispatch::{DispatchResultWithPostInfo},
    traits::{Currency, Get, ReservableCurrency,},
    pallet_prelude::*, PalletId,
};
use frame_support::traits::tokens::fungibles::{Transfer};

use orml_auction::Pallet as AuctionModule;
use parami_nft::Pallet as NFTModule;

use frame_system::pallet_prelude::*;
use orml_traits::{Auction, OnNewBidResult, AuctionHandler, AssetHandler, Change,};
use primitives::{AuctionId, ItemId, AuctionItem, AuctionType, AuctionInfo, };
use parami_nft::Pallet as NFTPallet;
use sp_runtime::{traits::{AccountIdConversion, Zero},};

pub mod weights;

pub use weights::WeightInfo;

pub use pallet::*;

pub const PALLET_ID: PalletId = PalletId(*b"par/auct");

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::sp_runtime::traits::{CheckedSub,};

    #[pallet::config]
    pub trait Config:
        frame_system::Config +
        parami_nft::Config +
        parami_assets::Config +
        orml_auction::Config
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;
        
        /// Minimum duration
        type MinimumAuctionDuration: Get<Self::BlockNumber>;

        /// auction handler
        type Handler: AuctionHandler<Self::AccountId, BalanceOfAsset<Self>, Self::BlockNumber, AuctionId>;

        /// time to close auction
        type AuctionTimeToClose: Get<Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    type BalanceOfAsset<T> = <T as orml_auction::Config>::Balance;

    #[pallet::storage]
    #[pallet::getter(fn auctions)]
    pub(super) type Auctions<T: Config> = StorageMap<_, Blake2_128Concat, T::AuctionId, AuctionInfo<T::AccountId, BalanceOfAsset<T>, T::BlockNumber>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn assets_in_auction)]
    pub(super) type AssetsInAuction<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, bool, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_auction_item)]
    pub(super) type AuctionItems<T: Config> = StorageMap<_, Blake2_128Concat, T::AuctionId, AuctionItem<T::AccountId, T::BlockNumber, BalanceOfAsset<T>, T::AssetId>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn auction_end_time)]
    pub(super) type AuctionEndTime<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::BlockNumber, Blake2_128Concat, T::AuctionId, (), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    #[pallet::metadata(
        T::AccountId = "AccountId",
        T::AuctionId = "AuctionId",
        T::BlockNumber = "BlockNumber",
        BalanceOfAsset<T> = "BalanceOfAsset",
    )]
    pub enum Event<T: Config> {
        /// new auction created
        NewAuctionItem(T::AuctionId, T::AccountId, BalanceOfAsset<T>, BalanceOfAsset<T>, T::BlockNumber),
        Bid(T::AuctionId, T::AccountId, BalanceOfAsset<T>),
        AuctionFinalized(T::AuctionId, T::AccountId, BalanceOfAsset<T>),
        AuctionFinalizedNoBid(T::AuctionId),
    }

    #[pallet::error]
    pub enum Error<T> {
        AuctionEndIsLessThanMinimumDuration,
        AssetIsNotExist,
        NoPermissionToCreateAuction,
        AssetAlreadyInAuction,
        AuctionTypeIsNotSupported,
        AuctionNotExist,
        InvalidAuctionType,
        SelfBidNotAccepted,
        AuctionNotStarted,
        AuctionIsExpired,
        InvalidBidPrice,
        BidNotAccepted,
        InsufficientFunds,
    }
    
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as orml_auction::Config>::Balance: From<u128> + Into<u128>,
    {
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn create_new_auction(origin: OriginFor<T>, item_id: ItemId<T::AssetId>, value: BalanceOfAsset<T>, end_time: T::BlockNumber) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;

            let start_time: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

            let remaining_time: T::BlockNumber = end_time.checked_sub(&start_time).ok_or("Overflow")?;

            ensure!(remaining_time >= T::MinimumAuctionDuration::get(),
            Error::<T>::AuctionEndIsLessThanMinimumDuration);

            Self::create_auction(AuctionType::Auction, item_id, Some(end_time), from.clone(), value.clone(), start_time)?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn bid(origin: OriginFor<T>, id: T::AuctionId, value: BalanceOfAsset<T>) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin.clone())?;

            let auction_item: AuctionItem<T::AccountId, T::BlockNumber, BalanceOfAsset<T>, T::AssetId> = Self::get_auction_item(id.clone()).ok_or(Error::<T>::AuctionNotExist)?;
            ensure!(auction_item.auction_type == AuctionType::Auction, Error::<T>::InvalidAuctionType);
            ensure!(auction_item.recipient != from, Error::<T>::SelfBidNotAccepted);

            let block_number = <frame_system::Pallet<T>>::block_number();
            let auction = AuctionModule::<T>::auction_info(id).ok_or(Error::<T>::AuctionNotExist)?;
            AuctionModule::<T>::bid(origin.clone(), id, value)?;

            Self::auction_bid_handler(block_number, id, (from.clone(), value), auction.bid.clone())?;

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    impl<T: Config> Pallet<T> {
        pub fn auction_pool_id(asset_id: T::AssetId) -> T::AccountId {
            PALLET_ID.into_sub_account(asset_id)
        }

        fn create_auction(
            auction_type: AuctionType,
            item_id: ItemId<T::AssetId>,
            _end: Option<T::BlockNumber>,
            recipient: T::AccountId,
            initial_amount: BalanceOfAsset<T>,
            _start: T::BlockNumber,
        ) -> Result<T::AuctionId, DispatchError> {
            match item_id {
                ItemId::NFT(asset_id) => {
                    //Get asset detail
                    let asset = NFTPallet::<T>::get_asset(asset_id).ok_or(Error::<T>::AssetIsNotExist)?;
                    //Check ownership
                    let class_info = orml_nft::Pallet::<T>::classes(asset.0).ok_or(Error::<T>::NoPermissionToCreateAuction)?;
                    let class_info_data = class_info.data;
                    let token_info = orml_nft::Pallet::<T>::tokens(asset.0, asset.1).ok_or(Error::<T>::NoPermissionToCreateAuction)?;
                    ensure!(recipient == token_info.owner, Error::<T>::NoPermissionToCreateAuction);
                    ensure!(class_info_data.token_type.is_transferable(), Error::<T>::NoPermissionToCreateAuction);
                    ensure!(Self::assets_in_auction(asset_id) == None, Error::<T>::AssetAlreadyInAuction);

                    let start_time = <frame_system::Pallet<T>>::block_number();

                    let mut end_time = start_time + T::AuctionTimeToClose::get();
                    if let Some(_end_block) = _end {
                        end_time = _end_block
                    }
                    let auction_id = AuctionModule::<T>::new_auction(start_time, Some(end_time))?;

                    let new_auction_item = AuctionItem {
                        item_id,
                        recipient: recipient.clone(),
                        initial_amount: initial_amount,
                        amount: initial_amount,
                        start_time,
                        end_time,
                        auction_type,
                    };

                    <AuctionItems<T>>::insert(
                        auction_id,
                        new_auction_item,
                    );

                    <AssetsInAuction<T>>::insert(
                        asset_id,
                        true,
                    );

                    Self::deposit_event(Event::NewAuctionItem(auction_id, recipient, initial_amount, initial_amount, end_time));

                    Ok(auction_id)
                }
                _ => Err(Error::<T>::AuctionTypeIsNotSupported.into())
            }
        }

        fn remove_auction(id: T::AuctionId, item_id: ItemId<T::AssetId>) {
            if let Some(auction) = <Auctions<T>>::get(&id) {
                if let Some(end_block) = auction.end {
                    <AuctionEndTime<T>>::remove(end_block, id);
                    <Auctions<T>>::remove(&id);
                    match item_id {
                        ItemId::NFT(asset_id) => {
                            <AssetsInAuction<T>>::remove(asset_id);
                        }
                        _ => {}
                    }
                }
            }
        }

        fn auction_bid_handler(
            _now: T::BlockNumber,
            id: T::AuctionId,
            new_bid: (T::AccountId, BalanceOfAsset<T>),
            last_bid: Option<(T::AccountId, BalanceOfAsset<T>)>,
        ) -> DispatchResult {
            let (new_bidder, new_bid_price) = new_bid;
            ensure!(!new_bid_price.is_zero(), Error::<T>::InvalidBidPrice);

            <AuctionItems<T>>::try_mutate_exists(id, |auction_item| -> DispatchResult {
                let mut auction_item = auction_item.as_mut().ok_or("AuctionNotExist")?;

                let last_bid_price = last_bid.clone().map_or(Zero::zero(), |(_, price)| price); //get last bid price
                let last_bidder = last_bid.as_ref().map(|(who, _)| who);

                match auction_item.item_id {
                    ItemId::NFT(asset_id) => {

                        let auction_pool_id = Self::auction_pool_id(asset_id);

                        if let Some(last_bidder) = last_bidder {
                            // unlock reserve amount
                            if !last_bid_price.is_zero() {
                                //Unreserve balance of last bidder
                                <parami_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                                    asset_id,
                                    &auction_pool_id,
                                    &last_bidder,
                                    <T as parami_assets::Config>::Balance::from(
                                        last_bid_price.into()
                                    ),
                                    true
                                )?;
                            }
                        }
        
                        let new_bid_amount = <T as parami_assets::Config>::Balance::from(new_bid_price.into());
                        ensure!(<parami_assets::Pallet<T>>::balance(asset_id, &new_bidder) >= new_bid_amount, "InsufficientFunds");

                        // Lock fund of new bidder
                        <parami_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                            asset_id,
                            &new_bidder,
                            &auction_pool_id,
                            new_bid_amount,
                            true
                        )?;
                        auction_item.amount = new_bid_price.clone();

                    }
                    _ => {}
                }
            
                Ok(())
            })
        }
    }

    impl<T: Config> AuctionHandler<T::AccountId, BalanceOfAsset<T>, T::BlockNumber, T::AuctionId>
    for Pallet<T>
    {
        fn on_new_bid(
            _now: T::BlockNumber,
            _id: T::AuctionId,
            _new_bid: (T::AccountId, BalanceOfAsset<T>),
            _last_bid: Option<(T::AccountId, BalanceOfAsset<T>)>,
        ) -> OnNewBidResult<T::BlockNumber> {
            OnNewBidResult {
                accept_bid: true,
                auction_end_change: Change::NoChange,
            }
        }

        fn on_auction_ended(auction_id: T::AuctionId, winner: Option<(T::AccountId, BalanceOfAsset<T>)>) {
            if let Some(auction_item) = <AuctionItems<T>>::get(&auction_id) {
                Self::remove_auction(auction_id.clone(), auction_item.item_id);

                // ads list
                // 1.set ads slot 2.unreserve assets to ads pool
                // Transfer balance from high bidder to asset owner
                if let Some(current_bid) = winner {
                    let (high_bidder, high_bid_price): (T::AccountId, BalanceOfAsset<T>) = current_bid;
                    
                    match auction_item.item_id {
                        ItemId::NFT(asset_id) => {
                            let auction_pool_id = Self::auction_pool_id(asset_id);

                            <AssetsInAuction<T>>::remove(asset_id);

                            // set ads slot
                            // let asset_transfer = NFTModule::<T>::do_transfer(&auction_item.recipient, &high_bidder, asset_id);

                            // unreserve
                            let asset_transfer = <parami_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                                asset_id,
                                &auction_pool_id,
                                &high_bidder,
                                <T as parami_assets::Config>::Balance::from(
                                    high_bid_price.into()
                                ),
                                true
                            );

                            match asset_transfer {
                                Err(_) => (),
                                Ok(_) => {
                                    Self::deposit_event(Event::AuctionFinalized(auction_id, high_bidder, high_bid_price));
                                }
                            }
                        }
                        _ => {}
                    }
                } else {
                    Self::deposit_event(Event::AuctionFinalizedNoBid(auction_id));
                }
            }

        }
    }
}

impl<T: Config> AssetHandler<T::AssetId> for Pallet<T>
{
    fn check_item_in_auction(asset_id: T::AssetId) -> bool {
        if Self::assets_in_auction(asset_id) == Some(true) {
            return true;
        }
        return false;
    }
}