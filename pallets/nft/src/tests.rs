use crate::{
    mock::*, AccountOf, ClaimStartAt, Deposits, Error, External, IcoMeta, IcoMetaOf,
    InflueceMiningMetaStore, InfluenceMiningMetaOf, Metadata, Ported, Porting, Preferred,
};

use codec::Decode;
use frame_support::{assert_err, assert_noop, assert_ok};
use parami_primitives::constants::DOLLARS;
use parami_traits::{transferable::Transferable, types::Network, Swaps};
use parking_lot::RwLock;
use sp_core::offchain::{testing, OffchainWorkerExt, TransactionPoolExt};
use sp_runtime::offchain::testing::PoolState;
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;
use std::sync::Arc;

#[test]
fn should_import() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x02];
        let did = DID_BOB;

        let _result = Linker::insert_link(did, Network::Ethereum, "something".into(), did);

        assert_ok!(Nft::port(
            Origin::signed(BOB),
            Network::Ethereum,
            namespace.clone(),
            token.clone(),
            SIGNING_ETH_ADDR.into(),
            SIGNATURE,
        ));

        let maybe_porting = <Porting<Test>>::get((Network::Ethereum, &namespace, &token));
        assert_ne!(maybe_porting, None);

        let porting = maybe_porting.unwrap();
        assert_eq!(porting.task.owner, DID_BOB);
        assert_eq!(porting.task.network, Network::Ethereum);
        assert_eq!(porting.task.namespace, namespace);
        assert_eq!(porting.task.token, token);
        assert_eq!(porting.deadline, 5);
        assert_eq!(porting.created, 0);
    });
}

#[test]
fn should_fail_when_imported() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x01];

        assert_noop!(
            Nft::port(
                Origin::signed(BOB),
                Network::Ethereum,
                namespace.clone(),
                token.clone(),
                SIGNING_ETH_ADDR.into(),
                SIGNATURE,
            ),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_fail_when_importing() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x02];
        let did = DID_BOB;

        let _result = Linker::insert_link(did, Network::Ethereum, "something".into(), did);

        assert_ok!(Nft::port(
            Origin::signed(BOB),
            Network::Ethereum,
            namespace.clone(),
            token.clone(),
            SIGNING_ETH_ADDR.into(),
            SIGNATURE,
        ));

        assert_noop!(
            Nft::port(
                Origin::signed(BOB),
                Network::Ethereum,
                namespace.clone(),
                token.clone(),
                SIGNING_ETH_ADDR.into(),
                SIGNATURE,
            ),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::kick(Origin::signed(BOB)));

        let maybe_nft = Nft::preferred(DID_BOB);
        assert_ne!(maybe_nft, None);

        let nft = maybe_nft.unwrap();

        let maybe_meta = <Metadata<Test>>::get(nft);
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();
        assert_eq!(meta.owner, DID_BOB);
        assert_eq!(meta.class_id, NEXT_INSTANCE_ID);
        assert_eq!(meta.minted, false);
        assert_eq!(meta.token_asset_id, NEXT_INSTANCE_ID);
    });
}

#[test]
fn pay_1000_ad3_should_elevate_token_price_by_1x() {
    new_test_ext().execute_with(|| {
        let required_ad3_amount = elevate_token_price_to_target(2 * DOLLARS);
        log::info!("required_ad3_amount is {}", required_ad3_amount);
        assert!(required_ad3_amount < 1000 * DOLLARS);
    });
}

