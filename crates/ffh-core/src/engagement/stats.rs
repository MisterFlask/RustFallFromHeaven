//! Engagement combat's derived unit stats (plan sections 3 and 4): a unit's role, its HP, and its
//! attack A and defence Dv per hit.
//!
//! **Roles** ([`Game::unit_combat_role`]): the type's role (`GameData::unit_roles`, every land and
//! immobile type has one), a Frontline unit with Formation being a Screen (Q7).
//!
//! **HP** ([`Game::unit_engage_max_hp`]): the type's `iHP`, else its role's; the damage field stays
//! a percentage of it (G3).
//!
//! **Formation battles' per-unit values** ([`Game::unit_engage_speed`], `_dodge`, `_crit_bonus`,
//! `_rows`, `_best_row`, `_actions`, `_flanks`): the type's own (`iEngageSpeed`, `iEngageDodge`,
//! `iEngageCritBonus`, `EngageRows`), else its role's.
//!
//! **A and Dv per hit** ([`Game::engage_hit`], G1, Q1, Q2, Q9). One hit has a hitter and a target.
//! The hitter's A and the target's Dv are its base (the type's strength times the role's percent,
//! or the type's `iAttack` / `iDefense`, plus the unit's own flat strength and its typed strength
//! scaled by the opponent's resistances, both times the role's percent) under one summed
//! percentage modifier, through the same terms as the duel ([`Game::unit_combat_terms`]):
//!
//! | Term | A (the hitter) | Dv (the target) |
//! |---|---|---|
//! | the promotions' own percentage (Q1) | `iCombatPercent` | `iCombatPercentDefense` |
//! | Homeland (in the owner's borders, where it fights) | yes | yes |
//! | Stigmata (Armageddon counter) | yes | yes |
//! | bonuses against the opponent's promotions, unit combat, domain, animals | its own against the target | its own against the hitter |
//! | unit class | its class attack modifier | its class defence modifier |
//! | the handicap's animal and barbarian modifiers | no | yes: today's duel puts them on one strength of the pair |
//! | the defended plot's defence, fortification, city, hills, feature and terrain defence; `CITY_BARBARIAN_DEFENSE_MODIFIER` | no | on the defending side only |
//! | city attack, hills, feature and terrain attack against the defended plot, `AMPHIB_ATTACK_MODIFIER` | on the attacking side only | no |
//! | `RIVER_ATTACK_MODIFIER` | removed: the river costs slots (Q9) | |
//! | kamikaze, free wins against barbarians | not read (FfH sets neither) | |
//! | an army commander's aura (phase 7) | [`Game::army_aura_pct`] | [`Game::army_aura_pct`] |
//!
//! So an attacker being hit has no plot defence, fortification or city defence, and a defender
//! hitting back has no city attack, terrain attack or amphibious term (G1). Every unit of the
//! defending side fights on the defended plot and shares its terms.
//!
//! **Job strengths** ([`Game::engage_job_attack`], [`Game::engage_job_defense`]): the same stats
//! against an opponent not known yet, which the auto-pick sorts candidates by (plan 5.4).
//!
//! **Breakdowns** ([`Game::engage_stat`]): a stat with its parts, for the battle log and the
//! preview.

use ffh_data::*;

use super::battle::{Pair, Side};
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
    /// for Homeland). `None`: an approach not known (a hypothetical estimate): by land, each unit
    /// where it stands.
    pub from: Option<PlotIdx>,
    /// The defended city's defense modifier (with buildings) to assume in place of its current
    /// one: the estimator's planned bombard. `None`: the city's own.
    pub city_defense: Option<i32>,
}

impl HitCtx {
    /// An attack from `from` on `to`, at the city's current defense.
    pub fn new(from: PlotIdx, to: PlotIdx) -> HitCtx {
        HitCtx { to, from: Some(from), city_defense: None }
    }
}

