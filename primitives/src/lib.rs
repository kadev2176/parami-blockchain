#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::{H160, H256};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type Balance = u128;

pub type AssetId = u32;

pub type BlockNumber = u32;

pub type DecentralizedId = H160;

pub type Hash = H256;

pub type Index = u32;

pub type Moment = u64;

pub type Signature = MultiSignature;

pub type Timestamp = u64;

pub mod constants {
    use super::{Balance, BlockNumber};

    pub const UNITS: Balance = 1_000_000_000_000_000_000;

    pub const DOLLARS: Balance = UNITS;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

    pub const MILLISECS_PER_BLOCK: u64 = 6000;

    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
    pub const EPOCH_DURATION_IN_SLOTS: u64 = {
        const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

        (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
    };

    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
}

pub mod names {
    pub const SOCIETY: &[u8; 8] = b"py/socie";
    pub const TREASURY: &[u8; 8] = b"py/trsry";

    pub const AD: &[u8; 8] = b"prm/ad  ";
    pub const ADVERTISER: &[u8; 8] = b"prm/ader";
    pub const DID: &[u8; 8] = b"prm/did ";
    pub const LINKER: &[u8; 8] = b"prm/link";
    pub const MAGIC: &[u8; 8] = b"prm/stab";
    pub const SWAP: &[u8; 8] = b"prm/swap";

    pub const CHAIN_BRIDGE: &[u8; 8] = b"chnbrdge";
}

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    use constants::CENTS;

    (items as Balance) * 15 * CENTS + (bytes as Balance) * 6 * CENTS
}
