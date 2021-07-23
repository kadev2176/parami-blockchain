#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use crate::mock::{Event, *};
use std::convert::TryInto;

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
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE)));
	});
}

#[test]
fn create_advertiser_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE)), Error::<Runtime>::DIDNotExists);
    });
}
