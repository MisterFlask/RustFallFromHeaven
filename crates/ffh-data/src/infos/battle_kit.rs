//! This game's own battle kit of unit types and promotions (docs/CARDS.md, section 2; the
//! great-person deck's batch "battle more"): what a unit's shield, arrows, drums, wings, wards and
//! walls change in a formation battle (docs/ENGAGEMENTS.md). One block, [`BattleKit`], read the same
//! way for a unit type (`units.jsonc`) and a promotion (`promotions.jsonc`): a unit has what its type
//! and its promotions have.
//!
//! | Key | What |
//! |---|---|
//! | `battle_shield` | while it stands in the front row, the shots, stones and damage spells aimed down its lanes hit it instead; it takes this percent less damage from SHOOT, BOMBARD and SPELL hits |
//! | `shots_at_walls` | its SHOOT takes the enemy's standing Sentinels first, as BOMBARD does |
//! | `battle_drums` | while it is committed to a battle, its side is immune to fear and has this much more speed in round 1 |
//! | `charge_anywhere` | its charges reach any enemy whatever the ground, as where flanking works |
//! | `battle_immune` | `SPELLS` (spell damage does not hurt it), `CHARMS` (its side's charms do not land on it), `HEXES` (the enemy's hexes do not land on it) |
//! | `battle_sentinels` | when its plot is attacked, its side fights with this many more Sentinels |
//! | `battle_sentinel_defense` | the percent those Sentinels' HP and defence grow by, as a city's defence grows its walls |

use crate::registry::Rd;

/// A unit type's or promotion's battle kit (module comment). Every field is 0 or false without its
/// key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BattleKit {
    /// `battle_shield`: while it stands in the front row, the SHOOT, BOMBARD and SPELL actions aimed
    /// at a unit in its lanes hit it instead (as a Sentinel takes them); it takes this percent less
    /// damage from SHOOT, BOMBARD and SPELL hits.
    pub shield: i32,
    /// `shots_at_walls`: its SHOOT takes the enemy's standing Sentinels on the field first, as
    /// BOMBARD does.
    pub shots_at_walls: bool,
    /// `battle_drums`: while it is committed to a battle (and does not sit out), its side's units are
    /// immune to fear (the morale check) and have this much more speed in round 1.
    pub drums: i32,
    /// `charge_anywhere`: its CHARGE is a charge where flanking works whatever the ground: it reaches
    /// any enemy, takes the highest threat for its HP left, and deals the first round's bonus.
    pub charge_anywhere: bool,
    /// `battle_immune`: `SPELLS`: spell damage does not hurt it.
    pub immune_spells: bool,
    /// `battle_immune`: `CHARMS`: its side's charms do not land on it.
    pub immune_charms: bool,
    /// `battle_immune`: `HEXES`: the enemy's hexes do not land on it (the pre-battle hexes of a
    /// presence included).
    pub immune_hexes: bool,
    /// `battle_sentinels`: when its plot is attacked, its side fights with this many more Sentinels
    /// (`UNIT_SENTINEL`'s values, whole at the start of each battle; they hold no plot).
    pub sentinels: i32,
    /// `battle_sentinel_defense`: the percent its `battle_sentinels`' HP and defence (and magic
    /// defence) grow by, as a city's defence grows its walls.
    pub sentinel_defense: i32,
}

impl BattleKit {
    /// `a` with `b` added: numbers summed (the shield the larger), flags or-ed.
    pub fn plus(self, b: &BattleKit) -> BattleKit {
        BattleKit {
            shield: self.shield.max(b.shield),
            shots_at_walls: self.shots_at_walls || b.shots_at_walls,
            drums: self.drums + b.drums,
            charge_anywhere: self.charge_anywhere || b.charge_anywhere,
            immune_spells: self.immune_spells || b.immune_spells,
            immune_charms: self.immune_charms || b.immune_charms,
            immune_hexes: self.immune_hexes || b.immune_hexes,
            sentinels: self.sentinels + b.sentinels,
            sentinel_defense: self.sentinel_defense.max(b.sentinel_defense),
        }
    }

    /// Whether it has any key.
    pub fn any(&self) -> bool {
        *self != BattleKit::default()
    }
}

/// The tokens of `battle_immune`.
pub const IMMUNITY_TOKENS: [&str; 3] = ["SPELLS", "CHARMS", "HEXES"];

/// The [`BattleKit`] keys of an entry (`what` names it in warnings). An unknown `battle_immune`
/// token warns and is skipped.
pub fn read_battle_kit(rd: &Rd, what: &str) -> BattleKit {
    let mut kit = BattleKit {
        shield: rd.int("battle_shield"),
        shots_at_walls: rd.bool("shots_at_walls"),
        drums: rd.int("battle_drums"),
        charge_anywhere: rd.bool("charge_anywhere"),
        sentinels: rd.int("battle_sentinels"),
        sentinel_defense: rd.int("battle_sentinel_defense"),
        ..BattleKit::default()
    };
    if rd.has("battle_immune") {
        for token in rd.string_list("battle_immune") {
            match token.trim() {
                "SPELLS" => kit.immune_spells = true,
                "CHARMS" => kit.immune_charms = true,
                "HEXES" => kit.immune_hexes = true,
                other => rd.cx.warn(format!("{what}: unknown battle_immune {other:?} (not one of {IMMUNITY_TOKENS:?})")),
            }
        }
    }
    kit
}
