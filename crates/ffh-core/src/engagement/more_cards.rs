//! The great-person cards of the batch "battle more" on `Game` (docs/CARDS.md): what a unit's
//! battle kit (`ffh_data::BattleKit`, its type's and its promotions') makes of its fighter, the
//! Sentinels a battle's defender brings from the field (the Bastion's, the Cairn's), the Cairn's
//! defence, the drums at the morale check, arrows at the walls and the harpoons at sea. The
//! resolver's side is `formation/more_cards.rs`.
//!
//! | Card | Key | Here |
//! |---|---|---|
//! | Shieldbearer | `battle_shield` | the fighter's `kit.shield` |
//! | Fire Arrows | `shots_at_walls`, `damage_vs_walls` (promotion) | the fighter's `kit.shots_at_walls`; [`Game::unit_damage_vs_walls`] |
//! | War Drums | `battle_drums` | the fighter's `kit.drums`; [`Game::engage_drummed`] at the morale check |
//! | Gryphon Riders | `charge_anywhere`, `no_rest_on_water` | the fighter's `kit.charge_anywhere`; `ffh::cards::battle_more` for the water |
//! | Runeguard, Witch-Hunter, Automatons | `battle_immune` | the fighter's `kit.immune_*` |
//! | Lantern of Lugus | `attack_damage_type` (unit type) | [`Game::unit_attack_damage_type`] (`arms.rs`) |
//! | Bastion | `battle_sentinels` (unit type) | [`Game::engage_add_field_sentinels`] |
//! | Cairn Golem | `battle_sentinels`, `engagement_defense` (improvement) | [`Game::engage_add_field_sentinels`]; [`Game::plot_engagement_defense`] |
//! | Leviathan Harpoons | `naval_damage_vs` | [`Game::unit_naval_damage_pct`] (the naval duel) |
//!
//! **Field Sentinels.** The defender's candidates' `battle_sentinels` (the Bastion's two) and the
//! defended plot's improvement's (the Cairn's one, when the defender has a unit there) are Sentinels
//! of the defender's side: `UNIT_SENTINEL`'s values, the HP and the defence grown by the plot's
//! `engagement_defense` as a city's defence grows its walls. They stand in the defender's cells
//! after the city's Sentinels (the front row, the lanes nearest the centre first), cost no slot, are
//! whole at the start of each battle (nothing keeps their damage) and hold no plot: once the
//! defender's units are gone, the plot falls whatever they did.

use ffh_data::*;

use super::formation::more_cards::Kit;
use super::formation::{Fighter, Params, Side, Spell};
use super::report::{Immunities, LogFighter, Stat, Term, TermSource};
use super::slots::Slots;
use crate::game::Game;
use crate::ids::*;

/// Where a battle's Sentinel comes from: the defended city's walls, or the field (a Bastion, a
/// Cairn) with the defender's owner, the defended plot's improvement and the percent the
/// improvement's defence grows its values by.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SentinelSource {
    City(CityId),
    Field { owner: PlayerId, improvement: Option<ImprovementType>, grow: i32 },
}

impl Game {
    /// Unit `u`'s battle kit: its type's and its promotions' (`GameData::unit_battle_kit`).
    pub fn unit_battle_kit(&self, u: UnitId) -> BattleKit {
        let unit = &self.units[u];
        self.d.unit_battle_kit(unit.unit_type, |p| unit.has_promotion[p])
    }

    /// Unit `u`'s battle kit on its fighter `f` (module comment).
    pub(crate) fn engage_fighter_kit(&self, u: UnitId, f: &mut Fighter) {
        let k = self.unit_battle_kit(u);
        f.kit = Kit {
            shield: k.shield.clamp(0, 100),
            shots_at_walls: k.shots_at_walls,
            drums: k.drums.max(0),
            charge_anywhere: k.charge_anywhere,
            immune_spells: k.immune_spells,
            immune_charms: k.immune_charms,
            immune_hexes: k.immune_hexes,
        };
    }

