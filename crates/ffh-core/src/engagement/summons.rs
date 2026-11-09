//! A caster's summons in its battles (the owner's decisions, 2026-11-05; docs/ENGAGEMENTS.md,
//! "Summons"). A temporary summon spell (`SpellInfo::battle_summon`: Summon Air Elemental, Summon
//! Einherjar, Create Puppet, ...) is never cast on the map: its unit is an extra unit of each
//! battle its caster fights in, on either side, and exists only for that battle (no duration on
//! the map).
//!
//! - **Who** ([`Game::unit_battle_summons`]): every committed unit that knows a battle summon (its
//!   prerequisites as for its battle spells, the spell's civilization and its Python requirement:
//!   the Sand Lion's desert) calls its strongest one
//!   ([`ffh_data::GameData::battle_summons`]), twice with Twincast. A summon calls nothing.
//! - **How many and where** ([`Game::engage_summon_calls`]): each side calls at most
//!   `ENGAGEMENT_SUMMON_EXTRA_SLOTS` slots of summons (a large one counts 2) beyond the units it
//!   commits, the strongest summons first (then the caster's place in the engagement's order),
//!   each in a free cell of its side's grid: the rows of its rank order, the lanes nearest its
//!   caster's first (then the lower). A summon with no free cell is not called. Deterministic, no
//!   RNG; the defender's summons are planned on its auto formation, as the battle forms it.
//! - **The units** ([`Game::engage_call_summons`]): each summon is a real unit made on its
//!   caster's plot as the spell would make it (`Game::unit_init_summon`: its summoner, its
//!   duration, the caster's summon perks or promotions, the spell's promotion, the Summoner's
//!   Bond), marked [`crate::unit::Unit::battle_summon`] (no pool or defence picks it), and
//!   committed to the battle like any unit (the morale check, the opening, XP for none: it shares
//!   no XP).
//! - **After the battle** ([`Game::engage_dismiss_summons`]): the summons that did not fall leave
//!   the game once the kills' effects are made, before the result: a plot whose last defenders
//!   were summons falls.

use ffh_data::*;

use super::arrange::Place;
use super::formation::Side;
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;
use crate::unit::SummonHow;

/// A unit a battle calls ([`Game::engage_summon_calls`]): a summon of a caster, or one a kill may
/// create; its side and its cell.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SummonCall {
    /// The caster (a summon), or the unit whose kills create it (a unit created on a kill whose
    /// creator is itself a summon: that summon's caster).
    pub caster: UnitId,
    pub what: Called,
    pub unit_type: UnitType,
    pub side: Side,
    pub lane: u8,
    pub row: u8,
}

/// What a [`SummonCall`] calls.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Called {
    /// A caster's summon, by its spell.
    Summon(SpellType),
    /// A unit a kill may create (`UnitCreateFromCombat`), off the field until one does: of the
    /// unit `caster` itself (`None`), or of the summon of call `Some(k)`.
    OnKill(Option<usize>),
}

/// A unit a battle called, made ([`Game::engage_call_summons`]): its cell, side and (a unit a
/// kill creates) its creator.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CalledUnit {
    pub place: Place,
    pub side: Side,
    pub spawner: Option<UnitId>,
}

/// Units a kill may create, at most, per creator and battle.
const ON_KILL_MAX: usize = 2;

impl Game {
    /// The battle summons unit `u` calls into its battles (module comment): its strongest battle
    /// summon, twice with Twincast; none for a summon called into a battle.
    pub fn unit_battle_summons(&self, u: UnitId) -> Vec<SpellType> {
        let unit = &self.units[u];
        if unit.battle_summon {
            return Vec::new();
        }
        let civ = self.player_get_civilization_type(unit.owner);
        let known = self.d.battle_summons(unit.unit_type, |p| unit.has_promotion[p], unit.religion, civ);
        // The spell's Python requirement as on the map (the Sand Lion's desert under its caster).
        let best = known.into_iter().find(|&s| self.d.spells[s].py_requirement.is_empty() || self.unit_plot_opt(u).is_some() && self.py_can_cast(u, s));
        match best {
            Some(s) => vec![s; 1 + self.unit_is_twincast(u) as usize],
            None => Vec::new(),
        }
    }

