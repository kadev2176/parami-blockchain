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

pub mod v6 {
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

    mod old {
        // The old explicit storage item.

        use frame_support::pallet_prelude::OptionQuery;

        use super::*;
        #[derive(
            Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        )]
        #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
        pub struct I8Score {
            pub extrinsic: i8,
            pub intrinsic: i8,
        }

        #[frame_support::storage_alias]
        pub(super) type PersonasOf<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, H160, Blake2_256, Vec<u8>, I8Score, OptionQuery>;
    }

    use crate::PersonasOf;
    use parami_traits::Tags;
    impl<T: Config> OnRuntimeUpgrade for FixWrongStructure<T> {
        fn on_runtime_upgrade() -> Weight {
            use frame_support::log::info;
            let version = StorageVersion::get::<Pallet<T>>();

            if version != 5 {
                return 0;
            }

            let tags = vec!["Telegram", "DeFi", "Ethereum", "Twitter"];
            let dids = Self::data();

            let mut count = 0;
            for did in dids {
                let did_id = T::DecentralizedId::from(did.to_fixed_bytes());
                for tag in tags.clone() {
                    let tag = tag.as_bytes().to_vec();
                    // For score read as I32 is zero, there are two situations:
                    // 1. score is indeed zero.
                    // 2. score is I8 and can't be read as I32.
                    // We read those scores as I8 and convert them to I32, so:
                    // if score is indeed zero, it will not be changed,
                    // or score is I8, it will be fixed.
                    let new_personas = crate::PersonasOf::<T>::get(did_id, tag.clone());
                    if new_personas.score() == 0 {
                        let old_personas = old::PersonasOf::<T>::get(did, tag.clone());
                        if let Some(old_score) = old_personas {
                            info!(
                                "found a undecodable score, {:?}, {:?}, {:?}",
                                did, tag, old_score
                            );
                            count += 1;
                            old::PersonasOf::<T>::remove(did, tag.clone());
                            PersonasOf::<T>::insert(
                                did_id,
                                tag,
                                crate::types::Score::new(old_score.intrinsic as i32)
                                    .accure_extrinsic(old_score.extrinsic as i32),
                            );
                        }
                    }
                }
            }

            info!("total undecodeable scores: {:?}", count);
            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(6));
            return 1;
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

            let version = StorageVersion::get::<Pallet<T>>();

            assert!(version == 5);
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

            let version = StorageVersion::get::<Pallet<T>>();
            assert!(version == 6);

            let data = Self::data();

            info!("post upgrade scores");
            // we can verify all scores of affect dids are correct now.
            for did in data {
                let did_id = T::DecentralizedId::from(did.to_fixed_bytes());
                let scores = crate::PersonasOf::<T>::iter_prefix_values(did_id);
                for s in scores {
                    info!("post upgrade scores: {:?}, {:?}", did, s);
                }
            }

            Ok(())
        }
    }

    impl<T: Config> FixWrongStructure<T> {
        fn data() -> Vec<H160> {
            let dids = [
                "0xa5d238b273b638862f1bf0cc0a2cbadec3615508",
                "0x26355f994080894d8abcffc6573f1521aadd129b",
                "0x44bc92aa8500b223f8b1372aaddaaa2fa86681d9",
                "0xf1b4bb074ccfba7ddfbf85f5ac410ac005d21cbf",
                "0x7ced764b6c13cd1ac0aff02ab5eae64711011aa2",
                "0x047f4c21589256c5b6f2956afe54e83e44924b2b",
                "0x07cd7510b655998c49a025c5ef6b1f7c5d8c42ca",
                "0x537ae65ee94647f4c9d0c7d46118a64407b4dc4c",
                "0xf4196ab781a8eb3ad7a48a3220aa1eee7ddaaaff",
                "0xf9044edc1fa558f730065d55cfc6af8916a50ae6",
                "0x2296e905f23536c9638fc55cc6e240bfa324846a",
                "0x25ac59a10054081a38359c1f4bdf7b57e04f5c0a",
                "0x5ae44d59e05db2f59031e9ed65178a11dc17f0a4",
                "0x57bf1c4c89fa5d28ee73f8270a5966dbd214c495",
                "0x69ab224ea4afa4955e11b0b689330aa6245bacfe",
                "0xa8f96e643a2f42ef2db8648517b8554207493f94",
                "0xd20cdcf5e4d6d7ae8f98573f9599598f9b6f6b52",
                "0xe505c74f92e7d813d926bb7527e09849fd295672",
                "0x2e9dd48018e30e0c21c716f62202c8aefd8a3e3b",
                "0x30489f3ec4e3b440e4fa503243b54d8d8e2f9024",
                "0x4d3ef95a38eba590bb23181867d9cc299dd272ab",
                "0x72d2111e7fdbaa806189ec4c85877600651c8d49",
                "0x72362550af1457bdaf779858fe3a71e1de71697b",
                "0x70fcd619259084a95dc0b738de630d15ba7b9932",
                "0x64f1073d02e73f18ad620ba8896e7dd7d2c3c54b",
                "0x976ae402f359a51f2929cab62e74105caf494a37",
                "0x2cc5e38ce6d9cc74651f4ff6db2f1a6acea7d992",
                "0x660a4b77b73eeb2a5bf39f569d2c4b66d706ccd4",
                "0x680a7bfae9c2ad26263ea809a1ca5c759f6c50ec",
                "0x76d3655e0493f570cf1af660d0811fc8cf6ccc3f",
                "0x1f32e31612ad68cf1c13c0d4e846f5e4b10daa20",
                "0xd01b1cbbeb113919464c9799c8085823b4348531",
                "0x1a4d3eed1ee1ddf96f4900b5a65efc5a2a5ff269",
                "0x2a38ff877da0d28920aa98a3c8a8940f679f2ae1",
                "0x3bd87603a32ddeacca7c86abc77f265c6b04c5c4",
                "0xcff50205c0dc7aeeb02d65e9479ff85152b96b2a",
                "0x7432a0f2f4153c326960cfc22e18f9eb1736270c",
                "0x24e0054bdfd4b300ec48087f6ae03bbf3e645adf",
                "0x98597ae093b231ff1c4c930410bd8e8867625ec2",
                "0x46e6f5437d9078d3bbde72aee1ab950054ba8eeb",
                "0x1e024e1c43c8e0d422309604f0ebcc94d581f3c7",
                "0x7a544b19011807c514c06232b9e6cae334a685fc",
                "0xe4a7367375b20e9d5a43b7cc18db83d895469723",
                "0x9a806e4b6712d42d3ba5604fd90efd8a82b382dc",
                "0x62a9de6b6616a9afaf8f0138e30032f5ae3566af",
                "0x51e5bc19d2f35f132d986334f623bd3c94c9d7af",
                "0x79ac752178855e18df348191de4022f4e5f4dada",
                "0x32152052eacdec125989ed57c4b2a38449e1f220",
                "0x3e58ff00c1e48e8fd4d8aca3b0d07c781837fdcd",
                "0xc9ca957bb3a1e0bc4f76df28a46d7d2fa43b5bb4",
                "0x276731636bc5c0a0e716dca0d6d7d95b1927c19f",
                "0xf0805a2c17a2ae098ae56cd8ad12ba46b6010146",
                "0x24ea88e4a39c5f7c0fd4aaf899d2e9f6ef587e37",
                "0x9ad3e01623fd83107ae10b5af3442f22fddfaada",
                "0xc226acaedb0ebf95b6f65b3d48146922e20d2c73",
                "0x0600ee07e7b8f12fb6e134267b18646905f90d80",
                "0xf3f035249cc972274115968098d6aa5dae9e6c1e",
                "0x5f9a78e538c6639eb7c9eeb6dc6b0916ac1b0668",
                "0xcf88753c633bdb41998b20f66659898ea8b0461b",
                "0xaf5c7820c765004ea5f3b22e9ae84f87ff6b16b8",
                "0x6c8a3777e2a2a2e6a99eaf9d60414a51eb67e7d7",
                "0x9932a389e22874a35d03104a1c7900e44af02d49",
                "0x1caf213e16e7086cd2bcd4ef06d0d4ca48be59c6",
                "0x08120b55564e308eb86372698e74d5c75e12a327",
                "0x523f5bf49b8b98ecc8ae10d109cb353bd83db90d",
                "0xd5c8016807c48f5665b79dc19fc4165a6696a8c1",
                "0xf202127140ddff5b97b5214b1025c940ec524d32",
                "0x639390f9ec83f40ebee27789f18ad137ab6a0465",
                "0x58963d3c746eb9198646f97acbac79b67541dd8f",
                "0x9050759cc15be2336c15d10e38a7207c28f27c55",
                "0x90fbfccd8c6683e9151c7b8a3e94cd32c2080da6",
                "0xd74c13c125d323bf37d2924c528db3843299cf9b",
                "0x9cdb8138c04d898b18e1f4180652b6d33d55fafa",
                "0xecb9f63f386c998ade46b432daf0c8deacbece80",
                "0xa0e49a7876ba96176dd092196e5a311e8c35c5ef",
                "0x37200640f0bc6ab9e300f2d054475a89daacd87c",
                "0xee56abf4f6e1eec26f5fcbaf124ca20ac55cde18",
                "0xd6d926a8e2a4f0c6ac8f166238e2e7425cb5bc46",
                "0x99042bd2473dc0a49d689dde0af06ac3ba22d87b",
                "0xbf7a279fc0b7f82fc57cbd481a37acfd5821d3c3",
                "0x5a7412e14688a7f79652665ade7d2f500595e1e6",
                "0x4d0cd1fb6b5b10b5dac440bc922a94a8413d08d0",
                "0x168e3a3240432b3f240e7c517b46c534d4b10307",
                "0x6b7fe6b6919d5367c7a91176cbdf42397f82509a",
                "0x8abcc172c9f437022df030751221fe9d31123aa4",
                "0x74d9b91f470a8e9d57ee307b1acd127f1f5a0f18",
                "0x2a0fa615c1773e361cd4c120076b61b3faf5bbac",
                "0x5df979c7350e7a75e848de9308fa60ecbec5548f",
                "0x521ee236a05b26f4190c93a21ae4d2d234044bc9",
                "0xd2efeb37fac3a6b49f92a5879a6ad170a10d830e",
                "0xc8b12b4e2bfda5b693b29814694c57d854e3f637",
                "0x0db19450b561cc03044e5b459d21c3000c3a7cfd",
                "0x5cc0066f5e38f167dd94a1cc8108b46aebb93a96",
                "0xd1944d88bf1ee04c3529bab8d75a685bd5335e65",
                "0x69eef59af27e08549d4892992dd021d97217a520",
                "0x92fa4ae66f72f0724c15e48ea1dc76de6dd6dabb",
            ];

            return dids
                .into_iter()
                .map(|a| H160::from_str(a).unwrap())
                .collect();
        }
    }
}