//return required ad3 amount
fn elevate_token_price_to_target(target_ad3_amount_per_1000_token: u128) -> u128 {
    let nft = Nft::preferred(DID_ALICE).unwrap();

    assert_ok!(Nft::mint_nft_power(
        Origin::signed(ALICE),
        nft,
        b"Test Token".to_vec(),
        b"XTT".to_vec(),
        1_000_000 * DOLLARS
    ));

    assert_ok!(Swap::create(Origin::signed(ALICE), nft));
    assert_ok!(Swap::add_liquidity(
        Origin::signed(ALICE),
        nft,
        1000 * DOLLARS,
        1000 * DOLLARS,
        1_000_000 * DOLLARS,
        1
    ));

    let ad3_balance_of_bob_before_buying_token = Balances::free_balance(BOB);

    let mut ad3_amount_per_1000_token = Swap::token_out_dry(nft, 1000 * DOLLARS).unwrap();
    while ad3_amount_per_1000_token < target_ad3_amount_per_1000_token {
        Swap::buy_tokens(
            Origin::signed(BOB),
            nft,
            100_000 * DOLLARS,
            1000 * DOLLARS,
            100,
        )
        .unwrap();
        ad3_amount_per_1000_token = Swap::token_out_dry(nft, 1000 * DOLLARS).unwrap();
    }

    let ad3_balance_of_bob_after_buying_token = Balances::free_balance(BOB);

    ad3_balance_of_bob_before_buying_token - ad3_balance_of_bob_after_buying_token
}

#[test]
fn all_roles_claim_should_success_when_time_elapsed_100_percent() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec(),
            1_000_000 * DOLLARS
        ));

        assert_ok!(Nft::start_ico(
            Origin::signed(ALICE),
            nft,
            3000 * DOLLARS,
            1_000_000 * DOLLARS
        ));

        assert_ok!(Nft::participate_ico(
            Origin::signed(BOB),
            nft,
            666666666666666666666666
        ));
        assert_ok!(Nft::participate_ico(
            Origin::signed(CHARLIE),
            nft,
            333333333333333333333333
        ));
        assert_ok!(Nft::end_ico(Origin::signed(ALICE), nft));

        System::set_block_number(5);

        assert_ok!(Nft::claim(Origin::signed(BOB), nft));
        assert_ok!(Nft::claim(Origin::signed(CHARLIE), nft));

        assert_eq!(Assets::balance(nft, &BOB), 666666666666666666666333);
        assert_eq!(Assets::balance(nft, &CHARLIE), 333333333333333333333000);

        assert_eq!(<Deposits<Test>>::get(nft, &DID_BOB), None);
        assert_eq!(<Deposits<Test>>::get(nft, &DID_CHARLIE), None);
    });
}

#[test]
fn claim_should_success_when_claim_multi_times_in_same_block() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec(),
            1_000_000 * DOLLARS
        ));

        assert_ok!(Nft::start_ico(
            Origin::signed(ALICE),
            nft,
            3000 * DOLLARS,
            1_000_000 * DOLLARS
        ));

        assert_ok!(Nft::participate_ico(
            Origin::signed(BOB),
            nft,
            2000 * DOLLARS
        ));
        assert_ok!(Nft::participate_ico(
            Origin::signed(CHARLIE),
            nft,
            1000 * DOLLARS
        ));
        assert_ok!(Nft::end_ico(Origin::signed(ALICE), nft));

        System::set_block_number(1);
        assert_ok!(Nft::claim(Origin::signed(BOB), nft));
        assert_eq!(Assets::balance(nft, &BOB), 400 * DOLLARS);

        assert_ok!(Nft::claim(Origin::signed(BOB), nft));
        assert_eq!(Assets::balance(nft, &BOB), 400 * DOLLARS);

        assert_ok!(Nft::claim(Origin::signed(BOB), nft));
        assert_eq!(Assets::balance(nft, &BOB), 400 * DOLLARS);
    });
}

fn mock_validate_request(
    ether_endpoint: &str,
    body: String,
    response: &str,
) -> testing::PendingRequest {
    testing::PendingRequest {
        method: "POST".into(),
        uri: ether_endpoint.into(),
        sent: true,
        headers: vec![(
            "User-Agent".into(),
            "GoogleBot (compatible; ParamiWorker/1.0; +http://parami.io/worker/)".into(),
        )],
        body: body.into(),
        response: Some(response.into()),
        ..Default::default()
    }
}

