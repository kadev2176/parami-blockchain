#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok,};
use parami_nft::{TokenType, CollectionType};

use orml_auction::Pallet as OrmlAuction;
use orml_nft::Pallet as OrmlNft;

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