    /// Unit `u`'s damage against Sentinels, percent more (`damage_vs_walls`: its type's and its
    /// promotions').
    pub fn unit_damage_vs_walls(&self, u: UnitId) -> i32 {
        let d = self.d;
        let unit = &self.units[u];
        d.units[unit.unit_type].damage_vs_walls + d.promotions.keys().filter(|&p| unit.has_promotion[p]).map(|p| d.promotions[p].damage_vs_walls).sum::<i32>()
    }

    /// Whether the committed units `units[range]` that do not sit out beat drums (War Drums): the
    /// side is immune to fear at the morale check.
    pub(crate) fn engage_drummed(&self, units: &[UnitId], range: std::ops::Range<usize>, sat_out: &[bool]) -> bool {
        range.into_iter().any(|i| !sat_out[i] && self.unit_battle_kit(units[i]).drums > 0)
    }

    /// The defended plot's improvement's `engagement_defense` (the Cairn), if any.
    pub fn plot_engagement_defense(&self, p: PlotIdx) -> Option<(ImprovementType, i32)> {
        let i = self.map.plots[p].improvement_type?;
        let pct = self.d.improvements[i].engagement_defense;
        (pct != 0).then_some((i, pct))
    }

    /// The Sentinels the defender brings from the field to a battle on plot `to` with the defending
    /// `candidates` (module comment): their count and the percent their values grow by (the most of
    /// their sources': a unit's `battle_sentinel_defense`, the improvement's `engagement_defense`).
    /// None without a candidate.
    pub fn engage_field_sentinels(&self, candidates: &[UnitId], to: PlotIdx) -> (u8, i32) {
        if candidates.is_empty() {
            return (0, 0);
        }
        let (mut n, mut grow) = (0, 0);
        for &u in candidates {
            let k = self.unit_battle_kit(u);
            if k.sentinels > 0 {
                n += k.sentinels;
                grow = grow.max(k.sentinel_defense);
            }
        }
        if let Some(i) = self.map.plots[to].improvement_type.filter(|&i| self.d.improvements[i].battle_sentinels > 0) {
            n += self.d.improvements[i].battle_sentinels;
            grow = grow.max(self.d.improvements[i].engagement_defense);
        }
        (n.clamp(0, 9) as u8, grow.max(0))
    }

    /// Sets `slots`' field Sentinels for a battle on `to` with the defending `candidates` (module
    /// comment): the defender's grid gets their cells; they cost no slot.
    pub fn engage_add_field_sentinels(&self, slots: &mut Slots, candidates: &[UnitId], to: PlotIdx) {
        let (n, grow) = self.engage_field_sentinels(candidates, to);
        slots.field_sentinels = n;
        slots.field_grow = grow;
    }

    /// A field Sentinel's max HP: `UNIT_SENTINEL`'s grown by `grow` percent.
    pub fn field_sentinel_max_hp(&self, grow: i32) -> i32 {
        (self.sentinel_stat(StatType::Hp).max(1) * (100 + grow.max(0)) / 100).max(1)
    }

    /// A field Sentinel's Dv (`defense`) or magic defence, x100: `UNIT_SENTINEL`'s, x10, grown by
    /// `grow` percent.
    pub fn field_sentinel_def_x100(&self, grow: i32, magic: bool) -> i32 {
        let s = if magic { StatType::MagicDefense } else { StatType::Defense };
        (self.sentinel_stat(s) * 10 * (100 + grow.max(0)) / 100).max(1)
    }

    /// A field Sentinel as a fighter of the defender in cell `place`, whole.
    pub(crate) fn engage_field_sentinel_fighter(&self, k: usize, place: (u8, u8), grow: i32, params: &Params) -> Fighter {
        let d = self.d;
        let ty = self.sentinel_unit_type();
        let mut f = Fighter::new(Side::Defender, self.field_sentinel_max_hp(grow));
        f.tag = u32::MAX - 100 - k as u32;
        (f.lane, f.row) = place;
        f.speed = self.sentinel_stat(StatType::Speed);
        f.actions = d.engage_actions(ty, false);
        f.a = self.sentinel_a_x100();
        f.accuracy = self.sentinel_stat(StatType::Accuracy);
        f.evasion = self.sentinel_stat(StatType::Evasion);
        f.crit = self.sentinel_stat(StatType::Crit);
        f.spell = Spell::bolt(params);
        f.fixed = true;
        f
    }

