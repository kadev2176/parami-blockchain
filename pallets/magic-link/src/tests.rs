#![cfg(test)]

use super::{Event as AdEvent, *};
use crate::mock::{Event as MEvent, *};
use frame_support::{assert_noop, assert_ok};
use sp_core::Pair;

#[test]
fn create_advertiser_should_work() {
	ExtBuilder::default().build().execute_with(|| {});
}