fn offchain_execute(
    mock_requests: Vec<testing::PendingRequest>,
    test_executable: impl FnOnce(Arc<RwLock<PoolState>>) -> (),
) {
    let (offchain, state) = testing::TestOffchainExt::new();
    let (pool, pool_state) = testing::TestTransactionPoolExt::new();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    {
        let mut state = state.write();
        for r in mock_requests.into_iter() {
            state.expect_request(r);
        }
    }

    t.execute_with(|| test_executable(pool_state));
}

#[test]
fn should_success_when_validate_etherum_token_owner() {
    let ether_endpoint = "http://etherum.endpoint/example";
    let _links: &[Vec<u8>] = &[vec![
        219, 208, 68, 36, 49, 141, 30, 6, 179, 66, 89, 173, 214, 75, 241, 10, 142, 180, 90, 135,
    ]];
    let contract_address = b"contractaddress";
    let token = 546u64;

    let body = Nft::construct_request_body(contract_address, &token.to_be_bytes());
    let res = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000dbd04424318d1e06b34259add64bf10a8eb45a87"}"#;

    offchain_execute(
        vec![mock_validate_request(ether_endpoint.into(), body, res)],
        |_| {
            let result = Nft::ocw_validate_etherum_token_owner(
                ether_endpoint,
                b"contractaddress",
                &token.to_be_bytes(),
                &SIGNING_ETH_ADDR,
            );

            assert_ok!(result);
        },
    );
}

#[test]
fn should_fail_when_task_owner_not_token_owner() {
    let ether_endpoint = "http://etherum.endpoint/example";
    let _links: &[Vec<u8>] = &[[0; 32].into()];
    let contract_address = b"contractaddress";
    let token = 546u64;

    let body = Nft::construct_request_body(contract_address, &token.to_be_bytes());
    let res = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000dbd04424318d1e06b34259add64bf10a8eb45a87"}"#;

    offchain_execute(
        vec![mock_validate_request(ether_endpoint.into(), body, res)],
        |_| {
            let result = Nft::ocw_validate_etherum_token_owner(
                ether_endpoint,
                b"contractaddress",
                &token.to_be_bytes(),
                &[0xee; 20],
            );

            assert_noop!(result, Error::<Test>::NotTokenOwner);
        },
    );
}

// TODO: we should test with response with status code 400, however, substrate doesn't support mocking status code for now.
#[test]
fn should_fail_when_server_response_not_expected() {
    let ether_endpoint = "http://etherum.endpoint/example";
    let _links: &[Vec<u8>] = &[[0; 32].into()];
    let contract_address = b"contractaddress";
    let token = 546u64;

    let body = Nft::construct_request_body(contract_address, &token.to_be_bytes());
    let res = r#"{"jsonrpc":"2.0","id":1,"result":"invalid argument: xxxx"}"#;

    offchain_execute(
        vec![mock_validate_request(ether_endpoint.into(), body, res)],
        |_| {
            let result = Nft::ocw_validate_etherum_token_owner(
                ether_endpoint,
                b"contractaddress",
                &token.to_be_bytes(),
                &SIGNING_ETH_ADDR,
            );

            assert_noop!(result, Error::<Test>::OcwParseError);
        },
    );
}

