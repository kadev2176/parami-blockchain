use crate::{mock::*, AdsOf, Config, DeadlineOf, EndtimeOf, Error, Metadata, SlotOf};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use parami_traits::Tags;
use sp_runtime::traits::Hash;
use sp_runtime::MultiAddress;
use sp_std::collections::btree_map::BTreeMap;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
        ];

        let mut hashes = BTreeMap::new();
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.insert(hash, true);
        }

        let metadata = vec![0u8; 64];

        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            tags,
            metadata.clone(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        assert_eq!(<AdsOf<Test>>::get(&DID_ALICE).unwrap().len(), 1);

        let maybe_ad = <Metadata<Test>>::iter().next();
        assert_ne!(maybe_ad, None);

        let (ad, meta) = maybe_ad.unwrap();
        assert_eq!(meta.creator, DID_ALICE);
        assert_eq!(meta.metadata, metadata);
        assert_eq!(meta.reward_rate, 1);
        assert_eq!(meta.created, 0);

        assert_eq!(<EndtimeOf<Test>>::get(&ad), Some(1));

        assert_eq!(<Test as Config>::Tags::tags_of(&ad), hashes);
    });
}

#[test]
fn should_fail_when_min_greater_than_max() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
        ];

        let mut hashes = BTreeMap::new();
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.insert(hash, true);
        }

        let metadata = vec![0u8; 64];

        assert_noop!(
            Ad::create(
                Origin::signed(ALICE),
                tags,
                metadata.clone(),
                1,
                1,
                1u128,
                11u128,
                10u128
            ),
            Error::<Test>::WrongPayoutSetting
        );
    });
}

#[test]
fn should_fail_when_tag_not_exists() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8; 6],
        ];

        assert_noop!(
            Ad::create(
                Origin::signed(ALICE),
                tags,
                [0u8; 64].into(),
                1,
                1,
                1u128,
                0,
                10u128
            ),
            Error::<Test>::TagNotExists
        );
    });
}

#[test]
fn should_update_reward_rate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            vec![],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_reward_rate(Origin::signed(ALICE), ad, 2));

        assert_eq!(<Metadata<Test>>::get(&ad).unwrap().reward_rate, 2);
    });
}

#[test]
fn should_fail_when_not_exists_or_not_owned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Ad::update_reward_rate(Origin::signed(ALICE), Default::default(), 2),
            Error::<Test>::NotExists
        );

        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            vec![],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_noop!(
            Ad::update_reward_rate(Origin::signed(BOB), ad, 2),
            Error::<Test>::NotOwned
        );
    });
}

#[test]
fn should_update_tags() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
        ];

        let mut hashes = BTreeMap::new();
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.insert(hash, true);
        }

        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_tags(Origin::signed(ALICE), ad, tags));

        assert_eq!(<Test as Config>::Tags::tags_of(&ad), hashes);
    });
}

#[test]
fn should_generate_unique_slot_pot() {
    new_test_ext().execute_with(|| {
        let pot1 = Ad::generate_slot_pot(0);
        let pot2 = Ad::generate_slot_pot(1);

        assert_ne!(pot1, pot2);
    });
}

