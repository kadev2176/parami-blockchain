use crate::{mock::*, Config, Error, Linked, LinksOf, PendingOf, Registrar};
use codec::Decode;
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use parami_ocw::USER_AGENT;
use parami_traits::types::Network;
use sp_core::offchain::{testing, OffchainWorkerExt, TransactionPoolExt};

macro_rules! assert_tx {
    ($tx:tt, $call:expr) => {
        let tx = Extrinsic::decode(&mut &*$tx).unwrap();

        assert_eq!(tx.signature, None);

        assert_eq!(tx.call, $call);
    };
}
#[test]
fn should_link() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_ok!(Linker::insert_pending(
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
        ));

        let maybe_pending = <PendingOf<Test>>::get(Network::Telegram, &DID_ALICE);
        assert_ne!(maybe_pending, None);

        let pending = maybe_pending.unwrap();
        assert_eq!(pending.task, profile);
        assert_eq!(pending.deadline, <Test as Config>::PendingLifetime::get());

        assert_ok!(Linker::insert_link(
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            DID_ALICE,
        ));

        assert_eq!(<PendingOf<Test>>::get(Network::Telegram, &DID_ALICE), None);

        assert!(<Linked<Test>>::get(Network::Telegram, &profile));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, Network::Telegram),
            Some(profile)
        );
    })
}

#[test]
fn should_fail_when_exists() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Linker::insert_pending(DID_ALICE, Network::Polkadot, POLKA.to_vec()),
            Error::<Test>::Exists
        );

        assert_noop!(
            Linker::insert_pending(DID_BOB, Network::Polkadot, POLKA.to_vec()),
            Error::<Test>::Exists
        );
    })
}

#[test]
fn should_ocw_submit() {
    let (offchain, _) = testing::TestOffchainExt::new();
    let (pool, state) = testing::TestTransactionPoolExt::new();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    t.execute_with(|| {
        Linker::ocw_submit_link(DID_ALICE, Network::Telegram, Vec::<u8>::new(), false);

        let tx = state.write().transactions.pop().unwrap();

        assert_tx!(
            tx,
            Call::Linker(crate::Call::submit_link {
                did: DID_ALICE,
                site: Network::Telegram,
                profile: Vec::<u8>::new(),
                validated: false,
            })
        );
    });
}

#[test]
fn should_register() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::submit_register(Origin::signed(ALICE), CHARLIE));

        assert_ne!(Did::did_of(CHARLIE), None);
    })
}

#[test]
fn should_submit() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_ok!(Linker::submit_link(
            Origin::none(),
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            true,
        ));

        assert!(<Linked<Test>>::get(Network::Telegram, &profile));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, Network::Telegram),
            Some(profile)
        );
    })
}

#[test]
fn should_submit_when_pending() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_ok!(Linker::link_sociality(
            Origin::signed(ALICE),
            Network::Telegram,
            profile.clone(),
        ));

        assert_ne!(<PendingOf<Test>>::get(Network::Telegram, &DID_ALICE), None);

        assert_ok!(Linker::submit_link(
            Origin::none(),
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            false,
        ));

        assert_eq!(<PendingOf<Test>>::get(Network::Telegram, &DID_ALICE), None);
    })
}

#[test]
fn should_link_sociality() {
    let profile: String = "https://t.me/AmeliaParami".into();

    let (offchain, state) = testing::TestOffchainExt::new();
    let (pool, tx) = testing::TestTransactionPoolExt::new();

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: profile.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(Vec::new()),
            sent: true,
            ..Default::default()
        });
    }

    let profile = profile.as_bytes().to_vec();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    t.execute_with(|| {
        assert_ok!(Linker::link_sociality(
            Origin::signed(ALICE),
            Network::Telegram,
            profile.clone(),
        ));

        Linker::offchain_worker(0);

        Linker::offchain_worker(5);

        let tx = tx.write().transactions.pop().unwrap();

        assert_tx!(
            tx,
            Call::Linker(crate::Call::submit_link {
                did: DID_ALICE,
                site: Network::Telegram,
                profile: profile,
                validated: false,
            })
        );
    });
}

