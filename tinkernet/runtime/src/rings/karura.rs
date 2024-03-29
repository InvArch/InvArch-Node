use super::RingsChain;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::BoundedSlice;
use scale_info::TypeInfo;
use xcm::latest::{Junction, Junctions, MultiLocation};

pub struct Karura;

#[allow(non_camel_case_types)]
#[derive(Encode, Decode, Clone, Eq, PartialEq, MaxEncodedLen, Debug, TypeInfo)]
pub enum KaruraAssets {
    KAR,
    LKSM,
    tKSM,
    KSM,
    Local([u8; 20]),
}

impl RingsChain for Karura {
    type Assets = KaruraAssets;

    fn get_asset_location(asset: &Self::Assets) -> MultiLocation {
        use KaruraAssets::*;
        match asset {
            KAR => MultiLocation {
                parents: 0,
                interior: Junctions::X1(Junction::from(BoundedSlice::truncate_from(
                    &hex_literal::hex!("0080"),
                ))),
            },
            LKSM => MultiLocation {
                parents: 0,
                interior: Junctions::X1(Junction::from(BoundedSlice::truncate_from(
                    &hex_literal::hex!("0083"),
                ))),
            },
            tKSM => MultiLocation {
                parents: 0,
                interior: Junctions::X1(Junction::from(BoundedSlice::truncate_from(
                    &hex_literal::hex!("0300000000"),
                ))),
            },
            KSM => MultiLocation {
                parents: 1,
                interior: Junctions::Here,
            },
            Local(address) => MultiLocation {
                parents: 0,
                interior: Junctions::X1(Junction::from(BoundedSlice::truncate_from(address))),
            },
        }
    }

    fn get_location() -> MultiLocation {
        MultiLocation {
            parents: 1,
            interior: Junctions::X1(Junction::Parachain(2000)),
        }
    }

    fn get_main_asset() -> Self::Assets {
        KaruraAssets::KAR
    }
}