impl Game {
    /// The unit's engagement role (plan 3): its type's, a Frontline unit with Formation being a
    /// Screen (`GameData::screen_role`, Q7). `None` for sea and air units, which never take part
    /// in a land engagement.
    pub fn unit_combat_role(&self, u: UnitId) -> Option<CombatRoleType> {
        let unit = &self.units[u];
        let role = self.d.unit_roles[unit.unit_type]?;
        match self.d.screen_role {
            Some((from, promotion, to)) if role == from && unit.has_promotion[promotion] => Some(to),
            _ => Some(role),
        }
    }

    /// The unit's maximum HP in an engagement (plan 4.1): its type's `iHP`, else its role's.
    /// Units without a role (ships, air units: never in a land engagement) answer the duel's
    /// `MAX_HIT_POINTS`.
    pub fn unit_engage_max_hp(&self, u: UnitId) -> i32 {
        let hp_override = self.d.units[self.units[u].unit_type].hp_override;
        if hp_override > 0 {
            return hp_override;
        }
        match self.unit_combat_role(u) {
            Some(role) => self.d.combat_roles[role].max_hp,
            None => self.d.defines.max_hit_points,
        }
    }

    // A unit's values in a formation battle (docs/ENGAGEMENTS.md, "Per-unit values"): its type's
    // own, else its role's (the unit's own role: `unit_combat_role`); `GameData::engage_*`. The
    // resolver's fighters, the auto formation, the preview, the view and the client read these.

    /// Unit `u`'s speed in a formation battle before first strikes: its type's `iEngageSpeed`,
    /// else its role's.
    pub fn unit_engage_speed(&self, u: UnitId) -> i32 {
        self.d.engage_speed(self.units[u].unit_type, self.unit_combat_role(u))
    }

    /// Unit `u`'s dodge (points of miss chance on physical actions against it): its type's
    /// `iEngageDodge`, else its role's.
    pub fn unit_engage_dodge(&self, u: UnitId) -> i32 {
        self.d.engage_dodge(self.units[u].unit_type, self.unit_combat_role(u))
    }

    /// Unit `u`'s crit bonus (points on its own crit chance): its type's `iEngageCritBonus`, else
    /// its role's.
    pub fn unit_engage_crit_bonus(&self, u: UnitId) -> i32 {
        self.d.engage_crit_bonus(self.units[u].unit_type, self.unit_combat_role(u))
    }

    /// What unit `u` does from each row before its extra actions: its type's `EngageRows`, else
    /// its role's rows.
    pub fn unit_engage_rows(&self, u: UnitId) -> [RowActions; ROWS] {
        self.d.engage_rows(self.units[u].unit_type, self.unit_combat_role(u))
    }

    /// Unit `u`'s best row (of [`Self::unit_engage_rows`]: the most actions, the first such).
    pub fn unit_engage_best_row(&self, u: UnitId) -> usize {
        self.d.engage_best_row(self.units[u].unit_type, self.unit_combat_role(u))
    }

    /// What unit `u` does from each row in a battle: its rows, with `ENGAGEMENT_EXTRA_ACTIONS`
    /// more in its best row for a type of tier `ENGAGEMENT_EXTRA_ACTION_TIER` or above and for a
    /// hero.
    pub fn unit_engage_actions(&self, u: UnitId) -> [RowActions; ROWS] {
        self.d.engage_actions(self.units[u].unit_type, self.unit_combat_role(u), self.unit_is_hero(u))
    }

    /// Whether unit `u` is a flanker (one of its rows charges): anti-flank units in its reach are
    /// taken first.
    pub fn unit_engage_flanks(&self, u: UnitId) -> bool {
        self.d.engage_flanks(self.units[u].unit_type, self.unit_combat_role(u))
    }

