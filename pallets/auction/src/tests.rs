#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok,};
use parami_nft::{TokenType, CollectionType};

use orml_auction::Pallet as OrmlAuction;
use orml_nft::Pallet as OrmlNft;
use parami_assets::Pallet as Assets;

fn init_test_nft(owner: Origin) {
    assert_ok!(NFTModule::<Runtime>::create_class(
        owner.clone(),
        vec![1],        
        TokenType::BoundToAddress,
        CollectionType::Collectable,
    ));

    assert_ok!(NFTModule::<Runtime>::mint(
        owner.clone(),
        CLASS_ID,
        vec![1],
        vec![1],
        vec![1],
        vec![1],
        1
    ));
}

#[test]
fn create_new_auction_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        init_test_nft(origin.clone());

        let nft_asset_data = OrmlNft::<Runtime>::tokens(CLASS_ID, TOKEN_ID);
        println!("nft data => {:?}", nft_asset_data);

        assert_ok!(AuctionPallet::create_new_auction(origin.clone(), ItemId::NFT(0), 50, 101));

        let auction = <orml_auction::Auctions::<Runtime>>::get(0);
        println!("auction => {:?}", auction);

        assert_eq!(AuctionPallet::assets_in_auction(0), Some(true));
    });
}

#[test]
fn create_auction_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let bob = Origin::signed(BOB);

        assert_ok!(NFTModule::<Runtime>::create_class(
            owner.clone(),
            vec![1],            
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));

        assert_ok!(NFTModule::<Runtime>::mint(
            owner.clone(),
            CLASS_ID,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        assert_ok!(NFTModule::<Runtime>::create_class(
            bob.clone(),
            vec![1],            
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));

        assert_ok!(NFTModule::<Runtime>::mint(
            bob.clone(),
            1,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        // have permission to create auction
        assert_noop!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(1), 50, 101), Error::<Runtime>::NoPermissionToCreateAuction);

        assert_ok!(NFTModule::<Runtime>::create_class(
            owner.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        assert_ok!(NFTModule::<Runtime>::mint(
            owner.clone(),
            2,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        // Not BoundToAddress
        assert_noop!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(2), 50, 101), Error::<Runtime>::NotBounded);

        // Asset is already in auction
        assert_ok!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101));
        assert_noop!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101), Error::<Runtime>::AssetAlreadyInAuction);
    });
}

#[test]
fn remove_auction_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        init_test_nft(origin.clone());
        assert_ok!(AuctionPallet::create_new_auction(origin.clone(), ItemId::NFT(0), 50, 101));

        AuctionPallet::remove_auction(0, ItemId::NFT(0));

        assert_eq!(OrmlAuction::<Runtime>::auctions(0), None);
        assert_eq!(AuctionPallet::assets_in_auction(0), None);
    });
}

#[test]
fn bid_works() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(BOB);
        let bidder = Origin::signed(ALICE);

        init_test_nft(owner.clone());

        // transfer asset to bidder
        assert_ok!(Assets::<Runtime>::transfer(
            owner.clone(),
            0,
            ALICE,
            10000,
        ));
        assert_eq!(Assets::<Runtime>::balance(0, ALICE), 10000);

        assert_ok!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101));

        assert_ok!(AuctionPallet::bid(bidder, 0, 200));
        assert_eq!(last_event(), mock::Event::AuctionPallet(crate::Event::Bid(0, 0, ALICE, 200)));

        let pool_account = AuctionPallet::auction_pool_id(0);
        assert_eq!(Assets::<Runtime>::balance(0, pool_account), 200);
    });
}

#[test]
fn cannot_bid_on_non_existent_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let bidder = Origin::signed(ALICE);

        assert_noop!(
            AuctionPallet::bid(bidder, 0, 100), 
            Error::<Runtime>::AuctionNotExist
        );
    });
}

#[test]
fn cannot_bid_with_insufficient_funds() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(BOB);
        // let bidder = Origin::signed(ALICE);

        init_test_nft(owner.clone());

        assert_ok!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101));

        assert_eq!(Assets::<Runtime>::balance(0, ALICE), 0);

        // assert_noop!(AuctionPallet::bid(bidder, 0, 200), Error::<Runtime>::InsufficientFunds);
    });
}

#[test]
fn cannot_bid_on_own_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);

        init_test_nft(owner.clone());

        assert_ok!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101));

        assert_noop!(
            AuctionPallet::bid(owner, 0, 50), 
            Error::<Runtime>::SelfBidNotAccepted
        );
    });
}

#[test]
fn ads_list_after_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(BOB);
        let bidder = Origin::signed(ALICE);

        init_test_nft(owner.clone());

        assert_ok!(Assets::<Runtime>::transfer(
            owner.clone(),
            0,
            ALICE,
            10000,
        ));
        assert_eq!(Assets::<Runtime>::balance(0, ALICE), 10000);

        assert_eq!(NFTModule::<Runtime>::get_assets_by_owner(BOB), [0]);

        assert_ok!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101));

        assert_ok!(AuctionPallet::bid(bidder, 0, 200));
        assert_eq!(last_event(), mock::Event::AuctionPallet(crate::Event::Bid(0, 0, ALICE, 200)));

        let pool_account = AuctionPallet::auction_pool_id(0);

        // pub some extra fund to pool for minimum balance reason
        assert_ok!(Assets::<Runtime>::transfer(
            owner.clone(),
            0,
            pool_account,
            1000,
        ));
        assert_eq!(Assets::<Runtime>::balance(0, pool_account), 1200);

        run_to_block(102);

        let ads_slot = parami_nft::AdsSlot::<u64, u128, u64> {
            start_time: 101,
            end_time: 201,
            deposit: 200,
            media: b"https://www.baidu.com".to_vec(),
            owner: ALICE,
        };
        assert_eq!(NFTModule::<Runtime>::get_ads_slot(0), Some(ads_slot));

        // Auction Finalized
        assert_eq!(
            last_event(),
            mock::Event::AuctionPallet(crate::Event::AuctionFinalized(0, 1, 200))
        );
    });
}

#[test]
fn cannot_bid_on_ended_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(BOB);
        let bidder = Origin::signed(ALICE);

        init_test_nft(owner.clone());
        assert_ok!(AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101));

        System::set_block_number(101);

        assert_noop!(
            AuctionPallet::bid(bidder, 0, 200), 
            orml_auction::Error::<Runtime>::AuctionIsExpired
        );
    });
}