#[test]
fn should_bid() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();
        let meta = Nft::meta(nft).unwrap();
        let endtime = 43200;

        // ad1
        assert_ok!(Ad::create(
            Origin::signed(BOB),
            vec![],
            [0u8; 64].into(),
            1,
            endtime,
            1u128,
            0,
            10u128
        ));

        let ad1 = <Metadata<Test>>::iter_keys().next().unwrap();

        // 2. bob bid for ad1

        let slot = <SlotOf<Test>>::get(nft);
        assert_eq!(slot, None);

        let bob_bid_fraction = 400;

        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(BOB),
            ad1,
            nft,
            bob_bid_fraction,
            None,
            None
        ));

        // ensure: deadline, slot, remain
        assert_eq!(<EndtimeOf<Test>>::get(&ad1), Some(endtime));
        assert_eq!(<DeadlineOf<Test>>::get(nft, &ad1), Some(endtime));

        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(slot.ad_id, ad1);

        // 3. charlie bid for ad2
        // ad2

        assert_ok!(Ad::create(
            Origin::signed(CHARLIE),
            vec![],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        let ad2 = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_noop!(
            Ad::bid_with_fraction(
                Origin::signed(CHARLIE),
                ad2,
                nft,
                bob_bid_fraction.saturating_mul(120).saturating_div(100),
                None,
                None
            ),
            Error::<Test>::Underbid
        );

        assert_eq!(
            Assets::balance(meta.token_asset_id, CHARLIE),
            CHARLIE_BALANCE
        );
        let charlie_bid_fraction = bob_bid_fraction
            .saturating_mul(120)
            .saturating_div(100)
            .saturating_add(1);
        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(CHARLIE),
            ad2,
            nft,
            charlie_bid_fraction,
            None,
            None
        ));
        assert_eq!(
            Assets::balance(meta.token_asset_id, CHARLIE),
            CHARLIE_BALANCE - charlie_bid_fraction
        );

        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(slot.ad_id, ad2);

        let locked_fraction = Assets::balance(meta.token_asset_id, slot.budget_pot);
        assert_eq!(locked_fraction, charlie_bid_fraction);

        // ensure: deadline, slot, remain

        assert_eq!(<EndtimeOf<Test>>::get(&ad2), Some(1));
        assert_eq!(<DeadlineOf<Test>>::get(nft, &ad1), None);
        assert_eq!(<DeadlineOf<Test>>::get(nft, &ad2), Some(1));
    });
}

#[test]
fn should_fail_to_add_budget_when_fungible_not_same_with_bid() {
    new_test_ext().execute_with(|| {
        assert_ok!(Assets::force_create(
            Origin::root(),
            9,
            MultiAddress::Id(BOB),
            true,
            1
        ));
        let fungible_id = 9;
        assert_ok!(Assets::mint(
            Origin::signed(BOB),
            fungible_id,
            MultiAddress::Id(BOB),
            1000
        ));

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            vec![],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        let nft = Nft::preferred(DID_ALICE).unwrap();
        let ad = <Metadata<Test>>::iter_keys().next().unwrap();
        let bob_bid_fraction = 250;

        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(BOB),
            ad,
            nft,
            bob_bid_fraction,
            None,
            None
        ));
        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(Ad::slot_current_fraction_balance(&slot), bob_bid_fraction);

        let new_budget = 250;
        let new_fungibles = 123;
        assert_noop!(
            Ad::add_budget(
                Origin::signed(BOB),
                ad,
                nft,
                new_budget,
                Some(fungible_id),
                Some(new_fungibles)
            ),
            Error::<Test>::FungibleNotForSlot
        );
    });
}

#[test]
fn should_add_budget() {
    new_test_ext().execute_with(|| {
        assert_ok!(Assets::force_create(
            Origin::root(),
            9,
            MultiAddress::Id(BOB),
            true,
            1
        ));
        let fungible_id = 9;
        assert_ok!(Assets::mint(
            Origin::signed(BOB),
            fungible_id,
            MultiAddress::Id(BOB),
            1000
        ));

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            vec![],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        let nft = Nft::preferred(DID_ALICE).unwrap();
        let ad = <Metadata<Test>>::iter_keys().next().unwrap();
        let bob_bid_fraction = 250;
        let bob_bid_fungible = 100;

        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(BOB),
            ad,
            nft,
            bob_bid_fraction,
            Some(fungible_id),
            Some(bob_bid_fungible)
        ));
        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(Ad::slot_current_fraction_balance(&slot), bob_bid_fraction);

        let new_budget = 250;
        let new_fungibles = 123;
        assert_ok!(Ad::add_budget(
            Origin::signed(BOB),
            ad,
            nft,
            new_budget,
            Some(fungible_id),
            Some(new_fungibles)
        ));
        assert_eq!(
            Assets::balance(slot.fungible_id.unwrap(), slot.budget_pot),
            bob_bid_fungible + new_fungibles
        );
        assert_eq!(
            Assets::balance(slot.fraction_id, BOB),
            BOB_BALANCE - bob_bid_fraction - new_budget
        );

        assert_eq!(
            Ad::slot_current_fraction_balance(&slot),
            bob_bid_fraction + new_budget
        );
    });
}

