pub mod v4 {
    use frame_support::migration::move_prefix;
    use frame_support::storage::storage_prefix;
    use frame_support::traits::fungibles::Inspect;
    use frame_support::traits::fungibles::Mutate;
    use frame_support::traits::OnRuntimeUpgrade;
    use frame_support::weights::Weight;
    use sp_runtime::traits::Saturating;

    use crate::Config;
    use crate::Deposit;
    use crate::Pallet;
    use crate::StorageVersion;
    use crate::{BalanceOf, ClaimStartAt, Deposits, HeightOf, IcoMeta, IcoMetaOf, Metadata, NftOf};
    use parami_primitives::constants::DOLLARS;

    #[derive(Debug)]
    pub enum Error {
        NumberConversionFailed,
    }

    pub struct FixDeposit<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for FixDeposit<T> {
        fn on_runtime_upgrade() -> Weight {
            let default_expected_currency: BalanceOf<T> = TryInto::try_into(100 * DOLLARS)
                .map_err(|_e| Error::NumberConversionFailed)
                .unwrap();

            for (nft_id, meta) in Metadata::<T>::iter() {
                if meta.minted {
                    if Deposit::<T>::get(nft_id).is_none() {
                        let deposit = Deposits::<T>::get(nft_id, &meta.owner)
                            .unwrap_or(default_expected_currency);
                        Deposit::<T>::insert(nft_id, deposit);

                        log::info!("empty deposit: {:?} {:?}", nft_id, deposit);
                    }
                }
            }
            0
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            let mut count = 0;
            for _ in Deposit::<T>::iter() {
                count += 1;
            }

            log::info!("before deposit count: {:?}", count);
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let mut count = 0;
            for _ in Deposit::<T>::iter() {
                count += 1;
            }

            log::info!("after deposit count: {:?}", count);
            Ok(())
        }
    }

    pub struct MigrateIcoMeta<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateIcoMeta<T> {
        fn on_runtime_upgrade() -> Weight {
            let version = StorageVersion::get::<Pallet<T>>();

            if version != 3 {
                return 0;
            }

            let token_should_issued: BalanceOf<T> = TryInto::try_into(10_000_000 * DOLLARS)
                .map_err(|_e| Error::NumberConversionFailed)
                .unwrap();

            let swapped_tokens: BalanceOf<T> = TryInto::try_into(1_000_000 * DOLLARS)
                .map_err(|_e| Error::NumberConversionFailed)
                .unwrap();

            for (nft_id, meta) in Metadata::<T>::iter() {
                if meta.minted {
                    if IcoMetaOf::<T>::get(nft_id).is_none() {
                        log::info!("start to migrate nft_id {:?}", nft_id);
                        let deposit = Deposit::<T>::get(nft_id).unwrap();

                        let issued = T::Assets::total_issuance(nft_id);

                        IcoMetaOf::<T>::insert(
                            nft_id,
                            IcoMeta::<T> {
                                done: true,
                                expected_currency: deposit,
                                offered_tokens: swapped_tokens,
                                pot: Pallet::<T>::generate_ico_pot(&nft_id),
                            },
                        );

                        let owner_account = parami_did::Pallet::<T>::lookup_did(meta.owner);
                        if let Some(account) = owner_account {
                            let should_mint = token_should_issued.saturating_sub(issued);
                            let result = T::Assets::mint_into(nft_id, &account, should_mint);
                            if result.is_err() {
                                log::error!("token transfer error {:?} {:?}", nft_id, result);
                                panic!("token tranfer error");
                            }
                        } else {
                            log::error!("did not linked to account: {:?}", owner_account);
                        }

                        log::info!("end to migrate nft_id {:?}", nft_id);
                    }
                }
            }

            move_prefix(
                &storage_prefix(b"Nft", b"Date"),
                &storage_prefix(b"Nft", b"IcoStartAt"),
            );

            StorageVersion::put::<Pallet<T>>(&StorageVersion::new(4));
            0
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            use frame_support::{migration::storage_key_iter, Twox64Concat};
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert!(storage_version == 3, "current storage version should be 3");

            let mut key_count = 0;

            for _ in <Metadata<T>>::iter() {
                key_count += 1;
            }

            let mut ico_meta_count = 0;
            for _ in <IcoMetaOf<T>>::iter() {
                ico_meta_count += 1;
            }

            let mut date_count = 0;

            for _ in storage_key_iter::<NftOf<T>, HeightOf<T>, Twox64Concat>(b"Nft", b"Date") {
                date_count += 1;
            }

            log::info!("metadata key count: {:?}", key_count);
            log::info!("ico meta key count: {:?}", ico_meta_count);
            log::info!("date key count: {:?}", date_count);
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let storage_version = StorageVersion::get::<Pallet<T>>();
            assert!(storage_version == 4, "current storage version should be 4");

            let mut key_count = 0;

            for _ in <Metadata<T>>::iter() {
                key_count += 1;
            }

            let mut ico_meta_count = 0;
            for _ in <IcoMetaOf<T>>::iter() {
                ico_meta_count += 1;
            }

            let mut claim_start_at_count = 0;
            for (key, value) in <ClaimStartAt<T>>::iter() {
                claim_start_at_count += 1;
                log::info!("ico start at: key: {:?} value: {:?}", key, value);
            }

            log::info!("metadata key count: {:?}", key_count);
            log::info!("ico meta key count: {:?}", ico_meta_count);
            log::info!("ico start at count: {:?}", claim_start_at_count);
            Ok(())
        }
    }
}
