// This file is part of Substrate.

// Copyright (C) 2018-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Low-level types used throughout the Substrate code.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode, MaxEncodedLen};
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature, OpaqueExtrinsic, RuntimeDebug,
};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

// NOTE:
// - `Signature` is `MultiSignature`
// - `::Signer` is `MultiSigner`.
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

// /// The type for looking up accounts. We don't expect more than 4 billion of them.
// pub type AccountIndex = u32;

/// Balance of an account. Enough for 10 decimals with 100_000_000 total supply.
pub type Balance = u128;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Token ID
pub type TokenId = u64;

/// App-specific crypto used for reporting equivocation/misbehavior in BABE and
/// GRANDPA. Any rewards for misbehavior reporting will be paid out to this
/// account.
#[cfg(feature = "std")]
pub mod report {
    use super::{Signature, Verify};
    use frame_system::offchain::AppCrypto;
    use sp_core::crypto::{key_types, KeyTypeId};

    /// Key type for the reporting module. Used for reporting BABE and GRANDPA
    /// equivocations.
    pub const KEY_TYPE: KeyTypeId = key_types::REPORTING;

    mod app {
        use sp_application_crypto::{app_crypto, sr25519};
        app_crypto!(sr25519, super::KEY_TYPE);
    }

    /// Identity of the equivocation/misbehavior reporter.
    pub type ReporterId = app::Public;

    /// An `AppCrypto` type to allow submitting signed transactions using the reporting
    /// application key as signer.
    pub struct ReporterAppCrypto;

    impl AppCrypto<<Signature as Verify>::Signer, Signature> for ReporterAppCrypto {
        type RuntimeAppPublic = ReporterId;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }
}

/// Country ID
pub type CountryId = u64;
/// Auction ID
pub type AuctionId = u64;

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AuctionType {
    Auction,
    BuyNow,
}

#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, Clone, RuntimeDebug)]
pub struct AuctionInfo<AccountId, Balance, BlockNumber> {
    /// Current bidder and bid price.
    pub bid: Option<(AccountId, Balance)>,
    /// Define which block this auction will be started.
    pub start: BlockNumber,
    /// Define which block this auction will be ended.
    pub end: Option<BlockNumber>,
}

#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, Clone, RuntimeDebug)]
pub struct AuctionItem<AccountId, BlockNumber, Balance, AssetId> {
    pub item_id: ItemId<AssetId>,
    pub recipient: AccountId,
    pub initial_amount: Balance,
    pub amount: Balance,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub auction_type: AuctionType,
}

/// Public item id for auction
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ItemId<AssetId> {
    NFT(AssetId),
    Block(u64),
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SocialTokenCurrencyId {
    NativeToken(TokenId),
    SocialToken(TokenId),
    DEXShare(TokenId, TokenId),
    MiningResource(TokenId),
}

impl SocialTokenCurrencyId {
    pub fn is_native_token_currency_id(&self) -> bool {
        matches!(self, SocialTokenCurrencyId::NativeToken(_))
    }

    pub fn is_social_token_currency_id(&self) -> bool {
        matches!(self, SocialTokenCurrencyId::SocialToken(_))
    }

    pub fn is_dex_share_social_token_currency_id(&self) -> bool {
        matches!(self, SocialTokenCurrencyId::DEXShare(_, _))
    }

    pub fn is_mining_resource_currency(&self) -> bool {
        matches!(self, SocialTokenCurrencyId::MiningResource(_))
    }

    pub fn split_dex_share_social_token_currency_id(&self) -> Option<(Self, Self)> {
        match self {
            SocialTokenCurrencyId::DEXShare(token_currency_id_0, token_currency_id_1) => {
                Some((SocialTokenCurrencyId::NativeToken(*token_currency_id_0), SocialTokenCurrencyId::SocialToken(*token_currency_id_1)))
            }
            _ => None,
        }
    }

    pub fn join_dex_share_social_currency_id(currency_id_0: Self, currency_id_1: Self) -> Option<Self> {
        match (currency_id_0, currency_id_1) {
            (SocialTokenCurrencyId::NativeToken(token_currency_id_0), SocialTokenCurrencyId::SocialToken(token_currency_id_1)) => {
                Some(SocialTokenCurrencyId::DEXShare(token_currency_id_0, token_currency_id_1))
            }
            (SocialTokenCurrencyId::SocialToken(token_currency_id_0), SocialTokenCurrencyId::NativeToken(token_currency_id_1)) => {
                Some(SocialTokenCurrencyId::DEXShare(token_currency_id_1, token_currency_id_0))
            }
            _ => None,
        }
    }
}

/// GroupCollection Id
pub type GroupCollectionId = u64;
/// Asset Id
pub type AssetId = u64;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, MaxEncodedLen)]
#[repr(u8)]
pub enum ReserveIdentifier {
    /// collator selection
	CollatorSelection,
    /// evm storage deposit
	EvmStorageDeposit,
    /// evm developer deposit
	EvmDeveloperDeposit,
    /// honzon
	Honzon,
    /// nft
	Nft,
    /// Transaction Payment
	TransactionPayment,

	// always the last, indicate number of variants
	Count,
}
/// nft balance
pub type NFTBalance = u128;

