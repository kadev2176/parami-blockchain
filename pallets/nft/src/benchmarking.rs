use super::*;

#[allow(unused)]
use crate::Pallet as Nft;
use codec::Encode;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::tokens::fungibles::Inspect;
use frame_system::RawOrigin;
use parami_did::Pallet as Did;
use parami_linker::Pallet as Linker;
use sp_io::hashing::keccak_256;
use sp_runtime::traits::{Bounded, Saturating, Zero};

fn alice() -> libsecp256k1::SecretKey {
    libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub fn eth_public(secret: &libsecp256k1::SecretKey) -> libsecp256k1::PublicKey {
    libsecp256k1::PublicKey::from_secret_key(secret)
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub fn eth_address(secret: &libsecp256k1::SecretKey) -> sp_core::H160 {
    sp_core::H160::from_slice(&keccak_256(&eth_public(secret).serialize()[1..65])[12..])
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
// Constructs a message and signs it.
pub fn eth_sign(secret: &libsecp256k1::SecretKey, msg: &Vec<u8>) -> [u8; 65] {
    let msg = keccak_256(msg);
    let (sig, recovery_id) = libsecp256k1::sign(&libsecp256k1::Message::parse(&msg), secret);
    let mut r = [0u8; 65];
    r[0..64].copy_from_slice(&sig.serialize()[..]);
    r[64] = recovery_id.serialize();
    r
}

// address: 0x51a29c53D4054363048a390f04eE93d8ef1924E1
// private Key: 30ee1f7356ae9729e8ef5b9310455da361267ca38b1a87d16222958b0ac0bc81
#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
fn gen_signature<T>(did: &T::DecentralizedId) -> ([u8; 20], [u8; 65])
where
    T: parami_did::Config,
{
    let private_key = alice();
    let public_address = eth_address(&private_key);

    let mut msg = parami_primitives::signature::generate_message(did.clone());
    let mut length = parami_primitives::signature::usize_to_u8_array(msg.len());
    let mut data = b"\x19Ethereum Signed Message:\n".encode();
    data.append(&mut length);
    data.append(&mut msg);

    let sig: [u8; 65] = eth_sign(&private_key, &data);
    let public_address_in_bytes: &[u8] = &public_address.as_ref();

    let mut public_address_last_20_bytes: [u8; 20] = Default::default();
    public_address_last_20_bytes
        .copy_from_slice(&public_address_in_bytes[&public_address_in_bytes.len() - 20..]);

    (public_address_last_20_bytes, sig)
}

benchmarks! {
    where_clause {
        where
        T: parami_did::Config,
        T: parami_linker::Config
    }

    port {
        let caller: T::AccountId = whitelisted_caller();

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let did = Did::<T>::did_of(&caller).unwrap();

        Linker::<T>::submit_link(RawOrigin::None.into(), did, Network::Ethereum, vec![1u8; 20], true)?;

        let (eth_address, sig) = gen_signature::<T>(&did);

    }: _(RawOrigin::Signed(caller.clone()), Network::Ethereum, vec![1u8; 20], vec![1u8; 32], eth_address.to_vec(), sig)
    verify {
        assert_ne!(<Porting<T>>::get((Network::Ethereum, &vec![1u8; 20], &vec![1u8; 32])), None);
    }

    kick {
        let caller: T::AccountId = whitelisted_caller();

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller.clone()))
    verify {
        let did = Did::<T>::did_of(&caller).unwrap();
    }

    back {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        T::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        Nft::<T>::kick(RawOrigin::Signed(kol.clone()).into())?;

        let did = Did::<T>::did_of(&kol).unwrap();
        let nft = <Preferred<T>>::get(&did).unwrap();
    }: _(RawOrigin::Signed(caller), nft, pot)
    verify {
        let meta = <Metadata<T>>::get(nft).unwrap();
        assert_eq!(T::Currency::free_balance(&meta.pot), pot);
    }

    mint {
        let n in 1 .. 1000 - 4;
        let s in 1 .. 1000 - 4;

        let name = vec![b'x'; n as usize];
        let symbol = vec![b'x'; n as usize];

        let caller: T::AccountId = whitelisted_caller();

        let supporter: T::AccountId = account("supporter", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&supporter, max);
        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(supporter.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        Nft::<T>::kick(RawOrigin::Signed(caller.clone()).into())?;

        let did = Did::<T>::did_of(&caller).unwrap();
        let nft = <Preferred<T>>::get(&did).unwrap();

        Nft::<T>::back(RawOrigin::Signed(supporter).into(), nft, pot)?;
    }: _(RawOrigin::Signed(caller), nft, name, symbol)
    verify {
        let meta = <Metadata<T>>::get(nft).unwrap();
        assert!(meta.minted);
    }

    claim {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        T::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        Nft::<T>::kick(RawOrigin::Signed(kol.clone()).into())?;

        let did = Did::<T>::did_of(&kol).unwrap();
        let nft = <Preferred<T>>::get(&did).unwrap();

        Nft::<T>::back(RawOrigin::Signed(caller.clone()).into(), nft, pot)?;

        Nft::<T>::mint(RawOrigin::Signed(kol).into(), nft, b"Test Token".to_vec(), b"XTT".to_vec())?;
    }: _(RawOrigin::Signed(caller.clone()), nft)
    verify {
        assert!(T::Assets::balance(nft, &caller) > Zero::zero());
    }

    submit_porting {
        let caller: T::AccountId = whitelisted_caller();

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();
        let (eth_address, sig) = gen_signature::<T>(&did);
        Nft::<T>::port(RawOrigin::Signed(caller).into(), Network::Ethereum, vec![1u8; 20], vec![1u8; 32], eth_address.to_vec(), sig)?;
    }: _(RawOrigin::None, did, Network::Ethereum, vec![1u8; 20], vec![1u8; 32], true)
    verify {
        assert_eq!(<Porting<T>>::get((Network::Ethereum, &vec![1u8; 20], &vec![1u8; 32])), None);
        assert_ne!(<Ported<T>>::get((Network::Ethereum, &vec![1u8; 20], &vec![1u8; 32])), None);
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