#[test]
fn should_import_nft_by_ocw() {
    let ether_endpoint = "http://etherum.endpoint/example";
    let profile: Vec<u8> = vec![
        219, 208, 68, 36, 49, 141, 30, 6, 179, 66, 89, 173, 214, 75, 241, 10, 142, 180, 90, 135,
    ];
    let contract_address = b"contractaddress";
    let token = 546u64.to_be_bytes();

    let body = Nft::construct_request_body(contract_address, &token);
    let res = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000dbd04424318d1e06b34259add64bf10a8eb45a87"}"#;

    offchain_execute(
        vec![mock_validate_request(ether_endpoint.into(), body, res)],
        |pool_state| {
            let namespace = contract_address.to_vec();
            let did = DID_BOB;

            let _result = Linker::insert_link(did, Network::Ethereum, profile, did);

            assert_ok!(Nft::port(
                Origin::signed(BOB),
                Network::Ethereum,
                namespace.clone(),
                token.into(),
                SIGNING_ETH_ADDR.into(),
                SIGNATURE
            ));

            assert_ok!(Nft::ocw_begin_block(System::block_number()));

            let tx = pool_state.write().transactions.pop().unwrap();

            assert!(pool_state.read().transactions.is_empty());

            let tx = Extrinsic::decode(&mut &*tx).unwrap();

            assert_eq!(tx.signature, None);
            assert_eq!(
                tx.call,
                Call::Nft(crate::Call::submit_porting {
                    did: DID_BOB,
                    network: Network::Ethereum,
                    namespace: contract_address.to_vec(),
                    token: token.to_vec(),
                    validated: true
                })
            );
        },
    );
}

#[test]
fn should_sumbit_porting() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x22];
        let did = DID_BOB;

        let _result = Linker::insert_link(did, Network::Ethereum, "something".into(), did);

        assert_ok!(Nft::port(
            Origin::signed(BOB),
            Network::Ethereum,
            namespace.clone(),
            token.clone(),
            SIGNING_ETH_ADDR.into(),
            SIGNATURE,
        ));
        assert_ok!(Nft::submit_porting(
            frame_system::RawOrigin::None.into(),
            DID_BOB,
            Network::Ethereum,
            namespace.clone(),
            token.clone(),
            true,
        ));

        let token: &Vec<u8> = &token.into();
        assert!(<Porting<Test>>::get((Network::Ethereum, &namespace, token)).is_none());

        assert_eq!(
            <Ported<Test>>::get((Network::Ethereum, &namespace, token)).expect("should be ported"),
            // genesis config creates the first token, so we got 2 here.
            NEXT_INSTANCE_ID,
        );

        let external = <External<Test>>::get(NEXT_INSTANCE_ID).expect("external should have data");
        assert_eq!(external.owner, did);
        assert_eq!(external.network, Network::Ethereum);
        assert_eq!(external.namespace, namespace);
        assert_eq!(external.token, token.clone());

        let subaccount_id =
            <Test as crate::Config>::PalletId::get().into_sub_account_truncating(NEXT_INSTANCE_ID);

        let metadata = <Metadata<Test>>::get(NEXT_INSTANCE_ID).expect("meta should have data");
        assert_eq!(metadata.owner, did);
        assert_eq!(metadata.class_id, NEXT_INSTANCE_ID);
        assert_eq!(metadata.pot, subaccount_id);
        assert_eq!(metadata.minted, false);
        assert_eq!(metadata.token_asset_id, NEXT_INSTANCE_ID);

        let preferred = <Preferred<Test>>::get(DID_BOB).expect("prefered should have data");
        assert_eq!(preferred, NEXT_INSTANCE_ID);
    });
}

#[test]
fn should_transfer_all_assets() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            0,
            b"Test Token1".to_vec(),
            b"XTT1".to_vec(),
            500
        ));

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            1,
            b"Test Token2".to_vec(),
            b"XTT2".to_vec(),
            1000
        ));

        assert_eq!(Assets::balance(0, &ALICE), 500);
        assert_eq!(Assets::balance(1, &ALICE), 1000);
        assert_eq!(Assets::balance(0, &BOB), 0);
        assert_eq!(Assets::balance(1, &BOB), 0);

        assert_ok!(<(Nft,)>::transfer_all(&ALICE, &BOB));

        assert_eq!(Assets::balance(0, &ALICE), 0);
        assert_eq!(Assets::balance(1, &ALICE), 0);
        assert_eq!(Assets::balance(0, &BOB), 500);
        assert_eq!(Assets::balance(1, &BOB), 1000);
    });
}

