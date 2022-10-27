use crate as parami_stake;
use frame_support::traits::{ConstU128, ConstU32};
use frame_support::{parameter_types, traits::GenesisBuild, PalletId};
use frame_system::{self as system, EnsureRoot};

use sp_core::{sr25519, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub const ALICE: sr25519::Public = sr25519::Public([1; 32]);
//public key of secret phrase "/Bob"
pub const BOB: sr25519::Public = sr25519::Public([
    202, 53, 5, 56, 6, 22, 158, 222, 140, 155, 250, 77, 126, 205, 109, 249, 77, 192, 15, 146, 38,
    242, 60, 222, 145, 142, 98, 183, 108, 10, 190, 123,
]);
pub const CHARLIE: sr25519::Public = sr25519::Public([3; 32]);
pub const DAVE: sr25519::Public = sr25519::Public([4; 32]);

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

        Stake: parami_stake::{Pallet, Call, Storage, Event<T>},
    }
);

type AssetId = u64;
type Balance = u128;
type BlockNumber = u64;

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
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
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

parameter_types! {
    pub const AssetDeposit: Balance = 100;
    pub const ApprovalDeposit: Balance = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 0;
    pub const MetadataDepositPerByte: Balance = 0;
}

impl pallet_assets::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type AssetAccountDeposit = ConstU128<10000u128>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
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

parameter_types! {
    pub const StakePalletId: PalletId = PalletId(*b"prm/stak");
    pub const OneMillionNormalizedInitDailyOutputConst: Balance = (500_000u128 * 10u128.pow(18)) / 7u128;
    pub const SevenDaysInBlockNum: BlockNumber = 7 * 24 * 60 * 5; //7 days
}

impl parami_stake::Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type Currency = Balances;
    type PalletId = StakePalletId;
    type OneMillionNormalizedInitDailyOutput = OneMillionNormalizedInitDailyOutputConst;
    type DurationInBlockNum = SevenDaysInBlockNum;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> { balances: vec![] }
        .assimilate_storage(&mut t)
        .unwrap();

    pallet_assets::GenesisConfig::<Test> {
        assets: vec![],
        metadata: vec![],
        accounts: vec![],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
