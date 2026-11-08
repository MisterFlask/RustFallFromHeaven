//! Engagement combat's unit stats (docs/UNIT_STATS.md; plan sections 3 and 4): what a unit does
//! from each row, its battle stats, and its attack A and defence Dv, magic and magic defence per
//! hit.
//!
//! **Rows** ([`Game::unit_engage_rows`]): its type's own (`engage_rows` in units.jsonc), with its
//! rank preference (`engage_row_order`) and its kind, the action of the row it prefers
//! ([`Game::unit_engage_kind`]). A unit that strikes from the row it prefers and holds a promotion
//! with `anti_flank` (Formation) is anti-flanking (Q7). There are no roles (2026-11-06).
//!
//! **Stats** ([`Game::unit_stat`]): the type's sheet (`stats` in units.jsonc) plus the unit's own
//! points (`Unit::stat_flat`) under its own percentage (`Unit::stat_pct`), both from its
//! promotions' `stat_changes` and every change of its strength. HP, speed, accuracy, evasion and
//! crit are read so ([`Game::unit_engage_max_hp`], `_speed`, `_accuracy`, `_evasion`, `_crit`); the
//! damage field stays a percentage of the HP (G3).
//!
//! **A and Dv per hit** ([`Game::engage_hit`], G1, Q1, Q2, Q9). One hit has a hitter and a target.
//! The hitter's A and the target's Dv are its base (its sheet's attack or defence plus its own
//! points, x10, and its typed strength scaled by the opponent's resistances) under one summed
//! percentage modifier: its own percentage and the duel's terms ([`Game::unit_combat_terms`]):
//!
//! | Term | A (the hitter) | Dv (the target) |
//! |---|---|---|
//! | its own percentage (Q1) | its attack's (`stat_pct`) | its defence's |
//! | Homeland (in the owner's borders, where it fights) | yes | yes |
//! | Stigmata (Armageddon counter) | yes | yes |
//! | bonuses against the opponent's promotions, unit combat, domain, animals | its own against the target | its own against the hitter |
//! | unit class | its class attack modifier | its class defence modifier |
//! | the handicap's animal and barbarian modifiers | no | yes: today's duel puts them on one strength of the pair |
//! | the defended plot's own defence (its terrain or feature cover, hills, a fort, the city's buildings and culture, FfH's range defence) and `CITY_BARBARIAN_DEFENSE_MODIFIER` | no | **no** in a formation battle (the ground's rules replace cover); on the defending side against a missile ([`HitCtx::cover`]) |
//! | the unit's own fortification, city defence (`iCityDefense`), hills, feature and terrain defence | on the defending side | on the defending side |
//! | city attack, hills, feature and terrain attack against the defended plot | on the attacking side | on the attacking side |
//! | `AMPHIB_ATTACK_MODIFIER` | on the attacking side only | no |
//! | `RIVER_ATTACK_MODIFIER` | removed: the river costs slots (Q9) | |
//! | kamikaze, free wins against barbarians | not read (FfH sets neither) | |
//! | an army commander's aura (phase 7) | [`Game::army_aura_pct`] | [`Game::army_aura_pct`] |
//!
//! A unit's bonuses for where it fights count on both its A and its Dv (the owner's rule,
//! 2026-11-05; docs/UNIT_STATS.md): a defender's fortification, city, hills, feature and terrain
//! defence on its hits back too, an attacker's city, hills, feature and terrain attack on the blows
//! it takes too, since in a formation battle both sides hit (they were one-sided, G1, as in the
//! duel). The landing penalty stays on the attacker's A. Every unit of the defending side fights on
//! the defended plot and shares its terms. The plot itself gives Dv
//! nothing (revision 4): a city, a forest or jungle and hills act through the ground's rules of a
//! formation battle (`formation::Ground`) and the slots.
//!
//! **Magic per hit** (SPELL): the caster's magic against the target's magic defence, each its
//! sheet's plus its own points, x10, under its own percentage alone (no term of the table), and
//! the target's resistance to the spell's damage type ([`Pair::spell_pct`]).
//!
//! **Against nobody in particular** ([`Game::engage_a_alone`], [`Game::engage_job_defense`]): the
//! same stats against an opponent not known yet: the auto formation's strength, a charge's threat
//! and the view's headline.
//!
//! **Breakdowns** ([`Game::engage_stat`]): a stat with its parts, for the battle log and the
//! preview.

