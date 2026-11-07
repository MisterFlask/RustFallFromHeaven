//! The battle magic of the great-person deck's battle cards on `Game` (docs/CARDS.md, the battle
//! magic batch; the resolver's side is `formation/magic.rs`): what a unit's type and promotions
//! (`ffh_data::BattleMagic`) and its battle spells make of its fighter, the pairs that need it,
//! the battle's illusions, and the post-battle steps of Contingency.
//!
//! - **The fighter** ([`Game::engage_fighter_magic`]): an Evoker's damage spells deal more (the
//!   magic bolt and Wonder's damage outcomes too; Wonder's other outcomes go with its charms and
//!   heals), a Battlemage's hit with its pattern; a Spellsword casts its strike spell; the
//!   Abjurer's wards, a presence (Gravity Well, the Whisperer), the Halo, a Spellbreaker's
//!   dispel and what a unit with dispellable promotions loses to it; Contingency's opening cast
//!   (a defender only: "when my stack is attacked") and its recall (only with a capital to go
//!   to); the illusions a caster casts.
//! - **The set-up** ([`Game::engage_magic_setup`]): a familiar guards the caster it is bound to
//!   (its `summoner`) when both fight on one side; a side facing a Whisperer gets its same-side
//!   pairs (the maddened unit hits its own side); a hitter's damage against summons (a unit with a
//!   duration) is in its pairs.
//! - **The battle** ([`super::engage::Setup::battle`]): the illusions are fighters of the
//!   formation only, after the set-up's (their map entries are past its fighters).
//! - **After the battle** ([`Game::engage_magic_after`], in the post-battle steps where withdrawn
//!   defenders flee): a recalled unit (Contingency's recall: it left the battle in place of a
//!   killing hit) goes to its owner's capital with 1 HP; a defender's opening cast and a recall
//!   use their promotion up.

use ffh_data::*;

use super::arrange::Place;
use super::engage::{damage_from_hp, Eng, Fought, Setup};
use super::formation::{Bare, Fighter, IllusionSpell, Pair, Params, Side, Spell, WonderOutcome};
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;
use crate::unit::apply_combat_modifier;

impl Game {
    /// Unit `u`'s battle magic: its type's and its promotions' (`GameData::unit_battle_magic`).
    pub fn unit_battle_magic(&self, u: UnitId) -> BattleMagic {
        let unit = &self.units[u];
        self.d.unit_battle_magic(unit.unit_type, |p| unit.has_promotion[p])
    }

    /// Unit `u`'s battle magic on its fighter `f` of side `side` (module comment).
    pub(crate) fn engage_fighter_magic(&self, u: UnitId, side: Side, f: &mut Fighter, ctx: &HitCtx, params: &Params) {
        let m = self.unit_battle_magic(u);
        if !m.any() && !self.unit_has_dispellable(u) {
            return;
        }
        let casts = f.actions.iter().any(|r| r.kind == ActionKind::Spell);
        // Spellsword: its strike spell, with half its attack as its magic (`engage_magic_x100`).
        if let Some(s) = m.strike_spell {
            let si = &self.d.spells[s];
            f.spell = Spell::battle(s.idx() as u32, si.battle_power_or_default(), si.battle_pattern);
            f.magic.strike_spell = true;
        }
        // Evoker: its damage spells deal more; it casts no charm, hex or heal (its battle spells
        // have none: `GameData::engage_battle_spells`), nor Wonder's other outcomes.
        if m.spell_power != 0 {
            let more = |p: i32| p * (100 + m.spell_power).max(0) / 100;
            f.spell.power = more(f.spell.power);
            for (_, o) in f.wonder.iter_mut() {
                if let WonderOutcome::Damage(sp) = o {
                    sp.power = more(sp.power);
                }
            }
        }
        if m.damage_only {
            f.wonder.retain(|(_, o)| matches!(o, WonderOutcome::Damage(_)));
        }
        // Battlemage: its damage spells' pattern.
        if let Some(p) = m.spell_pattern {
            f.spell.pattern = Some(p);
            f.spell.splash = false;
            for (_, o) in f.wonder.iter_mut() {
                if let WonderOutcome::Damage(sp) = o {
                    sp.pattern = Some(p);
                    sp.splash = false;
                }
            }
        }
        f.magic.hex_ward = m.hex_ward;
        f.magic.end_charms = m.end_charms;
        f.magic.presence = m.presence;
        f.magic.revive = m.revive;
        f.magic.dispel = m.dispel;
        f.magic.damage_vs_summons = m.damage_vs_summons;
        f.magic.opening_cast = m.opening_cast && side == Side::Defender && casts;
        f.magic.recall = m.death_recall && self.player_get_capital_city(self.units[u].owner).is_some();
        f.magic.bare = self.engage_bare(u, side, ctx, params);
        if casts {
            if let Some(s) = self.unit_engage_battle_spells(u).illusions {
                if let Some(i) = self.d.spells[s].battle_illusions {
                    f.magic.illusions = Some(IllusionSpell { id: s.idx() as u32, count: i.count, hp: i.hp });
                }
            }
        }
    }