#[test]
fn should_drawback_when_ad_expired() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();
        let meta = Nft::meta(nft).unwrap();

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            vec![],
            [0u8; 64].into(),
            1,
            43200 * 2,
            1u128,
            0,
            10u128
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid

        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(BOB),
            ad,
            nft,
            400,
            None,
            None
        ));
        assert_eq!(Assets::balance(meta.token_asset_id, BOB), 100);

        // 2. step in

        System::set_block_number(43200);

        Ad::on_initialize(System::block_number());

        // ensure slot, remain

        assert_eq!(<SlotOf<Test>>::get(nft), None);

        // 3. step in
        System::set_block_number(43200 * 2);

        Ad::on_initialize(System::block_number());

        // ensure remain
        assert_eq!(Assets::balance(meta.token_asset_id, BOB), 500);
    });
}
macro_rules! prepare_pay {
    ($a:expr,$b:expr,$c: expr) => {
        _prepare_pay($a, $b, $c)
    };

    () => {
        _prepare_pay(1u128, 0u128, 10u128)
    };
}

type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type NftOf<T> = <T as parami_nft::Config>::AssetId;
fn _prepare_pay(base: u128, min: u128, max: u128) -> (HashOf<Test>, NftOf<Test>) {
    // 1. prepare

    let nft = Nft::preferred(DID_ALICE).unwrap();
    // create ad

    assert_ok!(Ad::create(
        Origin::signed(BOB),
        vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8]
        ],
        [0u8; 64].into(),
        1,
        1,
        base,
        min,
        max
    ));

    let ad = <Metadata<Test>>::iter_keys().next().unwrap();

    // bid

    assert_ok!(Ad::bid_with_fraction(
        Origin::signed(BOB),
        ad,
        nft,
        400,
        None,
        None
    ));

    return (ad, nft);
}

#[test]
fn should_pay() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay!();

        // 2. pay

        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_CHARLIE,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        let nft_meta = Nft::meta(nft).unwrap();
        assert_eq!(Assets::balance(nft_meta.token_asset_id, &CHARLIE), 502);

        assert_eq!(
            Tag::get_score(&DID_CHARLIE, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8]),
            11
        );
    });
}

#[test]
fn should_pay_3_for_taga5_tagb2() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay!();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA5_TAGB2,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA5_TAGB2), 3);
    });
}

#[test]
fn should_pay_0_when_all_tags_score_are_zero() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay!();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA0_TAGB0,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA0_TAGB0), 0);
    });
}

#[test]
fn should_pay_5_when_all_tags_score_are_zero_with_payout_min_is_5() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay!(1u128, 5u128, 10u128);
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA0_TAGB0,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA0_TAGB0), 5);
    });
}
#[test]
fn should_pay_10_when_all_tags_are_full_score() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay!();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA100_TAGB100,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(
            Assets::balance(nft_meta.token_asset_id, &TAGA100_TAGB100),
            10
        );
    });
}
#[test]
fn should_pay_10_when_all_tags_are_full_score_or_overflow() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay!();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA120_TAGB0,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA120_TAGB0), 10);
    });
}

#[test]
fn should_pay_dual() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8]],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        assert_ok!(Assets::force_create(
            Origin::root(),
            9,
            MultiAddress::Id(BOB),
            true,
            1
        ));
        assert_ok!(Assets::mint(
            Origin::signed(BOB),
            9,
            MultiAddress::Id(BOB),
            1000
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid
        assert_eq!(Assets::balance(9, BOB), 1000);
        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(BOB),
            ad,
            nft,
            400,
            Some(9),
            Some(400)
        ));

        // 2. pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_CHARLIE,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(slot.fungible_id, Some(9));
        assert_eq!(Assets::balance(9, &CHARLIE), 5);
    });
}

#[test]
fn should_pay_failed() {
    use sp_runtime::MultiAddress;

    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8]],
            [0u8; 64].into(),
            1,
            1,
            1u128,
            0,
            10u128
        ));

        assert_ok!(Assets::force_create(
            Origin::root(),
            9,
            MultiAddress::Id(BOB),
            true,
            1
        ));
        assert_ok!(Assets::mint(
            Origin::signed(BOB),
            9,
            MultiAddress::Id(BOB),
            1000
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid

        assert_ok!(Ad::bid_with_fraction(
            Origin::signed(BOB),
            ad,
            nft,
            13,
            Some(9),
            Some(13)
        ));

        // 2. pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_CHARLIE,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_noop!(
            Ad::pay(
                Origin::signed(BOB),
                ad,
                nft,
                DID_TAGA100_TAGB100,
                vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
                None
            ),
            Error::<Test>::InsufficientFractions
        );
    });
}