use ffh_data::*;

use super::formation::{Ground, Pair, Side};
use crate::unit::apply_combat_modifier;
use super::report::{Stat, TermSource};
use crate::game::Game;
use crate::ids::*;
use crate::unit::{term, CombatTerms, TermLog};

/// Where a hit happens (plan 4.2): what [`Game::engage_hit`] reads besides the two units.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct HitCtx {
    /// The defended plot: where every unit of the defending side fights.
    pub to: PlotIdx,
    /// The plot the attack comes from (the amphibious penalty; where the attacking side fights,
    /// for Homeland). `None`: an approach not known (the view's stats): by land, each unit where
    /// it stands.
    pub from: Option<PlotIdx>,
    /// The defended plot's own defence counts in the defenders' Dv (its cover, hills, a fort,
    /// the city's walls and culture, range defence, the city's defence against barbarians): a
    /// missile's strike, which comes from outside the field (Q11, as before formation battles).
    /// A formation battle has none (the ground's rules replace cover).
    pub cover: bool,
}

impl HitCtx {
    /// An attack from `from` on `to`: a formation battle's.
    pub fn new(from: PlotIdx, to: PlotIdx) -> HitCtx {
        HitCtx { to, from: Some(from), cover: false }
    }

    /// A missile's strike from `from` on `to`: the plot's own defence counts.
    pub fn missile(from: PlotIdx, to: PlotIdx) -> HitCtx {
        HitCtx { cover: true, ..HitCtx::new(from, to) }
    }
}

impl Game {
    /// Whether unit `u` can take part in a land engagement (`GameData::type_engages_on_land`): a
    /// land or immobile type. Sea and air units never do. Whether it fights is its strength.
    pub fn unit_engages_on_land(&self, u: UnitId) -> bool {
        self.d.type_engages_on_land(self.units[u].unit_type)
    }

    /// Unit `u`'s rank preference: its type's rows in the order the auto formation tries them.
    pub fn unit_engage_row_order(&self, u: UnitId) -> [usize; ROWS] {
        self.d.engage_row_order(self.units[u].unit_type)
    }

    /// Unit `u`'s kind: what its type does from the row it prefers (`GameData::engage_kind`), the
    /// auto formation's order (`formation::kind_pick_order`). Its type's rows, before a battle
    /// spell takes the place of its attacks.
    pub fn unit_engage_kind(&self, u: UnitId) -> ActionKind {
        self.d.engage_kind(self.units[u].unit_type)
    }

    /// Whether unit `u` is anti-flanking (Q7): it strikes from the row it prefers and holds a
    /// promotion with `anti_flank` (Formation). Whatever an enemy flanker can reach, such units in
    /// its reach are taken first.
    pub fn unit_engage_anti_flank(&self, u: UnitId) -> bool {
        self.unit_engage_kind(u) == ActionKind::Strike && self.units[u].has_promotion.iter_enumerated().any(|(p, &has)| has && self.d.promotions[p].anti_flank)
    }

    /// Unit `u`'s stat `s` in a battle (docs/UNIT_STATS.md): its type's sheet plus its own points,
    /// under its own percentage; at least 0 (HP at least 1). A type without a sheet (it never
    /// fights a land battle) answers 0, and the duel's `MAX_HIT_POINTS` for HP.
    pub fn unit_stat(&self, u: UnitId, s: StatType) -> i32 {
        let unit = &self.units[u];
        let Some(sheet) = self.d.units[unit.unit_type].stats else {
            return if s == StatType::Hp { self.d.defines.max_hit_points } else { 0 };
        };
        let v = apply_combat_modifier(sheet[s.idx()] + unit.stat_flat[s.idx()], unit.stat_pct[s.idx()]);
        if s == StatType::Hp {
            v.max(1)
        } else {
            v.max(0)
        }
    }