    /// A Sentinel of `src` in cell `place` with `hp` (centi-HP) as the battle log and the plan show
    /// it: the city's ([`Game::city_sentinel_log_fighter`]) or a field one, of the defender's owner.
    pub fn sentinel_log_fighter(&self, src: SentinelSource, place: (u8, u8), hp: i32) -> LogFighter {
        let (owner, improvement, grow) = match src {
            SentinelSource::City(c) => return self.city_sentinel_log_fighter(c, place, hp),
            SentinelSource::Field { owner, improvement, grow } => (owner, improvement, grow),
        };
        let d = self.d;
        let ty = self.sentinel_unit_type();
        let a = self.sentinel_a_x100();
        let base_dv = self.sentinel_stat(StatType::Defense) * 10;
        let base_md = self.sentinel_stat(StatType::MagicDefense) * 10;
        let mag = self.sentinel_stat(StatType::Magic) * 10;
        let src = improvement.map_or(TermSource::Own, TermSource::Improvement);
        let terms: Vec<Term> = (grow != 0).then_some(Term { src, pct: grow }).into_iter().collect();
        let grown = |base: i32| Stat { base_x100: base, terms: terms.clone(), total_x100: (base * (100 + grow) / 100).max(1) };
        LogFighter {
            unit: LogFighter::SENTINEL_UNIT,
            sentinel: true,
            unit_type: ty,
            name: String::new(),
            owner,
            shown_owner: owner,
            side: Side::Defender,
            place: Some(place),
            large: false,
            level: 0,
            xp: 0,
            hero: false,
            max_hp: self.field_sentinel_max_hp(grow),
            hp_start: hp,
            hp_battle: hp,
            speed: self.sentinel_stat(StatType::Speed),
            first_strikes: 0,
            chance_first_strikes: 0,
            rolled_first_strikes: 0,
            actions: d.engage_actions(ty, false),
            spell: None,
            accuracy: self.sentinel_stat(StatType::Accuracy),
            evasion: self.sentinel_stat(StatType::Evasion),
            crit: self.sentinel_stat(StatType::Crit),
            anti_flank: false,
            flanker: false,
            withdraw_pct: 0,
            combat_limit: 100,
            collateral_limit: None,
            def_strike: None,
            immune: Immunities::default().with(Immunities::DEF_STRIKE, true).with(Immunities::FEAR, true),
            fear: false,
            a: Stat { base_x100: a, terms: Vec::new(), total_x100: a },
            dv: grown(base_dv),
            mag: Stat { base_x100: mag, terms: Vec::new(), total_x100: mag },
            mdef: grown(base_md),
            fate: None,
            dealt: 0,
            taken: 0,
            healed: 0,
        }
    }

    /// The percentage of the damage ship `u` deals to `other` in the rounds it wins of a naval
    /// battle (`naval_damage_vs`, its promotions' against `other`'s class: the Leviathan Harpoons).
    pub fn unit_naval_damage_pct(&self, u: UnitId, other: UnitId) -> i32 {
        let d = self.d;
        if self.unit_get_domain_type(u) != DomainType::DOMAIN_SEA {
            return 100;
        }
        let Some(class) = d.units[self.units[other].unit_type].unit_class_type else { return 100 };
        let unit = &self.units[u];
        let more: i32 = d.promotions.keys().filter(|&p| unit.has_promotion[p]).map(|p| d.promotions[p].naval_damage_vs[class]).sum();
        (100 + more).max(0)
    }
}

#[cfg(test)]
#[path = "more_cards_tests.rs"]
mod tests;
