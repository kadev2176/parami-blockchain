use crate::types::{Score, SingleMetricScore};
use crate::MetaOf;
use crate::Metadata;
use crate::PersonasOf;
use crate::TagHash;
use crate::{Config, Pallet};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::traits::StorageVersion;
use frame_support::{traits::Get, weights::Weight, Identity, RuntimeDebug};
use log;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::H160;
use sp_runtime::traits::Saturating;

pub mod v3 {
    use crate::types;
    use parami_traits::Tags;
    use sp_std::prelude::*;

    use super::*;

    pub mod old {
        use super::*;

        pub type V2MetaOf<T> =
            V2Metadata<<T as Config>::DecentralizedId, <T as frame_system::Config>::BlockNumber>;

        #[derive(
            Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        )]
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
        pub struct V2Metadata<N, V> {
            pub creator: N,
            pub created: V,
        }

        // The old explicit storage item.
        #[frame_support::storage_alias]
        pub type Metadata<T: Config> = StorageMap<Pallet<T>, Identity, TagHash, V2MetaOf<T>>;
    }

    pub struct AddTagNameMigration<T>(sp_std::marker::PhantomData<T>);

    impl<T: crate::Config> OnRuntimeUpgrade for AddTagNameMigration<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 2 {
                return 0;
            }

            let exist_tags = [
                "Discord".as_bytes(),
                "DeFi".as_bytes(),
                "Ethereum".as_bytes(),
                "Kusama".as_bytes(),
                "Polkadot".as_bytes(),
                "Telegram".as_bytes(),
                "Twitter".as_bytes(),
            ];

            for tag in exist_tags {
                let op_meta: Option<old::V2MetaOf<T>> =
                    old::Metadata::<T>::get(Pallet::<T>::key(&tag.to_vec()));
                if op_meta.is_none() {
                    log::error!("tag not existing: {:?}", tag);
                    continue;
                }
                let meta = op_meta.unwrap();
                Metadata::<T>::insert(
                    Pallet::<T>::key(&tag.to_vec()),
                    types::Metadata {
                        creator: meta.creator.clone(),
                        created: meta.created.clone(),
                        tag: tag.to_vec(),
                    },
                );
            }

            // Deal with one tag that no one know the real meaning of it.
            let unknown_tag =
                hex::decode("82d324a29d3d3d3fe76eb33907ae9b8b940ee997c4684dc601ae8c06313a1d1d")
                    .unwrap();
            let unknown_tag_u8_32: [u8; 32] = unknown_tag.try_into().unwrap();
            let op_meta = old::Metadata::<T>::get(unknown_tag_u8_32);
            if op_meta.is_some() {
                let meta = op_meta.unwrap();
                Metadata::<T>::insert(
                    unknown_tag_u8_32,
                    types::Metadata {
                        creator: meta.creator.clone(),
                        created: meta.created.clone(),
                        tag: b"unknown".to_vec(),
                    },
                );
            } else {
                log::error!("tag not existing: unknown tag");
            }

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(3));

            exist_tags.len().try_into().unwrap()
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            let count = <old::Metadata<T>>::iter().count();
            log::info!("count of Metadata is {:?}", count);

            let final_key: Vec<u8> =
                <old::Metadata<T> as frame_support::storage::generator::StorageMap<
                    TagHash,
                    old::V2MetaOf<T>,
                >>::storage_map_final_key(Pallet::<T>::key(
                    &"Telegram".as_bytes().to_vec(),
                ));

            log::info!("final key is {:?}", hex::encode(&final_key));

            let op_meta1 =
                frame_support::storage::unhashed::get::<old::V2MetaOf<T>>(&final_key.as_slice());

            log::info!(
                "meta1 of Telegram before migration is {:?}",
                op_meta1.unwrap()
            );

            let op_meta: Option<old::V2MetaOf<T>> =
                <old::Metadata<T>>::get(Pallet::<T>::key(&"Telegram".as_bytes().to_vec()));

            log::info!(
                "meta of Telegram before migration is {:?}",
                op_meta.unwrap()
            );

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let count = Metadata::<T>::iter_values()
                .map(|meta| {
                    log::info!(
                        "updated meta is {:?}, tag is {}",
                        &meta,
                        sp_std::str::from_utf8(&meta.tag).unwrap()
                    );
                    meta
                })
                .filter(|meta| meta.tag.len() != 0)
                .count();
            if count != 8 {
                Err("there are some tag meta whose tag value does not exist")
            } else {
                Ok(())
            }
        }
    }
}