#[test]
fn should_mint_asset() {
    use frame_support::traits::tokens::fungibles::Inspect;
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_eq!(Assets::balance(nft, &ALICE), 0);
        let result = Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"TT".to_vec(),
            200,
        );
        assert_ok!(result);
        assert_eq!(Assets::total_issuance(nft), 200);
        assert_eq!(Assets::balance(nft, ALICE), 200);
    });
}

#[test]
fn should_start_ico() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"TT".to_vec(),
            200,
        ));
        assert_eq!(Assets::balance(nft, ALICE), 200);
        assert_ok!(Nft::start_ico(Origin::signed(ALICE), nft, 100, 50));
        assert_eq!(Assets::balance(nft, ALICE), 150);

        let ico_meta = IcoMetaOf::<Test>::get(nft).unwrap();
        assert_eq!(ico_meta.done, false);
        assert_eq!(ico_meta.expected_currency, 100);
        assert_eq!(ico_meta.offered_tokens, 50);
        assert_eq!(Assets::balance(nft, ico_meta.pot), 50);
    });
}

#[test]
fn should_participate_ico() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        let result = Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"TT".to_vec(),
            200,
        );
        assert_ok!(result);
        assert_ok!(Nft::start_ico(Origin::signed(ALICE), nft, 100, 50));

        let meta = IcoMetaOf::<Test>::get(nft).unwrap();

        assert_eq!(Assets::balance(nft, meta.pot), 50);

        assert_eq!(Balances::free_balance(ALICE), 3000000 * DOLLARS);
        assert_eq!(Balances::free_balance(BOB), 3000000 * DOLLARS);
        assert_ok!(Nft::participate_ico(Origin::signed(BOB), nft, 50));
        assert_eq!(Balances::free_balance(BOB), 3000000 * DOLLARS - 100);
        assert_eq!(Balances::free_balance(ALICE), 3000000 * DOLLARS + 100);

        assert_eq!(Assets::balance(nft, meta.pot), 50);
    });
}

#[test]
fn should_end_ico() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        let result = Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"TT".to_vec(),
            200,
        );
        assert_ok!(result);
        assert_ok!(Nft::start_ico(Origin::signed(ALICE), nft, 50, 100));
        assert_ok!(Nft::participate_ico(Origin::signed(BOB), nft, 50));
        assert_ok!(Nft::end_ico(Origin::signed(ALICE), nft));

        let ico_meta = IcoMetaOf::<Test>::get(nft).unwrap();
        let meta = Metadata::<Test>::get(nft).unwrap();
        let block_num = ClaimStartAt::<Test>::get(nft).unwrap();
        assert_eq!(ico_meta.done, true);
        assert_eq!(Assets::balance(nft, meta.pot), 50);
        assert_eq!(block_num, 0);
    });
}

#[test]
fn should_generate_unique_pot_for_ico_meta() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();
        let meta_pot = Nft::generate_claim_pot(&nft);
        let ico_pot_1 = Nft::generate_ico_pot(&nft);
        let ico_pot_2 = Nft::generate_ico_pot(&(nft + 1));

        assert_ne!(meta_pot, ico_pot_1);
        assert_ne!(ico_pot_1, ico_pot_2);
    });
}

