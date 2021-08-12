#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin, ExistenceRequirement::KeepAlive, ReservableCurrency},
	transactional,
	weights::PostDispatchInfo,
	PalletId,
};
use frame_system::pallet_prelude::*;
use sp_runtime::{
	traits::{AccountIdConversion, One, Saturating, Verify},
	DispatchErrorWithPostInfo, FixedPointNumber, PerU16,
};
use sp_std::vec::Vec;

mod mock;
mod tests;

pub use parami_did::DidMethodSpecId;
pub use parami_primitives::Balance;
mod utils;
pub use utils::*;
mod types;
pub use types::*;
mod constants;
pub use constants::*;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config:
		pallet_timestamp::Config<AccountId = parami_primitives::AccountId> + parami_did::Config
	{
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Required `origin` for updating configuration
		type ConfigOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// an advertiser was created. \[who, did, advertiserId\]
		CreatedAdvertiser(T::AccountId, DidMethodSpecId, AdvertiserId),
		/// an advertisement was created. \[did, advertiserId, adId\]
		CreatedAd(DidMethodSpecId, AdvertiserId, AdId),
		AdReward(AdvertiserId, AdId, Balance),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The DID does not exist.
		DIDNotExists,
		/// Id overflow.
		NoAvailableId,
		/// Cannot find the advertiser.
		AdvertiserNotExists,
		/// Invalid Tag Coefficient Count
		InvalidTagCoefficientCount,
		/// Invalid Tag Type
		InvalidTagType,
		/// Duplicated Tag Type
		DuplicatedTagType,
		AdvertisementNotExists,
		NoPermission,
		ObsoletedDID,
		InvalidTagScoreDeltaLen,
		AdPaymentExpired,
		TagScoreDeltaOutOfRange,
		DuplicatedReward,
		TooEarlyToRedeem,
		AdvertiserExists,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_runtime_upgrade() -> Weight {
			0
		}
		fn integrity_test() {}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub advertiser_deposit: Balance,
		pub ad_deposit: Balance,
		pub extra_reward: Balance,
		pub time_decay_coefficient: PerU16,
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				advertiser_deposit: UNIT.saturating_mul(500),
				ad_deposit: UNIT.saturating_mul(20),
				extra_reward: UNIT.saturating_mul(3),
				time_decay_coefficient: PerU16::from_percent(1),
				_phantom: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			AdvertiserDeposit::<T>::put(self.advertiser_deposit);
			AdDeposit::<T>::put(self.ad_deposit);
			ExtraReward::<T>::put(self.extra_reward);
			TimeDecayCoefficient::<T>::put(self.time_decay_coefficient);
		}
	}

	/// A Coefficient to calculate the decay of an user score
	#[pallet::storage]
	pub type TimeDecayCoefficient<T: Config> = StorageValue<_, PerU16, ValueQuery>;

	/// the sender of `payout` will take an extra reward
	#[pallet::storage]
	pub type ExtraReward<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// ad deposit
	#[pallet::storage]
	pub type AdDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// advertiser deposit
	#[pallet::storage]
	pub type AdvertiserDeposit<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// Next available ID.
	#[pallet::storage]
	pub type NextId<T: Config> = StorageValue<_, GlobalId, ValueQuery>;

	/// an index for advertisers
	#[pallet::storage]
	pub type Advertisers<T: Config> =
		StorageMap<_, Blake2_128Concat, DidMethodSpecId, AdvertiserOf<T>>;

	/// an index for querying did by AdvertiserId
	#[pallet::storage]
	pub type AdvertiserById<T: Config> = StorageMap<_, Twox64Concat, AdvertiserId, DidMethodSpecId>;

	/// an index for advertisements
	#[pallet::storage]
	pub type Advertisements<T: Config> =
		StorageDoubleMap<_, Twox64Concat, AdvertiserId, Twox64Concat, AdId, AdvertisementOf<T>>;

	/// an index to tag score by tag type for every user.
	#[pallet::storage]
	pub type UserTagScores<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		DidMethodSpecId,
		Identity,
		TagType,
		(TagScore, T::Moment),
		ValueQuery,
		TagScoreDefault<T>,
	>;

	/// an index for rewards. The secondary key: `(user_did, media_did)`
	#[pallet::storage]
	pub type Rewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		AdId,
		Blake2_128Concat,
		(DidMethodSpecId, DidMethodSpecId),
		(),
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((1_000, DispatchClass::Operational))]
		#[transactional]
		pub fn update_time_decay_coefficient(
			origin: OriginFor<T>,
			#[pallet::compact] coefficient: PerU16,
		) -> DispatchResultWithPostInfo {
			T::ConfigOrigin::ensure_origin(origin)?;
			TimeDecayCoefficient::<T>::put(coefficient);
			Ok(().into())
		}

		#[pallet::weight((1_000, DispatchClass::Operational))]
		#[transactional]
		pub fn update_extra_reward(
			origin: OriginFor<T>,
			#[pallet::compact] extra_reward: Balance,
		) -> DispatchResultWithPostInfo {
			T::ConfigOrigin::ensure_origin(origin)?;
			ExtraReward::<T>::put(extra_reward);
			Ok(().into())
		}

		#[pallet::weight(1_000_000_000)]
		#[transactional]
		pub fn create_advertiser(
			origin: OriginFor<T>,
			#[pallet::compact] reward_pool: Balance,
		) -> DispatchResultWithPostInfo {
			let who: T::AccountId = ensure_signed(origin)?;
			let did: DidMethodSpecId = Self::ensure_did(&who)?;
			ensure!(Advertisers::<T>::get(&did).is_none(), Error::<T>::AdvertiserExists);

			let advertiser_id = Self::inc_id()?;
			let (deposit_account, reward_pool_account) = Self::ad_accounts(advertiser_id);

			let deposit = AdvertiserDeposit::<T>::get();
			<T as Config>::Currency::transfer(&who, &deposit_account, s!(deposit), KeepAlive)?;
			<T as Config>::Currency::transfer(
				&who,
				&reward_pool_account,
				s!(reward_pool),
				KeepAlive,
			)?;

			let a = Advertiser {
				created_time: now::<T>(),
				advertiser_id,
				deposit,
				deposit_account,
				reward_pool_account,
			};
			Advertisers::<T>::insert(did, a);
			AdvertiserById::<T>::insert(advertiser_id, did);
			Self::deposit_event(Event::CreatedAdvertiser(who, did, advertiser_id));
			Ok(().into())
		}

		#[pallet::weight(1_000_000_000)]
		#[transactional]
		pub fn create_ad(
			origin: OriginFor<T>,
			signer: T::AccountId,
			tag_coefficients: Vec<(TagType, TagCoefficient)>,
			media_reward_rate: PerU16,
		) -> DispatchResultWithPostInfo {
			let who: T::AccountId = ensure_signed(origin)?;

			ensure!(
				tag_coefficients.len() <= MAX_TAG_COUNT,
				Error::<T>::InvalidTagCoefficientCount
			);
			ensure!(!tag_coefficients.is_empty(), Error::<T>::InvalidTagCoefficientCount);

			for (tag_type, _) in &tag_coefficients {
				ensure!(*tag_type < MAX_TAG_TYPE_COUNT, Error::<T>::InvalidTagType);
				let mut count = 0;
				tag_coefficients.iter().for_each(|(t, _)| {
					if tag_type == t {
						count += 1;
					}
				});
				ensure!(count == 1, Error::<T>::DuplicatedTagType);
			}

			let did: DidMethodSpecId = Self::ensure_did(&who)?;
			let ad_id = Self::inc_id()?;
			let advertiser = Advertisers::<T>::get(&did).ok_or(Error::<T>::AdvertiserNotExists)?;
			let deposit = AdDeposit::<T>::get();

			<T as Config>::Currency::transfer(
				&who,
				&advertiser.deposit_account,
				s!(deposit),
				KeepAlive,
			)?;
			<T as Config>::Currency::reserve(&advertiser.deposit_account, s!(deposit))?;

			let ad = Advertisement {
				created_time: now::<T>(),
				deposit,
				tag_coefficients,
				signer,
				media_reward_rate,
			};
			Advertisements::<T>::insert(advertiser.advertiser_id, ad_id, ad);
			Self::deposit_event(Event::CreatedAd(did, advertiser.advertiser_id, ad_id));
			Ok(().into())
		}

		/// advertiser pays some AD3 to user.
		#[pallet::weight(1_000_000_000)]
		#[transactional]
		pub fn ad_payout(
			origin: OriginFor<T>,
			ad_id: AdId,
			user_did: DidMethodSpecId,
			media_did: DidMethodSpecId,
			tag_score_delta: Vec<TagScore>,
		) -> DispatchResultWithPostInfo {
			let advertiser: T::AccountId = ensure_signed(origin)?;
			let advertiser_did: DidMethodSpecId = Self::ensure_did(&advertiser)?;

			let advertiser =
				Advertisers::<T>::get(&advertiser_did).ok_or(Error::<T>::AdvertiserNotExists)?;
			let ad = Advertisements::<T>::get(advertiser.advertiser_id, ad_id)
				.ok_or(Error::<T>::AdvertisementNotExists)?;
			let user = Self::lookup_index(user_did)?;
			let media = Self::lookup_index(media_did)?;

			ensure!(
				tag_score_delta.len() == ad.tag_coefficients.len(),
				Error::<T>::InvalidTagScoreDeltaLen
			);
			ensure!(
				Rewards::<T>::get(ad_id, (user_did, media_did)).is_none(),
				Error::<T>::DuplicatedReward
			);

			let (reward, reward_media, reward_user) =
				calc_reward::<T>(&ad, &user_did, Some(&tag_score_delta))?;
			<T as Config>::Currency::transfer(
				&advertiser.reward_pool_account,
				&user,
				s!(reward_user),
				KeepAlive,
			)?;
			<T as Config>::Currency::transfer(
				&advertiser.reward_pool_account,
				&media,
				s!(reward_media),
				KeepAlive,
			)?;

			Rewards::<T>::insert(ad_id, (user_did, media_did), ());
			Self::deposit_event(Event::AdReward(advertiser.advertiser_id, ad_id, reward));
			Ok(().into())
		}

		/// If advertiser fails to pay to user and media, everyone can trigger
		/// the process of payment.
		/// For the sake of fairness, the extrinsic sender will gain some extra AD3.
		#[pallet::weight(1_000_000_000)]
		#[transactional]
		pub fn payout(
			origin: OriginFor<T>,
			signature: Vec<u8>,
			advertiser_did: DidMethodSpecId,
			ad_id: AdId,
			user_did: DidMethodSpecId,
			media_did: DidMethodSpecId,
			timestamp: T::Moment,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;

			let advertiser =
				Advertisers::<T>::get(&advertiser_did).ok_or(Error::<T>::AdvertiserNotExists)?;
			let ad = Advertisements::<T>::get(advertiser.advertiser_id, ad_id)
				.ok_or(Error::<T>::AdvertisementNotExists)?;
			let user = Self::lookup_index(user_did)?;
			let media = Self::lookup_index(media_did)?;

			let signature = sr25519_signature(&signature)?;
			let deadline =
				timestamp.saturating_add(s!(ADVERTISER_PAYMENT_WINDOW + USER_PAYMENT_WINDOW));
			let advertiser_payment_deadline =
				timestamp.saturating_add(s!(ADVERTISER_PAYMENT_WINDOW));

			// check timestamp
			let now = now::<T>();
			ensure!(now <= deadline, Error::<T>::AdPaymentExpired);
			ensure!(now > advertiser_payment_deadline, Error::<T>::TooEarlyToRedeem);

			let data =
				codec::Encode::encode(&(user_did, media_did, advertiser_did, timestamp, ad_id));
			ensure!(signature.verify(&data[..], &ad.signer), Error::<T>::NoPermission);

			ensure!(
				Rewards::<T>::get(ad_id, (user_did, media_did)).is_none(),
				Error::<T>::DuplicatedReward
			);
			let (reward, reward_media, reward_user) = calc_reward::<T>(&ad, &user_did, None)?;

			let mut free: Balance = s!(free_balance::<T>(&advertiser.reward_pool_account));
			if free > reward_user {
				<T as Config>::Currency::transfer(
					&advertiser.reward_pool_account,
					&user,
					s!(reward_user),
					KeepAlive,
				)?;
				free = free.saturating_sub(reward_user);
			} else {
				<T as Config>::Currency::transfer(
					&advertiser.deposit_account,
					&user,
					s!(reward_user),
					KeepAlive,
				)?;
			}

			if free > reward_media {
				<T as Config>::Currency::transfer(
					&advertiser.reward_pool_account,
					&media,
					s!(reward_media),
					KeepAlive,
				)?;
				free = free.saturating_sub(reward_media);
			} else {
				<T as Config>::Currency::transfer(
					&advertiser.deposit_account,
					&media,
					s!(reward_media),
					KeepAlive,
				)?;
			}

			let extra_reward = ExtraReward::<T>::get();
			if free > extra_reward {
				<T as Config>::Currency::transfer(
					&advertiser.reward_pool_account,
					&sender,
					s!(extra_reward),
					KeepAlive,
				)?;
			} else {
				<T as Config>::Currency::transfer(
					&advertiser.deposit_account,
					&sender,
					s!(extra_reward),
					KeepAlive,
				)?;
			}

			Rewards::<T>::insert(ad_id, (user_did, media_did), ());
			Self::deposit_event(Event::AdReward(advertiser.advertiser_id, ad_id, reward));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn ensure_did(who: &T::AccountId) -> ResultPost<DidMethodSpecId> {
		let did: Option<DidMethodSpecId> = parami_did::Pallet::<T>::lookup_account(who.clone());
		ensure!(did.is_some(), Error::<T>::DIDNotExists);
		Ok(did.expect("Must be Some"))
	}

	fn lookup_index(did: DidMethodSpecId) -> ResultPost<T::AccountId> {
		let who: Option<T::AccountId> = parami_did::Pallet::<T>::lookup_index(did);
		ensure!(who.is_some(), Error::<T>::ObsoletedDID);
		Ok(who.expect("Must be Some"))
	}

	fn ad_accounts(id: AdvertiserId) -> (T::AccountId, T::AccountId) {
		let deposit = PalletId(*b"prm/ad/d");
		let reward_pool = PalletId(*b"prm/ad/r");
		(deposit.into_sub_account(id), reward_pool.into_sub_account(id))
	}

	fn inc_id() -> Result<GlobalId, DispatchError> {
		NextId::<T>::try_mutate(|id| -> Result<GlobalId, DispatchError> {
			let current_id = *id;
			*id = id.checked_add(GlobalId::one()).ok_or(Error::<T>::NoAvailableId)?;
			Ok(current_id)
		})
	}
}
