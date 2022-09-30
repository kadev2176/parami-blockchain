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
            let version = StorageVersion::get::<Pallet<T>>();
            if version != 3 {
                return 0;
            }

            PersonasOf::<T>::translate_values(|v: SingleMetricScore| {
                let score = v.current_score.max(0).min(50);
                return Some(Score::new(score));
            });

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(4));

            return 1;
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::{log::info, migration::storage_iter_with_suffix};

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

            let mut iter = PersonasOf::<T>::iter_values();

            let mut num = 0;
            while let Some(score) = iter.next() {
                log::info!("{:?}", score);
                num += 1;
            }

            info!("after score: {:?}", num);

            Ok(())
        }
    }
}
