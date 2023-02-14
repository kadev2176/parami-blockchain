#![cfg_attr(not(feature = "std"), no_std)]

pub use ocw::eth_abi;
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
mod ocw;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::{
            fungibles::{
                metadata::Mutate as FungMetaMutate, Create as FungCreate, Inspect,
                Mutate as FungMutate, Transfer as FungTransfer,
            },
            nonfungibles::{Create as NftCreate, Mutate as NftMutate},
        },
        Currency, EnsureOrigin,
        ExistenceRequirement::{self},
        Get, StorageVersion,
    },
    PalletId,
};
use frame_system::offchain::SendTransactionTypes;
use parami_assetmanager::AssetIdManager;
use parami_did::EnsureDid;
use parami_traits::{
    types::{Network, Task},
    Links, Nfts, Swaps,
};
use sp_core::U512;
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32BitUnsigned, Bounded, One, Saturating, Zero},
    DispatchError, RuntimeDebug,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    prelude::*,
};
use types::ImportTask;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetOf<T> = <T as Config>::AssetId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type ExternalOf<T> = types::External<DidOf<T>>;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<DidOf<T>, AccountOf<T>, NftOf<T>, AssetOf<T>>;
type NftOf<T> = <T as Config>::AssetId;
type TaskOf<T> = Task<ImportTask<DidOf<T>>, HeightOf<T>>;
type IcoMeta<T> = types::IcoMeta<BalanceOf<T>, AccountOf<T>>;
type InfluenceMiningMetaOf<T> = types::InfluenceMiningMeta<BalanceOf<T>, AccountOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + parami_did::Config
        + parami_ocw::Config
        + SendTransactionTypes<Call<Self>>
        + parami_assetmanager::Config
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Fragments (fungible token) ID type
        type AssetId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy
            + MaxEncodedLen;

        type AssetIdManager: AssetIdManager<Self, AssetId = AssetOf<Self>>;

        /// The assets trait to create, mint, and transfer fragments (fungible token)
        type Assets: FungCreate<AccountOf<Self>, AssetId = AssetOf<Self>>
            + FungMetaMutate<AccountOf<Self>, AssetId = AssetOf<Self>>
            + FungMutate<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>
            + FungTransfer<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>;

        /// The ICO baseline of donation for currency
        #[pallet::constant]
        type DefaultInitialMintingDeposit: Get<BalanceOf<Self>>;

        /// The ICO lockup period for fragments, KOL will not be able to claim before this period
        #[pallet::constant]
        type InitialMintingLockupPeriod: Get<HeightOf<Self>>;

        /// The ICO value base of fragments, system will mint triple of the value
        /// once for KOL, once to swaps, once to supporters
        /// The maximum value of fragments is decuple of this value
        #[pallet::constant]
        type InitialMintingValueBase: Get<BalanceOf<Self>>;

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// The links trait
        type Links: Links<DidOf<Self>>;

        /// The NFT trait to create, mint non-fungible token
        type Nft: NftCreate<AccountOf<Self>, ItemId = NftOf<Self>, CollectionId = NftOf<Self>>
            + NftMutate<AccountOf<Self>, ItemId = NftOf<Self>, CollectionId = NftOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Lifetime of a pending task
        #[pallet::constant]
        type PendingLifetime: Get<HeightOf<Self>>;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The swaps trait
        type Swaps: Swaps<
            AccountOf<Self>,
            AssetId = AssetOf<Self>,
            QuoteBalance = BalanceOf<Self>,
            TokenBalance = BalanceOf<Self>,
        >;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        type NftId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy
            + MaxEncodedLen;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Total deposit in pot
    #[pallet::storage]
    pub(super) type Deposit<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, BalanceOf<T>>;

    /// Deposits by supporter in pot
    #[pallet::storage]
    pub(super) type Deposits<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        NftOf<T>,
        Identity,
        T::DecentralizedId, // Supporter
        BalanceOf<T>,
    >;

    #[pallet::storage]
    pub(super) type ClaimedFragmentAmount<T: Config> =
        StorageDoubleMap<_, Twox64Concat, NftOf<T>, Identity, T::DecentralizedId, BalanceOf<T>>;

    /// Importing in progress
    #[pallet::storage]
    pub(super) type Porting<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Twox64Concat, Network>,
            NMapKey<Blake2_128, Vec<u8>>, // Namespace
            NMapKey<Blake2_128, Vec<u8>>, // Token
        ),
        TaskOf<T>,
    >;

    /// Ported NFTs
    #[pallet::storage]
    pub(super) type Ported<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Twox64Concat, Network>,
            NMapKey<Blake2_128, Vec<u8>>, // Namespace
            NMapKey<Blake2_128, Vec<u8>>, // Token
        ),
        NftOf<T>,
    >;

    /// Imported NFTs
    #[pallet::storage]
    pub(super) type External<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, ExternalOf<T>>;

    /// Metadata
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, MetaOf<T>>;

    /// Did's preferred Nft.
    #[pallet::storage]
    #[pallet::getter(fn preferred)]
    pub(super) type Preferred<T: Config> = StorageMap<_, Identity, T::DecentralizedId, NftOf<T>>;

    /// Block Number Claim Started At
    #[pallet::storage]
    pub(super) type ClaimStartAt<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, HeightOf<T>>;

    #[pallet::storage]
    pub(super) type IcoMetaOf<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, IcoMeta<T>>;

    #[pallet::storage]
    pub(super) type InflueceMiningMetaStore<T: Config> =
        StorageMap<_, Twox64Concat, NftOf<T>, InfluenceMiningMetaOf<T>>;

    #[pallet::type_value]
    pub(crate) fn DefaultId<T: Config>() -> NftOf<T> {
        One::one()
    }

    /// Next available class ID
    #[pallet::storage]
    pub(super) type NextClassId<T: Config> = StorageValue<_, NftOf<T>, ValueQuery, DefaultId<T>>;

    #[pallet::storage]
    pub(super) type ValidateEndpoint<T: Config> =
        StorageMap<_, Twox64Concat, Network, BoundedVec<u8, ConstU32<128>>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// NFT Created \[did, instance\]
        Created(T::DecentralizedId, NftOf<T>),
        /// NFT fragments Minted \[did, instance, value\]
        Backed(T::DecentralizedId, NftOf<T>, BalanceOf<T>),
        /// NFT fragments Claimed \[did, instance, value\]
        Claimed(T::DecentralizedId, NftOf<T>, BalanceOf<T>),
        /// NFT fragments Minted \[kol, instance, token, name, symbol, tokens\]
        Minted(
            T::DecentralizedId,
            NftOf<T>,
            AssetOf<T>,
            Vec<u8>,
            Vec<u8>,
            BalanceOf<T>,
        ),
        /// Import NFT Failed \[did, network, namespace, token_id\]
        ImportFailed(T::DecentralizedId, Network, Vec<u8>, Vec<u8>),

        // NFT Influencemining Activity Started \[nftId, budget_in_tokens\]
        InfluenceMiningActivityStarted(NftOf<T>, BalanceOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            match Self::ocw_begin_block(block_number) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("An error occurred in OCW: {:?}", e);
                }
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        BadMetadata,
        Deadline,
        Exists,
        InsufficientBalance,
        Minted,
        NotExists,
        Overflow,
        YourSelf,
        NetworkNotLinked,
        OcwParseError,
        NotTokenOwner,
        InvalidSignature,
        InsufficientToken,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Import an existing NFT for crowdfunding.
        #[pallet::weight(<T as Config>::WeightInfo::port())]
        pub fn port(
            origin: OriginFor<T>,
            network: Network,
            namespace: Vec<u8>,
            token: Vec<u8>,
            owner_address: Vec<u8>,
            signature: parami_primitives::signature::Signature,
        ) -> DispatchResult {
            let (owner, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                !<Porting<T>>::contains_key((network, &namespace, &token)),
                Error::<T>::Exists
            );

            ensure!(
                !<Ported<T>>::contains_key((network, &namespace, &token)),
                Error::<T>::Exists
            );

            let msg = parami_primitives::signature::generate_message(owner.clone());
            let address = parami_primitives::signature::recover_address(
                network,
                owner_address.clone(),
                signature,
                msg,
            )
            .map_err(|_e| Error::<T>::InvalidSignature)?;
            ensure!(address == owner_address, Error::<T>::InvalidSignature);

            let created = <frame_system::Pallet<T>>::block_number();
            let lifetime = T::PendingLifetime::get();
            let deadline = created.saturating_add(lifetime);

            <Porting<T>>::insert(
                (network, &namespace.clone(), &token.clone()),
                Task {
                    task: types::ImportTask {
                        owner,
                        network,
                        namespace,
                        token,
                        owner_address: address,
                    },
                    deadline,
                    created,
                },
            );

            Ok(())
        }

        /// Create a new NFT for crowdfunding.
        #[pallet::weight(<T as Config>::WeightInfo::kick())]
        pub fn kick(origin: OriginFor<T>) -> DispatchResult {
            let (owner, _) = EnsureDid::<T>::ensure_origin(origin)?;

            Self::create(owner)?;

            Ok(())
        }

        /// Claim the fragments.
        /// ClaimInfo calculation Rules: ref to comment on [`get_claim_info_inner`](fn@get_claim_info_inner)
        #[pallet::weight(<T as Config>::WeightInfo::claim())]
        pub fn claim(origin: OriginFor<T>, nft: NftOf<T>) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            let claimed_tokens: BalanceOf<T> =
                <ClaimedFragmentAmount<T>>::get(nft, &did).unwrap_or(0u32.into());
            let (total_tokens, unlocked_tokens, claimable_tokens) = Self::get_claim_info_inner(
                nft,
                &did,
                T::InitialMintingValueBase::get(),
                T::InitialMintingLockupPeriod::get(),
                &claimed_tokens,
            )?;

            let meta = <Metadata<T>>::get(nft).ok_or(Error::<T>::NotExists)?;

            T::Assets::transfer(
                meta.token_asset_id,
                &meta.pot,
                &who,
                claimable_tokens,
                false,
            )?;

            <ClaimedFragmentAmount<T>>::mutate(nft, &did, |maybe| {
                if let Some(already_claimed) = maybe {
                    already_claimed.saturating_accrue(claimable_tokens);
                } else {
                    *maybe = Some(claimable_tokens);
                }
            });

            // When all the token has been unlocked, remove the Deposits of ${did}
            if unlocked_tokens == total_tokens {
                <Deposits<T>>::remove(nft, &did);
            }

            Self::deposit_event(Event::Claimed(did, nft, claimable_tokens));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn force_submit_ported_nft(
            origin: OriginFor<T>,
            did: DidOf<T>,
            network: Network,
            namespace: Vec<u8>,
            token: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ensure!(
                parami_did::Pallet::<T>::lookup_did(did).is_some(),
                Error::<T>::NotTokenOwner
            );

            ensure!(
                <Ported<T>>::get((network, namespace.clone(), token.clone())).is_none(),
                Error::<T>::Exists
            );

            let id = Self::create(did)?;

            <Ported<T>>::insert((network, namespace.clone(), token.clone()), id);

            <External<T>>::insert(
                id,
                types::External {
                    network,
                    namespace: namespace.clone(),
                    token: token.clone(),
                    owner: did,
                },
            );

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn submit_porting(
            origin: OriginFor<T>,
            did: DidOf<T>,
            network: Network,
            namespace: Vec<u8>,
            token: Vec<u8>,
            validated: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let task = <Porting<T>>::get((network, &namespace, &token));

            ensure!(task.is_some(), Error::<T>::NotExists);

            let task = task.unwrap();

            if validated {
                let id = Self::create(task.task.owner)?;

                <Ported<T>>::insert((network, namespace.clone(), token.clone()), id);

                <External<T>>::insert(
                    id,
                    types::External {
                        network,
                        namespace: namespace.clone(),
                        token: token.clone(),
                        owner: task.task.owner,
                    },
                );
            } else {
                Self::deposit_event(Event::ImportFailed(
                    did,
                    network,
                    namespace.clone(),
                    token.clone(),
                ));
            }

            <Porting<T>>::remove((network, namespace, token));
            Ok(().into())
        }

        //FIXME: Weight
        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn mint_nft_power(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            name: Vec<u8>,
            symbol: Vec<u8>,
            minting_tokens: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let (did, account) = EnsureDid::<T>::ensure_origin(origin)?;
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;

            ensure!(did == meta.owner, Error::<T>::NotTokenOwner);
            ensure!(!meta.minted, Error::<T>::Minted);

            let limit = T::StringLimit::get() as usize - 4;
            ensure!(
                0 < name.len() && name.len() <= limit,
                Error::<T>::BadMetadata
            );
            ensure!(
                0 < symbol.len() && symbol.len() <= limit,
                Error::<T>::BadMetadata
            );

            let is_valid_char =
                |c: &u8| c.is_ascii_whitespace() || c.is_ascii_alphanumeric() || *c == b'_';

            ensure!(name.iter().all(is_valid_char), Error::<T>::BadMetadata);
            ensure!(symbol.iter().all(is_valid_char), Error::<T>::BadMetadata);

            Self::create_nft_and_mint_power(
                nft_id,
                meta,
                minting_tokens,
                name,
                symbol,
                &account,
                did,
            )?;

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn start_ico(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            expected_currency: BalanceOf<T>,
            offered_tokens: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let (did, account) = EnsureDid::<T>::ensure_origin(origin)?;
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            ensure!(meta.minted, Error::<T>::NotExists);
            ensure!(did == meta.owner, Error::<T>::NotTokenOwner);
            ensure!(IcoMetaOf::<T>::get(nft_id).is_none(), Error::<T>::Deadline);

            let balance = T::Assets::balance(meta.token_asset_id, &account);

            ensure!(offered_tokens <= balance, Error::<T>::InsufficientToken);
            // start initial coin offering
            Self::start_initial_coin_offering(
                nft_id,
                expected_currency,
                offered_tokens,
                meta.token_asset_id,
                &account,
            )?;

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn end_ico(origin: OriginFor<T>, nft_id: NftOf<T>) -> DispatchResultWithPostInfo {
            let (did, account) = EnsureDid::<T>::ensure_origin(origin)?;
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            ensure!(did == meta.owner, Error::<T>::NotTokenOwner);
            ensure!(meta.minted, Error::<T>::NotExists);
            let ico_meta = IcoMetaOf::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            let pot_tokens = T::Assets::balance(meta.token_asset_id, &ico_meta.pot);

            Self::end_ico_inner(nft_id, ico_meta, meta, pot_tokens, account)?;

            Ok(().into())
        }

        // FIXME: weight
        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn participate_ico(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            token_amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let (did, account) = EnsureDid::<T>::ensure_origin(origin)?;
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            let ico_meta = IcoMetaOf::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            ensure!(!ico_meta.done, Error::<T>::Deadline);

            Self::buy_tokens_in_ico(nft_id, &did, token_amount, &account, &meta, &ico_meta)?;

            Ok(().into())
        }

        // FIXME: weight
        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn start_dao_influenceming_activity(
            origin: OriginFor<T>,
            nft_id: NftOf<T>,
            budget_in_tokens: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, account) = EnsureDid::<T>::ensure_origin(origin)?;
            let meta = Metadata::<T>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            ensure!(did == meta.owner, Error::<T>::NotTokenOwner);
            ensure!(meta.minted, Error::<T>::NotExists);

            let balance = T::Assets::balance(meta.token_asset_id, &account);

            ensure!(budget_in_tokens <= balance, Error::<T>::InsufficientToken);

            Self::start_dao_influencemining_acitivy_inner(
                nft_id,
                budget_in_tokens,
                meta.token_asset_id,
                &account,
            )?;

            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn set_validate_endpoint(
            origin: OriginFor<T>,
            network: Network,
            endpoint: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let endpoint: BoundedVec<u8, ConstU32<128>> = endpoint
                .try_into()
                .map_err(|_| "Endpoint exceeds maximum length")?;
            <ValidateEndpoint<T>>::insert(network, endpoint);
            Ok(())
        }

        // we should keep this call idempotent in case of the following conditions:

        // 1. this call is not transactional
        // 2. and failed due to lacks of token or currency
        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn force_import_kol(
            origin: OriginFor<T>,
            // we can't get root account from a sudo call.
            root_account: AccountOf<T>,
            kol_account: AccountOf<T>,
            power_name: Vec<u8>,
            ether_addr: Vec<u8>,
            ether_token_id: Vec<u8>,
            token_amount: BalanceOf<T>,
            currency_amount: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_root(origin.clone())?;

            let did_op = parami_did::Pallet::<T>::lookup_did_by_account_id(kol_account.clone());
            let did = if let Some(did) = did_op {
                did
            } else {
                parami_did::Pallet::<T>::create(kol_account.clone(), None)?
            };

            let nft_id = if let Some(id) = <Ported<T>>::get((
                Network::Ethereum,
                ether_addr.clone(),
                ether_token_id.clone(),
            )) {
                id
            } else {
                let id = Self::create(did)?;

                <Ported<T>>::insert(
                    (
                        Network::Ethereum,
                        ether_addr.clone(),
                        ether_token_id.clone(),
                    ),
                    id,
                );

                <External<T>>::insert(
                    id,
                    types::External {
                        network: Network::Ethereum,
                        namespace: ether_addr.clone(),
                        token: ether_token_id.clone(),
                        owner: did,
                    },
                );

                id
            };

            let meta = Metadata::<T>::get(nft_id).unwrap();
            if !meta.minted {
                Self::create_nft_and_mint_power(
                    nft_id,
                    meta,
                    token_amount,
                    power_name.clone(),
                    power_name,
                    &kol_account,
                    did,
                )?;
            }

            let _result = T::Swaps::new(nft_id);
            if T::Swaps::liquidity(nft_id, &kol_account) == 0u32.into() {
                T::Currency::transfer(
                    &root_account,
                    &kol_account,
                    currency_amount,
                    ExistenceRequirement::KeepAlive,
                )?;
                T::Swaps::mint(
                    &kol_account,
                    nft_id,
                    currency_amount,
                    currency_amount,
                    token_amount,
                    false,
                )?;
            }

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub deposit: Vec<(NftOf<T>, BalanceOf<T>)>,
        pub deposits: Vec<(NftOf<T>, T::DecentralizedId, BalanceOf<T>)>,
        pub next_instance_id: NftOf<T>,
        pub nfts: Vec<(NftOf<T>, T::DecentralizedId, bool)>,
        pub externals: Vec<(NftOf<T>, Network, Vec<u8>, Vec<u8>, T::DecentralizedId)>,
        pub validate_endpoints: Vec<(Network, Vec<u8>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                deposit: Default::default(),
                deposits: Default::default(),
                next_instance_id: Default::default(),
                nfts: Default::default(),
                externals: Default::default(),
                validate_endpoints: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NextClassId<T>>::put(self.next_instance_id);

            for (instance_id, deposit) in &self.deposit {
                <Deposit<T>>::insert(instance_id, deposit);
            }

            for (instance_id, did, deposit) in &self.deposits {
                <Deposits<T>>::insert(instance_id, did, deposit);
            }

            for (id, owner, minted) in &self.nfts {
                let id = *id;
                let minted = *minted;

                if id >= self.next_instance_id {
                    panic!("NFT ID must be less than next_instance_id");
                }

                let pot: AccountOf<T> = Pallet::<T>::generate_claim_pot(&id);

                <Metadata<T>>::insert(
                    id,
                    types::Metadata {
                        owner: owner.clone(),
                        pot: pot.clone(),
                        class_id: id,
                        token_asset_id: id,
                        minted,
                    },
                );

                <Preferred<T>>::insert(owner, id);

                if minted {
                    // MARK: pallet_uniques does not support genesis
                    T::Nft::create_collection(&id, &pot, &pot).unwrap();
                    T::Nft::mint_into(&id, &id, &pot).unwrap();

                    <ClaimStartAt<T>>::insert(id, HeightOf::<T>::zero());
                }
            }

            for (id, network, namespace, token, owner) in &self.externals {
                let id = *id;
                let network = *network;
                let owner = *owner;

                <Ported<T>>::insert((network, namespace.clone(), token.clone()), id);

                <External<T>>::insert(
                    id,
                    types::External {
                        network,
                        namespace: namespace.clone(),
                        token: token.clone(),
                        owner,
                    },
                );
            }

            for (network, endpoint) in &self.validate_endpoints {
                let endpoint: BoundedVec<u8, ConstU32<128>> = endpoint.clone().try_into().unwrap();
                <ValidateEndpoint<T>>::insert(network, endpoint);
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match source {
                TransactionSource::Local | TransactionSource::InBlock => { /* allowed */ }
                _ => return InvalidTransaction::Call.into(),
            };

            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("nft")
                    .priority(T::UnsignedPriority::get())
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(false)
                    .build()
            };

            match call {
                Call::submit_porting { .. } => valid_tx(b"submit_porting".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn create(owner: DidOf<T>) -> Result<NftOf<T>, DispatchError> {
        let id =
            <T as crate::Config>::AssetIdManager::next_id().map_err(|_e| Error::<T>::Overflow)?;
        let pot = Self::generate_claim_pot(&id);

        ensure!(!<Metadata<T>>::contains_key(id), Error::<T>::Exists);

        <Metadata<T>>::insert(
            id,
            types::Metadata {
                owner,
                pot,
                class_id: id,
                token_asset_id: id,
                minted: false,
            },
        );

        if !<Preferred<T>>::contains_key(&owner) {
            <Preferred<T>>::insert(&owner, id);
        }

        Self::deposit_event(Event::Created(owner, id));

        Ok(id)
    }

    fn try_into<S, D>(value: S) -> Result<D, DispatchError>
    where
        S: TryInto<u128>,
        D: TryFrom<u128>,
    {
        let value: u128 = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        let value: D = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        Ok(value)
    }

    /// ClaimInfo calculation Rules:
    ///   a. tokens_of_backer = depositOf(backer) / total_deposit
    ///   b. unlock following linear unlock style in T::InitialMintingLockupPeriod::get()
    ///   c. so, given block_number n, unlocked_token = tokens_of_backer * (n - minted_block_number) / InitialMintingLockupPeriod
    ///   d. the, given block_number n, claimable_token = unlock_token - claimed_token.
    fn get_claim_info_inner(
        nft: NftOf<T>,
        did: &DidOf<T>,
        _initial_tokens: BalanceOf<T>,
        initial_minting_lockup_period: HeightOf<T>,
        claimed_tokens: &BalanceOf<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), DispatchError> {
        let height = <frame_system::Pallet<T>>::block_number();

        let minted_block_number = <ClaimStartAt<T>>::get(nft).ok_or(Error::<T>::NotExists)?;

        let passed_blocks = height - minted_block_number;

        let ico_meta = <IcoMetaOf<T>>::get(nft).ok_or(Error::<T>::NotExists)?;

        let deposit = <Deposits<T>>::get(nft, &did).ok_or(Error::<T>::NotExists)?;

        let passed_blocks = passed_blocks.min(initial_minting_lockup_period);

        let deposit: U512 = Self::try_into(deposit)?;
        let passed_blocks: U512 = Self::try_into(passed_blocks)?;
        let lockup_period: U512 = Self::try_into(initial_minting_lockup_period)?;
        let expected_currency: U512 = Self::try_into(ico_meta.expected_currency)?;
        let offered_tokens: U512 = Self::try_into(ico_meta.offered_tokens)?;

        let tokens = deposit * offered_tokens / expected_currency;

        let unlocked_tokens = tokens * passed_blocks / lockup_period;
        let unlocked_tokens: BalanceOf<T> = Self::try_into(unlocked_tokens)?;
        let claimable_tokens = unlocked_tokens - *claimed_tokens;

        Ok((Self::try_into(tokens)?, unlocked_tokens, claimable_tokens))
    }
}

impl<T: Config> Nfts<AccountOf<T>> for Pallet<T> {
    type DecentralizedId = DidOf<T>;
    type Balance = BalanceOf<T>;
    type NftId = <T as pallet::Config>::AssetId;

    fn get_claim_info(
        nft_id: Self::NftId,
        claimer: &Self::DecentralizedId,
    ) -> Result<(Self::Balance, Self::Balance, Self::Balance), DispatchError> {
        let claimed_tokens: BalanceOf<T> =
            <ClaimedFragmentAmount<T>>::get(nft_id, &claimer).unwrap_or(0u32.into());
        Self::get_claim_info_inner(
            nft_id,
            claimer,
            T::InitialMintingValueBase::get(),
            T::InitialMintingLockupPeriod::get(),
            &claimed_tokens,
        )
    }

    fn get_nft_pot(nft_id: Self::NftId) -> Option<AccountOf<T>> {
        <Metadata<T>>::get(nft_id).map(|meta| meta.pot)
    }
}
use parami_traits::transferable::Transferable;

impl<T: Config> Transferable<AccountOf<T>> for Pallet<T> {
    fn transfer_all(src: &AccountOf<T>, dest: &AccountOf<T>) -> Result<(), DispatchError> {
        for (_nft_id, nft_meta) in <Metadata<T>>::iter() {
            let balance = T::Assets::balance(nft_meta.token_asset_id, &src);
            T::Assets::transfer(nft_meta.token_asset_id, src, dest, balance, false)?;
        }

        Ok(())
    }
}

impl<T: Config> Pallet<T> {
    fn mint_tokens(
        amount: BalanceOf<T>,
        metadata: &MetaOf<T>,
        owner_account: &AccountOf<T>,
        name: &Vec<u8>,
        symbol: &Vec<u8>,
    ) -> Result<(), DispatchError> {
        let asset_id = metadata.token_asset_id;
        T::Assets::create(asset_id, owner_account.clone(), true, One::one())?;
        T::Assets::set(asset_id, owner_account, name.clone(), symbol.clone(), 18)?;
        T::Assets::mint_into(asset_id, owner_account, amount)?;

        Ok(())
    }

    fn create_nft_and_mint_power(
        nft_id: NftOf<T>,
        meta: MetaOf<T>,
        minting_tokens: BalanceOf<T>,
        name: Vec<u8>,
        symbol: Vec<u8>,
        owner_account: &AccountOf<T>,
        owner_did: DidOf<T>,
    ) -> Result<(), DispatchError> {
        // create nft collection
        T::Nft::create_collection(&meta.class_id, &meta.pot, &meta.pot)?;
        T::Nft::mint_into(&meta.class_id, &nft_id, &meta.pot)?;

        // mint funds
        Self::mint_tokens(minting_tokens, &meta, &owner_account, &name, &symbol)?;

        // update metadata
        <Metadata<T>>::insert(
            nft_id,
            &MetaOf::<T> {
                minted: true,
                ..meta
            },
        );

        // send event
        Self::deposit_event(Event::Minted(
            owner_did,
            nft_id,
            nft_id,
            name,
            symbol,
            minting_tokens,
        ));

        Ok(())
    }

    fn start_initial_coin_offering(
        nft_id: NftOf<T>,
        expected_currency: BalanceOf<T>,
        offered_tokens: BalanceOf<T>,
        asset_id: AssetOf<T>,
        owner_account: &AccountOf<T>,
    ) -> Result<(), DispatchError> {
        let balance = T::Assets::balance(asset_id, owner_account);
        ensure!(balance >= offered_tokens, Error::<T>::InsufficientToken);

        let pot = Self::generate_ico_pot(&nft_id);
        T::Assets::transfer(asset_id, owner_account, &pot, offered_tokens, false)?;

        let ico_meta = IcoMeta::<T> {
            expected_currency,
            offered_tokens,
            done: false,
            pot,
        };

        IcoMetaOf::<T>::insert(nft_id, ico_meta);

        Ok(())
    }

    fn buy_tokens_in_ico(
        nft_id: NftOf<T>,
        did: &DidOf<T>,
        token_amount: BalanceOf<T>,
        dst_account: &AccountOf<T>,
        meta: &MetaOf<T>,
        ico_meta: &IcoMeta<T>,
    ) -> Result<(), DispatchError> {
        let pot = &ico_meta.pot;
        let remained_tokens = T::Assets::balance(meta.token_asset_id, pot);
        ensure!(
            remained_tokens >= token_amount,
            Error::<T>::InsufficientToken
        );
        let account_balance = T::Currency::free_balance(&dst_account);
        let required_balance = Self::calculate_required_currency(token_amount, &ico_meta)?;
        ensure!(
            account_balance >= required_balance,
            Error::<T>::InsufficientBalance
        );
        ensure!(
            Deposit::<T>::get(nft_id).unwrap_or(0u32.into()) + required_balance
                <= ico_meta.expected_currency,
            Error::<T>::InsufficientBalance
        );

        let owner_account = parami_did::Pallet::<T>::lookup_did(meta.owner).unwrap();

        T::Currency::transfer(
            &dst_account,
            &owner_account,
            required_balance,
            ExistenceRequirement::KeepAlive,
        )?;

        Deposit::<T>::mutate(nft_id, |maybe| {
            if let Some(deposit) = maybe {
                deposit.saturating_accrue(required_balance);
            } else {
                *maybe = Some(required_balance);
            }
        });

        Deposits::<T>::mutate(nft_id, did, |maybe| {
            if let Some(deposit) = maybe {
                deposit.saturating_accrue(required_balance);
            } else {
                *maybe = Some(required_balance);
            }
        });

        Ok(())
    }

    fn calculate_required_currency(
        token_amount: BalanceOf<T>,
        ico_meta: &IcoMeta<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let amount: U512 = Self::try_into(token_amount)?;
        let offered_tokens: U512 = Self::try_into(ico_meta.offered_tokens)?;
        let expected_currency: U512 = Self::try_into(ico_meta.expected_currency)?;

        let required_currency = amount * expected_currency / offered_tokens;

        Ok(Self::try_into(required_currency)?)
    }

    fn calculate_required_token(
        currency_amount: BalanceOf<T>,
        ico_meta: &IcoMeta<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let amount: U512 = Self::try_into(currency_amount)?;
        let offered_tokens: U512 = Self::try_into(ico_meta.offered_tokens)?;
        let expected_currency: U512 = Self::try_into(ico_meta.expected_currency)?;

        let required_token = amount * offered_tokens / expected_currency;

        Ok(Self::try_into(required_token)?)
    }

    fn end_ico_inner(
        nft_id: NftOf<T>,
        mut ico_meta: IcoMeta<T>,
        meta: MetaOf<T>,
        pot_tokens: BalanceOf<T>,
        account: AccountOf<T>,
    ) -> Result<(), DispatchError> {
        let deposit = Deposit::<T>::get(nft_id).unwrap_or(0u32.into());

        let mature_ico_quota = Self::calculate_required_token(deposit, &ico_meta)?;
        ensure!(
            pot_tokens >= mature_ico_quota,
            Error::<T>::InsufficientToken
        );

        // tranfer tokens to meta pot
        T::Assets::transfer(
            meta.token_asset_id,
            &ico_meta.pot,
            &meta.pot,
            mature_ico_quota,
            false,
        )?;

        // refund ico pot balance
        if pot_tokens > mature_ico_quota {
            T::Assets::transfer(
                meta.token_asset_id,
                &ico_meta.pot,
                &account,
                pot_tokens - mature_ico_quota,
                false,
            )?;
        }

        let ico_ended_at = <frame_system::Pallet<T>>::block_number();
        <ClaimStartAt<T>>::insert(nft_id, ico_ended_at);
        ico_meta.done = true;
        IcoMetaOf::<T>::insert(nft_id, ico_meta);
        Ok(())
    }

    fn start_dao_influencemining_acitivy_inner(
        nft_id: NftOf<T>,
        budget_in_tokens: BalanceOf<T>,
        asset_id: AssetOf<T>,
        owner_account: &AccountOf<T>,
    ) -> Result<(), DispatchError> {
        let pot = Self::generate_influence_mining_pot(&nft_id);
        T::Assets::transfer(asset_id, owner_account, &pot, budget_in_tokens, false)?;

        let old_meta = <InflueceMiningMetaStore<T>>::get(nft_id);
        ensure!(old_meta.is_none(), Error::<T>::Exists);
        let im_meta = InfluenceMiningMetaOf::<T> {
            budget_in_tokens,
            done: false,
            pot,
        };

        <InflueceMiningMetaStore<T>>::insert(nft_id, im_meta);

        Self::deposit_event(Event::<T>::InfluenceMiningActivityStarted(
            nft_id,
            budget_in_tokens,
        ));

        Ok(())
    }

    fn generate_claim_pot(nft_id: &NftOf<T>) -> AccountOf<T> {
        return T::PalletId::get().into_sub_account_truncating(&nft_id);
    }

    fn generate_ico_pot(nft_id: &NftOf<T>) -> AccountOf<T> {
        use sp_core::Encode;
        let mut pot_seed = b"ico".to_vec();
        pot_seed.append(&mut nft_id.encode());
        return T::PalletId::get().into_sub_account_truncating(&pot_seed);
    }

    pub fn generate_influence_mining_pot(nft_id: &NftOf<T>) -> AccountOf<T> {
        use sp_core::Encode;
        let mut pot_seed = b"ifm".to_vec();
        pot_seed.append(&mut nft_id.encode());
        return T::PalletId::get().into_sub_account_truncating(&pot_seed);
    }
}