#[test]
fn should_calculate_price_correct() {
    new_test_ext().execute_with(|| {
        let required_currency = Nft::calculate_required_currency(
            50,
            &IcoMeta::<Test> {
                expected_currency: 100,
                offered_tokens: 50,
                done: true,
                pot: ALICE,
            },
        )
        .unwrap();

        assert_eq!(required_currency, 100);

        let required_currency = Nft::calculate_required_currency(
            50,
            &IcoMeta::<Test> {
                expected_currency: 99,
                offered_tokens: 50,
                done: true,
                pot: ALICE,
            },
        )
        .unwrap();

        assert_eq!(required_currency, 99);

        let required_currency = Nft::calculate_required_currency(
            49,
            &IcoMeta::<Test> {
                expected_currency: 99,
                offered_tokens: 50,
                done: true,
                pot: ALICE,
            },
        )
        .unwrap();

        assert_eq!(required_currency, 97);

        let required_currency = Nft::calculate_required_currency(
            1,
            &IcoMeta::<Test> {
                expected_currency: 99,
                offered_tokens: 50,
                done: true,
                pot: ALICE,
            },
        )
        .unwrap();

        assert_eq!(required_currency, 1);

        let required_token = Nft::calculate_required_token(
            1,
            &IcoMeta::<Test> {
                expected_currency: 99,
                offered_tokens: 50,
                done: true,
                pot: ALICE,
            },
        )
        .unwrap();

        assert_eq!(required_token, 0);
    });
}

#[test]
fn should_failed_if_ico_with_wrong_params() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Nft::mint_nft_power(
                Origin::signed(ALICE),
                10086,
                b"TT".to_vec(),
                b"TT".to_vec(),
                100,
            ),
            Error::<Test>::NotExists,
        );

        let nft = Nft::preferred(DID_ALICE).unwrap();
        assert_noop!(
            Nft::mint_nft_power(
                Origin::signed(BOB),
                nft,
                b"TT".to_vec(),
                b"TT".to_vec(),
                100,
            ),
            Error::<Test>::NotTokenOwner,
        );

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100,
        ));

        assert_noop!(
            Nft::mint_nft_power(
                Origin::signed(ALICE),
                nft,
                b"TT".to_vec(),
                b"TT".to_vec(),
                100,
            ),
            Error::<Test>::Minted,
        );
    });
}

#[test]
fn should_failed_to_participate_ico_if_with_wrong_params() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();
        assert_noop!(
            Nft::participate_ico(Origin::signed(ALICE), nft, 50),
            Error::<Test>::NotExists,
        );

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100,
        ));

        assert_ok!(Nft::start_ico(Origin::signed(ALICE), nft, 50, 50));
        assert_noop!(
            Nft::participate_ico(Origin::signed(ALICE), nft, 150),
            Error::<Test>::InsufficientToken
        );
        assert_ok!(Nft::end_ico(Origin::signed(ALICE), nft));

        assert_noop!(
            Nft::participate_ico(Origin::signed(ALICE), nft, 50),
            Error::<Test>::Deadline,
        );
    });
}

#[test]
fn should_failed_to_end_ico_if_with_wrong_params() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();
        assert_noop!(
            Nft::end_ico(Origin::signed(ALICE), nft),
            Error::<Test>::NotExists
        );

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100,
        ));

        assert_noop!(
            Nft::end_ico(Origin::signed(BOB), nft),
            Error::<Test>::NotTokenOwner
        );
    });
}

#[test]
fn should_claim_for_ico_meta() {
    use crate::Nfts;
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100 * DOLLARS,
        ));
        assert_ok!(Nft::start_ico(
            Origin::signed(ALICE),
            nft,
            20 * DOLLARS,
            20 * DOLLARS
        ));
        assert_ok!(Nft::participate_ico(Origin::signed(BOB), nft, 10 * DOLLARS));

        assert_ok!(Nft::end_ico(Origin::signed(ALICE), nft));

        let (total, unlocked, claimable) = Nft::get_claim_info(nft, &DID_BOB).unwrap();
        assert_eq!(total, 10 * DOLLARS);
        assert_eq!(unlocked, 0);
        assert_eq!(claimable, 0);

        // InitialMintingLockupPeriod is 5
        System::set_block_number(1);
        let (total, unlocked, claimable) = Nft::get_claim_info(nft, &DID_BOB).unwrap();
        assert_eq!(total, 10 * DOLLARS);
        assert_eq!(unlocked, 2 * DOLLARS);
        assert_eq!(claimable, 2 * DOLLARS);

        assert_ok!(Nft::claim(Origin::signed(BOB), nft));

        System::set_block_number(2);
        let (total, unlocked, claimable) = Nft::get_claim_info(nft, &DID_BOB).unwrap();
        assert_eq!(total, 10 * DOLLARS);
        assert_eq!(unlocked, 4 * DOLLARS);
        assert_eq!(claimable, 2 * DOLLARS);

        System::set_block_number(5);
        let (total, unlocked, claimable) = Nft::get_claim_info(nft, &DID_BOB).unwrap();
        assert_eq!(total, 10 * DOLLARS);
        assert_eq!(unlocked, 10 * DOLLARS);
        assert_eq!(claimable, 8 * DOLLARS);

        System::set_block_number(6);
        let (total, unlocked, claimable) = Nft::get_claim_info(nft, &DID_BOB).unwrap();
        assert_eq!(total, 10 * DOLLARS);
        assert_eq!(unlocked, 10 * DOLLARS);
        assert_eq!(claimable, 8 * DOLLARS);
    });
}

