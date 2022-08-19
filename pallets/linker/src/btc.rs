use sp_runtime_interface::runtime_interface;

pub use hashing::{checksum, ripemd160, sha256d};

#[runtime_interface]
pub trait Hashing {
    fn checksum(_input: &[u8]) -> [u8; 4] {
        panic!("Depercated, use parami_primitives::signature::btc instead.");
    }

    fn sha256d(_bytes: &[u8]) -> [u8; 32] {
        panic!("Depercated, use parami_primitives::signature::btc instead.");
    }

    fn ripemd160(_bytes: &[u8]) -> [u8; 20] {
        #[cfg(feature = "std")]
        {
            panic!("Depercated, use parami_primitives::signature::btc instead.");
        }

        #[cfg(not(feature = "std"))]
        {
            unimplemented!()
        }
    }
}