    /// The unit's maximum HP in an engagement (plan 4.1): its HP stat.
    pub fn unit_engage_max_hp(&self, u: UnitId) -> i32 {
        self.unit_stat(u, StatType::Hp)
    }

    // A unit's values in a formation battle (docs/ENGAGEMENTS.md, "Per-unit values"): its stats,
    // and its type's own rows. The
    // resolver's fighters, the auto formation, the preview, the view and the client read these.

    /// Unit `u`'s speed in a formation battle before first strikes.
    pub fn unit_engage_speed(&self, u: UnitId) -> i32 {
        self.unit_stat(u, StatType::Speed)
    }

    /// Unit `u`'s accuracy: its chance to hit with a physical action, before the target's evasion.
    pub fn unit_engage_accuracy(&self, u: UnitId) -> i32 {
        self.unit_stat(u, StatType::Accuracy)
    }

    /// Unit `u`'s evasion: taken off a physical hitter's chance to hit it.
    pub fn unit_engage_evasion(&self, u: UnitId) -> i32 {
        self.unit_stat(u, StatType::Evasion)
    }

    /// Unit `u`'s crit chance on STRIKE, CHARGE, STAB and SHOOT.
    pub fn unit_engage_crit(&self, u: UnitId) -> i32 {
        self.unit_stat(u, StatType::Crit)
    }

    /// What unit `u` does from each row before its extra actions: its type's `engage_rows`; a unit
    /// with magic that knows a battle spell casts it in place of each
    /// attack (`rows_with_battle_spell`; docs/ENGAGEMENTS.md, "Battle spells").
    pub fn unit_engage_rows(&self, u: UnitId) -> [RowActions; ROWS] {
        let rows = self.d.engage_rows(self.units[u].unit_type);
        if self.unit_battle_spells(u).any() && self.unit_stat(u, StatType::Magic) > 0 {
            rows_with_battle_spell(rows)
        } else {
            rows
        }
    }

    /// Unit `u`'s best row (of [`Self::unit_engage_rows`]: the most actions, the first such).
    pub fn unit_engage_best_row(&self, u: UnitId) -> usize {
        rows_best_row(&self.unit_engage_rows(u))
    }

    /// What unit `u` does from each row in a battle: its rows, with `ENGAGEMENT_EXTRA_ACTIONS`
    /// more in its best row for a type of tier `ENGAGEMENT_EXTRA_ACTION_TIER` or above and for a
    /// hero.
    pub fn unit_engage_actions(&self, u: UnitId) -> [RowActions; ROWS] {
        rows_with_extra_actions(self.unit_engage_rows(u), self.d.engage_extra_actions(self.units[u].unit_type, self.unit_is_hero(u)))
    }

    /// Whether unit `u` is a flanker (one of its rows charges): anti-flank units in its reach are
    /// taken first.
    pub fn unit_engage_flanks(&self, u: UnitId) -> bool {
        rows_charge(&self.unit_engage_rows(u))
    }

    /// The battle spell unit `u` knows (`GameData::engage_damage_spell`: the strongest damage or
    /// battle-only spell whose prerequisites it meets, its religion included), whatever its rows.
    pub fn unit_battle_spell(&self, u: UnitId) -> Option<SpellType> {
        self.unit_battle_spells(u).attack
    }

    /// Unit `u`'s battle spells: its damage spell, its heal, its charms and hexes
    /// (`GameData::engage_battle_spells`), whatever its rows.
    pub fn unit_battle_spells(&self, u: UnitId) -> BattleSpells<SpellType> {
        let unit = &self.units[u];
        self.d.engage_battle_spells(unit.unit_type, |p| unit.has_promotion[p], unit.religion)
    }