    /// The A of `hitter` (on side `hitter_side`) against `target` and the Dv of `target` against
    /// `hitter`, x100 (plan 4.2; the module comment's table): the pair table's entry.
    pub fn engage_hit(&self, hitter: UnitId, target: UnitId, hitter_side: Side, ctx: &HitCtx) -> Pair {
        let a = self.engage_combat_terms(hitter, Some(target), ctx, hitter_side, true, &mut TermLog::none());
        let dv = self.engage_combat_terms(target, Some(hitter), ctx, hitter_side.other(), false, &mut TermLog::none());
        Pair { a: a.strength(), dv: dv.strength() }
    }

    /// The auto-pick's job strength of an attacker (plan 5.4): its A against the defended plot,
    /// with the terms that do not depend on the target (its own percentage, city attack, terrain
    /// attack, amphibious), x100. A plot with a city or a fort counts as a city.
    pub fn engage_job_attack(&self, u: UnitId, ctx: &HitCtx) -> i32 {
        self.engage_combat_terms(u, None, ctx, Side::Attacker, true, &mut TermLog::none()).strength()
    }

    /// The auto-pick's job strength of a defender (plan 5.4): its Dv on the defended plot against
    /// a hitter not known yet (its own percentage, the plot's defense with the city's buildings,
    /// fortification, city defense, its terrain defense), x100.
    pub fn engage_job_defense(&self, u: UnitId, ctx: &HitCtx) -> i32 {
        self.engage_combat_terms(u, None, ctx, Side::Defender, false, &mut TermLog::none()).strength()
    }

    /// A stat with its parts (the battle log's `Stat`): `u`'s A (`attack`) or Dv on side `side`
    /// against `opponent` (`None`: an opponent not known yet, the card's headline). Its terms sum
    /// to the modifier; the own percentage is listed by promotion (the rest as `Own`).
    pub fn engage_stat(&self, u: UnitId, opponent: Option<UnitId>, side: Side, attack: bool, ctx: &HitCtx) -> Stat {
        let mut terms = Vec::new();
        let t = self.engage_combat_terms(u, opponent, ctx, side, attack, &mut TermLog { details: None, terms: Some(&mut terms) });
        Stat { base_x100: t.base, role_pct: self.engage_role_pct(u, attack), terms, total_x100: t.strength() }
    }

    /// Phase 7 hook (plan 14): the percentages an army commander's aura adds to the A and Dv of
    /// army `a`'s units. None in phase 6.
    pub fn army_aura_pct(&self, _a: ArmyId) -> (i32, i32) {
        (0, 0)
    }

    /// The role's A or Dv percent of the unit (100 without a role).
    fn engage_role_pct(&self, u: UnitId, attack: bool) -> i32 {
        match self.unit_combat_role(u) {
            Some(r) if attack => self.d.combat_roles[r].attack_percent,
            Some(r) => self.d.combat_roles[r].defense_percent,
            None => 100,
        }
    }