    /// Whether unit `u` holds a dispellable promotion.
    fn unit_has_dispellable(&self, u: UnitId) -> bool {
        let unit = &self.units[u];
        self.d.promotions.keys().any(|p| unit.has_promotion[p] && self.d.promotions[p].dispellable)
    }

    /// What unit `u` (on side `side` of `ctx`) loses while dispelled (Spellbreaker): its A, Dv
    /// and magic defence against nobody in particular without its dispellable promotions' attack,
    /// defence, magic defence (points and percentages) and typed strength, in percent of them with
    /// those promotions, and the speed of their first strikes. `None` without one.
    fn engage_bare(&self, u: UnitId, side: Side, ctx: &HitCtx, params: &Params) -> Option<Bare> {
        let d = self.d;
        let unit = &self.units[u];
        let held: Vec<PromotionType> = d.promotions.keys().filter(|&p| unit.has_promotion[p] && d.promotions[p].dispellable).collect();
        if held.is_empty() {
            return None;
        }
        let sum = |f: &dyn Fn(&PromotionInfo) -> i32| held.iter().map(|&p| f(&d.promotions[p])).sum::<i32>();
        let stat = |s: StatType| (sum(&|i| i.stat_changes[s.idx()].flat), sum(&|i| i.stat_changes[s.idx()].percent));
        let typed = sum(&|i| i.damage_type_combat.iter().sum::<i32>());
        let pct = |with: i32, without: i32| (without.max(1) as i64 * 100 / with.max(1) as i64) as i32;
        let (fa, pa) = stat(StatType::Attack);
        let (fd, pd) = stat(StatType::Defense);
        let (fm, pm) = stat(StatType::MagicDefense);
        let a = self.engage_combat_terms(u, None, ctx, side, true, &mut crate::unit::TermLog::none());
        let dv = self.engage_combat_terms(u, None, ctx, side, false, &mut crate::unit::TermLog::none());
        let a_bare = crate::unit::CombatTerms { base: a.base - fa * 10 - typed * 100, modifier: a.modifier - pa }.strength();
        let dv_bare = crate::unit::CombatTerms { base: dv.base - fd * 10, modifier: dv.modifier - pd }.strength();
        let md_with = self.engage_magic_x100(u, false);
        let md_base = d.units[unit.unit_type].stats.map_or(0, |sheet| sheet[StatType::MagicDefense.idx()]) + unit.stat_flat[StatType::MagicDefense.idx()] - fm;
        let md_bare = 1.max(apply_combat_modifier(md_base * 10, unit.stat_pct[StatType::MagicDefense.idx()] - pm));
        Some(Bare {
            a_pct: pct(a.strength(), a_bare),
            dv_pct: pct(dv.strength(), dv_bare),
            md_pct: pct(md_with, md_bare),
            speed: -params.first_strike_speed * sum(&|i| i.first_strikes_change).max(0),
        })
    }