pub mod v4 {
    use super::*;
    pub struct MigrationScore<T: Config>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrationScore<T> {
        fn on_runtime_upgrade() -> Weight {
            use frame_support::{log::info, migration::storage_iter_with_suffix};
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 3 {
                return 0;
            }

            PersonasOf::<T>::translate_values(|v: SingleMetricScore| {
                let score = v.current_score.max(0).min(50);
                return Some(Score::new(score));
            });

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(4));

            info!("running tag migration");
            return 1;
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

            info!("begin before score");
            let mut iter =
                storage_iter_with_suffix::<SingleMetricScore>(b"Tag", b"PersonasOf", b"");

            let mut num = 0;
            while let Some(_) = iter.next() {
                num += 1;
            }

            info!("before score: {:?}", num);

            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

            info!("begin after score");
            let mut iter = PersonasOf::<T>::iter_values();

            let mut num = 0;
            while let Some(score) = iter.next() {
                num += 1;
            }

            info!("after score: {:?}", num);

            Ok(())
        }
    }
}

pub mod v5 {
    use crate::TagsOf;
    use crate::Vec;
    use frame_support::pallet_prelude::ValueQuery;
    use frame_support::{
        dispatch::DispatchResult,
        ensure,
        storage::PrefixIterator,
        traits::{Currency, ExistenceRequirement::KeepAlive, StorageVersion, WithdrawReasons},
        Blake2_256, StorageHasher,
    };
    use sp_std::str::FromStr;
    use sp_std::vec;

    use super::*;
    pub struct FixWrongStructure<T: Config>(sp_std::marker::PhantomData<T>);

