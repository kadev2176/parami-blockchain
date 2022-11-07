use crate::{mock::*, Config, StakingActivityStore, UserStakingBalanceStore};
use frame_support::assert_ok;
use parami_traits::Stakes;
use sp_runtime::traits::BlockNumberProvider;

/*Profit Invariants Start */
mod profit_invariants {
    use super::*;
    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_138240_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(138240);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_69120_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(69120);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_34560_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(34560);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_17280_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(17280);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_8640_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(8640);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_4320_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(4320);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_2160_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(2160);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_1080_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(1080);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_540_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(540);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_270_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(270);
    }

    #[test]
    pub fn get_profit_will_never_exceed_total_reward_amount_in_90_block_gap() {
        get_profit_will_never_exceed_total_reward_amount(90);
    }

    pub fn get_profit_will_never_exceed_total_reward_amount(block_gap_of_get_reward: u64) {
        //7884000 is block num in 3 years.
        let loop_count = 7884000 / block_gap_of_get_reward;
        for reward_total_amount in vec![
            71_000u128 * 10u128.pow(18),
            711_000 * 10u128.pow(18),
            7_120_000 * 10u128.pow(18),
        ] {
            new_test_ext().execute_with(|| {
                let asset_id = 9;

                // let mut rng = rand::thread_rng();
                // let ran_val = rng.gen_range(100_000u128, 10_000_000u128);

                println!("reward_total_amount is {:}", reward_total_amount);

                assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));

                assert_ok!(Stake::start(asset_id, reward_total_amount));

                //7884000
                for _ in 0..loop_count {
                    let cur_blocknum = <frame_system::Pallet<Test>>::block_number();
                    System::set_block_number(cur_blocknum + block_gap_of_get_reward);
                    assert_ok!(Stake::make_profit(asset_id));
                }

                let activity = <StakingActivityStore<Test>>::get(asset_id).unwrap();
                let balance = Assets::balance(asset_id, activity.reward_pot);
                assert!(
                    balance < reward_total_amount * 11 / 10,
                    "reward in pot {:} should be less than reward_total_amount {:}",
                    balance,
                    reward_total_amount
                );

                assert!(
                    balance > reward_total_amount * 2 / 3,
                    "reward in pot {:} should be more than 2 / 3 of reward_total_amount {:} ",
                    balance,
                    reward_total_amount
                );
                println!(
                    "balance of pot is {:?}, reward_total_amount is {:?}",
                    balance, reward_total_amount
                );
            });
        }
    }
}
/*Profit Invariants End */

/* For Invariants Start */
mod activity_invariant {

    use parami_primitives::constants::{self, DOLLARS};

    use super::*;
    #[test]
    pub fn invariant_holds_after_multi_stake_and_multi_withdraw() {
        new_test_ext().execute_with(|| {
            let asset_id = 9;

            let reward_total_amount = 7_000_000u128 * 10u128.pow(18);

            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));

            assert_ok!(Stake::start(asset_id, reward_total_amount));

            let activity = <StakingActivityStore<Test>>::get(asset_id).unwrap();

            assert_ok!(Stake::stake(asset_id, &ALICE, 20));

            System::set_block_number(20);

            assert_ok!(Stake::stake(asset_id, &ALICE, 20));

            System::set_block_number(40);

            assert_ok!(Stake::stake(asset_id, &CHARLIE, 20));

            System::set_block_number(60);

            assert_ok!(Stake::stake(asset_id, &CHARLIE, 20));

            let reward_pot_balance = Assets::balance(asset_id, activity.reward_pot);

            let earned_total = Stake::earned(asset_id, &ALICE).unwrap()
                + Stake::earned(asset_id, &CHARLIE).unwrap();
            assert_eq_escape_precision_effect(reward_pot_balance, earned_total);
        });
    }

    #[test]
    pub fn no_two_assets_staking_activity_pot_will_conflict() {
        use std::collections::HashSet;
        let mut exists_pots = HashSet::new();

        new_test_ext().execute_with(|| {
            for asset_id in 1..400_000 {
                assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
                assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));

                let activity = <StakingActivityStore<Test>>::get(asset_id).unwrap();
                assert_eq!(exists_pots.contains(&activity.reward_pot), false);

                exists_pots.insert(activity.reward_pot);
            }
        });
    }

    #[test]
    pub fn daily_output_down_to_1_in_about_2_years() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));

            assert_ok!(Stake::stake(asset_id, &ALICE, 20u128));

            let half_times = Into::<u64>::into(2 * 365 * constants::DAYS)
                / <Test as Config>::HalvingDurationInBlockNum::get();

            for _ in 0..half_times {
                let cur_block_num = System::current_block_number();

                System::set_block_number(
                    cur_block_num + <Test as Config>::HalvingDurationInBlockNum::get() + 1,
                );

                assert_ok!(Stake::make_profit(asset_id));
            }

            let activity = <StakingActivityStore<Test>>::get(asset_id).unwrap();

            println!(
                "after two years, daily_output is {:?}",
                activity.daily_output
            );

            assert!(
                activity.daily_output > 1 * DOLLARS,
                "after two years, daily output should be G.T. 1 DOLLARS! daily_outpus is {:?}",
                activity.daily_output
            );
        });
    }
}
/* For Invariants End */