    /// The units the battle of `att` (the attacker's formation) against `candidates` on `ctx`
    /// calls (module comment), in the order they are called: each side's summons, then the units
    /// its kills may create. `sentinels`: as [`Game::engage_setup`]'s.
    pub(crate) fn engage_summon_calls(&self, ctx: &HitCtx, slots: &Slots, att: &[Place], candidates: &[UnitId], sentinels: bool) -> Vec<SummonCall> {
        let Ok((slots, _, def)) = self.engage_def_layout(ctx, slots.clone(), candidates, sentinels) else { return Vec::new() };
        let lanes = self.engage_lanes(&slots);
        let mut out: Vec<SummonCall> = Vec::new();
        for (side, formation, enemies) in [(Side::Attacker, att, def.len()), (Side::Defender, &def[..], att.len())] {
            let mut sorted: Vec<Place> = formation.to_vec();
            sorted.sort_by_key(|p| (p.lane, p.row));
            let n = lanes[side.index()];
            let mut taken: Vec<(u8, u8)> = Vec::new();
            for p in &sorted {
                for l in p.lane..=p.lane + self.unit_engage_large(p.unit) as u8 {
                    taken.push((l, p.row));
                }
            }
            if side == Side::Defender {
                taken.extend(self.engage_sentinel_cells(&slots));
            }
            // A free cell for a unit of type `ty` near lane `near`: the rows of its rank order, the
            // lanes nearest `near` first, then the lower.
            let cell_for = |ty: UnitType, near: u8, taken: &[(u8, u8)]| {
                let large = self.d.units[ty].large;
                let free = |l: u8, r: u8| l < n && !taken.contains(&(l, r)) && (!large || (l + 1 < n && !taken.contains(&(l + 1, r))));
                let mut lanes_near: Vec<u8> = (0..n).collect();
                lanes_near.sort_by_key(|&l| ((l as i32 - near as i32).abs(), l));
                self.d.engage_row_order(ty).iter().map(|&r| r.min(ROWS - 1) as u8).flat_map(|r| lanes_near.iter().map(move |&l| (l, r))).find(|&(l, r)| free(l, r))
            };
            let take = |taken: &mut Vec<(u8, u8)>, ty: UnitType, (l, r): (u8, u8)| {
                taken.push((l, r));
                if self.d.units[ty].large {
                    taken.push((l + 1, r));
                }
            };
            // The summons: the strongest first, within the extra slots.
            let mut wants: Vec<(usize, Place, SpellType)> = Vec::new();
            for (k, p) in sorted.iter().enumerate() {
                for s in self.unit_battle_summons(p.unit) {
                    wants.push((k, *p, s));
                }
            }
            wants.sort_by_key(|&(k, _, s)| (std::cmp::Reverse(self.d.battle_summon_strength(s)), k));
            let mut budget = self.d.defines.engagement_summon_extra_slots;
            let first = out.len();
            for (_, caster, spell) in wants {
                let Some(ty) = self.d.spells[spell].create_unit_type else { continue };
                let size = 1 + self.d.units[ty].large as i32;
                if size > budget {
                    continue;
                }
                let Some((lane, row)) = cell_for(ty, caster.lane, &taken) else { continue };
                take(&mut taken, ty, (lane, row));
                budget -= size;
                out.push(SummonCall { caster: caster.unit, what: Called::Summon(spell), unit_type: ty, side, lane, row });
            }
            // The units the side's kills may create: up to `ON_KILL_MAX` per creator (no more than
            // the enemies), each with a free cell; the formation's creators, then the summons.
            let creators: Vec<(UnitType, UnitId, u8, Option<usize>)> = sorted
                .iter()
                .map(|p| (self.units[p.unit].unit_type, p.unit, p.lane, None))
                .chain((first..out.len()).map(|k| (out[k].unit_type, out[k].caster, out[k].lane, Some(k))))
                .collect();
            for (ty, who, lane_at, of) in creators {
                let Some(made) = self.d.units[ty].unit_create_from_combat else { continue };
                if !self.type_engages_on_land_and_fights(made) {
                    continue;
                }
                for _ in 0..ON_KILL_MAX.min(enemies) {
                    let Some((lane, row)) = cell_for(made, lane_at, &taken) else { break };
                    take(&mut taken, made, (lane, row));
                    out.push(SummonCall { caster: who, what: Called::OnKill(of), unit_type: made, side, lane, row });
                }
            }
        }
        out
    }

