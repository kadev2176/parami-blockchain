use crate::{mock::*, ControllerAccountOf, Error, StableAccountOf};
use frame_support::{assert_noop, assert_ok};

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::free_balance(&BOB), 100);

        assert_ok!(Magic::create_stable_account(
            Origin::signed(BOB),
            MAGIC_BOB,
            10
        ));

        assert_eq!(Balances::free_balance(&BOB), 100 - 50 - 10);

        let maybe_stash = <StableAccountOf<Test>>::get(&BOB);
        assert_ne!(maybe_stash, None);

        let stash = maybe_stash.unwrap();
        assert_eq!(stash.controller_account, BOB);
        assert_eq!(stash.magic_account, MAGIC_BOB);

        assert_eq!(Balances::free_balance(&stash.stash_account), 10);
        assert_eq!(Balances::free_balance(&MAGIC_BOB), 50);

        assert_eq!(<ControllerAccountOf<Test>>::get(&MAGIC_BOB), Some(BOB));
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Magic::create_stable_account(Origin::signed(BOB), MAGIC_BOB, 100),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_fail_when_magic_is_controller() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Magic::create_stable_account(Origin::signed(BOB), BOB, 1),
            Error::<Test>::ControllerEqualToMagic
        );
    });
}

#[test]
fn should_transfer() {
    new_test_ext().execute_with(|| {
        assert_ok!(Magic::change_controller(Origin::signed(MAGIC_ALICE), BOB));

        assert_eq!(<StableAccountOf<Test>>::get(&ALICE), None);

        let maybe_stash = <StableAccountOf<Test>>::get(&BOB);
        assert_ne!(maybe_stash, None);

        let stash = maybe_stash.unwrap();
        assert_eq!(stash.controller_account, BOB);
        assert_eq!(stash.magic_account, MAGIC_ALICE);

        assert_eq!(Balances::free_balance(&stash.stash_account), 10);
        assert_eq!(Balances::free_balance(&MAGIC_ALICE), 50);
        assert_eq!(Balances::free_balance(&BOB), 100 + 40);

        assert_eq!(<ControllerAccountOf<Test>>::get(&MAGIC_ALICE), Some(BOB));
    });
}