    /// The spell unit `u` casts with its SPELL actions: its battle spell ([`Self::unit_battle_spell`]);
    /// `None` when it knows none (it casts the magic bolt) or casts nothing (no row of it has SPELL).
    pub fn unit_engage_spell(&self, u: UnitId) -> Option<SpellType> {
        self.unit_engage_battle_spells(u).attack
    }

    /// The battle spells unit `u` casts with its SPELL actions ([`Self::unit_battle_spells`]); none
    /// when no row of it has SPELL.
    pub fn unit_engage_battle_spells(&self, u: UnitId) -> BattleSpells<SpellType> {
        if !self.unit_engage_actions(u).iter().any(|r| r.kind == ActionKind::Spell) {
            return BattleSpells::default();
        }
        self.unit_battle_spells(u)
    }

    /// The defended plot's ground for a formation battle (spec 1.6): a city on it; a forest or
    /// jungle (a feature that narrows the field: `iEngagementSlotsChange` below 0, the forests and
    /// the jungle); hills.
    pub fn engage_ground(&self, to: PlotIdx) -> Ground {
        let plot = &self.map.plots[to];
        Ground {
            city: self.plot_is_city(to, false, None),
            forest: plot.feature_type.is_some_and(|f| self.d.features[f].engagement_slots_change < 0),
            hills: self.plot_is_hills(to),
        }
    }

    /// The A of `hitter` (on side `hitter_side`) against `target` and the Dv of `target` against
    /// `hitter`, x100 (plan 4.2; the module comment's table), and the hitter's magic against the
    /// target's magic defence and resistance: the pair table's entry.
    pub fn engage_hit(&self, hitter: UnitId, target: UnitId, hitter_side: Side, ctx: &HitCtx) -> Pair {
        let a = self.engage_combat_terms(hitter, Some(target), ctx, hitter_side, true, &mut TermLog::none());
        let dv = self.engage_combat_terms(target, Some(hitter), ctx, hitter_side.other(), false, &mut TermLog::none());
        Pair {
            a: a.strength(),
            dv: dv.strength(),
            ma: self.engage_magic_x100(hitter, true),
            md: self.engage_magic_x100(target, false),
            spell_pct: self.engage_spell_pct(hitter, Some(target)),
            pct: self.engage_pair_pct(hitter, Some(target)),
        }
    }

    /// The percentage of `hitter`'s hits' damage on `target` (`None`: a city's Sentinel): `100 +`
    /// its damage against the target's unit combat (and against a large target, or walls), times
    /// `100 -` the target's resistance to the hitter's unit combat, at least 0 (docs/UNIT_ROLES.md,
    /// L1 and L2).
    pub fn engage_pair_pct(&self, hitter: UnitId, target: Option<UnitId>) -> i32 {
        let ht = &self.d.units[self.units[hitter].unit_type];
        let up = match target {
            Some(t) => {
                let mut v = self.unit_engage_damage_vs(hitter, self.unit_get_unit_combat_type(t));
                if self.unit_engage_large(t) {
                    v += ht.damage_vs_large;
                }
                v
            }
            None => ht.damage_vs_walls,
        };
        let down = target.map_or(0, |t| self.unit_engage_resist_vs(t, self.unit_get_unit_combat_type(hitter)));
        ((100 + up).max(0) as i64 * (100 - down).max(0) as i64 / 100) as i32
    }

    /// Unit `u`'s damage against unit combat `c` (percent more; its type's `damage_vs` and its
    /// promotions').
    pub fn unit_engage_damage_vs(&self, u: UnitId, c: Option<UnitCombatType>) -> i32 {
        let Some(c) = c else { return 0 };
        let unit = &self.units[u];
        self.d.units[unit.unit_type].damage_vs[c] + self.d.promotions.keys().filter(|&p| unit.has_promotion[p]).map(|p| self.d.promotions[p].damage_vs[c]).sum::<i32>()
    }

