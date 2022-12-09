use crate::mock::*;
use crate::BucketClaimedSharesStore;
use crate::LastClockIn;
use crate::{Error, LotteryMetadataStore};
use frame_support::traits::fungibles::{Create, Mutate};
use frame_support::{assert_noop, assert_ok};
use parami_primitives::constants::DOLLARS;
use sp_core::sr25519;
use sp_runtime::traits::One;

#[test]
fn should_enable_clockin() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 100_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS,
            ],
            5,
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));
        let meta = <LotteryMetadataStore<Test>>::get(nft_id).unwrap();
        assert_eq!(meta.start_at, 10);
        assert_eq!(meta.asset_id, 1);
        assert_eq!(meta.level_probability, vec![10, 20, 30, 40, 50]);
        assert_eq!(
            meta.level_upper_bounds,
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS,
            ]
        );
        assert_eq!(meta.shares_per_bucket, 5);
        assert_eq!(meta.award_per_share, 100 * DOLLARS);
        assert_eq!(meta.bucket_size, 10);

        let pot = meta.pot;
        assert_eq!(Assets::balance(nft_id, pot), 5_000 * DOLLARS);
    });
}

#[test]
fn should_not_enable_clockin_when_level_param_invalid() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        {
            //for level_endpoints.length != level_probability.length
            assert_noop!(
                ClockIn::enable_clock_in(
                    Origin::signed(ALICE),
                    nft_id,
                    vec![10, 20, 30],
                    vec![
                        1_000 * DOLLARS,
                        10_000 * DOLLARS,
                        100_000 * DOLLARS,
                        1_000_000 * DOLLARS
                    ],
                    5,
                    100 * DOLLARS,
                    5_000 * DOLLARS,
                ),
                Error::<Test>::MetaParamInvalid
            );
        }

        {
            // for level_probability is empty
            assert_noop!(
                ClockIn::enable_clock_in(
                    Origin::signed(ALICE),
                    nft_id,
                    vec![],
                    vec![],
                    5,
                    100 * DOLLARS,
                    5_000 * DOLLARS,
                ),
                Error::<Test>::MetaParamInvalid
            );
        }
    });
}

#[test]
fn should_add_token_reward() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS,
            ],
            5,
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));

        let meta = <LotteryMetadataStore<Test>>::get(nft_id).unwrap();
        let pot = meta.pot;
        assert_eq!(Assets::balance(nft_id, pot.clone()), 5_000 * DOLLARS);
        assert_ok!(ClockIn::add_token_reward(
            Origin::signed(ALICE),
            nft_id,
            4_900 * DOLLARS
        ));
        assert_eq!(Assets::balance(nft_id, pot), 9_900 * DOLLARS);
    });
}

#[test]
fn should_update_clockin() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS,
            ],
            5,
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));

        assert_ok!(ClockIn::update_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 60],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                2_000_000 * DOLLARS
            ],
            5,
            100 * DOLLARS,
        ));

        let meta = <LotteryMetadataStore<Test>>::get(nft_id).unwrap();
        assert_eq!(meta.level_probability.get(4).unwrap().clone(), 60);
        assert_eq!(
            meta.level_upper_bounds.get(3).unwrap().clone(),
            2_000_000 * DOLLARS
        );
        //TODO(ironman_ch): where set this bucket_size
        assert_eq!(meta.bucket_size, 10);
    });
}

#[test]
fn should_disable_clockin() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS,
            ],
            5,
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));

        let before_balance = Assets::balance(nft_id, ALICE);
        assert_ok!(ClockIn::disable_clock_in(Origin::signed(ALICE), nft_id));
        let after_balance = Assets::balance(nft_id, ALICE);
        assert_eq!(before_balance + 5_000 * DOLLARS, after_balance);

        let meta = <LotteryMetadataStore<Test>>::get(nft_id);
        assert_eq!(meta, None);
    });
}

#[test]
fn generate_different_pot() {
    new_test_ext().execute_with(|| {
        let pot1 = crate::Pallet::<Test>::generate_reward_pot(&1u32);
        let pot2 = crate::Pallet::<Test>::generate_reward_pot(&2u32);
        assert_ne!(pot1, pot2);
    });
}

