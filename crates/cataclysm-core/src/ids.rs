//! Strongly typed integer identifiers for `cataclysm-core`.

macro_rules! id_type {
    ($name:ident, $inner:ty) => {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub $inner);

        impl $name {
            #[inline]
            pub const fn idx(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id_type!(PlayerId, u16);
id_type!(PlotIdx, u32);
id_type!(SquadId, u32);
id_type!(MemberId, u32);
id_type!(CityId, u32);
id_type!(HouseId, u32);
id_type!(CaravanId, u32);
id_type!(BattleId, u32);
id_type!(GreatPersonId, u32);
id_type!(CrusadeId, u32);

/// Sentinel player id for neutral/barbarian forces (always last player slot).
pub const NEUTRAL_SLOT_SENTINEL: u16 = u16::MAX;
