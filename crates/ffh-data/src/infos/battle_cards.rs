//! The battle keys of the great-person cards' content (docs/CARDS.md, section 2; the batch
//! "battle arms": the cards that act in formation battles and at walls). Each key is general,
//! named for what it does rather than for the card that first uses it:
//!
//! | Key | On | What |
//! |---|---|---|
//! | `pre_battle_damage` | building | at the start of every battle at its city, each attacking unit in a row loses a percent of its max HP |
//! | `sentinel_ranged_resist` | building | its city's Sentinels take less damage from bombards, BOMBARD and SHOOT |
//! | `sentinel_unit` | building | its city's Sentinels fight with another unit type's attack and HP |
//! | `hold_melee_rounds` | building | in the first battle at its city each turn, the attacker's STRIKE, CHARGE and STAB are spent for some rounds |
//! | `last_stand_damage` | building | once half the city's committed defenders have fallen, the rest deal more damage |
//! | `never_lost_damage` | building | while the city has never changed hands, its defenders deal more damage |
//! | `exclusive_with` | building | a city with a building of one of these classes cannot build it |
//! | `engagement_attacker_slots_change` | improvement | the attacker's slots in battles on its plot |
//! | `bombard_range`, `bombard_besieges` | unit | it bombards a city this many plots away; a city it bombards counts as besieged until its next turn |
//! | `engage_first_round` | unit | what it does in a battle's first round, in place of its row's action |
//! | `first_melee_pattern` | promotion | the attack pattern of the unit's first STRIKE or CHARGE each round |
//! | `heal_behind` | promotion | at the end of each round the unit heals the ally behind it |
//! | `unit_classes` | promotion | only units of these classes may take it |
//! | `attack_damage_type` | promotion | the unit's physical attack deals this damage type |

use crate::enums::*;
use crate::infos::engagement::{ActionKind, AttackPattern, ROWS, ROW_TOKENS};
use crate::registry::Rd;

/// A building's `pre_battle_damage`: at the start of every battle at its city, each attacking
/// unit in row `row` loses `percent` of its max HP, of `damage_type` (its resistance to it cuts
/// the loss), never below 1 HP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreBattleDamage {
    /// 0 front, 1 middle, 2 rear.
    pub row: usize,
    pub percent: i32,
    pub damage_type: Option<DamageType>,
}

/// What a building does to its city's battles (this game's own, the cards' keys; module comment).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildingBattle {
    pub pre_battle_damage: Option<PreBattleDamage>,
    /// Percent less damage its city's Sentinels take from bombards on the map and from BOMBARD
    /// and SHOOT in battle.
    pub sentinel_ranged_resist: i32,
    /// Its city's Sentinels fight with this unit type's attack and HP (one cell each, grown by the
    /// city's defence as before).
    pub sentinel_unit: Option<UnitType>,
    /// In the first battle at its city each game turn, the attacker's STRIKE, CHARGE and STAB
    /// actions are spent in this many first rounds.
    pub hold_melee_rounds: i32,
    /// In a battle at its city, once half of the defender's committed units (not Sentinels) have
    /// fallen, the rest deal this percent more damage.
    pub last_stand_damage: i32,
    /// While its city has never changed hands, the defender's units deal this percent more
    /// damage in its battles.
    pub never_lost_damage: i32,
}

impl BuildingBattle {
    /// Whether it does anything.
    pub fn any(&self) -> bool {
        *self != BuildingBattle::default()
    }
}

/// A building's battle keys (module comment).
pub fn read_building_battle(rd: &Rd, building: &str) -> BuildingBattle {
    BuildingBattle {
        pre_battle_damage: read_pre_battle_damage(rd, building),
        sentinel_ranged_resist: rd.int("sentinel_ranged_resist"),
        sentinel_unit: rd.rf::<UnitType>("sentinel_unit"),
        hold_melee_rounds: rd.int("hold_melee_rounds"),
        last_stand_damage: rd.int("last_stand_damage"),
        never_lost_damage: rd.int("never_lost_damage"),
    }
}

/// `pre_battle_damage`: `{"row": "FRONT"|"MIDDLE"|"REAR", "percent", "damage_type"}`. An unknown
/// row warns and reads as none.
fn read_pre_battle_damage(rd: &Rd, building: &str) -> Option<PreBattleDamage> {
    let c = rd.child("pre_battle_damage")?;
    let token = c.string("row");
    let Some(row) = ROW_TOKENS.iter().position(|&r| r == token.trim()) else {
        rd.cx.warn(format!("{building}: pre_battle_damage row {token:?} is not FRONT, MIDDLE or REAR"));
        return None;
    };
    Some(PreBattleDamage { row: row.min(ROWS - 1), percent: c.int("percent"), damage_type: c.rf::<DamageType>("damage_type") })
}

/// A unit type's `engage_first_round`: `{"action", "pattern"}`, what it does in a battle's first
/// round from any row in place of its row's action (with the row's count), and its pattern then
/// (`SINGLE` without one). An unknown action or pattern, or NONE, warns and reads as none.
pub fn read_engage_first_round(rd: &Rd, unit_type: &str) -> Option<(ActionKind, AttackPattern)> {
    let c = rd.child("engage_first_round")?;
    let token = c.string("action");
    let kind = match ActionKind::parse(&token) {
        Some(k) if k != ActionKind::None => k,
        _ => {
            rd.cx.warn(format!("{unit_type}: engage_first_round action {token:?} is not an action"));
            return None;
        }
    };
    let pattern = if c.has("pattern") {
        let p = c.string("pattern");
        match AttackPattern::parse(&p) {
            Some(p) => p,
            None => {
                rd.cx.warn(format!("{unit_type}: engage_first_round pattern {p:?} is not a pattern"));
                AttackPattern::Single
            }
        }
    } else {
        AttackPattern::Single
    };
    Some((kind, pattern))
}

/// A promotion's `first_melee_pattern`: an attack pattern (`None` without the key; an unknown one
/// warns and reads as none).
pub fn read_first_melee_pattern(rd: &Rd, promotion: &str) -> Option<AttackPattern> {
    if !rd.has("first_melee_pattern") {
        return None;
    }
    let p = rd.string("first_melee_pattern");
    match AttackPattern::parse(&p) {
        Some(AttackPattern::Single) => None,
        Some(p) => Some(p),
        None => {
            rd.cx.warn(format!("{promotion}: first_melee_pattern {p:?} is not a pattern"));
            None
        }
    }
}