    /// Whether units of type `t` fight land battles: a land type with a stat sheet.
    fn type_engages_on_land_and_fights(&self, t: UnitType) -> bool {
        self.d.type_engages_on_land(t) && self.d.units[t].stats.is_some()
    }

    /// The units of `calls` made (module comment), each in its cell of its side: a summon as its
    /// spell makes it, a unit a kill may create as the kill would (its creator's duration and
    /// hidden nationality), each marked a battle summon until the battle is over.
    pub(crate) fn engage_call_summons(&mut self, calls: &[SummonCall]) -> Vec<CalledUnit> {
        let d = self.d;
        let mut out: Vec<CalledUnit> = Vec::with_capacity(calls.len());
        let mut made: Vec<Option<UnitId>> = vec![None; calls.len()];
        for (k, c) in calls.iter().enumerate() {
            if !self.units.contains(c.caster) || self.unit_is_dead(c.caster) {
                continue;
            }
            let u = match c.what {
                Called::Summon(spell) => {
                    let info = &d.spells[spell];
                    let how = SummonHow { permanent: false, copy_casters_promotions: info.copy_casters_promotions, ability: info.ability, promotion: info.create_unit_promotion };
                    self.unit_init_summon(c.caster, c.unit_type, how)
                }
                Called::OnKill(of) => {
                    let Some(creator) = (match of {
                        None => Some(c.caster),
                        Some(j) => made[j],
                    }) else {
                        continue;
                    };
                    let (owner, x, y) = (self.units[creator].owner, self.units[creator].x, self.units[creator].y);
                    let u = self.player_init_unit(owner, c.unit_type, x, y, None, None);
                    // As `combatWon` makes it: the creator's duration, its hidden nationality.
                    let duration = self.units[creator].duration;
                    if duration > 0 {
                        self.unit_change_duration(u, duration);
                        if self.plot_is_outside_borders_of(self.unit_plot(u), self.unit_get_team(u)) {
                            // Summons do not count as units outside the borders (undoing the
                            // count `unit_set_xy` made).
                            self.players[owner].num_outside_units -= 1;
                        }
                    }
                    if self.unit_is_hidden_nationality(creator) {
                        if let Some(hidden) = opt_from_i32::<PromotionType>(d.defines.hidden_nationality_promotion) {
                            self.unit_set_has_promotion(u, hidden, true);
                        }
                    }
                    self.units[u].battle_summon = true;
                    out.push(CalledUnit { place: Place { unit: u, lane: c.lane, row: c.row }, side: c.side, spawner: Some(creator) });
                    made[k] = Some(u);
                    continue;
                }
            };
            self.units[u].battle_summon = true;
            made[k] = Some(u);
            out.push(CalledUnit { place: Place { unit: u, lane: c.lane, row: c.row }, side: c.side, spawner: None });
        }
        out
    }

    /// After the battle (`apply.rs`), the units the battle's kills created that fought: one a
    /// summon created stays the battle's only; one a unit of the map created stays (it is no
    /// battle summon any more).
    pub(crate) fn engage_keep_created(&mut self, created: &[(UnitId, UnitId)]) {
        for &(u, creator) in created {
            if !self.units.contains(u) || self.unit_is_dead(u) {
                continue;
            }
            let summoned = !self.units.contains(creator) || self.units[creator].battle_summon;
            if !summoned {
                self.units[u].battle_summon = false;
            }
        }
    }

    /// After the battle (module comment): the summons among `units` still in the game leave it.
    pub(crate) fn engage_dismiss_summons(&mut self, units: &[UnitId]) {
        for &u in units {
            if self.units.contains(u) && self.units[u].battle_summon {
                self.tel_ctx_push("dismissed");
                self.unit_kill(u, false, None);
                self.tel_ctx_pop();
            }
        }
    }
}