#[test]
fn should_verify_telegram() {
    const HTM: &[u8] = include_bytes!("../artifacts/telegram.html");
    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");

    let htm = HTM.to_vec();
    let jpg = JPG.to_vec();

    let profile: String = "https://t.me/AmeliaParami".into();
    let avatar: String = "https://cdn5.telesco.pe/file/did.jpg".into();

    let (offchain, state) = testing::TestOffchainExt::new();

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: profile.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(htm),
            sent: true,
            ..Default::default()
        });
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: avatar.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(jpg),
            sent: true,
            ..Default::default()
        });
    }

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    t.execute_with(|| {
        assert_ok!(Linker::ocw_verify_telegram(DID_ALICE, profile));
    });
}

#[test]
fn should_verify_twitter() {
    const HTM: &[u8] = include_bytes!("../artifacts/twitter.html");
    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");

    let htm = HTM.to_vec();
    let jpg = JPG.to_vec();

    let profile: String = "https://twitter.com/ParamiProtocol".into();
    let avatar: String = "https://pbs.twimg.com/profile_images/1380053132760125441/did.jpg".into();

    let (offchain, state) = testing::TestOffchainExt::new();

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: profile.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(htm),
            sent: true,
            ..Default::default()
        });
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: avatar.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(jpg),
            sent: true,
            ..Default::default()
        });
    }

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    t.execute_with(|| {
        assert_ok!(Linker::ocw_verify_twitter(DID_ALICE, profile));
    });
}

#[test]
fn should_link_crypto() {
    new_test_ext().execute_with(|| {
        let address = vec![0u8; 20];
        let signature = [0u8; 65];

        assert_ok!(Linker::link_crypto(
            Origin::signed(ALICE),
            Network::Unknown,
            address.clone(),
            signature,
        ));

        assert!(<Linked<Test>>::get(Network::Unknown, &address));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, Network::Unknown),
            Some(address)
        );
    });
}

#[test]
fn should_deposit_and_trust() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::deposit(Origin::signed(BOB), 10));

        assert_eq!(Balances::free_balance(&BOB), 90);
        assert_eq!(Balances::reserved_balance(BOB), 10);

        assert_ok!(Linker::force_trust(Origin::root(), DID_BOB));

        assert_eq!(<Registrar<Test>>::get(&DID_BOB), Some(true));

        assert_ok!(Linker::force_block(Origin::root(), DID_BOB));

        assert_eq!(<Registrar<Test>>::get(&DID_BOB), Some(false));

        assert_eq!(Balances::free_balance(&BOB), 90);
        assert_eq!(Balances::reserved_balance(BOB), 0);
    });
}

#[test]
fn should_link_via_registrar() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_noop!(
            Linker::submit_link(
                Origin::signed(BOB),
                DID_ALICE,
                Network::Telegram,
                profile.clone(),
                true
            ),
            Error::<Test>::Blocked
        );

        assert_ok!(Linker::deposit(Origin::signed(ALICE), 10));
        assert_ok!(Linker::force_trust(Origin::root(), DID_ALICE));

        assert_ok!(Linker::submit_link(
            Origin::signed(ALICE),
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            true
        ));

        assert_ok!(Linker::submit_score(
            Origin::signed(ALICE),
            DID_ALICE,
            b"telegram".to_vec(),
            50
        ));
    });
}

#[test]
fn should_force_unlink() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::force_unlink(
            Origin::root(),
            DID_ALICE,
            Network::Polkadot,
        ));
    })
}

#[test]
fn should_bind_by_linker() {
    let profile = b"https://t.me/AmeliaParami".to_vec();

    new_test_ext().execute_with(|| {
        assert_ok!(Linker::set_linker_account(Origin::root(), ALICE));
        assert_ok!(Linker::bind(
            Origin::signed(ALICE),
            DID_BOB,
            Network::Telegram,
            profile.clone()
        ));

        assert_eq!(Linked::<Test>::get(Network::Telegram, &profile), true);
        assert_eq!(
            LinksOf::<Test>::get(DID_BOB, Network::Telegram).unwrap(),
            profile
        );
    })
}

#[test]
fn fail_to_bind_if_origin_not_linker_account() {
    let profile = b"https://t.me/AmeliaParami".to_vec();

    new_test_ext().execute_with(|| {
        assert_noop!(
            Linker::bind(
                Origin::signed(ALICE),
                DID_BOB,
                Network::Telegram,
                profile.clone()
            ),
            Error::<Test>::NotAuthroized
        );
    })
}
