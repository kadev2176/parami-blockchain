#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, MaybeDisplay, MaybeFromStr, Verify},
    MultiAddress, MultiSignature, Perbill,
};

#[derive(Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BalanceWrapper<Balance> {
    pub amount: Balance,
}

#[cfg(feature = "std")]
impl<T> Serialize for BalanceWrapper<T>
where
    T: MaybeDisplay + MaybeFromStr,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.amount.to_string())
    }
}

#[cfg(feature = "std")]
impl<'de, T> Deserialize<'de> for BalanceWrapper<T>
where
    T: MaybeDisplay + MaybeFromStr,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s
            .parse::<T>()
            .map_err(|_| serde::de::Error::custom("Parse from string failed"))?;

        Ok(s.into())
    }
}

impl<T> From<T> for BalanceWrapper<T>
where
    T: MaybeDisplay + MaybeFromStr,
{
    fn from(amount: T) -> Self {
        BalanceWrapper { amount }
    }
}

impl<T> Into<u32> for BalanceWrapper<T>
where
    T: Into<u32>,
{
    fn into(self) -> u32 {
        self.amount.into()
    }
}

impl<T> Into<u64> for BalanceWrapper<T>
where
    T: Into<u64>,
{
    fn into(self) -> u64 {
        self.amount.into()
    }
}

impl<T> Into<u128> for BalanceWrapper<T>
where
    T: Into<u128>,
{
    fn into(self) -> u128 {
        self.amount.into()
    }
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// ID of an asset.
pub type AssetId = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Expressing timestamp.
pub type Moment = u64;

/// DID of an account.
pub type DecentralizedId = sp_core::H160;

pub mod constants {
    use super::{Balance, BlockNumber, Perbill};

    /// This determines the average expected block time that we are targeting.
    /// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
    /// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
    /// up by `pallet_aura` to implement `fn slot_duration()`.
    ///
    /// Change this to adjust the block time.
    pub const MILLISECS_PER_BLOCK: u64 = 12000;

    // NOTE: Currently it is not possible to change the slot duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

    // Time is measured by number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;

    // Unit = the base number of indivisible units for balances
    pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
    pub const CENTS: Balance = DOLLARS / 100;
    pub const MILLICENTS: Balance = CENTS / 1_000;

    /// The existential deposit. Set to 1/10 of the Connected Relay Chain.
    pub const EXISTENTIAL_DEPOSIT: Balance = MILLICENTS;

    /// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
    /// used to limit the maximal weight of a single extrinsic.
    pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

    /// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
    /// `Operational` extrinsics.
    pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10 * MINUTES;
    pub const EPOCH_DURATION_IN_SLOTS: u64 = {
        const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

        (EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
    };
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