    /// The set-up's battle magic (module comment) on its fighters `fighters` and pairs `pairs`
    /// (`n * n`) of the committed `units` (by set-up index, the attacker's first), each on its
    /// side.
    pub(crate) fn engage_magic_setup(&self, units: &[UnitId], sides: &[Side], fighters: &mut [Fighter], pairs: &mut [Pair], ctx: &HitCtx) {
        let n = fighters.len();
        let nu = units.len();
        // A familiar guards the caster it is bound to, on its side.
        for i in 0..nu {
            if !self.d.units[self.units[units[i]].unit_type].magic.familiar {
                continue;
            }
            let Some(c) = self.units[units[i]].summoner else { continue };
            if let Some(j) = (0..nu).find(|&j| units[j] == c && sides[j] == sides[i]) {
                fighters[i].magic.guards = Some(j as u16);
            }
        }
        // A side facing a Whisperer: its same-side pairs (the maddened one hits its own side).
        for side in [Side::Attacker, Side::Defender] {
            let faces = (0..nu).any(|i| sides[i] != side && fighters[i].magic.presence == Some(Presence::Madness));
            if !faces {
                continue;
            }
            for h in (0..nu).filter(|&h| sides[h] == side) {
                for t in (0..nu).filter(|&t| t != h && sides[t] == side) {
                    pairs[h * n + t] = self.engage_hit_ally(units[h], units[t], side, ctx);
                }
            }
        }
        // Damage against summons (a unit with a duration).
        for h in 0..nu {
            let more = fighters[h].magic.damage_vs_summons;
            if more == 0 {
                continue;
            }
            for t in 0..nu {
                if sides[t] != sides[h] && self.units[units[t]].duration > 0 {
                    let p = &mut pairs[h * n + t];
                    p.pct = (p.pct as i64 * (100 + more).max(0) as i64 / 100) as i32;
                }
            }
        }
    }

    /// The pair of `hitter` hitting `target`, both of side `side`: the hitter's A against it, the
    /// target's Dv against the hitter, their magic, the spell's and the pair's percentages.
    fn engage_hit_ally(&self, hitter: UnitId, target: UnitId, side: Side, ctx: &HitCtx) -> Pair {
        let a = self.engage_combat_terms(hitter, Some(target), ctx, side, true, &mut crate::unit::TermLog::none());
        let dv = self.engage_combat_terms(target, Some(hitter), ctx, side, false, &mut crate::unit::TermLog::none());
        Pair {
            a: a.strength(),
            dv: dv.strength(),
            ma: self.engage_magic_x100(hitter, true),
            md: self.engage_magic_x100(target, false),
            spell_pct: self.engage_spell_pct(hitter, Some(target)),
            pct: self.engage_pair_pct(hitter, Some(target)),
        }
    }

    /// After the battle (module comment), where withdrawn defenders flee: recalled units go to
    /// their capital with 1 HP and lose their recall; defenders that fought lose their opening
    /// cast.
    pub(crate) fn engage_magic_after(&mut self, e: &Eng, r: &Fought) {
        let d = self.d;
        let turn = self.game_turn();
        for i in 0..e.units.len() {
            let u = e.units[i];
            if !self.units.contains(u) || self.unit_is_dead(u) {
                continue;
            }
            let fm = &e.setup.fighters[i].magic;
            let (recalled, opened) = (fm.recall && r.withdrew(i), fm.opening_cast && !e.sat_out[i]);
            if recalled {
                let owner = self.units[u].owner;
                if let Some(cap) = self.player_get_capital_city(owner) {
                    let (x, y) = (self.cities[cap].x, self.cities[cap].y);
                    self.unit_join_group(u, None, true);
                    self.unit_set_xy(u, x, y, false);
                    let max = self.unit_engage_max_hp(u);
                    self.unit_set_damage(u, damage_from_hp(100, max), None);
                    let name = d.text.name(&d.units[self.units[u].unit_type].base.description).to_string();
                    self.log.msg(turn, Some(owner), || format!("{name} escapes death and returns to the capital"));
                }
            }
            for p in d.promotions.keys() {
                if !self.units.contains(u) || !self.units[u].has_promotion[p] {
                    continue;
                }
                let pm = &d.promotions[p].magic;
                if (recalled && pm.death_recall) || (opened && pm.opening_cast) {
                    self.unit_set_has_promotion(u, p, false);
                }
            }
        }
    }