#[test]
fn should_clock_in() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS
            ],
            5,
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));

        let meta = <LotteryMetadataStore<Test>>::get(nft_id).unwrap();
        let pot = meta.pot;

        assert_eq!(Assets::balance(nft_id, pot.clone()), 5_000 * DOLLARS);
        let before_balance = Assets::balance(nft_id, BOB);

        System::set_parent_hash(create_parent_hash_with(9).into());

        assert_ok!(ClockIn::clock_in(Origin::signed(BOB), nft_id));
        let after_balance = Assets::balance(nft_id, BOB);
        assert_eq!(before_balance + 100 * DOLLARS, after_balance);
        assert_eq!(Assets::balance(nft_id, pot), 4_900 * DOLLARS);

        let last_clock_in_bucket = LastClockIn::<Test>::get(nft_id, DID_BOB);
        assert_eq!(last_clock_in_bucket, 1);

        let claimed_shares = <BucketClaimedSharesStore<Test>>::get(nft_id, last_clock_in_bucket);
        assert_eq!(claimed_shares, 1);
    });
}

#[test]
fn should_clock_in_when_shares_per_bucket_arrives() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        let shares_per_bucket: usize = 2;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![
                1_000 * DOLLARS,
                10_000 * DOLLARS,
                100_000 * DOLLARS,
                1_000_000 * DOLLARS,
                10_000_000 * DOLLARS
            ],
            shares_per_bucket.try_into().unwrap(),
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));

        System::set_parent_hash(create_parent_hash_with(9).into());

        for i in 0usize..shares_per_bucket {
            let publics_inner = public_keys();
            let cur_public = publics_inner.get(i).unwrap();
            assert_ok!(ClockIn::clock_in(Origin::signed(*cur_public), nft_id));
        }

        let last_clock_in_bucket = LastClockIn::<Test>::get(nft_id, DID_BOB);
        let claimed_shares = <BucketClaimedSharesStore<Test>>::get(nft_id, last_clock_in_bucket);
        assert_eq!(claimed_shares, u32::try_from(shares_per_bucket).unwrap());

        let balance_before = <Test as parami_nft::Config>::Assets::balance(nft_id, FREDIE);
        assert_ok!(ClockIn::clock_in(Origin::signed(CHARLIE), nft_id));
        let balance_after = <Test as parami_nft::Config>::Assets::balance(nft_id, FREDIE);
        assert_eq!(balance_before, balance_after);
        let claimed_shares = <BucketClaimedSharesStore<Test>>::get(nft_id, last_clock_in_bucket);
        assert_eq!(claimed_shares, u32::try_from(shares_per_bucket).unwrap());
    });
}

#[test]
fn failed_to_clock_in_if_clocked_in_this_bucket() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![1_000, 10_000, 100_000, 1_000_000, 10_000_000 * DOLLARS],
            5,
            100 * DOLLARS,
            5_000 * DOLLARS,
        ));

        assert_ok!(ClockIn::clock_in(Origin::signed(BOB), nft_id));
        System::set_block_number(15);
        assert_noop!(
            ClockIn::clock_in(Origin::signed(BOB), nft_id),
            Error::<Test>::ClockedIn
        );

        System::set_block_number(20);
        assert_ok!(ClockIn::clock_in(Origin::signed(BOB), nft_id));
    });
}

#[test]
fn failed_to_clock_in_if_balance_not_enough() {
    new_test_ext().execute_with(|| {
        let nft_id = 1;
        System::set_block_number(10);
        <Assets as Create<<Test as frame_system::Config>::AccountId>>::create(
            nft_id,
            ALICE,
            true,
            One::one(),
        )
        .unwrap();
        <Test as parami_nft::Config>::Assets::mint_into(nft_id, &ALICE, 10_000 * DOLLARS).unwrap();

        assert_ok!(ClockIn::enable_clock_in(
            Origin::signed(ALICE),
            nft_id,
            vec![10, 20, 30, 40, 50],
            vec![1_000, 10_000, 100_000, 1_000_000, 10_000_000],
            5,
            100 * DOLLARS,
            100 * DOLLARS,
        ));

        let parent_hash = create_parent_hash_with(9);
        System::set_parent_hash(parent_hash.into());

        assert_ok!(ClockIn::clock_in(Origin::signed(BOB), nft_id));

        System::set_block_number(109);

        let parent_hash = create_parent_hash_with(9);
        System::set_parent_hash(parent_hash.into());

        assert_noop!(
            ClockIn::clock_in(Origin::signed(BOB), nft_id),
            Error::<Test>::InsufficientToken
        );
    });
}

fn create_parent_hash_with(last_u8: u8) -> [u8; 32] {
    let mut parent_hash = System::parent_hash();
    let parent_hash: &mut [u8] = parent_hash.as_mut();
    parent_hash[parent_hash.len() - 1] = last_u8;
    let parent_hash: [u8; 32] = parent_hash.try_into().unwrap();
    parent_hash
}

fn public_keys() -> Vec<sr25519::Public> {
    return vec![ALICE, BOB, CHARLIE, DAVE, EVA];
}
