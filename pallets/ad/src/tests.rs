#![cfg(test)]

use super::{Event as AdEvent, *};
use frame_support::{assert_noop, assert_ok};
use crate::mock::{Event as MEvent, *};
use utils::test_helper::*;

fn signer(who: AccountId) -> sp_runtime::MultiSigner {
    sp_runtime::MultiSigner::from(
        sp_core::sr25519::Public(
            std::convert::TryInto::<[u8; 32]>::try_into(
                who.as_ref()
            ).unwrap()))
}

#[test]
fn create_advertiser_should_work() {
	ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Did::register(Origin::signed(ALICE), signer(ALICE), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 0));

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
        assert_ok!(Did::register(Origin::signed(ALICE), signer(ALICE), None));
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE), 0), Error::<Runtime>::NoAvailableId);
    });
}
