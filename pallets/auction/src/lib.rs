#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    ensure, dispatch::{DispatchResultWithPostInfo},
    traits::{Currency, Get, ReservableCurrency,},
    pallet_prelude::*,
};

use orml_auction::Pallet as AuctionModule;

use frame_system::pallet_prelude::*;
use orml_traits::{Auction, OnNewBidResult, AuctionHandler, Change,};
use primitives::{AssetId, AuctionId, ItemId};

use sp_std::{vec::Vec,};

pub mod weights;

pub use weights::WeightInfo;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::sp_runtime::traits::{CheckedSub,};

    #[pallet::config]
    pub trait Config:
        frame_system::Config +
        pallet_timestamp::Config +
        orml_auction::Config
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;
        
        /// Minimum duration
        type MinimumAuctionDuration: Get<Self::BlockNumber>;
    }

    pub(super) type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn get_auction)]
    pub(super) type Auctions<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetId, Vec<T::AccountId>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    #[pallet::metadata(
    < T as frame_system::Config >::AccountId = "AccountId",
    <T as orml_auction::Config>::AuctionId = "AuctionId",
    )]
    pub enum Event<T: Config> {
        /// new auction created
        NewAuctionItem(T::AuctionId, T::AccountId, BalanceOf<T>, BalanceOf<T>, T::BlockNumber),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// auction end error
        AuctionEndIsLessThanMinimumDuration,
    }
    
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <<T as pallet::Config>::Currency as frame_support::traits::Currency<
            <T as frame_system::Config>::AccountId,
        >>::Balance: From<u128> + Into<u128>,
    {
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn create_new_auction(origin: OriginFor<T>, _item_id: ItemId, value: BalanceOf<T>, end_time: T::BlockNumber) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;

            let start_time: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

            let remaining_time: T::BlockNumber = end_time.checked_sub(&start_time).ok_or("Overflow")?;

            ensure!(remaining_time >= T::MinimumAuctionDuration::get(),
            Error::<T>::AuctionEndIsLessThanMinimumDuration);

            let auction_id = AuctionModule::<T>::new_auction(start_time, Some(end_time))?;
            Self::deposit_event(Event::NewAuctionItem(auction_id, from, value, value, end_time));

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    impl<T: Config> Pallet<T> {}

    impl<T: Config> AuctionHandler<T::AccountId, BalanceOf<T>, T::BlockNumber, AuctionId>
    for Pallet<T>
    {
        fn on_new_bid(
            _now: T::BlockNumber,
            _id: AuctionId,
            _new_bid: (T::AccountId, BalanceOf<T>),
            _last_bid: Option<(T::AccountId, BalanceOf<T>)>,
        ) -> OnNewBidResult<T::BlockNumber> {
            OnNewBidResult {
                accept_bid: true,
                auction_end_change: Change::NoChange,
            }
        }

        fn on_auction_ended(_id: AuctionId, _winner: Option<(T::AccountId, BalanceOf<T>)>) {}
    }
}