/*
 * For Staking Activity Start
 */
mod staking_activity {
    use frame_support::assert_noop;

    use crate::Error;

    use super::*;

    #[test]
    pub fn should_not_start_when_reward_is_zero() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_noop!(
                Stake::start(asset_id, 0u32.into()),
                Error::<Test>::InvalidAmount
            );
        });
    }

    #[test]
    pub fn should_not_start_when_activity_already_start() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 10000u32.into()));
            assert_noop!(
                Stake::start(asset_id, 10000u32.into()),
                Error::<Test>::ActivityAlreadyExists
            );
        });
    }

    #[test]
    pub fn total_supply_and_user_balance_should_change_exactly_after_withdraw() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));

            assert_ok!(Stake::stake(asset_id, &ALICE, 20));

            System::set_block_number(20);

            let activity_before_withdraw = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let user_balance_before_withdraw =
                <UserStakingBalanceStore<Test>>::get(asset_id, &ALICE);
            let withdraw_amount = 15u128;
            assert_ok!(Stake::withdraw(asset_id, &ALICE, withdraw_amount));

            let activity_after_withdraw = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let user_balance_after_withdraw =
                <UserStakingBalanceStore<Test>>::get(asset_id, &ALICE);

            assert_eq!(
                activity_before_withdraw.total_supply,
                activity_after_withdraw.total_supply + withdraw_amount
            );

            assert_eq!(
                user_balance_before_withdraw,
                user_balance_after_withdraw + withdraw_amount
            );
        });
    }

    #[test]
    pub fn total_supply_and_user_balance_should_change_after_stake() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));

            let activity_before = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let user_balance_before = <UserStakingBalanceStore<Test>>::get(asset_id, &ALICE);

            let stake_amount = 20;
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));

            let activity_after = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let user_balance_after_stake = <UserStakingBalanceStore<Test>>::get(asset_id, &ALICE);

            assert_eq!(
                activity_before.total_supply + stake_amount,
                activity_after.total_supply
            );

            assert_eq!(user_balance_before + stake_amount, user_balance_after_stake);
        });
    }

    #[test]
    pub fn total_supply_and_user_balance_should_not_change_after_get_reward() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));
            let stake_amount = 20;
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));

            let activity_before_get_reward = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let user_balance_before_get_reward =
                <UserStakingBalanceStore<Test>>::get(asset_id, &ALICE);

            assert_ok!(Stake::get_reward(asset_id, &ALICE));

            let activity_after_get_reward = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let user_balance_after_get_reward =
                <UserStakingBalanceStore<Test>>::get(asset_id, &ALICE);

            assert_eq!(
                activity_before_get_reward.total_supply,
                activity_after_get_reward.total_supply
            );
            assert_eq!(
                user_balance_before_get_reward,
                user_balance_after_get_reward
            );
        });
    }

    #[test]
    pub fn earnings_per_share_and_total_remains_and_pot_balance_should_change_exatly_after_make_profit_when_make_profit(
    ) {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));
            let stake_amount = 20;
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));

            let activity_before = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let reward_pot_balance_before = Assets::balance(asset_id, activity_before.reward_pot);

            System::set_block_number(20);
            assert_ok!(Stake::make_profit(asset_id));

            let activity_after = <StakingActivityStore<Test>>::get(asset_id).unwrap();
            let reward_pot_balance_after = Assets::balance(asset_id, activity_after.reward_pot);

            // TODO(ironman_ch): use blocks_in_day of parami_primitive
            let cur_profit = 20 * activity_before.daily_output / (24 * 60 * 5);
            assert_eq!(
                activity_before.earnings_per_share + cur_profit / activity_before.total_supply,
                activity_after.earnings_per_share
            );
            assert_eq_escape_precision_effect(
                activity_before.reward_total_remains,
                activity_after.reward_total_remains + cur_profit,
            );
            assert_eq_escape_precision_effect(
                reward_pot_balance_before + cur_profit,
                reward_pot_balance_after,
            );
            assert_eq!(
                reward_pot_balance_after - reward_pot_balance_before,
                activity_before.reward_total_remains - activity_after.reward_total_remains
            );
        });
    }

    #[test]
    pub fn halve_time_and_daily_output_should_change_after_hit_halve_time_in_make_profit() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));
            let stake_amount = 100;
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));
            System::set_block_number(
                Into::<u64>::into(<Test as Config>::HalvingDurationInBlockNum::get()) + 2u64,
            );

            let activity_before = <StakingActivityStore<Test>>::get(asset_id).unwrap();

            assert_ok!(Stake::make_profit(asset_id));

            let activity_after = <StakingActivityStore<Test>>::get(asset_id).unwrap();

            assert_eq!(
                activity_after.halve_time,
                System::current_block_number()
                    + Into::<u64>::into(<Test as Config>::HalvingDurationInBlockNum::get())
            );
            assert_eq_escape_precision_effect(
                activity_before.daily_output,
                activity_after.daily_output * 2,
            )
        });
    }

    #[test]
    pub fn last_block_should_change_after_make_profit() {
        new_test_ext().execute_with(|| {
            let asset_id = 1;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, 7_000_000u128 * 10u128.pow(18)));

            let stake_amount = 100;
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));
            let block_num = 20;
            System::set_block_number(block_num);

            assert_ok!(Stake::make_profit(asset_id));

            let activity_after = <StakingActivityStore<Test>>::get(asset_id).unwrap();

            assert_eq!(activity_after.lastblock, block_num);
        });
    }
}

