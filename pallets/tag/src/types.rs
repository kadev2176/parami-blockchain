use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Metadata<V, D, N> {
    pub creator: D,
    pub created: N,
    pub tag: V,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SingleMetricScore {
    pub current_score: i32,
    pub last_input: i32,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Score {
    extrinsic: i32,
    intrinsic: i32,
}

impl Score {
    const MIN_EXTRINSIC: i32 = -50;
    const MAX_EXTRINSIC: i32 = 50;

    pub fn new(intrinsic: i32) -> Score {
        assert!(intrinsic >= 0 && intrinsic <= 50);
        return Score {
            intrinsic,
            extrinsic: 0,
        };
    }

    pub fn score(&self) -> i32 {
        (self.extrinsic + self.intrinsic) as i32
    }

    pub fn accure_extrinsic(&self, rating: i32) -> Score {
        let extrinsic = (self.extrinsic + rating)
            .min(Score::MAX_EXTRINSIC)
            .max(Score::MIN_EXTRINSIC);

        return Score { extrinsic, ..*self };
    }

    pub fn with_intrinsic(&self, intrinsic: i32) -> Score {
        return Score { intrinsic, ..*self };
    }
}
