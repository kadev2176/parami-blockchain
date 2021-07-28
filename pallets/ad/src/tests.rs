#![cfg(test)]

use super::{Event as AdEvent, *};
use frame_support::{assert_noop, assert_ok};
use crate::mock::{Event as MEvent, *};
use utils::test_helper::*;

#[test]
fn create_advertiser_should_work() {
	ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 0));
        let advertiser = Advertisers::<Runtime>::get(d!(ALICE)).unwrap();

        let deposit = AdvertiserDeposit::<Runtime>::get();
        assert!(deposit > 0);
        assert_eq!(free_balance::<Runtime>(&advertiser.deposit_account), deposit);

        assert_last_event::<Runtime>(MEvent::Ad(
            AdEvent::CreatedAdvertiser(ALICE, d!(ALICE), advertiser_id)
        ));
    });
}

#[test]
fn create_advertiser_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE), 0), Error::<Runtime>::DIDNotExists);

        NextId::<Runtime>::put(GlobalId::MAX);
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE), 0), Error::<Runtime>::NoAvailableId);
    });
}

#[test]
fn create_ad_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 0));

        let ad_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3)]));

        let advertiser = Advertisers::<Runtime>::get(d!(ALICE)).unwrap();

        let deposit = AdDeposit::<Runtime>::get();
        assert!(deposit > 0);
        assert_eq!(reserved_balance::<Runtime>(&advertiser.deposit_account), deposit);

        let _ = Advertisements::<Runtime>::get(advertiser_id, ad_id).unwrap();
        assert_last_event::<Runtime>(MEvent::Ad(
            AdEvent::CreatedAd(d!(ALICE), advertiser_id, ad_id)
        ));
    });
}

#[test]
fn create_ad_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3),(4,4)]), Error::<Runtime>::InvalidTagCoefficientCount);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![]), Error::<Runtime>::InvalidTagCoefficientCount);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3)]), Error::<Runtime>::DIDNotExists);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (200, 2),(2,3)]), Error::<Runtime>::InvalidTagType);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(1,3)]), Error::<Runtime>::DuplicatedTagType);

        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3)]), Error::<Runtime>::AdvertiserNotExists);
    });
}