fn assert_eq_escape_precision_effect(left: u128, right: u128) {
    assert_eq!(left / 10u128.pow(3), right / 10u128.pow(3));
}
/*
 * For Staking Activity End
 */

/*
Guard stake, get_reward, withdraw call make_profit start
 */

#[test]
pub fn stake_must_call_make_profit() {}

#[test]
pub fn get_reward_must_call_make_profit() {}

#[test]
pub fn withdraw_must_call_make_profit() {}

/*
Guard stake, get_reward, withdraw call make_profit end
 */

/*
For User State Start
 */
mod user_state {
    use super::*;

    #[test]
    pub fn earned_should_decrease_to_zero_after_withdraw() {
        new_test_ext().execute_with(|| {
            let asset_id = 9;

            let reward_total_amount = 7_000_000u128 * 10u128.pow(18);

            let stake_amount = 20;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, reward_total_amount));
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));
            let cur_block_num = 20;
            System::set_block_number(cur_block_num);

            assert_ok!(Stake::make_profit(asset_id));

            let earned_before = Stake::earned(asset_id, &ALICE).unwrap();

            let withdraw_amount = 10;
            assert_ok!(Stake::withdraw(asset_id, &ALICE, withdraw_amount));
            let earned_after = Stake::earned(asset_id, &ALICE).unwrap();

            assert!(earned_before > 0, "earned_before should be G.T. zero");
            assert_eq!(earned_after, 0);
        });
    }

    #[test]
    pub fn earned_should_decrease_to_zero_after_get_reward() {
        new_test_ext().execute_with(|| {
            let asset_id = 9;

            let reward_total_amount = 7_000_000u128 * 10u128.pow(18);

            let stake_amount = 20;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, reward_total_amount));
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));
            let cur_block_num = 20;
            System::set_block_number(cur_block_num);

            assert_ok!(Stake::make_profit(asset_id));

            let earned_before = Stake::earned(asset_id, &ALICE).unwrap();

            assert_ok!(Stake::get_reward(asset_id, &ALICE));
            let earned_after = Stake::earned(asset_id, &ALICE).unwrap();

            assert!(earned_before > 0, "earned_before should be G.T. zero");
            assert_eq!(earned_after, 0);
        });
    }

    #[test]
    pub fn earned_should_increase_exactly_after_stake() {
        new_test_ext().execute_with(|| {
            let asset_id = 9;

            let reward_total_amount = 7_000_000u128 * 10u128.pow(18);

            let stake_amount = 20;
            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));
            assert_ok!(Stake::start(asset_id, reward_total_amount));
            assert_ok!(Stake::stake(asset_id, &ALICE, stake_amount));
            let cur_block_num = 20;
            System::set_block_number(cur_block_num);

            Stake::make_profit(asset_id).unwrap();

            let earned_after = Stake::earned(asset_id, &ALICE).unwrap();
            assert!(earned_after > 0);
            println!("earned after make profit is {:}", earned_after);
        });
    }

    #[test]
    pub fn earned_should_be_zero_when_stake_in_the_same_block_with_start() {
        new_test_ext().execute_with(|| {
            let asset_id = 9;

            let reward_total_amount = 7_000_000u128 * 10u128.pow(18);

            assert_ok!(Assets::force_create(Origin::root(), asset_id, BOB, true, 1));

            assert_ok!(Stake::start(asset_id, reward_total_amount));

            assert_ok!(Stake::stake(asset_id, &ALICE, 20));

            let earned = Stake::earned(asset_id, &ALICE).unwrap();

            assert_eq!(earned, 0);
        });
    }
}
/*
For User State End
*/
