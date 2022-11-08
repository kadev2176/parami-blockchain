#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod migrations;
mod types;

use frame_support::pallet_prelude::*;
use frame_support::traits::Hooks;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{Inspect as FungInspect, Transfer as FungTransfer},
        Currency, StorageVersion,
    },
    weights::{Pays, Weight},
    Blake2_256, PalletId, StorageHasher,
};

use crate::types::RewardInfo;
use frame_system::pallet_prelude::BlockNumberFor;
use frame_system::pallet_prelude::*;
use parami_did::EnsureDid;
use parami_did::Pallet as Did;
use parami_nft::Pallet as Nft;
use parami_traits::Tags;
use sp_core::crypto::AccountId32;
use sp_core::crypto::ByteArray;
use sp_core::U512;
use sp_io::hashing::keccak_256;
use sp_runtime::{
    traits::{AccountIdConversion, Hash, Saturating, Verify, Zero},
    DispatchError, MultiSignature,
};
use sp_std::prelude::*;
use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetsOf<T> = <T as parami_nft::Config>::AssetId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>;
type NftOf<T> = <T as parami_nft::Config>::AssetId;
type SlotMetaOf<T> = types::Slot<HashOf<T>, HeightOf<T>, NftOf<T>, AssetsOf<T>, AccountOf<T>>;
type TagOf = <Blake2_256 as StorageHasher>::Output;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config //
        + parami_did::Config
        + parami_nft::Config
        + parami_advertiser::Config
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The minimum fee balance required to keep alive an ad in fractions
        #[pallet::constant]
        type MinimumFeeBalance: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of budgets
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The maximum lifetime of a slot
        #[pallet::constant]
        type SlotLifetime: Get<HeightOf<Self>>;

        /// The means of storing the tags and tags of advertisement
        type Tags: Tags<TagOf, HashOf<Self>, DidOf<Self>>;

        /// The origin which may do calls
        type CallOrigin: EnsureOrigin<Self::Origin, Success = (DidOf<Self>, AccountOf<Self>)>;

        /// The origin which may forcibly drawback or destroy an advertisement or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Metadata of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Identity, HashOf<T>, MetaOf<T>>;

    /// Advertisement of an advertiser
    #[pallet::storage]
    #[pallet::getter(fn ads_of)]
    pub(super) type AdsOf<T: Config> = StorageMap<_, Identity, DidOf<T>, Vec<HashOf<T>>>;

    /// End time of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn endtime_of)]
    pub(super) type EndtimeOf<T: Config> = StorageMap<_, Identity, HashOf<T>, HeightOf<T>>;

    /// Deadline of an advertisement in a slot
    #[pallet::storage]
    #[pallet::getter(fn deadline_of)]
    pub(super) type DeadlineOf<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        NftOf<T>, // KOL NFT ID
        Identity,
        HashOf<T>,
        HeightOf<T>,
    >;

    /// Slot of a NFT
    #[pallet::storage]
    #[pallet::getter(fn slot_of)]
    pub(super) type SlotOf<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, SlotMetaOf<T>>;

    #[pallet::storage]
    pub(super) type Ad2DelegateAccount<T: Config> = StorageMap<_, Identity, HashOf<T>, DidOf<T>>;

    /// Payouts of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn payout)]
    pub(super) type Payout<T: Config> = StorageDoubleMap<
        _,
        Identity,
        HashOf<T>,
        Identity,
        DidOf<T>, //
        HeightOf<T>,
    >;

    #[pallet::storage]
    pub(super) type CanRate<T: Config> =
        StorageDoubleMap<_, Identity, HashOf<T>, Identity, DidOf<T>, bool, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New advertisement created \[id, creator\]
        Created(HashOf<T>, DidOf<T>),
        /// Budget added to an advertisement \[nft_id, owner, value\] slot
        Deposited(NftOf<T>, DidOf<T>, BalanceOf<T>),
        /// Advertisement updated \[id\]
        Updated(HashOf<T>),
        /// Advertiser bid for slot \[kol, id, value\]
        Bid(NftOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement (in slot) deadline reached \[kol, id, value\]
        End(NftOf<T>, HashOf<T>, BalanceOf<T>),
        /// Advertisement payout \[id, nft, visitor, value, referrer, value\]
        Paid(
            HashOf<T>,
            NftOf<T>,
            DidOf<T>,
            BalanceOf<T>,
            Option<DidOf<T>>,
            BalanceOf<T>,
        ),
        /// Swap Triggered \[id, kol, remain\]
        SwapTriggered(HashOf<T>, NftOf<T>, BalanceOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::begin_block(n).unwrap_or_else(|e| {
                sp_runtime::print(e);
                0
            })
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        EmptyTags,
        InsufficientBalance,
        InsufficientFractions,
        InsufficientFungibles,
        NotExists,
        NotMinted,
        NotOwnedOrDelegated,
        Paid,
        ScoreOutOfRange,
        TagNotExists,
        Underbid,
        FungiblesNotEqualToFractions,
        WrongPayoutSetting,
        DrawbackFailedForDidNotExists,
        SlotNotExists,
        FungibleNotForSlot,
        InvalidSignature,
        Overflow,
        Rated,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight((0, Pays::No))]
        pub fn create(
            origin: OriginFor<T>,
            tags: Vec<Vec<u8>>,
            metadata: Vec<u8>,
            reward_rate: u16,
            deadline: HeightOf<T>,
            payout_base: BalanceOf<T>,
            payout_min: BalanceOf<T>,
            payout_max: BalanceOf<T>,
            delegate_account: Option<DidOf<T>>,
        ) -> DispatchResult {
            let created = <frame_system::Pallet<T>>::block_number();

            ensure!(deadline > created, Error::<T>::Deadline);
            //TODO: ensure!(payout_base > xxx)
            ensure!(payout_min < payout_max, Error::<T>::WrongPayoutSetting);
            let (creator, who) = T::CallOrigin::ensure_origin(origin)?;

            for tag in &tags {
                ensure!(T::Tags::exists(tag), Error::<T>::TagNotExists);
            }

            // 1. derive deposit poll account and advertisement ID

            // TODO: use a HMAC-based algorithm.
            // FIXME: Ad id would be the same if user create multiple ads in one block
            let mut raw = <AccountOf<T>>::encode(&who);
            let mut ord = T::BlockNumber::encode(&created);
            raw.append(&mut ord);

            let id = <T as frame_system::Config>::Hashing::hash(&raw);

            // 2. insert metadata, ads_of, tags_of

            <Metadata<T>>::insert(
                &id,
                types::Metadata {
                    id,
                    creator,
                    metadata,
                    reward_rate,
                    created,
                    payout_base,
                    payout_min,
                    payout_max,
                },
            );

            <EndtimeOf<T>>::insert(&id, deadline);

            <AdsOf<T>>::mutate(&creator, |maybe| {
                if let Some(ads) = maybe {
                    ads.push(id);
                } else {
                    *maybe = Some(vec![id].into());
                }
            });

            for tag in tags {
                T::Tags::add_tag(&id, tag)?;
            }

            if let Some(did) = delegate_account {
                Ad2DelegateAccount::<T>::insert(id, did);
            }

            Self::deposit_event(Event::Created(id, creator));

            Ok(())
        }

        #[pallet::weight((0, Pays::No))]
        pub fn update_reward_rate(
            origin: OriginFor<T>,
            id: HashOf<T>,
            reward_rate: u16,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let mut meta = Self::ensure_owned_or_delegated_by_ad_id(did, id)?;

            meta.reward_rate = reward_rate;

            <Metadata<T>>::insert(&id, meta);

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight((0, Pays::No))]
        pub fn update_tags(
            origin: OriginFor<T>,
            id: HashOf<T>,
            tags: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let endtime = <EndtimeOf<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            Self::ensure_owned_or_delegated_by_ad_id(did, id)?;

            for tag in &tags {
                ensure!(T::Tags::exists(tag), Error::<T>::TagNotExists);
            }

            T::Tags::clr_tag(&id)?;
            for tag in tags {
                T::Tags::add_tag(&id, tag)?;
            }

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight((0, Pays::No))]
        pub fn bid_with_fraction(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            #[pallet::compact] fraction_value: BalanceOf<T>,
            fungible_id: Option<AssetsOf<T>>,
            fungibles: Option<BalanceOf<T>>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();
            let endtime = <EndtimeOf<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let fungibles = match fungibles {
                Some(fungibles) if fungibles > Zero::zero() && fungible_id.is_some() => {
                    let fungible_id = fungible_id.unwrap();
                    ensure!(
                        fungibles <= T::Assets::reducible_balance(fungible_id, &who, true),
                        Error::<T>::InsufficientFungibles
                    );
                    fungibles
                }
                _ => Zero::zero(),
            };

            let ad_meta = Self::ensure_owned_or_delegated_by_ad_id(did, ad_id)?;

            let nft_meta = Nft::<T>::meta(nft_id).ok_or(Error::<T>::NotMinted)?;
            ensure!(nft_meta.minted, Error::<T>::NotMinted);

            let created = <frame_system::Pallet<T>>::block_number();

            // check account has enough balance
            let fraction_balance =
                T::Assets::reducible_balance(nft_meta.token_asset_id, &who, false);
            ensure!(
                fraction_balance >= fraction_value,
                Error::<T>::InsufficientFractions
            );

            // 1. check slot of kol
            let slot = <SlotOf<T>>::get(nft_id);

            // 2. if slot is used
            // require a 20% increase of current budget
            // and drawback current ad

            if let Some(slot) = slot {
                let locked_fractions = Self::slot_current_fraction_balance(&slot);

                ensure!(
                    fraction_value.saturating_mul(100u32.into())
                        > locked_fractions.saturating_mul(120u32.into()),
                    Error::<T>::Underbid
                );

                Self::drawback(&slot)?;
            }

            // 3. deposit fractions and fungibles
            let pot = Self::generate_slot_pot(nft_id);
            T::Assets::transfer(nft_meta.token_asset_id, &who, &pot, fraction_value, false)?;

            if let Some(fungible_id) = fungible_id {
                T::Assets::transfer(fungible_id, &who, &pot, fungibles, false)?;
            }

            // 4. update slot

            let lifetime = T::SlotLifetime::get();
            let slotlife = created.saturating_add(lifetime);
            let deadline = if slotlife > endtime {
                endtime
            } else {
                slotlife
            };

            let slot = types::Slot {
                ad_id,
                nft_id,
                fraction_id: nft_meta.token_asset_id,
                budget_pot: pot,
                fungible_id,
                created,
            };

            <SlotOf<T>>::insert(nft_id, &slot);
            <DeadlineOf<T>>::insert(nft_id, &ad_id, deadline);
            <Metadata<T>>::insert(&ad_id, &ad_meta);

            Self::deposit_event(Event::Bid(nft_id, ad_id, fraction_value));

            Ok(())
        }

        #[pallet::weight((0, Pays::No))]
        pub fn add_budget(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            #[pallet::compact] fraction_value: BalanceOf<T>,
            fungible_id: Option<AssetsOf<T>>,
            fungible_value: Option<BalanceOf<T>>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();
            let endtime = <EndtimeOf<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;
            ensure!(endtime > height, Error::<T>::Deadline);

            let slot = <SlotOf<T>>::get(nft_id).ok_or(Error::<T>::SlotNotExists)?;
            ensure!(slot.ad_id == ad_id, Error::<T>::NotOwnedOrDelegated);

            ensure!(
                T::Assets::balance(slot.fraction_id, &who) >= fraction_value,
                Error::<T>::InsufficientFractions
            );

            ensure!(
                fungible_id == slot.fungible_id,
                Error::<T>::FungibleNotForSlot
            );
            if let (Some(fungible_id), Some(fungible_value)) = (fungible_id, fungible_value) {
                ensure!(
                    T::Assets::balance(fungible_id, &who) >= fungible_value,
                    Error::<T>::InsufficientFungibles
                );

                T::Assets::transfer(fungible_id, &who, &slot.budget_pot, fungible_value, false)?;
            }

            T::Assets::transfer(
                slot.fraction_id,
                &who,
                &slot.budget_pot,
                fraction_value,
                false,
            )?;

            Self::deposit_event(Event::Deposited(nft_id, did, fraction_value));

            Ok(())
        }

        /// The signature param is combined by ad_id, nft_id, visitor, scores, referrer
        ///
        #[pallet::weight((0, Pays::No))]
        pub fn claim(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            visitor: DidOf<T>,
            scores: Vec<(Vec<u8>, i8)>,
            referrer: Option<DidOf<T>>,
            signature: MultiSignature,
            signer: AccountId32, // advertiser or delegator
        ) -> DispatchResult {
            let (_, _) = EnsureDid::<T>::ensure_origin(origin)?;

            let msg = Self::construct_claim_sig_msg(&ad_id, nft_id, &visitor, &scores, &referrer);

            ensure!(
                signature.verify(msg.as_slice(), &signer.clone().into()),
                Error::<T>::InvalidSignature
            );

            let signer_account = T::AccountId::decode(&mut signer.as_slice().clone())
                .map_err(|_e| Error::<T>::NotOwnedOrDelegated)?;
            let signer_did = Did::<T>::lookup_did_by_account_id(signer_account.clone())
                .ok_or(Error::<T>::NotOwnedOrDelegated)?;
            Self::pay_inner(
                &ad_id,
                nft_id,
                &visitor,
                &scores,
                &referrer,
                &Option::Some(signer_did),
                &Option::Some(signer_account),
            )
        }

        #[pallet::weight((0, Pays::No))]
        pub fn claim_without_advertiser_signature(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            _scores: Vec<(Vec<u8>, i8)>,
            referrer: Option<DidOf<T>>,
        ) -> DispatchResult {
            let (origin_did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            let tag_hashes: Vec<TagOf> = T::Tags::tags_of(&ad_id).into_keys().collect();

            let tag_names = T::Tags::tag_names(tag_hashes).into_values();

            let scores = tag_names.map(|t| (t, -5i8)).collect();

            let result = Self::pay_inner(
                &ad_id,
                nft_id,
                &origin_did,
                &scores,
                &referrer,
                &Option::None,
                &Option::None,
            );

            CanRate::<T>::insert(ad_id, origin_did, true);

            result
        }

        #[pallet::weight((0, Pays::No))]
        pub fn rate(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            visitor_did: DidOf<T>,
            scores: Vec<(Vec<u8>, i8)>,
        ) -> DispatchResult {
            let (origin_did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            Self::ensure_owned_or_delegated_by_ad_id(origin_did, ad_id)?;
            ensure!(CanRate::<T>::get(ad_id, visitor_did), Error::<T>::Rated);

            CanRate::<T>::insert(ad_id, visitor_did, false);

            for (tag, score) in scores {
                ensure!(T::Tags::has_tag(&ad_id, &tag), Error::<T>::TagNotExists);
                ensure!(score >= -5 && score <= 5, Error::<T>::ScoreOutOfRange);

                // recover scores
                T::Tags::influence(&visitor_did, &tag, (score + 5) as i32)?;
            }

            return Ok(());
        }

        #[pallet::weight(<T as Config>::WeightInfo::pay())]
        pub fn pay(
            origin: OriginFor<T>,
            ad_id: HashOf<T>,
            nft_id: NftOf<T>,
            visitor: DidOf<T>,
            scores: Vec<(Vec<u8>, i8)>,
            referrer: Option<DidOf<T>>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            Self::pay_inner(
                &ad_id,
                nft_id,
                &visitor,
                &scores,
                &referrer,
                &Option::Some(did),
                &Option::Some(who),
            )
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {}

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {}
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {}
    }
}

impl<T: Config> Pallet<T> {
    fn begin_block(now: HeightOf<T>) -> Result<Weight, DispatchError> {
        let mut read = 0;
        let mut write = 0;

        let mut amount = 0;

        for (nft_id, ad_id, deadline) in <DeadlineOf<T>>::iter() {
            read += 1;

            if amount >= 100 {
                break;
            }

            if deadline > now {
                continue;
            }

            read += 1;
            let slot = <SlotOf<T>>::get(nft_id);
            if let Some(slot) = slot {
                if slot.ad_id != ad_id {
                    continue;
                }

                read += 2;
                write += 4;
                Self::drawback(&slot)?;

                amount += 1;
            }
        }

        let mut amount = 0;

        for (ad_id, endtime) in <EndtimeOf<T>>::iter() {
            read += 1;

            if amount >= 100 {
                break;
            }

            if endtime > now {
                continue;
            }

            write += 1;
            <EndtimeOf<T>>::remove(ad_id);

            amount += 1;
        }

        Ok(T::DbWeight::get().reads_writes(read as Weight, write as Weight))
    }

    fn drawback(slot: &SlotMetaOf<T>) -> Result<(), DispatchError> {
        let meta = <Metadata<T>>::get(slot.ad_id).ok_or(Error::<T>::NotExists)?;

        let owner_account =
            Did::<T>::lookup_did(meta.creator).ok_or(Error::<T>::DrawbackFailedForDidNotExists)?;

        if let Some(fungible_id) = slot.fungible_id {
            let locking_fungibles = T::Assets::balance(fungible_id, &slot.budget_pot);

            T::Assets::transfer(
                fungible_id,
                &slot.budget_pot,
                &owner_account,
                locking_fungibles,
                false,
            )?;
        }

        let locking_fractions = Self::slot_current_fraction_balance(&slot);
        T::Assets::transfer(
            slot.nft_id,
            &slot.budget_pot,
            &owner_account,
            locking_fractions,
            false,
        )?;

        <SlotOf<T>>::remove(slot.nft_id);

        <DeadlineOf<T>>::remove(slot.nft_id, slot.ad_id);

        Self::deposit_event(Event::End(slot.nft_id, slot.ad_id, locking_fractions));

        Ok(())
    }

    fn ensure_owned_or_delegated_by_ad_id(
        did: DidOf<T>,
        id: HashOf<T>,
    ) -> Result<MetaOf<T>, DispatchError> {
        let meta = <Metadata<T>>::get(&id).ok_or(Error::<T>::NotExists)?;
        ensure!(
            meta.creator == did
                || Ad2DelegateAccount::<T>::get(id)
                    .filter(|delegate| *delegate == did)
                    .is_some(),
            Error::<T>::NotOwnedOrDelegated
        );

        Ok(meta)
    }

    fn ensure_owned_or_delegated_by_meta(
        did: DidOf<T>,
        meta: &MetaOf<T>,
    ) -> Result<(), DispatchError> {
        ensure!(
            meta.creator == did
                || Ad2DelegateAccount::<T>::get(meta.id)
                    .filter(|delegate| *delegate == did)
                    .is_some(),
            Error::<T>::NotOwnedOrDelegated
        );
        Ok(())
    }

    fn slot_current_fraction_balance(slot: &SlotMetaOf<T>) -> BalanceOf<T> {
        T::Assets::balance(slot.fraction_id, &slot.budget_pot)
    }

    fn generate_slot_pot(nft_id: NftOf<T>) -> AccountOf<T> {
        let nft_raw = <NftOf<T>>::encode(&nft_id);
        let hash = <T as frame_system::Config>::Hashing::hash(&nft_raw);
        <T as Config>::PalletId::get().into_sub_account_truncating(hash)
    }

    fn construct_claim_sig_msg(
        ad_id: &HashOf<T>,
        nft_id: NftOf<T>,
        visitor: &DidOf<T>,
        scores: &Vec<(Vec<u8>, i8)>,
        referrer: &Option<DidOf<T>>,
    ) -> [u8; 32] {
        let mut msg_vec: Vec<u8> = vec![];
        msg_vec.extend(ad_id.as_ref());
        msg_vec.extend(nft_id.encode());
        msg_vec.extend(visitor.as_ref());

        for (tag, score) in scores {
            msg_vec.extend(tag);
            msg_vec.extend(score.encode());
        }

        if let Some(referrer_did) = referrer {
            msg_vec.extend(referrer_did.as_ref());
        }
        keccak_256(msg_vec.as_slice())
    }

    fn pay_inner(
        ad_id: &HashOf<T>,
        nft_id: NftOf<T>,
        visitor: &DidOf<T>,
        scores: &Vec<(Vec<u8>, i8)>,
        referrer: &Option<DidOf<T>>,
        signer_did: &Option<DidOf<T>>,
        signer_account: &Option<AccountOf<T>>,
    ) -> Result<(), DispatchError> {
        let height = <frame_system::Pallet<T>>::block_number();

        let endtime = <EndtimeOf<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;
        ensure!(endtime > height, Error::<T>::Deadline);

        let ad_meta = <Metadata<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;

        if let Some(did) = signer_did {
            Self::ensure_owned_or_delegated_by_meta(*did, &ad_meta)?;
        }

        let nft_meta = Nft::<T>::meta(nft_id).ok_or(Error::<T>::NotMinted)?;
        ensure!(nft_meta.minted, Error::<T>::NotMinted);

        let deadline = <DeadlineOf<T>>::get(nft_id, &ad_id).ok_or(Error::<T>::NotExists)?;
        ensure!(deadline > height, Error::<T>::Deadline);

        ensure!(
            !<Payout<T>>::contains_key(&ad_id, &visitor),
            Error::<T>::Paid
        );

        // 1. get slot, check current ad
        let slot = <SlotOf<T>>::get(nft_id).ok_or(Error::<T>::NotExists)?;
        ensure!(slot.ad_id == *ad_id, Error::<T>::Underbid);

        // 2. scoring visitor
        let RewardInfo {
            total: _amount,
            for_visitor: reward,
            for_referrer: award,
            fungibles,
        } = Self::calculate_reward_inner(&ad_id, &visitor, referrer, &ad_meta, &slot)?;

        // 3. influence visitor
        for (tag, score) in scores {
            ensure!(T::Tags::has_tag(&ad_id, &tag), Error::<T>::TagNotExists);
            ensure!(*score >= -5 && *score <= 5, Error::<T>::ScoreOutOfRange);

            let signer_is_advertiser = signer_account
                .clone()
                .map(|account| parami_advertiser::Pallet::<T>::is_advertiser(&account))
                .unwrap_or(false);

            if signer_is_advertiser || *score < 0i8 {
                T::Tags::influence(&visitor, &tag, *score as i32)?;
            }
        }

        // 4. payout assets
        // 4.1 pay nft fractions to visitor
        let account = Did::<T>::lookup_did(*visitor).ok_or(parami_did::Error::<T>::DidNotExists)?;
        T::Assets::transfer(slot.nft_id, &slot.budget_pot, &account, reward, false)?;

        // 4.2 pay nft fractions to referrer
        if let Some(referrer) = referrer {
            let referrer_account =
                Did::<T>::lookup_did(*referrer).ok_or(parami_did::Error::<T>::DidNotExists)?;
            T::Assets::transfer(
                slot.nft_id,
                &slot.budget_pot,
                &referrer_account,
                award,
                false,
            )?;
        }

        // 4.3 pay extra_fungible to visitor
        if let Some(fungible_id) = slot.fungible_id {
            T::Assets::transfer(fungible_id, &slot.budget_pot, &account, fungibles, false)?;
        }

        // 5. Update slot metadata
        <SlotOf<T>>::insert(nft_id, &slot);

        <Payout<T>>::insert(&ad_id, &visitor, height);

        Self::deposit_event(Event::Paid(
            ad_id.clone(),
            slot.nft_id,
            visitor.clone(),
            reward,
            referrer.clone(),
            award,
        ));

        // 6. drawback if advertiser does not have enough fees
        if Self::slot_current_fraction_balance(&slot) < T::MinimumFeeBalance::get() {
            Self::drawback(&slot)?;
        }

        Ok(())
    }

    fn calculate_reward_inner(
        ad_id: &HashOf<T>,
        visitor: &DidOf<T>,
        referrer: &Option<DidOf<T>>,
        ad_meta: &MetaOf<T>,
        slot: &SlotMetaOf<T>,
    ) -> Result<RewardInfo<BalanceOf<T>>, DispatchError> {
        let mut scoring = 5i32;

        let tags = T::Tags::tags_of(&ad_id);
        let personas = T::Tags::personas_of(&visitor);
        let length = tags.len();
        for (tag, score) in personas {
            let delta = if tags.contains_key(&tag) {
                score.saturating_mul(10)
            } else {
                score
            };
            scoring.saturating_accrue(delta);
        }

        scoring /= length.saturating_mul(10).saturating_add(1) as i32;

        if scoring < 0 {
            return Ok(RewardInfo {
                total: 0u32.into(),
                for_visitor: 0u32.into(),
                for_referrer: 0u32.into(),
                fungibles: 0u32.into(),
            });
        }

        let scoring = scoring as u32;

        let mut amount = ad_meta.payout_base.saturating_mul(scoring.into());
        if amount < ad_meta.payout_min {
            amount = ad_meta.payout_min;
        }
        if amount > ad_meta.payout_max {
            amount = ad_meta.payout_max;
        }

        let fraction_free_balance = Self::slot_current_fraction_balance(&slot);

        let amount = amount.min(fraction_free_balance);

        let award = if let Some(_referrer) = referrer {
            let rate = ad_meta.reward_rate.into();
            let award = amount.saturating_mul(rate) / 100u32.into();
            award
        } else {
            Zero::zero()
        };

        let fungibles: BalanceOf<T> = if let Some(fungible_id) = slot.fungible_id {
            let amount: U512 = Self::try_into(amount.clone())?;
            let free_balance: U512 = Self::try_into(fraction_free_balance)?;
            let fungibles_balance: U512 =
                Self::try_into(T::Assets::balance(fungible_id, &slot.budget_pot))?;

            let fungibles = fungibles_balance * amount / free_balance;
            let fungibles: u128 = fungibles.try_into()?;

            BalanceOf::<T>::try_from(fungibles).map_err(|_e| Error::<T>::Overflow)?
        } else {
            Zero::zero()
        };

        let reward = amount.saturating_sub(award.clone());
        Ok(RewardInfo {
            total: amount,
            for_visitor: reward,
            for_referrer: award,
            fungibles,
        })
    }

    pub fn cal_reward(
        ad_id: HashOf<T>,
        nft_id: NftOf<T>,
        did: DidOf<T>,
        referrer: Option<DidOf<T>>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let ad_meta = <Metadata<T>>::get(&ad_id).ok_or(Error::<T>::NotExists)?;
        let slot = SlotOf::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
        Self::calculate_reward_inner(&ad_id, &did, &referrer, &ad_meta, &slot)
            .map(|three_balance| three_balance.for_visitor)
    }

    pub fn try_into<TI: TryInto<u128>, TF: TryFrom<u128>>(value: TI) -> Result<TF, DispatchError> {
        let val: u128 = TryInto::try_into(value).map_err(|_| Error::<T>::Overflow)?;
        let ret_val: TF = TryFrom::try_from(val).map_err(|_| Error::<T>::Overflow)?;
        return Ok(ret_val);
    }
}