    impl<T: Config> FixWrongStructure<T> {
        fn data() -> Vec<(H160, Vec<u8>, i32)> {
            let data = vec![
                (
                    H160::from_str("0xa5d238b273b638862f1bf0cc0a2cbadec3615508").unwrap(),
                    "DeFi".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0xa5d238b273b638862f1bf0cc0a2cbadec3615508").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xa5d238b273b638862f1bf0cc0a2cbadec3615508").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    10,
                ),
                (
                    H160::from_str("0xa5d238b273b638862f1bf0cc0a2cbadec3615508").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x26355f994080894d8abcffc6573f1521aadd129b").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x26355f994080894d8abcffc6573f1521aadd129b").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x26355f994080894d8abcffc6573f1521aadd129b").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x44bc92aa8500b223f8b1372aaddaaa2fa86681d9").unwrap(),
                    "Discord".as_bytes().to_vec(),
                    1,
                ),
                (
                    H160::from_str("0x44bc92aa8500b223f8b1372aaddaaa2fa86681d9").unwrap(),
                    "DeFi".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x44bc92aa8500b223f8b1372aaddaaa2fa86681d9").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x44bc92aa8500b223f8b1372aaddaaa2fa86681d9").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    10,
                ),
                (
                    H160::from_str("0x44bc92aa8500b223f8b1372aaddaaa2fa86681d9").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xf1b4bb074ccfba7ddfbf85f5ac410ac005d21cbf").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xf1b4bb074ccfba7ddfbf85f5ac410ac005d21cbf").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xf1b4bb074ccfba7ddfbf85f5ac410ac005d21cbf").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x07cd7510b655998c49a025c5ef6b1f7c5d8c42ca").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x07cd7510b655998c49a025c5ef6b1f7c5d8c42ca").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x07cd7510b655998c49a025c5ef6b1f7c5d8c42ca").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x537ae65ee94647f4c9d0c7d46118a64407b4dc4c").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x537ae65ee94647f4c9d0c7d46118a64407b4dc4c").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x537ae65ee94647f4c9d0c7d46118a64407b4dc4c").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xf4196ab781a8eb3ad7a48a3220aa1eee7ddaaaff").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xf4196ab781a8eb3ad7a48a3220aa1eee7ddaaaff").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xf4196ab781a8eb3ad7a48a3220aa1eee7ddaaaff").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xf9044edc1fa558f730065d55cfc6af8916a50ae6").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xf9044edc1fa558f730065d55cfc6af8916a50ae6").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xf9044edc1fa558f730065d55cfc6af8916a50ae6").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x2296e905f23536c9638fc55cc6e240bfa324846a").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x25ac59a10054081a38359c1f4bdf7b57e04f5c0a").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x57bf1c4c89fa5d28ee73f8270a5966dbd214c495").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x57bf1c4c89fa5d28ee73f8270a5966dbd214c495").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x57bf1c4c89fa5d28ee73f8270a5966dbd214c495").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xa8f96e643a2f42ef2db8648517b8554207493f94").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xa8f96e643a2f42ef2db8648517b8554207493f94").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xa8f96e643a2f42ef2db8648517b8554207493f94").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xd20cdcf5e4d6d7ae8f98573f9599598f9b6f6b52").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xd20cdcf5e4d6d7ae8f98573f9599598f9b6f6b52").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xd20cdcf5e4d6d7ae8f98573f9599598f9b6f6b52").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x30489f3ec4e3b440e4fa503243b54d8d8e2f9024").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x30489f3ec4e3b440e4fa503243b54d8d8e2f9024").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x30489f3ec4e3b440e4fa503243b54d8d8e2f9024").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x4d3ef95a38eba590bb23181867d9cc299dd272ab").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x4d3ef95a38eba590bb23181867d9cc299dd272ab").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x4d3ef95a38eba590bb23181867d9cc299dd272ab").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x72d2111e7fdbaa806189ec4c85877600651c8d49").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x72d2111e7fdbaa806189ec4c85877600651c8d49").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x70fcd619259084a95dc0b738de630d15ba7b9932").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x70fcd619259084a95dc0b738de630d15ba7b9932").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x70fcd619259084a95dc0b738de630d15ba7b9932").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x64f1073d02e73f18ad620ba8896e7dd7d2c3c54b").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x64f1073d02e73f18ad620ba8896e7dd7d2c3c54b").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x64f1073d02e73f18ad620ba8896e7dd7d2c3c54b").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x976ae402f359a51f2929cab62e74105caf494a37").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x976ae402f359a51f2929cab62e74105caf494a37").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x976ae402f359a51f2929cab62e74105caf494a37").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x680a7bfae9c2ad26263ea809a1ca5c759f6c50ec").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x680a7bfae9c2ad26263ea809a1ca5c759f6c50ec").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x680a7bfae9c2ad26263ea809a1ca5c759f6c50ec").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x76d3655e0493f570cf1af660d0811fc8cf6ccc3f").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x76d3655e0493f570cf1af660d0811fc8cf6ccc3f").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x76d3655e0493f570cf1af660d0811fc8cf6ccc3f").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x1f32e31612ad68cf1c13c0d4e846f5e4b10daa20").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x1f32e31612ad68cf1c13c0d4e846f5e4b10daa20").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x1f32e31612ad68cf1c13c0d4e846f5e4b10daa20").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xd01b1cbbeb113919464c9799c8085823b4348531").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xd01b1cbbeb113919464c9799c8085823b4348531").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xd01b1cbbeb113919464c9799c8085823b4348531").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x1a4d3eed1ee1ddf96f4900b5a65efc5a2a5ff269").unwrap(),
                    "DeFi".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x1a4d3eed1ee1ddf96f4900b5a65efc5a2a5ff269").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x1a4d3eed1ee1ddf96f4900b5a65efc5a2a5ff269").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    10,
                ),
                (
                    H160::from_str("0x1a4d3eed1ee1ddf96f4900b5a65efc5a2a5ff269").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x3bd87603a32ddeacca7c86abc77f265c6b04c5c4").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x3bd87603a32ddeacca7c86abc77f265c6b04c5c4").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x3bd87603a32ddeacca7c86abc77f265c6b04c5c4").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xcff50205c0dc7aeeb02d65e9479ff85152b96b2a").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xcff50205c0dc7aeeb02d65e9479ff85152b96b2a").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xcff50205c0dc7aeeb02d65e9479ff85152b96b2a").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x24e0054bdfd4b300ec48087f6ae03bbf3e645adf").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x24e0054bdfd4b300ec48087f6ae03bbf3e645adf").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x24e0054bdfd4b300ec48087f6ae03bbf3e645adf").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x98597ae093b231ff1c4c930410bd8e8867625ec2").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x98597ae093b231ff1c4c930410bd8e8867625ec2").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x98597ae093b231ff1c4c930410bd8e8867625ec2").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x46e6f5437d9078d3bbde72aee1ab950054ba8eeb").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x46e6f5437d9078d3bbde72aee1ab950054ba8eeb").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x46e6f5437d9078d3bbde72aee1ab950054ba8eeb").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x7a544b19011807c514c06232b9e6cae334a685fc").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x7a544b19011807c514c06232b9e6cae334a685fc").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x7a544b19011807c514c06232b9e6cae334a685fc").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x9a806e4b6712d42d3ba5604fd90efd8a82b382dc").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x9a806e4b6712d42d3ba5604fd90efd8a82b382dc").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x9a806e4b6712d42d3ba5604fd90efd8a82b382dc").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x79ac752178855e18df348191de4022f4e5f4dada").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x79ac752178855e18df348191de4022f4e5f4dada").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x79ac752178855e18df348191de4022f4e5f4dada").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x32152052eacdec125989ed57c4b2a38449e1f220").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x32152052eacdec125989ed57c4b2a38449e1f220").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x32152052eacdec125989ed57c4b2a38449e1f220").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x3e58ff00c1e48e8fd4d8aca3b0d07c781837fdcd").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x3e58ff00c1e48e8fd4d8aca3b0d07c781837fdcd").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x3e58ff00c1e48e8fd4d8aca3b0d07c781837fdcd").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xc9ca957bb3a1e0bc4f76df28a46d7d2fa43b5bb4").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xf0805a2c17a2ae098ae56cd8ad12ba46b6010146").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xf0805a2c17a2ae098ae56cd8ad12ba46b6010146").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xf0805a2c17a2ae098ae56cd8ad12ba46b6010146").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x24ea88e4a39c5f7c0fd4aaf899d2e9f6ef587e37").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x24ea88e4a39c5f7c0fd4aaf899d2e9f6ef587e37").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x24ea88e4a39c5f7c0fd4aaf899d2e9f6ef587e37").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x9ad3e01623fd83107ae10b5af3442f22fddfaada").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x9ad3e01623fd83107ae10b5af3442f22fddfaada").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x9ad3e01623fd83107ae10b5af3442f22fddfaada").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xc226acaedb0ebf95b6f65b3d48146922e20d2c73").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xc226acaedb0ebf95b6f65b3d48146922e20d2c73").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xc226acaedb0ebf95b6f65b3d48146922e20d2c73").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xf3f035249cc972274115968098d6aa5dae9e6c1e").unwrap(),
                    "DeFi".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0xf3f035249cc972274115968098d6aa5dae9e6c1e").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xf3f035249cc972274115968098d6aa5dae9e6c1e").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    10,
                ),
                (
                    H160::from_str("0xf3f035249cc972274115968098d6aa5dae9e6c1e").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x5f9a78e538c6639eb7c9eeb6dc6b0916ac1b0668").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x5f9a78e538c6639eb7c9eeb6dc6b0916ac1b0668").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x5f9a78e538c6639eb7c9eeb6dc6b0916ac1b0668").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xcf88753c633bdb41998b20f66659898ea8b0461b").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xcf88753c633bdb41998b20f66659898ea8b0461b").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xcf88753c633bdb41998b20f66659898ea8b0461b").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x6c8a3777e2a2a2e6a99eaf9d60414a51eb67e7d7").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x6c8a3777e2a2a2e6a99eaf9d60414a51eb67e7d7").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x6c8a3777e2a2a2e6a99eaf9d60414a51eb67e7d7").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x9932a389e22874a35d03104a1c7900e44af02d49").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x9932a389e22874a35d03104a1c7900e44af02d49").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x9932a389e22874a35d03104a1c7900e44af02d49").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x1caf213e16e7086cd2bcd4ef06d0d4ca48be59c6").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x1caf213e16e7086cd2bcd4ef06d0d4ca48be59c6").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x1caf213e16e7086cd2bcd4ef06d0d4ca48be59c6").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x08120b55564e308eb86372698e74d5c75e12a327").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x08120b55564e308eb86372698e74d5c75e12a327").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x08120b55564e308eb86372698e74d5c75e12a327").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x523f5bf49b8b98ecc8ae10d109cb353bd83db90d").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x523f5bf49b8b98ecc8ae10d109cb353bd83db90d").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x523f5bf49b8b98ecc8ae10d109cb353bd83db90d").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xd5c8016807c48f5665b79dc19fc4165a6696a8c1").unwrap(),
                    "Discord".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xd5c8016807c48f5665b79dc19fc4165a6696a8c1").unwrap(),
                    "DeFi".as_bytes().to_vec(),
                    10,
                ),
                (
                    H160::from_str("0xd5c8016807c48f5665b79dc19fc4165a6696a8c1").unwrap(),
                    "Polkadot".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xd5c8016807c48f5665b79dc19fc4165a6696a8c1").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    14,
                ),
                (
                    H160::from_str("0xd5c8016807c48f5665b79dc19fc4165a6696a8c1").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    13,
                ),
                (
                    H160::from_str("0xd5c8016807c48f5665b79dc19fc4165a6696a8c1").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    11,
                ),
                (
                    H160::from_str("0x639390f9ec83f40ebee27789f18ad137ab6a0465").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x639390f9ec83f40ebee27789f18ad137ab6a0465").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x639390f9ec83f40ebee27789f18ad137ab6a0465").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x58963d3c746eb9198646f97acbac79b67541dd8f").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x58963d3c746eb9198646f97acbac79b67541dd8f").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x58963d3c746eb9198646f97acbac79b67541dd8f").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x9050759cc15be2336c15d10e38a7207c28f27c55").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x9050759cc15be2336c15d10e38a7207c28f27c55").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x9050759cc15be2336c15d10e38a7207c28f27c55").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x9cdb8138c04d898b18e1f4180652b6d33d55fafa").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x9cdb8138c04d898b18e1f4180652b6d33d55fafa").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x9cdb8138c04d898b18e1f4180652b6d33d55fafa").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xecb9f63f386c998ade46b432daf0c8deacbece80").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xecb9f63f386c998ade46b432daf0c8deacbece80").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xecb9f63f386c998ade46b432daf0c8deacbece80").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x37200640f0bc6ab9e300f2d054475a89daacd87c").unwrap(),
                    "DeFi".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x37200640f0bc6ab9e300f2d054475a89daacd87c").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x37200640f0bc6ab9e300f2d054475a89daacd87c").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    10,
                ),
                (
                    H160::from_str("0x37200640f0bc6ab9e300f2d054475a89daacd87c").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xee56abf4f6e1eec26f5fcbaf124ca20ac55cde18").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xee56abf4f6e1eec26f5fcbaf124ca20ac55cde18").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xee56abf4f6e1eec26f5fcbaf124ca20ac55cde18").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x99042bd2473dc0a49d689dde0af06ac3ba22d87b").unwrap(),
                    "Discord".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x99042bd2473dc0a49d689dde0af06ac3ba22d87b").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x99042bd2473dc0a49d689dde0af06ac3ba22d87b").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x99042bd2473dc0a49d689dde0af06ac3ba22d87b").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x5a7412e14688a7f79652665ade7d2f500595e1e6").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x5a7412e14688a7f79652665ade7d2f500595e1e6").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x4d0cd1fb6b5b10b5dac440bc922a94a8413d08d0").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    -6,
                ),
                (
                    H160::from_str("0x4d0cd1fb6b5b10b5dac440bc922a94a8413d08d0").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    -6,
                ),
                (
                    H160::from_str("0x168e3a3240432b3f240e7c517b46c534d4b10307").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x168e3a3240432b3f240e7c517b46c534d4b10307").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x168e3a3240432b3f240e7c517b46c534d4b10307").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x6b7fe6b6919d5367c7a91176cbdf42397f82509a").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x6b7fe6b6919d5367c7a91176cbdf42397f82509a").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x6b7fe6b6919d5367c7a91176cbdf42397f82509a").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x8abcc172c9f437022df030751221fe9d31123aa4").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x8abcc172c9f437022df030751221fe9d31123aa4").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x8abcc172c9f437022df030751221fe9d31123aa4").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x74d9b91f470a8e9d57ee307b1acd127f1f5a0f18").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x74d9b91f470a8e9d57ee307b1acd127f1f5a0f18").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    4,
                ),
                (
                    H160::from_str("0x74d9b91f470a8e9d57ee307b1acd127f1f5a0f18").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x2a0fa615c1773e361cd4c120076b61b3faf5bbac").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x2a0fa615c1773e361cd4c120076b61b3faf5bbac").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x2a0fa615c1773e361cd4c120076b61b3faf5bbac").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x5df979c7350e7a75e848de9308fa60ecbec5548f").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x5df979c7350e7a75e848de9308fa60ecbec5548f").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x5df979c7350e7a75e848de9308fa60ecbec5548f").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xc8b12b4e2bfda5b693b29814694c57d854e3f637").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xc8b12b4e2bfda5b693b29814694c57d854e3f637").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xc8b12b4e2bfda5b693b29814694c57d854e3f637").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x0db19450b561cc03044e5b459d21c3000c3a7cfd").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x0db19450b561cc03044e5b459d21c3000c3a7cfd").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x0db19450b561cc03044e5b459d21c3000c3a7cfd").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x5cc0066f5e38f167dd94a1cc8108b46aebb93a96").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x5cc0066f5e38f167dd94a1cc8108b46aebb93a96").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x5cc0066f5e38f167dd94a1cc8108b46aebb93a96").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0xd1944d88bf1ee04c3529bab8d75a685bd5335e65").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0xd1944d88bf1ee04c3529bab8d75a685bd5335e65").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0xd1944d88bf1ee04c3529bab8d75a685bd5335e65").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x69eef59af27e08549d4892992dd021d97217a520").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    3,
                ),
                (
                    H160::from_str("0x69eef59af27e08549d4892992dd021d97217a520").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x69eef59af27e08549d4892992dd021d97217a520").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x92fa4ae66f72f0724c15e48ea1dc76de6dd6dabb").unwrap(),
                    "Twitter".as_bytes().to_vec(),
                    9,
                ),
                (
                    H160::from_str("0x92fa4ae66f72f0724c15e48ea1dc76de6dd6dabb").unwrap(),
                    "Ethereum".as_bytes().to_vec(),
                    6,
                ),
                (
                    H160::from_str("0x92fa4ae66f72f0724c15e48ea1dc76de6dd6dabb").unwrap(),
                    "Telegram".as_bytes().to_vec(),
                    3,
                ),
            ];
            return data;
        }
    }

    mod old {
        // The old explicit storage item.

        use super::*;
        #[derive(
            Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        )]
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
        pub struct I8Score {
            extrinsic: i8,
            intrinsic: i8,
        }

        #[frame_support::storage_alias]
        pub(super) type PersonasOf<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, H160, Blake2_256, Vec<u8>, I8Score, ValueQuery>;
    }

    use crate::PersonasOf;
    impl<T: Config> OnRuntimeUpgrade for FixWrongStructure<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();

            if version != 4 {
                return 0;
            }

            let data = Self::data();
            for (did, tag, score) in data.into_iter() {
                old::PersonasOf::<T>::remove(did, tag.clone());
                PersonasOf::<T>::insert(
                    <T::DecentralizedId>::from(did.to_fixed_bytes()),
                    tag,
                    Score::new(score.min(50).max(0)),
                );
            }

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(5));
            return 1;
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

            let version = StorageVersion::get::<Pallet<T>>();
            assert!(version == 4);
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

            let version = StorageVersion::get::<Pallet<T>>();
            assert!(version == 5);

            let data = Self::data();

            for (did, tag, score) in data {
                let p = PersonasOf::<T>::get(T::DecentralizedId::from(did.to_fixed_bytes()), tag);

                info!("p: {:?}", p);
                assert_eq!(p.score(), score.min(50).max(0));
            }

            Ok(())
        }
    }
}