    /// The auto formation's familiars in their casters' lanes: a familiar whose bound caster
    /// (its `summoner`) is in `out` and shares no lane with it moves to a free cell of the
    /// caster's lane, in its own rank order; it stays where it is when there is none. `blocked`:
    /// cells already taken (a city's Sentinels).
    pub(crate) fn engage_familiar_places(&self, out: &mut [Place], lanes: u8, blocked: &[(u8, u8)]) {
        for k in 0..out.len() {
            let u = out[k].unit;
            if !self.d.units[self.units[u].unit_type].magic.familiar {
                continue;
            }
            let Some(c) = self.units[u].summoner.and_then(|c| out.iter().find(|p| p.unit == c).copied()) else { continue };
            let c_lanes = c.lane..=c.lane + self.unit_engage_large(c.unit) as u8;
            if c_lanes.contains(&out[k].lane) || self.unit_engage_large(u) {
                continue;
            }
            let taken = |l: u8, r: u8, out: &[Place]| {
                blocked.contains(&(l, r)) || out.iter().any(|p| p.unit != u && p.row == r && (p.lane..=p.lane + self.unit_engage_large(p.unit) as u8).contains(&l))
            };
            let cell = self.unit_engage_row_order(u).iter().map(|&r| r.min(ffh_data::ROWS - 1) as u8).flat_map(|r| c_lanes.clone().map(move |l| (l, r))).find(|&(l, r)| l < lanes && !taken(l, r, out));
            if let Some((l, r)) = cell {
                out[k].lane = l;
                out[k].row = r;
            }
        }
    }

    /// Whether committed unit `i` of `e` was recalled (Contingency: it left the battle in place of
    /// a killing hit).
    pub(crate) fn engage_recalled(&self, e: &Eng, r: &Fought, i: usize) -> bool {
        e.setup.fighters[i].magic.recall && r.withdrew(i)
    }
}

impl Setup {
    /// Card: the illusions of the battle's fighters (module comment), appended to `fighters` with
    /// their map entries past the set-up's fighters; and the familiars' guards from set-up indices
    /// to the battle's.
    pub(crate) fn battle_magic(&self, fighters: &mut Vec<Fighter>, map: &mut Vec<usize>) -> Vec<usize> {
        for f in fighters.iter_mut() {
            f.magic.guards = f.magic.guards.and_then(|g| map.iter().position(|&m| m == g as usize)).map(|k| k as u16);
        }
        let casters = super::formation::add_illusions(fighters, self.lanes);
        let n = self.fighters.len();
        for j in 0..casters.len() {
            map.push(n + j);
        }
        casters
    }

    /// Card: the battle's pair of fighters `h` and `t` (battle indices; `map` their set-up
    /// indices, the illusions from `base` on, each of caster `casters[t - base]`): an illusion
    /// never hits; one is hit as its caster is, its hitter's damage against summons added.
    pub(crate) fn battle_pair(&self, map: &[usize], casters: &[usize], base: usize, h: usize, t: usize) -> Pair {
        let n = self.fighters.len();
        if h >= base {
            return Pair::default();
        }
        if t >= base {
            let k = casters[t - base];
            let mut p = self.pairs[map[h] * n + map[k]];
            let more = self.fighters[map[h]].magic.damage_vs_summons;
            p.pct = (p.pct as i64 * (100 + more).max(0) as i64 / 100) as i32;
            return p;
        }
        self.pairs[map[h] * n + map[t]]
    }
}