    /// Unit `u`'s resistance to damage from unit combat `c` (percent less; its type's `resist_vs`
    /// and its promotions').
    pub fn unit_engage_resist_vs(&self, u: UnitId, c: Option<UnitCombatType>) -> i32 {
        let Some(c) = c else { return 0 };
        let unit = &self.units[u];
        self.d.units[unit.unit_type].resist_vs[c] + self.d.promotions.keys().filter(|&p| unit.has_promotion[p]).map(|p| self.d.promotions[p].resist_vs[c]).sum::<i32>()
    }

    /// Unit `u`'s armor: HP off each physical hit on it (its type's and its promotions').
    pub fn unit_engage_armor(&self, u: UnitId) -> i32 {
        let unit = &self.units[u];
        self.d.units[unit.unit_type].armor + self.d.promotions.keys().filter(|&p| unit.has_promotion[p]).map(|p| self.d.promotions[p].armor).sum::<i32>()
    }

    /// Unit `u`'s hits ignore armor (its type's or a promotion's `armor_piercing`).
    pub fn unit_engage_piercing(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        self.d.units[unit.unit_type].armor_piercing || self.d.promotions.keys().any(|p| unit.has_promotion[p] && self.d.promotions[p].armor_piercing)
    }

    /// Unit `u`'s hits cannot be withdrawn from (its type's or a promotion's `pursuit`).
    pub fn unit_engage_pursuit(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        self.d.units[unit.unit_type].pursuit || self.d.promotions.keys().any(|p| unit.has_promotion[p] && self.d.promotions[p].pursuit)
    }

    /// Unit `u`'s magic (`magic`) or magic defence in a battle, x100: its sheet's plus its own
    /// points, x10, under its own percentage (module comment, "Magic per hit").
    pub fn engage_magic_x100(&self, u: UnitId, magic: bool) -> i32 {
        self.engage_magic_terms(u, magic, &mut TermLog::none()).strength()
    }

    /// The percentage of its spells' damage `hitter` deals to `target`: 100 less the target's
    /// resistance to the damage type of the spell it casts (none for the magic bolt), at least 0.
    /// `None`: a target with no resistances (a city's Sentinel).
    pub fn engage_spell_pct(&self, hitter: UnitId, target: Option<UnitId>) -> i32 {
        let dt = self.unit_engage_spell(hitter).and_then(|sp| self.d.spells[sp].damage_type);
        match (dt, target) {
            (Some(dt), Some(t)) => (100 - self.unit_get_damage_type_resist(t, dt)).max(0),
            _ => 100,
        }
    }

    fn engage_magic_terms(&self, u: UnitId, magic: bool, log: &mut TermLog) -> CombatTerms {
        log.reset(self, u);
        let s = if magic { StatType::Magic } else { StatType::MagicDefense };
        let unit = &self.units[u];
        let base = self.d.units[unit.unit_type].stats.map_or(0, |sheet| sheet[s.idx()]) + unit.stat_flat[s.idx()];
        let own = unit.stat_pct[s.idx()];
        if log.listing() {
            self.engage_own_percent_terms(u, s, own, log);
        }
        CombatTerms { base: base * 10, modifier: own }
    }

    /// Unit `u`'s A on side `side` of the attack `ctx` against nobody in particular (its own
    /// percentage, and on the attacking side its city, terrain and amphibious terms), x100: a
    /// fighter's "most dangerous" and the auto formation's strength.
    pub fn engage_a_alone(&self, u: UnitId, side: Side, ctx: &HitCtx) -> i32 {
        self.engage_combat_terms(u, None, ctx, side, true, &mut TermLog::none()).strength()
    }

    /// Unit `u`'s Dv on side `side` of the attack `ctx` against a hitter with no unit terms (a
    /// city's Sentinel: docs/SIEGES.md), x100.
    pub fn engage_dv_alone(&self, u: UnitId, side: Side, ctx: &HitCtx) -> i32 {
        self.engage_combat_terms(u, None, ctx, side, false, &mut TermLog::none()).strength()
    }