    /// The base of the unit's A (`attack`) or Dv, x100 (plan 4.2): its type's `iCombat`
    /// (`iCombatDefense`) times the role's percent, or the type's `iAttack` (`iDefense`); plus,
    /// times the role's percent, the unit's own change to that strength (promotions' flat
    /// strength, FfH's Python) and its typed strength, each type's points scaled by the
    /// opponent's resistance to it (Q2).
    fn engage_base_x100(&self, u: UnitId, opponent: Option<UnitId>, attack: bool) -> i32 {
        let unit = &self.units[u];
        let info = &self.d.units[unit.unit_type];
        let role_pct = self.engage_role_pct(u, attack);
        let (type_str, own_str, fixed) = if attack {
            (info.combat, unit.base_combat, info.attack_override)
        } else {
            (info.combat_defense, unit.base_combat_defense, info.defense_override)
        };
        let fixed = if fixed > 0 { fixed * 100 } else { type_str * role_pct };
        let mut typed = 0;
        for damage_type in self.d.damages.keys() {
            let points = unit.damage_type_combat[damage_type];
            if points != 0 {
                let resist = opponent.map_or(0, |o| self.unit_get_damage_type_resist(o, damage_type));
                typed += points * (100 - resist);
            }
        }
        fixed + ((own_str - type_str) * 100 + typed) * role_pct / 100
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
        let d = self.d;
        log.reset(self, u);
        let unit = &self.units[u];
        let base = self.engage_base_x100(u, opponent, attack);
        let mut modifier = 0;

        // The unit's own percentage: its promotions' iCombatPercent on A, iCombatPercentDefense on
        // Dv (Q1); Homeland where it fights; Stigmata.
        let own = if attack { unit.extra_combat_percent } else { unit.extra_combat_percent_defense };
        let fights_on = match side {
            Side::Attacker => ctx.from.unwrap_or_else(|| self.unit_plot(u)),
            Side::Defender => ctx.to,
        };
        let in_borders = if self.map.plots[fights_on].owner == Some(unit.owner) { unit.combat_percent_in_borders } else { 0 };
        log.set(|det| det.extra_combat_percent = own + in_borders);
        if log.listing() {
            self.engage_own_percent_terms(u, attack, own, log);
        }
        modifier += own;
        modifier += log.push(in_borders, || TermSource::InBorders);
        modifier += log.push(self.unit_global_counter_modifier(u), || TermSource::GlobalCounter);

        if attack {
            // The attacker's bonuses against the plot it attacks (G1: none for a defender hitting
            // back); no river term (Q9: the river costs slots instead).
            if side == Side::Attacker {
                let city = match opponent {
                    Some(t) => self.plot_is_city(ctx.to, true, Some(self.unit_get_team(t))),
                    None => self.plot_is_city(ctx.to, true, None),
                };
                let (plot_terms, terrain_term) = self.combat_attack_plot_terms(u, ctx.to, city, 1, log);
                modifier += plot_terms + terrain_term;
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
            // The defended plot's terms (G1: none for an attacker being hit back).
            if side == Side::Defender {
                let team = self.unit_get_team(u);
                let ignore_building = opponent.is_some_and(|h| self.unit_ignore_building_defense(h));
                modifier += self.combat_plot_defense_terms(u, ctx.to, team, ignore_building, ctx.city_defense, log);
                if let Some(h) = opponent {
                    if self.unit_is_barbarian(h) && self.plot_is_city(ctx.to, true, Some(team)) {
                        modifier += term!(log, city_barbarian_defense_modifier, d.defines.city_barbarian_defense_modifier, TermSource::VsBarbarian);
                    }
                }
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

    /// The breakdown of the unit's own percentage `total` (A's or Dv's, Q1): each promotion it
    /// holds with one, the rest (spells, FfH's Python) as `Own`.
    fn engage_own_percent_terms(&self, u: UnitId, attack: bool, total: i32, log: &mut TermLog) {
        let held = &self.units[u].has_promotion;
        let mut rest = total;
        for p in self.d.promotions.keys() {
            if held[p] {
                let info = &self.d.promotions[p];
                let pct = if attack { info.combat_percent } else { info.combat_percent_defense };
                rest -= log.push(pct, || TermSource::Promotion(p));
            }
        }
        log.push(rest, || TermSource::Own);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::{CombatDetails, StrMode};

    /// Over many units, opponents and plots (with and without a planned bombard): the breakdown's
    /// parts sum to the modifier and give the pair table's value, and `unit_combat_terms` in the
    /// engagement modes is the same computation.
    #[test]
    fn breakdowns_sum_to_the_stats_on_every_pair() {
        let (g, units, plots) = crate::test_world::world();
        let mut pairs = 0;
        for (i, &h) in units.iter().enumerate() {
            let to = plots[(i * 7) % plots.len()];
            for city_defense in [None, Some(0), Some(30)] {
                let ctx = HitCtx { to, from: Some(g.unit_plot(h)), city_defense };
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
