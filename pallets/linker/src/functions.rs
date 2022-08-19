use crate::{Config, DidOf, Error, Event, Linked, LinksOf, Pallet, PendingOf};

use frame_support::ensure;
use parami_traits::types::{Network, Task};
use sp_runtime::DispatchResult;
use sp_std::prelude::*;

macro_rules! is_task {
    ($profile:expr, $prefix:expr) => {
        $profile.starts_with($prefix) && $profile.len() > $prefix.len()
    };
}

impl<T: Config> Pallet<T> {
    fn ensure_profile(did: &DidOf<T>, site: Network, profile: &[u8]) -> DispatchResult {
        use Network::*;

        ensure!(!<LinksOf<T>>::contains_key(did, site), Error::<T>::Exists);
        ensure!(
            !<Linked<T>>::contains_key(site, profile),
            Error::<T>::Exists
        );

        match site {
            Binance | Bitcoin | Eosio | Ethereum | Kusama | Polkadot | Solana | Tron | Near
            | Unknown => {}

            Discord if is_task!(profile, b"https://discordapp.com/users/") => {}
            Facebook if is_task!(profile, b"https://www.facebook.com/") => {}
            Github if is_task!(profile, b"https://github.com/") => {}
            HackerNews if is_task!(profile, b"https://news.ycombinator.com/user?id=") => {}
            Mastodon => {}
            Reddit if is_task!(profile, b"https://www.reddit.com/user/") => {}
            Telegram if is_task!(profile, b"https://t.me/") => {}
            Twitter if is_task!(profile, b"https://twitter.com/") => {}

            _ => Err(Error::<T>::UnsupportedSite)?,
        };

        Ok(())
    }

    pub fn veto_pending(did: DidOf<T>, site: Network, profile: Vec<u8>) -> DispatchResult {
        <PendingOf<T>>::remove(site, &did);

        Self::deposit_event(Event::<T>::ValidationFailed(did, site, profile));

        Ok(())
    }

    pub fn insert_link(
        did: DidOf<T>,
        site: Network,
        profile: Vec<u8>,
        registrar: DidOf<T>,
    ) -> DispatchResult {
        Self::ensure_profile(&did, site, &profile)?;

        <PendingOf<T>>::remove(site, &did);

        <Linked<T>>::insert(site, &profile, true);

        <LinksOf<T>>::insert(&did, site, profile.clone());

        Self::deposit_event(Event::<T>::AccountLinked(did, site, profile, registrar));

        Ok(())
    }

    pub fn insert_pending(did: DidOf<T>, site: Network, profile: Vec<u8>) -> DispatchResult {
        use frame_support::traits::Get;
        use sp_runtime::traits::Saturating;

        Self::ensure_profile(&did, site, &profile)?;

        ensure!(
            !<PendingOf<T>>::contains_key(site, &did),
            Error::<T>::Exists
        );

        let created = <frame_system::Pallet<T>>::block_number();
        let lifetime = T::PendingLifetime::get();
        let deadline = created.saturating_add(lifetime);

        <PendingOf<T>>::insert(
            site,
            &did,
            Task {
                task: profile,
                deadline,
                created,
            },
        );

        Ok(())
    }
}