    /// A defender's Dv on the defended plot against a hitter not known yet (its own percentage,
    /// fortification, its city and terrain defense), x100: the view's headline.
    pub fn engage_job_defense(&self, u: UnitId, ctx: &HitCtx) -> i32 {
        self.engage_combat_terms(u, None, ctx, Side::Defender, false, &mut TermLog::none()).strength()
    }

    /// A stat with its parts (the battle log's `Stat`): `u`'s A (`attack`) or Dv on side `side`
    /// against `opponent` (`None`: an opponent not known yet, the card's headline). Its terms sum
    /// to the modifier; the own percentage is listed by promotion (the rest as `Own`).
    pub fn engage_stat(&self, u: UnitId, opponent: Option<UnitId>, side: Side, attack: bool, ctx: &HitCtx) -> Stat {
        let mut terms = Vec::new();
        let t = self.engage_combat_terms(u, opponent, ctx, side, attack, &mut TermLog { details: None, terms: Some(&mut terms) });
        Stat { base_x100: t.base, terms, total_x100: t.strength() }
    }

    /// Unit `u`'s magic (`magic`) or magic defence with its parts (the battle log's `Stat`).
    pub fn engage_magic_stat(&self, u: UnitId, magic: bool) -> Stat {
        let mut terms = Vec::new();
        let t = self.engage_magic_terms(u, magic, &mut TermLog { details: None, terms: Some(&mut terms) });
        Stat { base_x100: t.base, terms, total_x100: t.strength() }
    }

    /// Phase 7 hook (plan 14): the percentages an army commander's aura adds to the A and Dv of
    /// army `a`'s units. None in phase 6.
    pub fn army_aura_pct(&self, _a: ArmyId) -> (i32, i32) {
        (0, 0)
    }

    /// The base of the unit's A (`attack`) or Dv, x100 (plan 4.2; docs/UNIT_STATS.md): its sheet's
    /// attack (defence) plus its own points, x10; plus its typed strength, each type's points
    /// scaled by the opponent's resistance to it (Q2). A type without a sheet (it never fights a
    /// land battle): its strength, x100.
    fn engage_base_x100(&self, u: UnitId, opponent: Option<UnitId>, attack: bool) -> i32 {
        let unit = &self.units[u];
        let s = if attack { StatType::Attack } else { StatType::Defense };
        let own = match self.d.units[unit.unit_type].stats {
            Some(sheet) => (sheet[s.idx()] + unit.stat_flat[s.idx()]) * 10,
            None => (if attack { unit.base_combat } else { unit.base_combat_defense }) * 100,
        };
        // Card: Elemental Forge (docs/CARDS.md): an attack of a damage type is cut by its resistance.
        let own = if attack { self.engage_arms_attack(u, opponent, own) } else { own };
        let mut typed = 0;
        for damage_type in self.d.damages.keys() {
            let points = unit.damage_type_combat[damage_type];
            if points != 0 {
                let resist = opponent.map_or(0, |o| self.unit_get_damage_type_resist(o, damage_type));
                typed += points * (100 - resist);
            }
        }
        own + typed
    }

