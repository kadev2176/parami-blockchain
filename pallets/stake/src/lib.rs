#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod types;

use frame_support::{
    traits::{
        tokens::fungibles::{
            InspectMetadata as FungMeta, Mutate as FungMutate, Transfer as FungTransfer,
        },
        Currency,
    },
    PalletId,
};
use sp_core::U512;
use sp_runtime::traits::{
    AccountIdConversion, AtLeast32BitUnsigned, Bounded, Hash, Saturating, Zero,
};
use sp_std::prelude::*;
use weights::WeightInfo;

type AssetIdOf<T> = <T as pallet::Config>::AssetId;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type StakingActivityOf<T> =
    types::StakingActivity<AssetIdOf<T>, AccountOf<T>, HeightOf<T>, BalanceOf<T>>;
/**
 * This const is the normalized INIT_DAILY_OUTPUT, and the normalized total amount is 1_000_000.
 *
 * In English:
 *
 * As summation of proportional series, we can resolve the INIT_DAILY_OUTPUT
 *
 * Assumptions:
 * 1. x = INIT_DAILY_OUTPUT
 * 2. n = 365/7 * 3 = 156 weeks
 * 3. S = 1_000_000
 * 4. half per week
 *
 * x + x/2 + x / 2^2 + ... + x/2^156 = 1_000_000
 *
 * as summation of proportional series says: S * (1 - q) = a1 - a_n+1
 *
 * bring in the variables:
 *
 * 1_000_000 * (1 - 1/2) = x - x * (1 / 2^157)
 *
 * so x ~= 500_000
 *
 *
 * In Chinese
 * 根据等比数列求和公式，求解一下等比数列的和
 * 1. 设 x 为第一周释放量
 * 2. 三年总共365/7 * 3 = 156周
 * 3. 按照3年释放100W币来做归一化，方便各中值的计算
 * 4. 每周发放量减半
 *
 * x + x/2 + x / 2^2 + ... + x/2^156 = 1_000_000
 *
 * 根据等比数列规律: (1 - q) * S = a1 - a_n+1，即S = (a1 - a_n+1) / 1 - q
 *
 * 带入该公式：1_000_000 = (x - x/2^157) / (1 - 1/2) -> x = 500_000 / (1 - 1/2^157) ~= 500_000
 */
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait
        type Currency: Currency<AccountOf<Self>>;

        /// Fungible token ID type
        type AssetId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy
            + MaxEncodedLen;

        /// The assets trait to create, mint, and transfer fungible tokens
        type Assets: FungMeta<AccountOf<Self>, AssetId = AssetIdOf<Self>>
            + FungMutate<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + FungTransfer<AccountOf<Self>, AssetId = AssetIdOf<Self>, Balance = BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of staking activity's reward
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type OneMillionNormalizedInitDailyOutput: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type DurationInBlockNum: Get<Self::BlockNumber>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::storage]
    pub(super) type StakingActivityStore<T: Config> = StorageMap<
        _,
        Twox64Concat,
        AssetIdOf<T>, // Asset ID
        StakingActivityOf<T>,
    >;

    #[pallet::storage]
    pub(super) type UserStakingRewardStore<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AssetIdOf<T>,
        Twox64Concat,
        AccountOf<T>,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub(super) type UserStakingBalanceStore<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AssetIdOf<T>,
        Twox64Concat,
        AccountOf<T>,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T> {}

    #[pallet::error]
    pub enum Error<T> {
        ActivityNotExists,
        ActivityAlreadyExists,
        ActivityNotStarted,
        InvalidAmount,
        TypeCastError,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}

    impl<T: Config> Pallet<T> {
        /*
        uint256 public constant DURATION = 7 days;
        uint256 public earnings_per_share; //每股分红
        uint256 public lastblock; //上次修改每股分红的时间
        uint256 public starttime = 111; //
        uint256 public DailyOutput = 1428 * 1e18; //10000/7
        uint256 public Halvetime; //减半的时间

        constructor ()public{
            Halvetime = block.timestamp + DURATION;
        }
        */

        pub fn start(
            asset_id: AssetIdOf<T>,
            reward_total_amount: BalanceOf<T>,
        ) -> Result<(), DispatchError> {
            let already_exists = <StakingActivityStore<T>>::contains_key(asset_id);
            ensure!(!already_exists, Error::<T>::ActivityAlreadyExists);

            let cur_blocknum = <frame_system::Pallet<T>>::block_number();
            let duration = T::DurationInBlockNum::get();

            let normalized_daily_output_u128: u128 = T::OneMillionNormalizedInitDailyOutput::get()
                .try_into()
                .map_err(|_| Error::<T>::TypeCastError)?;

            let normalized_daily_output = U512::try_from(normalized_daily_output_u128)
                .map_err(|_| Error::<T>::TypeCastError)?;
            let one_million_in_balance = U512::try_from(1_000_000u128 * 10u128.pow(18))
                .map_err(|_| Error::<T>::TypeCastError)?;
            let reward_total_amount_u128: u128 = TryInto::<u128>::try_into(reward_total_amount)
                .map_err(|_| Error::<T>::TypeCastError)?;
            let reward_total_amount_u512: U512 =
                U512::try_from(reward_total_amount_u128).map_err(|_| Error::<T>::TypeCastError)?;

            let daily_output_u512: U512 =
                normalized_daily_output * reward_total_amount_u512 / one_million_in_balance;
            let daily_output_u128: u128 = TryInto::<u128>::try_into(daily_output_u512)
                .map_err(|_| Error::<T>::TypeCastError)?;
            let daily_output_balance = TryInto::<BalanceOf<T>>::try_into(daily_output_u128)
                .map_err(|_| Error::<T>::TypeCastError)?;
            <StakingActivityStore<T>>::insert(
                asset_id,
                types::StakingActivity {
                    asset_id,
                    reward_total_amount,
                    reward_total_remains: reward_total_amount,
                    reward_pot: Self::to_staking_reward_pot(&asset_id),
                    start_block_num: cur_blocknum,
                    halve_time: cur_blocknum.saturating_add(duration),
                    lastblock: cur_blocknum,
                    total_supply: BalanceOf::<T>::zero(),
                    earnings_per_share: BalanceOf::<T>::zero(),
                    daily_output: daily_output_balance,
                },
            );
            Ok(())
        }

        /*
         function getPerBlockOutput() public view returns (uint256) {
               return DailyOutput.div(6646);// 13秒1个区块,每天大概是6646个区块 //https://etherscan.io/chart/blocktime
         }
        */
        pub fn get_per_block_output(asset_id: AssetIdOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            let activity =
                <StakingActivityStore<T>>::get(asset_id).ok_or(Error::<T>::ActivityNotExists)?;
            //one block per 12 seconds, so 1 day has 7200 blocks
            //TODO(ironman_ch): use const in parami_primitive
            Ok(activity.daily_output / 7200u32.into())
        }

        /*
         function getprofit() private returns (uint256) {
            if (block.timestamp > Halvetime){
                DailyOutput = DailyOutput.div(2); //减半
                Halvetime = block.timestamp + DURATION;
            }
            uint256 new_blocknum = block.number;
            if (new_blocknum <= lastblock) {
                return 0;
            }
            uint256 diff = new_blocknum.sub(lastblock);
            lastblock = new_blocknum;
            uint256 profit = diff.mul(getPerBlockOutput());
            return profit;
        }
         */
        fn get_profit(activity: &StakingActivityOf<T>) -> Result<BalanceOf<T>, DispatchError> {
            let cur_block_num = <frame_system::Pallet<T>>::block_number();
            if cur_block_num > activity.halve_time {
                <StakingActivityStore<T>>::mutate(activity.asset_id, |activity| {
                    if let Some(activity) = activity {
                        activity.daily_output = activity.daily_output / 2u32.into();
                        activity.halve_time = cur_block_num + T::DurationInBlockNum::get();
                    }
                });
            }
            let new_blocknum = cur_block_num;
            if new_blocknum <= activity.lastblock {
                return Ok(Zero::zero());
            }

            let diff: u32 = new_blocknum
                .saturating_sub(activity.lastblock)
                .try_into()
                .map_err(|_| Error::<T>::TypeCastError)?;

            <StakingActivityStore<T>>::mutate(activity.asset_id, |activity| {
                if let Some(activity) = activity {
                    activity.lastblock = new_blocknum;
                }
            });
            let per_block_output = Self::get_per_block_output(activity.asset_id)?;
            let profit = per_block_output.saturating_mul(diff.into());
            Ok(profit)
        }

        /*
            modifier make_profit() {
               uint256 amount = getprofit();
               if (amount > 0) {
                   yfi.mint(address(this), amount);
                   if (totalSupply() == 0){
                       earnings_per_share = 0;
                   }else{
                       earnings_per_share = earnings_per_share.add(
                       amount.div(totalSupply())
                   );
                   }

               }
               _;
           }
        */
        pub fn make_profit(asset_id: AssetIdOf<T>) -> Result<(), DispatchError> {
            let activity =
                <StakingActivityStore<T>>::get(asset_id).ok_or(Error::<T>::ActivityNotExists)?;
            let amount = Self::get_profit(&activity)?;
            // take the min of diff_profit and reward_total_remains
            let amount = amount.min(activity.reward_total_remains);

            if amount > Zero::zero() {
                let pot = Self::to_staking_reward_pot(&asset_id);
                T::Assets::mint_into(asset_id, &pot, amount)?;
                <StakingActivityStore<T>>::mutate(asset_id, |activity| {
                    if let Some(activity) = activity {
                        activity.reward_total_remains -= amount;
                    }
                });

                if activity.total_supply == Zero::zero() {
                    <StakingActivityStore<T>>::mutate(asset_id, |activity| {
                        if let Some(activity) = activity {
                            activity.earnings_per_share = Zero::zero();
                        }
                    });
                } else {
                    <StakingActivityStore<T>>::mutate(asset_id, |activity| {
                        if let Some(activity) = activity {
                            activity.earnings_per_share += amount / activity.total_supply;
                        }
                    });
                }
            }
            Ok(())
        }

        /*
        refer to YearnRewards's stake implementation:

        require(block.timestamp >starttime,"not start");
        require(amount > 0, "Cannot stake 0");
        if (earnings_per_share == 0){
            rewards[msg.sender] = 0;
        }else{
            rewards[msg.sender] = rewards[msg.sender].add(
                earnings_per_share.mul(amount)
            );
        }
        super.stake(amount);
        emit Staked(msg.sender, amount);
        */
        pub fn stake(
            amount: BalanceOf<T>,
            asset_id: AssetIdOf<T>,
            account: &AccountOf<T>,
        ) -> Result<(), sp_runtime::DispatchError> {
            // 1. call make_profit first
            Self::make_profit(asset_id)?;

            // Others
            let reward_activity =
                <StakingActivityStore<T>>::get(asset_id).ok_or(Error::<T>::ActivityNotExists)?;

            let cur_block = <frame_system::Pallet<T>>::block_number();
            ensure!(
                cur_block >= reward_activity.start_block_num,
                Error::<T>::ActivityNotStarted
            );
            ensure!(amount > Zero::zero(), Error::<T>::InvalidAmount);

            if reward_activity.earnings_per_share == Zero::zero() {
                <UserStakingRewardStore<T>>::mutate(asset_id, &account, |rewards| {
                    rewards.set_zero();
                });
            } else {
                <UserStakingRewardStore<T>>::mutate(asset_id, &account, |rewards| {
                    rewards.saturating_accrue(reward_activity.earnings_per_share * amount)
                });
            }

            Self::stake_inner(asset_id, &account, amount);

            Ok(())
            // TODO(ironman_ch): emit Staked(msg.sender, amount);
        }

        /*
         function withdraw(uint256 amount) public make_profit
           {
               require(amount > 0, "Cannot withdraw 0");
               getReward();

               rewards[msg.sender] = rewards[msg.sender].sub(
                   earnings_per_share.mul(amount)
               );
               super.withdraw(amount);
               emit Withdrawn(msg.sender, amount);
           }
        */
        pub fn withdraw(
            asset_id: AssetIdOf<T>,
            account: &AccountOf<T>,
            amount: BalanceOf<T>,
        ) -> Result<(), sp_runtime::DispatchError> {
            // 1. call make_profit();
            Self::make_profit(asset_id)?;

            ensure!(amount > Zero::zero(), Error::<T>::InvalidAmount);
            //

            let activity =
                <StakingActivityStore<T>>::get(asset_id).ok_or(Error::<T>::ActivityNotExists)?;

            Self::get_reward(asset_id, &account)?;

            <UserStakingRewardStore<T>>::mutate(asset_id, &account, |user_staking_reward| {
                user_staking_reward.saturating_accrue(activity.earnings_per_share * amount)
            });

            Self::withdraw_inner(asset_id, &account, amount);
            Ok(())
        }

        /*
         function exit() external {
               withdraw(balanceOf(msg.sender));
           }
        */
        pub fn exit(asset_id: AssetIdOf<T>, account: &AccountOf<T>) -> Result<(), DispatchError> {
            let amount = <UserStakingBalanceStore<T>>::get(asset_id, account);
            Self::withdraw(asset_id, account, amount)?;
            Ok(())
        }

        /*
         function getReward() public make_profit  {
               uint256 reward = earned(msg.sender);
               if (reward > 0) {
                   rewards[msg.sender] = earnings_per_share.mul(balanceOf(msg.sender));
                   yfi.safeTransfer(msg.sender, reward);
                   emit RewardPaid(msg.sender, reward);
               }
           }
        */
        pub fn get_reward(
            asset_id: AssetIdOf<T>,
            account: &AccountOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            //1. make_profit first
            Self::make_profit(asset_id)?;

            //Others
            let reward = Self::earned(asset_id, account)?;
            let activity =
                <StakingActivityStore<T>>::get(asset_id).ok_or(Error::<T>::ActivityNotExists)?;
            if reward > Zero::zero() {
                <UserStakingRewardStore<T>>::insert(
                    activity.asset_id,
                    account,
                    activity.earnings_per_share
                        * Self::staking_balance_of_inner(activity.asset_id, account),
                );
                Self::transfer_to(activity.asset_id, account, reward)?;
                //TODO(ironman_ch): emit RewardPaid(msg.sender, reward);
            }

            Ok(reward)
        }

        /*
         function earned(address account) public view returns (uint256) {
               uint256 _cal = earnings_per_share.mul(balanceOf(account));
               if (_cal < rewards[msg.sender]) {
                   return 0;
               } else {
                   return _cal.sub(rewards[msg.sender]);
               }
           }
        */
        pub fn earned(
            asset_id: AssetIdOf<T>,
            account: &AccountOf<T>,
        ) -> Result<BalanceOf<T>, DispatchError> {
            let activity =
                <StakingActivityStore<T>>::get(asset_id).ok_or(Error::<T>::ActivityNotExists)?;
            let cal = activity.earnings_per_share
                * Self::staking_balance_of_inner(activity.asset_id, account);

            let cur_reward_of_user = <UserStakingRewardStore<T>>::get(activity.asset_id, account);

            if cal < cur_reward_of_user {
                return Ok(Zero::zero());
            } else {
                return Ok(cal.saturating_sub(cur_reward_of_user));
            }
        }

        /*
        function stake(uint256 amount) public {
            _totalSupply = _totalSupply.add(amount);
            _balances[msg.sender] = _balances[msg.sender].add(amount);
            y.safeTransferFrom(msg.sender, address(this), amount);
        }
        */
        fn stake_inner(asset_id: AssetIdOf<T>, account: &AccountOf<T>, amount: BalanceOf<T>) {
            <StakingActivityStore<T>>::mutate(asset_id, |activity| {
                if let Some(activity) = activity {
                    activity.total_supply.saturating_accrue(amount)
                }
            });

            <UserStakingBalanceStore<T>>::mutate(asset_id, account, |user_balance| {
                user_balance.saturating_accrue(amount)
            })
        }

        fn withdraw_inner(asset_id: AssetIdOf<T>, account: &AccountOf<T>, amount: BalanceOf<T>) {
            <StakingActivityStore<T>>::mutate(asset_id, |activity| {
                if let Some(activity) = activity {
                    activity.total_supply.saturating_reduce(amount);
                }
            });

            <UserStakingBalanceStore<T>>::mutate(asset_id, account, |user_balance| {
                user_balance.saturating_reduce(amount)
            });
        }

        fn staking_balance_of_inner(
            asset_id: AssetIdOf<T>,
            account: &AccountOf<T>,
        ) -> BalanceOf<T> {
            <UserStakingBalanceStore<T>>::get(asset_id, account)
        }

        fn to_staking_reward_pot(asset_id: &AssetIdOf<T>) -> AccountOf<T> {
            let asset_id_raw = <AssetIdOf<T>>::encode(&asset_id);
            let hash = <T as frame_system::Config>::Hashing::hash(&asset_id_raw);
            <T as Config>::PalletId::get().into_sub_account_truncating(hash)
        }

        fn transfer_to(
            asset_id: AssetIdOf<T>,
            account: &AccountOf<T>,
            amount: BalanceOf<T>,
        ) -> Result<(), DispatchError> {
            let activity_reward_pot = Self::to_staking_reward_pot(&asset_id);
            T::Assets::transfer(asset_id, &activity_reward_pot, account, amount, false)?;
            Ok(())
        }
    }
}