#[test]
fn should_start_influence_mining_activity() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100_000_000 * DOLLARS,
        ));

        let nft_meta = <Metadata<Test>>::get(nft).unwrap();

        //before stats
        let asset_balance_of_owner_before = Assets::balance(nft_meta.token_asset_id, ALICE);

        let pot_account: AccountOf<Test> = Nft::generate_influence_mining_pot(&nft);

        let asset_balance_of_pot_before = Assets::balance(nft_meta.token_asset_id, &pot_account);

        //action
        assert_ok!(Nft::start_dao_influenceming_activity(
            Origin::signed(ALICE),
            nft,
            100_000 * DOLLARS
        ));

        //after stats
        let asset_balance_after = Assets::balance(nft_meta.token_asset_id, ALICE);
        let asset_balance_of_pot_after = Assets::balance(nft_meta.token_asset_id, &pot_account);

        let meta: InfluenceMiningMetaOf<Test> = <InflueceMiningMetaStore<Test>>::get(nft).unwrap();
        assert_eq!(meta.budget_in_tokens, 100_000 * DOLLARS);
        assert_eq!(
            asset_balance_after,
            asset_balance_of_owner_before - meta.budget_in_tokens
        );
        assert_eq!(meta.pot, pot_account);
        assert_eq!(
            asset_balance_of_pot_after,
            asset_balance_of_pot_before + meta.budget_in_tokens
        );
        assert_eq!(meta.pot, pot_account);
    });
}

#[test]
fn should_not_start_influence_mining_activity_when_already_started() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100_000_000 * DOLLARS,
        ));

        //action
        assert_ok!(Nft::start_dao_influenceming_activity(
            Origin::signed(ALICE),
            nft,
            100_000 * DOLLARS
        ));

        assert_err!(
            Nft::start_dao_influenceming_activity(Origin::signed(ALICE), nft, 100_000 * DOLLARS),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_not_start_influence_mining_activity_when_origin_from_non_owner() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100_000_000 * DOLLARS,
        ));

        //action
        assert_noop!(
            Nft::start_dao_influenceming_activity(Origin::signed(BOB), nft, 100_000 * DOLLARS),
            Error::<Test>::NotTokenOwner
        );
    });
}

#[test]
fn should_not_start_influence_mining_activity_when_balance_not_enough() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::mint_nft_power(
            Origin::signed(ALICE),
            nft,
            b"TT".to_vec(),
            b"TT".to_vec(),
            100 * DOLLARS,
        ));

        //action
        assert_noop!(
            Nft::start_dao_influenceming_activity(Origin::signed(BOB), nft, 100_000 * DOLLARS),
            Error::<Test>::NotTokenOwner
        );
    });
}

#[test]
fn should_not_start_influence_mining_activity_when_nft_not_minted() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        //action
        assert_noop!(
            Nft::start_dao_influenceming_activity(Origin::signed(ALICE), nft, 100_000 * DOLLARS),
            Error::<Test>::NotExists
        );
    });
}