    /// [`StrMode::EngageAttack`] (`attack`) and [`StrMode::EngageDefense`]: the terms of `u`'s
    /// A or Dv on side `side`, `opponent` the target it hits or the hitter that hits it (`None`:
    /// not known yet). The module comment's table.
    pub(crate) fn engage_combat_terms(
        &self,
        u: UnitId,
        opponent: Option<UnitId>,
        ctx: &HitCtx,
        side: Side,
        attack: bool,
        log: &mut TermLog,
    ) -> CombatTerms {
        log.reset(self, u);
        let unit = &self.units[u];
        let base = self.engage_base_x100(u, opponent, attack);
        let mut modifier = 0;

        // The unit's own percentage of its attack or defence (Q1: its promotions' and its strength
        // changes'); Homeland where it fights; Stigmata.
        let stat = if attack { StatType::Attack } else { StatType::Defense };
        let own = unit.stat_pct[stat.idx()];
        let fights_on = match side {
            Side::Attacker => ctx.from.unwrap_or_else(|| self.unit_plot(u)),
            Side::Defender => ctx.to,
        };
        let in_borders = if self.map.plots[fights_on].owner == Some(unit.owner) { unit.combat_percent_in_borders } else { 0 };
        log.set(|det| det.extra_combat_percent = own + in_borders);
        if log.listing() {
            self.engage_own_percent_terms(u, stat, own, log);
        }
        modifier += own;
        modifier += log.push(in_borders, || TermSource::InBorders);
        modifier += log.push(self.unit_global_counter_modifier(u), || TermSource::GlobalCounter);

        // Its bonuses for where it fights, on both its A and its Dv (the module comment): an
        // attacker's against the plot it attacks, a defender's own on the defended plot. No river
        // term (Q9: the river costs slots instead). A missile's strike keeps the duel's sides: the
        // attacker's on its A only, the defender's under its cover (below).
        if side == Side::Attacker && (attack || !ctx.cover) {
            let city = match opponent {
                Some(t) => self.plot_is_city(ctx.to, true, Some(self.unit_get_team(t))),
                None => self.plot_is_city(ctx.to, true, None),
            };
            let (plot_terms, terrain_term) = self.combat_attack_plot_terms(u, ctx.to, city, 1, log);
            modifier += plot_terms + terrain_term;
        }
        if side == Side::Defender && attack && !ctx.cover {
            modifier += self.engage_own_defense_terms(u, ctx.to, log);
        }
        if attack {
            // The landing penalty, on the attacker's hits only.
            if side == Side::Attacker {
                if let Some(from) = ctx.from {
                    modifier += term!(log, amphib_attack_modifier, self.unit_amphib_attack_modifier(u, from, ctx.to), TermSource::Amphibious);
                }
            }
        } else {
            // The handicap's animal and barbarian modifiers on the target only: today's duel puts
            // them on the defender's strength alone (C++ `currCombatStr(NULL, pDefender)` clears
            // the attacker's opponent), so one side of the pair keeps today's ratio.
            if let Some(h) = opponent {
                modifier += self.combat_handicap_terms(u, h, log);
            }
            // The unit's own defensive terms on the defended plot (on its A too, above). The
            // plot's own defence (cover, hills, a fort, the city's walls and culture, range
            // defence) and the city's barbarian defence give Dv nothing in a formation battle: the
            // ground's rules replace them (spec 1.6). Against a missile (`ctx.cover`) they count,
            // as the duel's.
            if side == Side::Defender && ctx.cover {
                let team = self.unit_get_team(u);
                let ignore_building = opponent.is_some_and(|h| self.unit_ignore_building_defense(h));
                modifier += self.combat_plot_defense_terms(u, ctx.to, team, ignore_building, log);
                if let Some(h) = opponent {
                    if self.unit_is_barbarian(h) && self.plot_is_city(ctx.to, true, Some(team)) {
                        let pct = self.d.defines.city_barbarian_defense_modifier;
                        modifier += term!(log, city_barbarian_defense_modifier, pct, TermSource::VsBarbarian);
                    }
                }
            } else if side == Side::Defender {
                modifier += self.engage_own_defense_terms(u, ctx.to, log);
            }
        }

        // Its bonuses against the opponent: as a hitter its attack bonuses, as a target its
        // defence bonuses.
        if let Some(o) = opponent {
            modifier += self.combat_vs_terms(u, o, !attack, 1, log);
        }

        // Phase 7: the army commander's aura.
        if let Some(a) = unit.army.filter(|&a| self.armies.contains(a)) {
            let (aura_a, aura_dv) = self.army_aura_pct(a);
            let commander = self.armies[a].commander.unwrap_or(u);
            modifier += log.push(if attack { aura_a } else { aura_dv }, || TermSource::Aura(commander));
        }

        log.set(|det| {
            det.modifier_total = modifier;
            det.base_combat_str = base / 100;
        });
        CombatTerms { base, modifier }
    }

