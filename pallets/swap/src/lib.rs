//! Buy tokens, sell tokens.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::traits::tokens::fungibles::{Inspect, Transfer};
use frame_support::traits::{Currency, ExistenceRequirement};
use frame_support::PalletId;
use sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub const PALLET_ID: PalletId = PalletId(*b"paraswap");

#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, RuntimeDebug)]
pub struct SwapPair<AccountId> {
    account: AccountId,
    native_balance: u128,
    asset_balance: u128,
    // charge swaper for 0.3%
    charge_rate: u32,
    issued_liquidity: u128,
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    /// The module configuration trait.
    pub trait Config:
        frame_system::Config + pallet_assets::Config + pallet_balances::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        // Go to hell, your types sucks
        type NativeBalance: IsType<<Self as pallet_balances::Config>::Balance>
            + Parameter
            + Member
            + Copy
            + AtLeast32BitUnsigned
            + IsType<u128>
            + IsType<<<Self as Config>::Currency as Currency<<Self as frame_system::Config>::AccountId>>::Balance>;
        type SwapAssetBalance: IsType<<Self as pallet_assets::Config>::Balance>
            + Parameter
            + Member
            + Copy
            + AtLeast32BitUnsigned
            + IsType<u128>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(
        T::AccountId = "AccountId",
        T::Balance = "Balance",
        T::AssetId = "AssetId",
        T::NativeBalance = "NativeBalance",
        T::SwapAssetBalance = "SwapAssetBalance"
    )]
    pub enum Event<T: Config> {
        /// New swap pair created
        Created(T::AssetId),
        /// Add liquidity
        LiquidityAdded(T::AccountId, T::AssetId),
        /// Remove liquidity
        LiquidityRemoved(T::AccountId, T::AssetId),
        /// Buy tokens
        SwapBuy(T::AccountId, T::AssetId, T::SwapAssetBalance),
        /// Sell tokens
        SwapSell(T::AccountId, T::AssetId, T::NativeBalance),
    }

    #[pallet::error]
    pub enum Error<T> {
        Exists,
        AssetNotFound,
        SwapNotFound,
        NativeAmountRequired,
        AssetAmountRequired,
        Overflow,
        /// remaining token/native token balance is too low
        InsufficientLiquidity,
        InsufficientNativeToken,
        InsufficientAssetToken,
        InsufficientNativeBalance,
        InsufficientAssetBalance,
        NativeAmountIsZero,
        AssetAmountIsZero,
        MintedLiquidityIsZero,
        LiquidityAmountIsZero,

        InsufficientPoolAssetAmount,
        InsufficientPoolNativeAmount,
    }

    #[pallet::storage]
    pub(super) type Swap<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, SwapPair<T::AccountId>, OptionQuery>;

    // (asset-id, account-id) => amount of LP-token
    // serve as the fake lp-token balance
    #[pallet::storage]
    pub(super) type LiquidityProvider<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    /*
    #[pallet::storage]
    #[pallet::getter(fn total_liquidity_of)]
    pub(super) type TotalLiquidity<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, T::Balance, ValueQuery>;
     */

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <<T as pallet::Config>::Currency as frame_support::traits::Currency<
            <T as frame_system::Config>::AccountId,
        >>::Balance: From<u128> + Into<u128>,
        <T as pallet_assets::Config>::Balance: From<u128>,
    {
        #[pallet::weight(0)]
        pub(super) fn create(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;

            let total_supply = <pallet_assets::Pallet<T>>::total_supply(asset_id);
            log::info!("!! create total supply => {:?}", total_supply);
            ensure!(total_supply > Zero::zero(), Error::<T>::AssetNotFound);

            log::debug!(
                "!! min balance => {:?}",
                <pallet_assets::Pallet<T>>::minimum_balance(asset_id)
            );

            ensure!(!Swap::<T>::contains_key(asset_id), Error::<T>::Exists);

            let pool_account_id = Self::asset_account_id(asset_id);
            <T as pallet::Config>::Currency::transfer(
                &sender,
                &pool_account_id,
                <T as pallet::Config>::Currency::minimum_balance(),
                ExistenceRequirement::KeepAlive,
            )?;
            <pallet_assets::Pallet<T>>::transfer(
                asset_id,
                &sender,
                &pool_account_id,
                <pallet_assets::Pallet<T>>::minimum_balance(asset_id),
                true,
            )?;

            Swap::<T>::insert(
                asset_id,
                SwapPair {
                    account: Self::asset_account_id(asset_id),
                    native_balance: 0,
                    asset_balance: 0,
                    charge_rate: 3,
                    issued_liquidity: 0,
                },
            );

            // creates pool account

            // TotalLiquidity::insert(asset_id, Zero::zero());
            Self::deposit_event(Event::Created(asset_id));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub(super) fn add_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            native_amount: T::NativeBalance,
            maybe_asset_amount: Option<T::SwapAssetBalance>,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;
            let mut pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            log::info!("trade pair => {:?}", pair);

            // sender's native balance check
            ensure!(native_amount > Zero::zero(), Error::<T>::NativeAmountIsZero);
            ensure!(
                <pallet_balances::Pallet<T> as Currency<_>>::free_balance(&sender)
                    > native_amount.into(),
                Error::<T>::InsufficientNativeBalance
            );

            let (pool_account_id, pool_native_amount) = Self::native_pool(asset_id);
            let (_, pool_asset_amount) = Self::asset_pool(asset_id);

            // unit conversion
            let native_amount: u128 = native_amount.into();
            let asset_amount: u128;
            let minted_liquidity;

            log::info!(
                "add liquidity native={:?} asset={:?}",
                native_amount,
                maybe_asset_amount
            );

            if pool_asset_amount == Zero::zero() || pair.issued_liquidity == 0 {
                // initial add liquidity
                ensure!(
                    maybe_asset_amount.is_some(),
                    Error::<T>::AssetAmountRequired
                );
                asset_amount = maybe_asset_amount.unwrap().into();
                minted_liquidity = native_amount;
            } else {
                let pool_asset_amount: u128 = pool_asset_amount.into();
                asset_amount = pool_asset_amount
                    .checked_mul(native_amount)
                    .and_then(|v| v.checked_div(pool_native_amount.into()))
                    .ok_or(Error::<T>::Overflow)?;

                if let Some(provide_amount) = maybe_asset_amount {
                    ensure!(
                        provide_amount >= asset_amount.into(),
                        Error::<T>::InsufficientAssetToken
                    );
                }

                minted_liquidity = pair
                    .issued_liquidity
                    .checked_mul(native_amount)
                    .and_then(|v| v.checked_div(pool_native_amount.into()))
                    .ok_or(Error::<T>::Overflow)?;

                log::info!("calculated asset={:?}", asset_amount);
            }

            ensure!(asset_amount > Zero::zero(), Error::<T>::AssetAmountIsZero);
            ensure!(
                minted_liquidity > Zero::zero(),
                Error::<T>::MintedLiquidityIsZero
            );

            // Equation:
            // pool_native_amount *asset_amount == pool_asset_amount * native_amount

            // sender asset amount check
            {
                let sender_asset_balance: T::SwapAssetBalance =
                    <pallet_assets::Pallet<T>>::balance(asset_id, &sender).into();
                ensure!(
                    sender_asset_balance > asset_amount.into(),
                    Error::<T>::InsufficientAssetBalance
                );
            }

            // TODO: liquidity token check?

            // native token inject
            <T as pallet::Config>::Currency::transfer(
                &sender,
                &pool_account_id,
                native_amount.into(),
                ExistenceRequirement::KeepAlive,
            )?;
            // asset token inject
            <pallet_assets::Pallet<T>>::transfer(
                asset_id,
                &sender,
                &pool_account_id,
                asset_amount.into(),
                true,
            )?;

            // LP handling
            LiquidityProvider::<T>::try_mutate_exists(
                asset_id,
                sender.clone(),
                |maybe_lp| -> DispatchResult {
                    let mut lp = maybe_lp.take().unwrap_or_default();
                    lp = lp
                        .checked_add(minted_liquidity)
                        .ok_or(Error::<T>::Overflow)?;
                    *maybe_lp = Some(lp);
                    Ok(())
                },
            )?;

            pair.issued_liquidity = pair
                .issued_liquidity
                .checked_add(minted_liquidity)
                .ok_or(Error::<T>::Overflow)?;
            Swap::<T>::insert(asset_id, pair);

            Self::deposit_event(Event::LiquidityAdded(sender, asset_id));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub(super) fn remove_liquidity(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            liquidity_amount: T::NativeBalance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;
            let mut pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            ensure!(
                liquidity_amount <= pair.issued_liquidity.into(),
                Error::<T>::InsufficientLiquidity
            );
            // FIXME: the error might be inarguments, or pair info
            ensure!(
                liquidity_amount > Zero::zero() && pair.issued_liquidity > 0,
                Error::<T>::LiquidityAmountIsZero
            );

            let (pool_account_id, pool_native_amount) = Self::native_pool(asset_id);
            let (_, pool_asset_amount) = Self::asset_pool(asset_id);

            ensure!(
                pool_native_amount > Zero::zero(),
                Error::<T>::InsufficientPoolNativeAmount
            );
            ensure!(
                pool_asset_amount > Zero::zero(),
                Error::<T>::InsufficientPoolAssetAmount
            );

            let pool_native_amount: u128 = pool_native_amount.into();
            let pool_asset_amount: u128 = pool_asset_amount.into();

            let native_amount: u128 = pool_native_amount
                .checked_mul(liquidity_amount.into())
                .and_then(|v| v.checked_div(pair.issued_liquidity))
                .ok_or(Error::<T>::Overflow)?;
            let asset_amount = pool_asset_amount
                .checked_mul(liquidity_amount.into())
                .and_then(|v| v.checked_div(pair.issued_liquidity))
                .ok_or(Error::<T>::Overflow)?;

            ensure!(native_amount > Zero::zero(), Error::<T>::NativeAmountIsZero);
            ensure!(asset_amount > Zero::zero(), Error::<T>::AssetAmountIsZero);

            // free balance check
            ensure!(
                pool_native_amount > native_amount,
                Error::<T>::InsufficientPoolNativeAmount
            );
            ensure!(
                pool_asset_amount > asset_amount,
                Error::<T>::InsufficientPoolAssetAmount
            );
            ensure!(
                LiquidityProvider::<T>::get(asset_id, sender.clone()) >= liquidity_amount.into(),
                Error::<T>::InsufficientLiquidity
            );

            // remove liquidity
            <T as pallet::Config>::Currency::transfer(
                &pool_account_id,
                &sender,
                native_amount.into(),
                ExistenceRequirement::AllowDeath,
            )?;
            // asset token inject
            <pallet_assets::Pallet<T>>::transfer(
                asset_id,
                &pool_account_id,
                &sender,
                asset_amount.into(),
                true,
            )?;

            // LP handling
            LiquidityProvider::<T>::try_mutate_exists(
                asset_id,
                sender.clone(),
                |maybe_lp| -> DispatchResult {
                    let mut lp = maybe_lp.take().unwrap_or_default();
                    lp = lp
                        .checked_sub(liquidity_amount.into())
                        .ok_or(Error::<T>::Overflow)?;
                    *maybe_lp = Some(lp);
                    Ok(())
                },
            )?;
            pair.issued_liquidity = pair
                .issued_liquidity
                .checked_sub(liquidity_amount.into())
                .ok_or(Error::<T>::Overflow)?;
            Swap::<T>::insert(asset_id, pair);

            Self::deposit_event(Event::LiquidityRemoved(sender, asset_id));

            Ok(().into())
        }

        /// Buy asset token with native token.
        #[pallet::weight(0)]
        pub(super) fn buy(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            native_amount: T::NativeBalance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;
            let pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            let (pool_account_id, pool_native_amount) = Self::native_pool(asset_id);
            let (_, pool_asset_amount) = Self::asset_pool(asset_id);
            ensure!(
                pool_asset_amount > Zero::zero(),
                Error::<T>::InsufficientPoolAssetAmount
            );
            ensure!(
                pool_native_amount > Zero::zero(),
                Error::<T>::InsufficientPoolNativeAmount
            );
            let pool_native_amount: u128 = pool_native_amount.into();
            let pool_asset_amount: u128 = pool_asset_amount.into();

            // TODO: support fees

            let new_native_amount = pool_native_amount
                .checked_add(native_amount.into())
                .ok_or(Error::<T>::Overflow)?;
            let asset_amount = pool_asset_amount
                .checked_mul(new_native_amount)
                .ok_or(Error::<T>::Overflow)?
                .checked_sub(
                    pool_asset_amount
                        .checked_mul(pool_native_amount)
                        .ok_or(Error::<T>::Overflow)?,
                )
                .ok_or(Error::<T>::Overflow)?
                .checked_div(new_native_amount)
                .ok_or(Error::<T>::Overflow)?;

            ensure!(native_amount > Zero::zero(), Error::<T>::NativeAmountIsZero);
            ensure!(asset_amount > Zero::zero(), Error::<T>::AssetAmountIsZero);

            // free balance check
            // pool's asset token
            ensure!(
                pool_asset_amount > asset_amount,
                Error::<T>::InsufficientPoolAssetAmount
            );
            // sender's native token
            ensure!(
                // <T as pallet::Config>::Currency::free_balance(&sender) > native_amount,
                <pallet_balances::Pallet<T> as Currency<_>>::free_balance(&sender)
                    > native_amount.into(),
                Error::<T>::InsufficientNativeBalance
            );

            log::debug!(
                "swap issued_liquidity={:?} pool={:?} / {:?}",
                pair.issued_liquidity,
                pool_native_amount,
                pool_asset_amount
            );

            // do transfer
            <T as pallet::Config>::Currency::transfer(
                &sender,
                &pool_account_id,
                native_amount.into(),
                ExistenceRequirement::AllowDeath,
            )?;
            // asset token inject
            <pallet_assets::Pallet<T>>::transfer(
                asset_id,
                &pool_account_id,
                &sender,
                asset_amount.into(),
                true,
            )?;

            // event
            Self::deposit_event(Event::SwapBuy(sender, asset_id, asset_amount.into()));

            Ok(().into())
        }

        /// Sell asset token for native token.
        #[pallet::weight(0)]
        pub(super) fn sell(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            asset_amount: T::SwapAssetBalance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;
            let pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            // check pool
            let (pool_account_id, pool_native_amount) = Self::native_pool(asset_id);
            let (_, pool_asset_amount) = Self::asset_pool(asset_id);
            ensure!(
                pool_native_amount > Zero::zero(),
                Error::<T>::InsufficientPoolNativeAmount
            );
            ensure!(
                pool_asset_amount > Zero::zero(),
                Error::<T>::InsufficientPoolAssetAmount
            );
            let pool_native_amount: u128 = pool_native_amount.into();
            let pool_asset_amount: u128 = pool_asset_amount.into();

            // calculate swap
            let new_asset_amount = pool_asset_amount
                .checked_add(asset_amount.into())
                .ok_or(Error::<T>::Overflow)?;
            let native_amount = pool_native_amount
                .checked_mul(new_asset_amount)
                .ok_or(Error::<T>::Overflow)?
                .checked_sub(
                    pool_asset_amount
                        .checked_mul(pool_native_amount)
                        .ok_or(Error::<T>::Overflow)?,
                )
                .ok_or(Error::<T>::Overflow)?
                .checked_div(new_asset_amount)
                .ok_or(Error::<T>::Overflow)?;

            ensure!(native_amount > Zero::zero(), Error::<T>::NativeAmountIsZero);
            ensure!(asset_amount > Zero::zero(), Error::<T>::AssetAmountIsZero);

            // free balance check
            {
                let sender_asset_balance: T::SwapAssetBalance =
                    <pallet_assets::Pallet<T>>::balance(asset_id, &sender).into();
                ensure!(
                    sender_asset_balance > asset_amount.into(),
                    Error::<T>::InsufficientAssetBalance
                );
            }
            ensure!(
                pool_native_amount > native_amount,
                Error::<T>::InsufficientPoolNativeAmount
            );

            // do transfer
            <pallet_assets::Pallet<T>>::transfer(
                asset_id,
                &sender,
                &pool_account_id,
                asset_amount.into(),
                true,
            )?;
            <T as pallet::Config>::Currency::transfer(
                &pool_account_id,
                &sender,
                native_amount.into(),
                ExistenceRequirement::AllowDeath,
            )?;

            // event
            Self::deposit_event(Event::SwapSell(sender, asset_id, native_amount.into()));

            Ok(().into())
        }
    }

    // public functions
    impl<T: Config> Pallet<T> {
        /// The account ID of the swap pool.
        ///
        /// This actually does computation. If you need to keep using it, then make sure you cache the
        /// value and only call this once.
        pub fn account_id() -> T::AccountId {
            PALLET_ID.into_account()
        }

        /// Return the pool account and amount of money in the pool.
        // The existential deposit is not part of the pool so airdrop account never gets deleted.
        fn pool() -> (T::AccountId, T::NativeBalance) {
            let account_id = Self::account_id();
            let balance = <T as pallet::Config>::Currency::free_balance(&account_id)
                .saturating_sub(<T as pallet::Config>::Currency::minimum_balance());

            (account_id, balance.into())
        }

        pub fn asset_account_id(asset_id: T::AssetId) -> T::AccountId {
            PALLET_ID.into_sub_account(asset_id)
        }

        fn asset_pool(asset_id: T::AssetId) -> (T::AccountId, T::SwapAssetBalance) {
            let account_id = Self::asset_account_id(asset_id);

            // FIXME: Should minimum_balance be considered here?
            let asset_balance = <pallet_assets::Pallet<T>>::balance(asset_id, &account_id);
            (account_id, asset_balance.into())
        }

        fn native_pool(asset_id: T::AssetId) -> (T::AccountId, T::NativeBalance) {
            let account_id = Self::asset_account_id(asset_id);

            let balance = <T as pallet::Config>::Currency::free_balance(&account_id)
                .saturating_sub(<T as pallet::Config>::Currency::minimum_balance());
            (account_id, balance.into())
        }
    }
}
