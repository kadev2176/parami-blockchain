use crate as parami_linker;
use frame_support::traits::ConstU32;
use frame_support::{parameter_types, traits::GenesisBuild, PalletId};
use frame_system::{self as system, EnsureRoot};
use parami_traits::types::Network;
use sp_core::{sr25519, H160, H256};
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Keccak256},
};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub type Extrinsic = TestXt<Call, ()>;

pub const POLKA: &[u8] = b"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";

pub const ALICE: sr25519::Public = sr25519::Public([1; 32]);
pub const BOB: sr25519::Public = sr25519::Public([2; 32]);
pub const CHARLIE: sr25519::Public = sr25519::Public([3; 32]);

pub const DID_ALICE: H160 = H160([
    0x32, 0xac, 0x79, 0x9d, //
    0x35, 0xde, 0x72, 0xa2, //
    0xae, 0x57, 0xa4, 0x6c, //
    0xa9, 0x75, 0x31, 0x9f, //
    0xbb, 0xb1, 0x25, 0xa9,
]);

pub const DID_BOB: H160 = H160([0xee; 20]);

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

        Did: parami_did::{Pallet, Call, Storage, Config<T>, Event<T>},
        Ocw: parami_ocw::{Pallet},
        Tag: parami_tag::{Pallet, Call, Storage, Config<T>, Event<T>},
        Linker: parami_linker::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

type Balance = u128;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type MaxConsumers = ConstU32<16>;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sr25519::Public;
    type Lookup = Did;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

impl parami_did::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type DecentralizedId = H160;
    type Hashing = Keccak256;
    type WeightInfo = ();
    type Nfts = ();
}

impl parami_ocw::Config for Test {}

parameter_types! {
    pub const SubmissionFee: Balance = 1;
}

impl parami_tag::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type DecentralizedId = H160;
    type SubmissionFee = SubmissionFee;
    type CallOrigin = parami_did::EnsureDid<Self>;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type WeightInfo = ();
}

parameter_types! {
    pub const PendingLifetime: u64 = 5;
    pub const UnsignedPriority: u64 = 3;
    pub const MinimumDeposit: Balance = 10;
    pub const LinkerPalletId: PalletId = PalletId(*b"prm/link");
}

impl parami_linker::Config for Test {
    type Event = Event;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type MinimumDeposit = MinimumDeposit;
    type PalletId = LinkerPalletId;
    type PendingLifetime = PendingLifetime;
    type Slash = ();
    type Tags = Tag;
    type UnsignedPriority = UnsignedPriority;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE, 100), (BOB, 100)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    parami_did::GenesisConfig::<Test> {
        ids: vec![(ALICE, DID_ALICE), (BOB, DID_BOB)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    parami_linker::GenesisConfig::<Test> {
        links: vec![(DID_ALICE, Network::Polkadot, POLKA.to_vec())],
        registrars: vec![DID_ALICE],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