    /// The defensive terms that are unit `u`'s own on the defended plot `p` (spec 1.6): its
    /// fortification, its city defence in a city, its hills defence on hills, its feature defence
    /// (else its terrain defence). The duel's `combat_plot_defense_terms` less the plot's defence.
    fn engage_own_defense_terms(&self, u: UnitId, p: PlotIdx, log: &mut TermLog) -> i32 {
        let team = self.unit_get_team(u);
        let mut modifier = term!(log, fortify_modifier, self.unit_fortify_modifier(u), TermSource::Fortify);
        if self.plot_is_city(p, true, Some(team)) {
            modifier += term!(log, city_defense_modifier, self.unit_city_defense_modifier(u), TermSource::UnitCityDefense);
        }
        if self.plot_is_hills(p) {
            modifier += term!(log, hills_defense_modifier, self.unit_hills_defense_modifier(u), TermSource::UnitHillsDefense);
        }
        match self.map.plots[p].feature_type {
            Some(feature) => {
                modifier += term!(log, feature_defense_modifier, self.unit_feature_defense_modifier(u, feature), TermSource::UnitFeatureDefense(feature));
            }
            None => {
                if let Some(terrain) = self.map.plots[p].terrain_type {
                    modifier += term!(log, terrain_defense_modifier, self.unit_terrain_defense_modifier(u, terrain), TermSource::UnitTerrainDefense(terrain));
                }
            }
        }
        modifier
    }

    /// The breakdown of the unit's own percentage `total` of stat `s` (Q1): each promotion it
    /// holds with one, the rest (spells, FfH's Python) as `Own`.
    fn engage_own_percent_terms(&self, u: UnitId, s: StatType, total: i32, log: &mut TermLog) {
        let held = &self.units[u].has_promotion;
        let mut rest = total;
        for p in self.d.promotions.keys() {
            if held[p] {
                rest -= log.push(self.d.promotions[p].stat_changes[s.idx()].percent, || TermSource::Promotion(p));
            }
        }
        log.push(rest, || TermSource::Own);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::{CombatDetails, StrMode};

    /// Over many units, opponents and plots (with and without a known approach, a missile's
    /// cover): the breakdown's
    /// parts sum to the modifier and give the pair table's value, and `unit_combat_terms` in the
    /// engagement modes is the same computation.
    #[test]
    fn breakdowns_sum_to_the_stats_on_every_pair() {
        let (g, units, plots) = crate::test_world::world();
        let mut pairs = 0;
        for (i, &h) in units.iter().enumerate() {
            let to = plots[(i * 7) % plots.len()];
            for (from, cover) in [(Some(g.unit_plot(h)), false), (None, false), (Some(g.unit_plot(h)), true)] {
                let ctx = HitCtx { to, from, cover };
                for &t in units.iter().skip(i % 5).step_by(5) {
                    if t == h {
                        continue;
                    }
                    for side in [Side::Attacker, Side::Defender] {
                        let pair = g.engage_hit(h, t, side, &ctx);
                        let a = g.engage_stat(h, Some(t), side, true, &ctx);
                        let dv = g.engage_stat(t, Some(h), side.other(), false, &ctx);
                        assert_eq!((a.total_x100, dv.total_x100), (pair.a, pair.dv));
                        let mut det = CombatDetails::default();
                        let ta = g.unit_combat_terms(h, Some(t), StrMode::EngageAttack { ctx: &ctx, side }, Some(&mut det)).unwrap();
                        assert_eq!(det.modifier_total, ta.modifier);
                        let td = g.unit_combat_terms(t, Some(h), StrMode::EngageDefense { ctx: &ctx, side: side.other() }, None).unwrap();
                        for (stat, terms) in [(&a, ta), (&dv, td)] {
                            assert_eq!((stat.base_x100, stat.total_x100), (terms.base, terms.strength()));
                            assert_eq!(stat.terms.iter().map(|t| t.pct).sum::<i32>(), terms.modifier, "{:?}", stat.terms);
                        }
                        pairs += 1;
                    }
                }
            }
        }
        assert!(pairs > 5000, "{pairs}");
    }
}
