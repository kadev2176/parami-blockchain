#![cfg(test)]

use super::*;
use crate as parami_swap;

// use frame_support::traits::tokens::fungibles::{Inspect, Transfer, Mutate};
use frame_support::{assert_ok, parameter_types};
use frame_system::EnsureRoot;
use sp_core::{sr25519, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

type Balance = u128;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: parami_assets::{Pallet, Call, Storage, Event<T>},
        Swap: parami_swap::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Call = Call;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
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
    type SS58Prefix = ();
    type OnSetCode = ();
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = u128;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}
parameter_types! {
    pub const AssetDeposit: Balance = 100;
    pub const ApprovalDeposit: Balance = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 10;
    pub const MetadataDepositPerByte: Balance = 1;
}
impl parami_assets::Config for Test {
    type Event = Event;
    // must be u128
    type Balance = u128;
    type AssetId = u32;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<u64>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
}

impl Config for Test {
    type Event = Event;
    type Currency = Balances;
    type NativeBalance = u128;
    // avoid name confliction with AssetBalance struct
    type SwapAssetBalance = u128;
}

const A: u64 = 1;
const B: u64 = 2;
const C: u64 = 3;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(A, 10000000), (B, 100000000), (C, 1)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}

#[test]
fn create_swap() {
    new_test_ext().execute_with(|| {
        let id = 1;
        // id = 1
        // min_balance = 1
        assert_ok!(Assets::create(Origin::signed(A), id, A, 1));
        // decimals = 0
        assert_ok!(Assets::set_metadata(
            Origin::signed(A),
            id,
            b"A Token".to_vec(),
            b"AAA".to_vec(),
            0
        ));

        assert_ok!(Assets::mint(Origin::signed(A), id, A, 10_00000000));
        assert_ok!(Assets::mint(Origin::signed(A), id, B, 10_00000));

        assert_ok!(Swap::create(Origin::signed(A), id));

        // create twice
        // assert!(Swap::create(Origin::signed(A), 1).is_err());
        // 1 native for 100 asset
        assert_ok!(Swap::add_liquidity(
            Origin::signed(A),
            id,
            1000,
            Some(1000_00)
        ));

        println!("native bal => {:?}", Balances::total_balance(&A));
        println!("asset bal => {:?}", Assets::balance(id, &A));

        assert_ok!(Swap::add_liquidity(Origin::signed(B), id, 1, None));

        println!("native bal => {:?}", Balances::total_balance(&B));
        println!("asset bal => {:?}", Assets::balance(id, &B));

        assert_ok!(Swap::buy(Origin::signed(A), id, 20));
        println!("asset bal => {:?}", Assets::balance(id, &A));
    });
}
