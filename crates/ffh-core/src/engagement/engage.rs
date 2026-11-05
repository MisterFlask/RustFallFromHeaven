//! The engagement on `Game` (spec 1.7 to 1.9; docs/ENGAGEMENTS.md, "The engagement"): the set-up
//! from the units onto the formation resolver, the morale check, the opening's defensive strikes,
//! the battle on the game's RNG, then the post-battle steps ([`super::apply`]) and the report.
//! Reached only through `group_attack_force` (every order of a group into an enemy land plot under
//! the option) and `unit_attack` (Taunt, the Shrine of Camulos, Gift of Kylorin: a one-unit
//! force).
//!
//! **Order** (its RNG draws in this order, each with its label):
//! 1. *Set-up*: slots and lanes, the attacker's pool and formation (the order's, checked, or the
//!    rules' auto formation), the defender's candidates and its formation (the auto formation).
//!    Nothing has changed yet: an empty formation or no defender is an `Err`. The [`Setup`] holds
//!    the committed units in the engagement's order (the attacker's by lane then row, then the
//!    defender's), each one as a resolver fighter and the pair table of A and Dv
//!    ([`Game::engage_hit`], which nothing before the rounds changes). Phase 7's pre-battle hook
//!    ([`Game::engage_pre_battle`]) reads it.
//! 2. *Morale* (Q8): a committed Fear defender (the strongest by base strength, level, then the
//!    lowest id) makes each committed attacker not immune roll today's check
//!    (`rand(40) > max(4, str + 20 + level - str(src) - level(src))`, "Im afeared!"): one that
//!    fails sits out (its attack is spent, its cell stays empty); then a Fear attacker that did not
//!    sit out does the same to the committed defenders, which stay on the plot but do not fight.
//! 3. *Opening*: defensive strikes (Q5): for each committed attacker that did not sit out, in the
//!    engagement's order and not immune, every unit on the plot with a strike chance, at war and
//!    not used up (committed or not) rolls, and the best success strikes (today's damage, never
//!    past 95%). It never kills. The round model's collateral step is gone: units with collateral
//!    damage splash their hits in the battle instead. Then phase 7's opening hook
//!    ([`Game::engage_opening_spells`]).
//! 4. *The battle*: the committed units that did not sit out, with their HP after the opening,
//!    fought by the formation resolver ([`formation::resolve`]) on `soren_rand`: per battle the
//!    chance first strikes; per pass the coins; per action the miss, the crit, r, a withdrawal,
//!    the splash's r's and withdrawals (docs/ENGAGEMENTS.md, "RNG").
//! 5. *Post-battle* ([`super::apply`], with phase 7's post-battle hook,
//!    [`Game::engage_post_battle_spells`]) and the report ([`super::report`]), telemetry. The plot
//!    falls only when no defender is left on it: then the launching group moves in.
//!
//! **Phase 7 hooks** (plan 6.2, 14): the three hooks are empty in phase 6. Each reads the set-up
//! and returns [`BattleEffect`]s; none writes `Game` or draws a random number, so the preview can
//! call the first two and see the same in-battle spells. A damage effect lowers its fighter's HP
//! before the battle, a modifier scales its pairs' A and Dv; the writes a cast makes to `Game`
//! come after the battle, through its `Cast` record.
//!
//! It never sets `combat_unit`, `attack_plot` or the combat timers, so `plot_is_fighting` stays
//! false and nothing is left between calls (plan 6.9).
//!
//! **Missiles** (Q11): a missile (`bSuicide`: Fireball, Meteor) ordered into an enemy plot does
//! not start an engagement: [`Game::engage_missile_strike`] hits one unit of the defender's
//! formation (`formation::missile_hit`: a strike's reach and choice, from outside the field, at
//! the strength it had before formation battles: `ENGAGEMENT_MISSILE_PERCENT` of a hit, the
//! plot's own defence in the target's Dv, no ground rules), then its collateral (today's), and
//! the missile dies.
//!
//! **The battle log** is written through an optional sink ([`super::sink`]) for a battle a human
//! could see; it draws no random number and changes no state (test `the_sink_changes_nothing`).

use ffh_data::*;

use super::apply::Post;
use super::arrange::Place;
use super::formation::{self, missile_hit, Fate, Fighter, Formation, Ground, Outcome, Pair, Params, Side, Spell};
use super::pick::WarTest;
use super::report::{BattleLog, BattleReport, CitySnap, EngageResult, LogFate, Outcome as ReportOutcome, PlotSnap, ReportSide};
use super::sink::Sink;
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;
use crate::unit::{DefStrikeHit, DefStrikeRoll, UnitOverlay};

/// HP in centi-HP from the damage field (plan 4.1): `(100 - damage) x max_hp`.
pub fn hp_from_damage(damage: i32, max_hp: i32) -> i32 {
    (100 - damage.clamp(0, 100)) * max_hp
}

/// The damage field from centi-HP (plan 4.1): `100 - ceil(hp / max_hp)`, so a fighter with HP
/// left never shows 100; 100 when `hp <= 0`.
pub fn damage_from_hp(hp: i32, max_hp: i32) -> i32 {
    if hp <= 0 || max_hp <= 0 {
        return 100;
    }
    let left = (hp + max_hp - 1) / max_hp;
    (100 - left).clamp(0, 99)
}

/// An order to fight an engagement (`group_attack_force`, `unit_attack`).
#[derive(Clone, Debug)]
pub(crate) struct EngageOrder {
    /// The attacking player and the plots.
    pub p: PlayerId,
    pub from: PlotIdx,
    pub to: PlotIdx,
    /// The group that gave the order: it advances when the plot falls.
    pub launcher: Option<GroupId>,
    /// The attacker's formation (checked against the pool and the slots); `None`: the rules' auto
    /// formation. Not saved: an order lives for the call.
    pub formation: Option<Vec<Place>>,
    /// A forced attack (`unit_attack`: Taunt, events): these units, auto-arranged, whatever their
    /// moves and attacks this turn (as today's `attack`); `formation` is not read.
    pub forced: Option<Vec<UnitId>>,
    /// Write the battle log: `None` decides by whether a human could see the battle.
    pub log: Option<bool>,
}

/// What an engagement did.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EngageOutcome {
    /// The battle report's id ([`Game::battle_report`]).
    pub report: u32,
    pub result: EngageResult,
    /// The attacker moved in.
    pub plot_taken: bool,
}

/// A phase 7 effect on a battle (plan 14), returned by the hooks; fighters are set-up indices.
/// None is returned in phase 6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BattleEffect {
    /// A combat percentage on the fighter's A (when it hits) and Dv (when it is hit).
    Mod { fighter: usize, a_pct: i32, dv_pct: i32 },
    /// Damage before the battle, in percent of the fighter's max HP (an opening damage spell);
    /// it never kills.
    Damage { fighter: usize, pct: i32 },
    /// A spell cast in the battle (its `SpellType`), for the deferred writes and the report.
    Cast { caster: usize, spell: SpellType },
}

/// An engagement's set-up ([`Game::engage_setup`]): what the real fight and the preview share.
#[derive(Clone, Debug)]
pub(crate) struct Setup {
    pub ctx: HitCtx,
    pub slots: Slots,
    pub lanes: [u8; 2],
    pub ground: Ground,
    pub params: Params,
    /// The defended plot holds a city (no defensive withdrawal, Q3).
    pub in_city: bool,
    /// The defender's candidates (as the attack meets them), and its formation's units.
    pub candidates: Vec<UnitId>,
    /// The committed units in the engagement's order (the attacker's formation by lane then row,
    /// then the defender's), each one's side and cell `(lane, row)`; the number of attackers.
    pub units: Vec<UnitId>,
    pub sides: Vec<Side>,
    pub places: Vec<(u8, u8)>,
    pub n_att: usize,
    /// Each committed unit as a fighter (HP from its damage field at the set-up), then the
    /// defended city's Sentinels (docs/SIEGES.md): `sides`, `places`, `fighters` and `pairs`
    /// cover the units (set-up indices `0..units.len()`) and then the Sentinels
    /// ([`Setup::sentinel_range`]).
    pub fighters: Vec<Fighter>,
    /// `n * n` (`n` the fighters, Sentinels included), `[h * n + t]`: the hitter's A against the
    /// target and the target's Dv against it (cross-side entries only).
    pub pairs: Vec<Pair>,
    /// The city whose Sentinels fight, and which of its Sentinels each Sentinel fighter is.
    pub sentinel_city: Option<CityId>,
    pub sentinels: Vec<usize>,
}

impl Setup {
    /// The Sentinels' set-up indices (after the units).
    pub fn sentinel_range(&self) -> std::ops::Range<usize> {
        self.units.len()..self.fighters.len()
    }

    /// Whether set-up index `i` is a Sentinel.
    pub fn is_sentinel(&self, i: usize) -> bool {
        i >= self.units.len()
    }

    /// The committed units of side `s` (indices; Sentinels not included).
    pub fn of(&self, s: Side) -> std::ops::Range<usize> {
        match s {
            Side::Attacker => 0..self.n_att,
            Side::Defender => self.n_att..self.units.len(),
        }
    }

    /// The candidates the defender does not commit.
    pub fn def_reserve(&self) -> Vec<UnitId> {
        let committed = &self.units[self.n_att..];
        self.candidates.iter().copied().filter(|u| !committed.contains(u)).collect()
    }

    /// The pair of set-up indices `h` (hitting) and `t`.
    pub fn pair(&self, h: usize, t: usize) -> Pair {
        let p = self.pairs[h * self.fighters.len() + t];
        Pair { a: p.a.max(1), dv: p.dv.max(1) }
    }

    /// The side's formation as places.
    pub fn formation(&self, s: Side) -> Vec<Place> {
        self.of(s).map(|i| Place { unit: self.units[i], lane: self.places[i].0, row: self.places[i].1 }).collect()
    }

    /// The battle of the committed units that fight (not `sat_out`, by unit) and the Sentinels,
    /// with HP `hp` (centi-HP, by set-up index): the resolver's formation, and each fighter's
    /// set-up index.
    pub fn battle(&self, sat_out: &[bool], hp: &[i32]) -> (Formation, Vec<usize>) {
        let n = self.fighters.len();
        let map: Vec<usize> = (0..self.units.len()).filter(|&i| !sat_out[i]).chain(self.sentinel_range()).collect();
        let fighters = map
            .iter()
            .map(|&i| {
                let mut f = self.fighters[i].clone();
                f.hp = hp[i];
                f
            })
            .collect();
        let f = Formation::new(self.params, self.ground, self.lanes, fighters, |h, t| self.pairs[map[h] * n + map[t]]);
        debug_assert_eq!(f.validate(), Ok(()), "an engagement's formation off its grid");
        (f, map)
    }

    /// Phase 7's effects applied: damage lowers `hp` (never below 1 HP), a modifier scales the
    /// fighter's A when it hits and its Dv when it is hit.
    pub fn apply_effects(&mut self, effects: &[BattleEffect], hp: &mut [i32]) {
        let n = self.fighters.len();
        for e in effects {
            match *e {
                BattleEffect::Mod { fighter, a_pct, dv_pct } => {
                    for o in 0..n {
                        let ha = &mut self.pairs[fighter * n + o].a;
                        *ha = (*ha as i64 * (100 + a_pct).max(0) as i64 / 100) as i32;
                        let td = &mut self.pairs[o * n + fighter].dv;
                        *td = (*td as i64 * (100 + dv_pct).max(0) as i64 / 100) as i32;
                    }
                }
                BattleEffect::Damage { fighter, pct } => {
                    let max = self.fighters[fighter].max_hp;
                    hp[fighter] = (hp[fighter] - pct.max(0) * max).max(100);
                }
                BattleEffect::Cast { .. } => {}
            }
        }
    }
}

/// A kill of the battle, in set-up indices.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Kill {
    pub victim: usize,
    pub killer: usize,
    pub round: u8,
}

/// The battle as fought, in set-up indices (the resolver's [`Outcome`] mapped back).
#[derive(Clone, Debug)]
pub(crate) struct Fought {
    /// Per committed unit: its fate, HP left (centi-HP; a unit that sat out keeps its HP) and max
    /// HP.
    pub fate: Vec<LogFate>,
    pub hp: Vec<i32>,
    pub max_hp: Vec<i32>,
    /// In kill order.
    pub kills: Vec<Kill>,
    /// `n * n`: damage `h` dealt `t` (centi-HP; battle XP's shares).
    dealt: Vec<i32>,
    pub rounds: i32,
    /// Per side: its line is gone (no unit of it on the field at the end).
    pub down: [bool; 2],
}

impl Fought {
    fn new(setup: &Setup, hp: &[i32], map: &[usize], out: &Outcome) -> Fought {
        // Every fighter of the set-up: the units, then the Sentinels.
        let n = setup.fighters.len();
        let mut fate = vec![LogFate::SatOut; n];
        let mut end_hp = hp.to_vec();
        for (k, e) in out.fighters.iter().enumerate() {
            let i = map[k];
            end_hp[i] = e.hp;
            fate[i] = match e.fate {
                Fate::Standing => LogFate::Standing,
                Fate::Fell { by, round } => LogFate::Fell { by: map[by as usize] as formation::FighterIdx, round },
                Fate::Withdrew { round } => LogFate::Withdrew { round },
            };
        }
        let mut dealt = vec![0; n * n];
        for (hk, &h) in map.iter().enumerate() {
            for (tk, &t) in map.iter().enumerate() {
                dealt[h * n + t] = out.dealt(hk, tk);
            }
        }
        Fought {
            fate,
            hp: end_hp,
            max_hp: setup.fighters.iter().map(|f| f.max_hp).collect(),
            kills: out.kills.iter().map(|k| Kill { victim: map[k.victim as usize], killer: map[k.killer as usize], round: k.round }).collect(),
            dealt,
            rounds: out.rounds as i32,
            down: out.down,
        }
    }

    /// The damage `h` dealt `t` (set-up indices, centi-HP).
    pub fn dealt(&self, h: usize, t: usize) -> i32 {
        self.dealt[h * self.fate.len() + t]
    }

    pub fn killed(&self, i: usize) -> bool {
        matches!(self.fate[i], LogFate::Fell { .. })
    }

    pub fn withdrew(&self, i: usize) -> bool {
        matches!(self.fate[i], LogFate::Withdrew { .. })
    }

    /// Standing or withdrawn: it fought and lives.
    pub fn survived(&self, i: usize) -> bool {
        matches!(self.fate[i], LogFate::Standing | LogFate::Withdrew { .. })
    }
}

/// The engagement's working value (plan 6.1): lives for the call, never saved.
pub(crate) struct Eng {
    pub p: PlayerId,
    pub team: TeamId,
    pub from: PlotIdx,
    pub to: PlotIdx,
    pub launcher: Option<GroupId>,
    pub setup: Setup,
    /// The committed units in the set-up's order; a unit converted after its kills (a Wolf into a
    /// Wolf Pack) is replaced by the new one.
    pub units: Vec<UnitId>,
    /// Failed the morale check.
    pub sat_out: Vec<bool>,
    /// Opening counts (telemetry): defensive strikes that hit.
    pub strikes: i32,
    /// The defended plot as it was.
    pub place: PlotSnap,
    /// Per side: its player, the player the other side saw, every real owner in the battle.
    pub players: [(PlayerId, PlayerId, Vec<PlayerId>); 2],
    /// Each committed unit's type and owner at the start.
    pub types: Vec<UnitType>,
    pub owners: Vec<PlayerId>,
}

impl Eng {
    pub fn side(&self, i: usize) -> Side {
        self.setup.sides[i]
    }

    /// The committed units of side `s` (indices).
    pub fn of(&self, s: Side) -> std::ops::Range<usize> {
        self.setup.of(s)
    }

    pub fn ctx(&self) -> &HitCtx {
        &self.setup.ctx
    }

    pub fn in_city(&self) -> bool {
        self.setup.in_city
    }
}

/// One roll of the morale check (plan 6.3; [`Game::engage_morale_rolls`]): set-up index `roller`
/// against the Fear source `src`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct MoraleRoll {
    pub roller: usize,
    pub src: usize,
    pub chance: i32,
    pub roll: i32,
    pub passed: bool,
}

/// The opening's rolls (Q5; [`Game::engage_opening_rolls`]): per committed attacker that was
/// struck at, its strike roll and the strike's damage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OpeningRolls {
    pub strikes: Vec<(usize, DefStrikeRoll, DefStrikeHit)>,
}

impl Game {
    /// Unit `u` as a fighter of side `side` in cell `place` (spec 1.2-1.6): its role's values
    /// through the per-unit accessors (speed, actions with the extra ones, dodge, crit bonus,
    /// flanker), its role's anti-flank, HP from the damage field (a large unit's more), first
    /// strikes and their chance, its A against nobody, withdrawal (attackers'
    /// `withdrawalProbability`; defenders' defensive withdrawal outside a city, none in one, Q3),
    /// combat limit (attackers only), collateral limit (units with collateral damage splash) and
    /// its extra (Barrage), collateral protection (Drill), its spell.
    pub(crate) fn engage_fighter(&self, u: UnitId, side: Side, place: (u8, u8), in_city: bool, ctx: &HitCtx, params: &Params) -> Fighter {
        let d = self.d;
        let max_hp = self.unit_engage_max_hp(u);
        let mut f = Fighter::new(side, max_hp);
        f.tag = u.0;
        (f.lane, f.row) = place;
        f.large = self.unit_engage_large(u);
        f.hp = hp_from_damage(self.units[u].damage, max_hp);
        f.speed = self.unit_engage_speed(u);
        f.first_strikes = self.unit_first_strikes(u);
        f.chance_first_strikes = self.unit_chance_first_strikes(u);
        f.actions = self.unit_engage_actions(u);
        f.a = self.engage_a_alone(u, side, ctx);
        f.dodge = self.unit_engage_dodge(u);
        f.crit_bonus = self.unit_engage_crit_bonus(u);
        f.anti_flank = self.unit_combat_role(u).is_some_and(|r| d.combat_roles[r].anti_flank);
        f.flanker = self.unit_engage_flanks(u);
        f.withdraw_pct = match side {
            Side::Attacker => self.unit_withdrawal_probability(u),
            Side::Defender if in_city => 0,
            Side::Defender => self.unit_get_withdrawl_prob_defensive(u),
        };
        f.combat_limit = if side == Side::Attacker { self.units[u].combat_limit.clamp(0, 100) } else { 100 };
        f.collateral_limit = (self.unit_collateral_damage(u) > 0).then(|| self.unit_collateral_damage_limit(u));
        f.collateral_extra = self.units[u].extra_collateral_damage;
        f.collateral_protection = self.units[u].collateral_damage_protection;
        f.spell = match self.unit_engage_spell(u) {
            Some(s) => Spell::known(s.idx() as u32),
            None => Spell::bolt(params),
        };
        f
    }

    /// Sentinel `k` of city `c` as a fighter in cell `place` (docs/SIEGES.md): `UNIT_SENTINEL`'s
    /// speed, rows, dodge and crit bonus, its A, the city's Sentinel HP and its damage; fixed.
    pub(crate) fn engage_sentinel_fighter(&self, c: CityId, k: usize, place: (u8, u8), params: &Params) -> Fighter {
        let d = self.d;
        let ty = self.sentinel_unit_type();
        let role = d.unit_roles[ty];
        let max_hp = self.city_sentinel_max_hp(c);
        let mut f = Fighter::new(Side::Defender, max_hp);
        f.tag = u32::MAX - k as u32;
        (f.lane, f.row) = place;
        f.hp = hp_from_damage(self.city_sentinel_damage(c, k), max_hp);
        f.speed = d.engage_speed(ty, role);
        f.actions = d.engage_actions(ty, role, false);
        f.a = self.sentinel_a_x100();
        f.dodge = d.engage_dodge(ty, role);
        f.crit_bonus = d.engage_crit_bonus(ty, role);
        f.spell = Spell::bolt(params);
        f.fixed = true;
        f
    }

    /// The set-up of an engagement once the attacker's formation is chosen (module comment, step
    /// 1): the defender's formation of `candidates`, both sides in the engagement's order, their
    /// fighters (then, with `sentinels`, the defended city's Sentinels: docs/SIEGES.md) and the
    /// pair table. The real fight ([`Game::engage`]) and the preview ([`Game::engage_preview`])
    /// both build their battle here, so its fighters and pairs are the fight's own. `Err`: an
    /// empty formation, or no defender (no unit and no Sentinel).
    pub(crate) fn engage_setup(&self, ctx: HitCtx, slots: Slots, att: Vec<Place>, candidates: Vec<UnitId>, sentinels: bool) -> Result<Setup, &'static str> {
        if att.is_empty() {
            return Err("no unit can attack there");
        }
        let walls = if sentinels && slots.sentinels > 0 { self.engage_city_sentinels(ctx.to) } else { None };
        if candidates.is_empty() && walls.is_none() {
            return Err("there is nobody to fight there");
        }
        let def = self.engage_defense_formation(&candidates, &slots, &ctx);
        if def.is_empty() && walls.is_none() {
            return Err("there is nobody to fight there");
        }
        let params = Params::from_defines(&self.d.defines);
        let in_city = self.plot_is_city(ctx.to, false, None);
        let ground = self.engage_ground(ctx.to);
        let lanes = self.engage_lanes(&slots);
        let mut att = att;
        let mut def = def;
        att.sort_by_key(|p| (p.lane, p.row));
        def.sort_by_key(|p| (p.lane, p.row));
        let n_att = att.len();
        let all: Vec<(Place, Side)> = att.iter().map(|&p| (p, Side::Attacker)).chain(def.iter().map(|&p| (p, Side::Defender))).collect();
        let units: Vec<UnitId> = all.iter().map(|(p, _)| p.unit).collect();
        let mut sides: Vec<Side> = all.iter().map(|&(_, s)| s).collect();
        let mut places: Vec<(u8, u8)> = all.iter().map(|(p, _)| (p.lane, p.row)).collect();
        let mut fighters: Vec<Fighter> = all.iter().map(|&(p, s)| self.engage_fighter(p.unit, s, (p.lane, p.row), in_city, &ctx, &params)).collect();
        // The city's Sentinels, in their cells, after the units.
        let (sentinel_city, ks) = match walls {
            Some((c, ks)) => (Some(c), ks),
            None => (None, Vec::new()),
        };
        let cells = self.engage_sentinel_cells(&slots);
        let ks: Vec<usize> = ks.into_iter().take(cells.len()).collect();
        if let Some(c) = sentinel_city {
            for (j, &k) in ks.iter().enumerate() {
                sides.push(Side::Defender);
                places.push(cells[j]);
                fighters.push(self.engage_sentinel_fighter(c, k, cells[j], &params));
            }
        }
        let n_units = units.len();
        let n = fighters.len();
        let mut pairs = vec![Pair::default(); n * n];
        for h in 0..n {
            for t in 0..n {
                if sides[h] == sides[t] {
                    continue;
                }
                pairs[h * n + t] = match (h < n_units, t < n_units, sentinel_city) {
                    (true, true, _) => self.engage_hit(units[h], units[t], sides[h], &ctx),
                    (true, false, Some(c)) => Pair { a: self.engage_a_alone(units[h], sides[h], &ctx), dv: self.city_sentinel_dv_x100(c) },
                    (false, true, _) => Pair { a: self.sentinel_a_x100(), dv: self.engage_dv_alone(units[t], sides[t], &ctx) },
                    _ => Pair::default(),
                };
            }
        }
        Ok(Setup { ctx, slots, lanes, ground, params, in_city, candidates, units, sides, places, n_att, fighters, pairs, sentinel_city, sentinels: ks })
    }

    /// Phase 7 hook (plan 6.2, 14): the pre-battle spells' effects on the battle of `setup` (its
    /// units committed, before the morale check and the opening). None in phase 6.
    pub(crate) fn engage_pre_battle(&self, _setup: &Setup) -> Vec<BattleEffect> {
        Vec::new()
    }

    /// Phase 7 hook (plan 6.4 step 3, 14): the opening spells' effects on the battle of `setup`
    /// with its HP `hp` after the defensive strikes (opening damage spells, Q11's missile casts).
    /// None in phase 6.
    pub(crate) fn engage_opening_spells(&self, _setup: &Setup, _hp: &[i32]) -> Vec<BattleEffect> {
        Vec::new()
    }

    /// Phase 7 hook (plan 6.7 step 6, 14): the post-battle spells cast after the battle of `setup`
    /// as it ended (`f`), once the medics have healed: `Cast` records whose writes to `Game` the
    /// post-battle steps make. None in phase 6.
    pub(crate) fn engage_post_battle_spells(&self, _setup: &Setup, _f: &Fought) -> Vec<BattleEffect> {
        Vec::new()
    }

    /// Whether a human could see this battle (its log is written): a human fights on either
    /// side, or a human's team sees the defended plot.
    fn engage_seen_by_human(&self, units: &[UnitId], p: PlayerId, to: PlotIdx) -> bool {
        self.players[p].human
            || units.iter().any(|&u| self.players[self.units[u].owner].human)
            || self.map.plots[to].units.iter().any(|&u| self.players[self.units[u].owner].human)
            || self.plot_is_visible_to_watching_human(to)
    }

    /// Fights an engagement (module comment). `Err`: refused, nothing changed.
    pub(crate) fn engage(&mut self, ord: EngageOrder) -> Result<EngageOutcome, &'static str> {
        if !self.telemetry.on() {
            return self.engage_inner(ord);
        }
        self.tel_ctx_push("combat");
        let r = self.engage_inner(ord);
        self.tel_ctx_pop();
        r
    }

    fn engage_inner(&mut self, ord: EngageOrder) -> Result<EngageOutcome, &'static str> {
        let EngageOrder { p, from, to, launcher, formation, forced, log } = ord;
        let ctx = HitCtx::new(from, to);

        // 1. Set-up.
        let slots = self.engage_slots(&ctx);
        let (pool, att, ordered) = match forced {
            Some(units) => {
                let pool: Vec<UnitId> = units
                    .into_iter()
                    .filter(|&u| self.units.contains(u) && !self.unit_is_dead(u) && self.unit_can_attack(u) && self.unit_combat_role(u).is_some())
                    .collect();
                let f = self.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
                (pool, f, false)
            }
            None => {
                let pool = self.engage_attackers(p, from, to, WarTest::AtWar);
                match formation {
                    Some(f) => {
                        self.engage_validate_formation(&pool, &f, &slots)?;
                        (pool, f, true)
                    }
                    None => {
                        let f = self.engage_auto_formation(&pool, Side::Attacker, &slots, &ctx);
                        (pool, f, false)
                    }
                }
            }
        };
        if att.is_empty() {
            return Err("no unit can attack there");
        }
        let force: Vec<UnitId> = att.iter().map(|q| q.unit).collect();
        let candidates = self.engage_defenders(to, p, &force, WarTest::AtWar);
        if candidates.is_empty() && slots.sentinels == 0 {
            return Err("there is nobody to fight there");
        }
        let setup = self.engage_setup(ctx, slots, att, candidates, true)?;
        let n = setup.units.len();
        let def_units: Vec<UnitId> = setup.units[setup.n_att..].to_vec();
        let att_units: Vec<UnitId> = setup.units[..setup.n_att].to_vec();
        let mut e = Eng {
            p,
            team: self.players[p].team,
            from,
            to,
            launcher,
            units: setup.units.clone(),
            sat_out: vec![false; n],
            strikes: 0,
            place: self.engage_place(from, to),
            players: [
                self.engage_side_players(Side::Attacker, p, to, &att_units, &pool, &def_units),
                self.engage_side_players(Side::Defender, p, to, &def_units, &setup.candidates, &att_units),
            ],
            types: setup.units.iter().map(|&u| self.units[u].unit_type).collect(),
            owners: setup.units.iter().map(|&u| self.units[u].owner).collect(),
            setup,
        };
        // The engagement noted for tools (`Game::combat_notes`, off by default; reads only): its
        // stacks are the units committed on each side.
        if let (true, Some(&attacker), Some(&defender)) = (self.combat_notes.is_some(), att_units.first(), def_units.first()) {
            let note = crate::log::CombatNote {
                turn: self.game_turn(),
                attacker,
                defender,
                attacker_owner: p,
                defender_owner: self.units[defender].owner,
                plot: to,
                attacker_stack: att_units.len() as i32,
                defender_stack: def_units.len() as i32,
            };
            if let Some(notes) = self.combat_notes.as_mut() {
                notes.push(note);
            }
        }
        let wants_log = log.unwrap_or_else(|| self.engage_seen_by_human(&e.units, p, to));
        let mut sink = if wants_log {
            let reserves: Vec<(UnitId, Side)> = pool
                .iter()
                .filter(|u| !e.units.contains(u))
                .map(|&u| (u, Side::Attacker))
                .chain(e.setup.def_reserve().into_iter().map(|u| (u, Side::Defender)))
                .collect();
            let mut s = Sink::new(self, &e, &reserves, [ordered, false]);
            // The plan as the attacker saw it, before anything changes.
            let forecast = self.engage_forecast_frozen(p, from, to, &e.setup.formation(Side::Attacker), |u| s.index_of(u));
            s.set_forecast(forecast);
            Some(s)
        } else {
            None
        };

        // Phase 7's pre-battle effects.
        let mut effects = self.engage_pre_battle(&e.setup);

        // 2. Morale.
        self.engage_morale(&mut e, &mut sink);

        // 3. Opening: defensive strikes (never kill), then phase 7's opening spells.
        self.engage_opening(&mut e, &mut sink);
        let mut hp: Vec<i32> =
            (0..e.setup.fighters.len()).map(|i| if i < n { hp_from_damage(self.units[e.units[i]].damage, e.setup.fighters[i].max_hp) } else { e.setup.fighters[i].hp }).collect();
        effects.extend(self.engage_opening_spells(&e.setup, &hp));
        if !effects.is_empty() {
            e.setup.apply_effects(&effects, &mut hp);
        }

        // 4. The battle.
        let (f, map) = e.setup.battle(&e.sat_out, &hp);
        let out = if sink.is_some() { formation::resolve(&f, &mut self.soren_rand) } else { formation::resolve_quiet(&f, &mut self.soren_rand) };
        let fought = Fought::new(&e.setup, &hp, &map, &out);
        if let Some(s) = sink.as_mut() {
            s.battle(self, &e, &f, &map, &out);
        }

        // 5. After the battle, the report, telemetry.
        let post = self.engage_post(&mut e, &fought, &mut sink);
        let report = self.engage_report(&e, &fought, &post, sink.map(|s| s.finish(&fought)));
        let id = self.battle_report_add(report);
        if self.telemetry.on() {
            let r = self.battle_report(id).expect("the report just added").clone();
            self.tel_engagement(&r, &e, &fought);
        }
        self.engage_messages(id);
        Ok(EngageOutcome { report: id, result: post.result, plot_taken: post.plot_taken })
    }

    /// The defended plot as it is now (the report's `place`).
    fn engage_place(&self, from: PlotIdx, to: PlotIdx) -> PlotSnap {
        let plot = &self.map.plots[to];
        let (x, y) = self.map.xy(to);
        let city = plot.plot_city.filter(|&c| self.cities.contains(c)).map(|c| {
            let city = &self.cities[c];
            let defense_pct = if self.city_is_occupation(c) { 0 } else { self.city_total_defense(c, false).max(0) };
            CitySnap { id: c, name: city.name.clone(), owner: city.owner, size: city.population, defense_pct }
        });
        PlotSnap {
            x,
            y,
            terrain: plot.terrain_type,
            feature: plot.feature_type,
            hills: self.plot_is_hills(to),
            river_crossing: self.plot_is_river_crossing(from, self.direction_xy_plots(from, to)),
            city,
        }
    }

    /// Side `side`'s player (the attacker: `p`; the defender: the city's owner, else its first
    /// committed unit's), the player the other side saw (hidden nationality, G5) and every real
    /// owner of its units in the battle (committed or not; the city's owner for the defender).
    fn engage_side_players(&self, side: Side, p: PlayerId, to: PlotIdx, committed: &[UnitId], all: &[UnitId], other: &[UnitId]) -> (PlayerId, PlayerId, Vec<PlayerId>) {
        let city_owner = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c)).map(|c| self.cities[c].owner);
        let other_team = other.first().map(|&u| self.unit_get_team(u));
        let player = match side {
            Side::Attacker => p,
            Side::Defender => city_owner.unwrap_or_else(|| self.units[committed[0]].owner),
        };
        let shown = match (side, committed.first()) {
            (Side::Defender, _) if city_owner == Some(player) => player,
            (_, Some(&u)) => self.unit_get_visual_owner(u, other_team),
            (_, None) => player,
        };
        let mut players: Vec<PlayerId> = all.iter().chain(committed).map(|&u| self.units[u].owner).collect();
        players.push(player);
        if side == Side::Defender {
            players.extend(city_owner);
        }
        players.sort();
        players.dedup();
        (player, shown, players)
    }

    /// The report of engagement `e` (spec 1.7, 2).
    fn engage_report(&self, e: &Eng, f: &Fought, post: &Post, log: Option<BattleLog>) -> BattleReport {
        let sides = [Side::Attacker, Side::Defender].map(|s| {
            let (player, shown_player, players) = e.players[s.index()].clone();
            let mut lost: Vec<(UnitType, i32)> = Vec::new();
            let (mut sat_out, mut fallen, mut escaped, mut standing) = (0, 0, 0, 0);
            for i in e.of(s) {
                match f.fate[i] {
                    LogFate::Fell { .. } => {
                        fallen += 1;
                        match lost.iter_mut().find(|(t, _)| *t == e.types[i]) {
                            Some((_, n)) => *n += 1,
                            None => lost.push((e.types[i], 1)),
                        }
                    }
                    LogFate::Withdrew { .. } => escaped += 1,
                    LogFate::SatOut => sat_out += 1,
                    LogFate::Standing => standing += 1,
                }
            }
            lost.sort();
            ReportSide {
                player,
                shown_player,
                players,
                slots: e.setup.slots.max(s),
                lanes: e.setup.lanes[s.index()] as i32,
                used: e.of(s).map(|i| e.setup.fighters[i].slots()).sum(),
                committed: e.of(s).len() as i32,
                sat_out,
                fallen,
                escaped,
                standing,
                lost,
            }
        });
        BattleReport {
            id: 0,
            turn: 0,
            plot: e.to,
            from: e.from,
            place: e.place.clone(),
            sides,
            outcome: ReportOutcome {
                result: post.result,
                rounds: f.rounds,
                plot_taken: post.plot_taken,
                city_captured: post.city_captured,
                moved_in: post.moved_in,
                defenders_left: post.defenders_left,
                sentinels: e.setup.sentinels.len() as i32,
                sentinels_breached: e.setup.sentinel_range().filter(|&i| matches!(f.fate[i], LogFate::Fell { .. })).count() as i32,
            },
            log: log.map(Box::new),
        }
    }

    /// The game log's message of report `id` to each player of each side, naming the other side
    /// as that side saw it (hidden nationality).
    fn engage_messages(&mut self, id: u32) {
        let Some(r) = self.battle_report(id) else { return };
        let turn = self.game_turn();
        let at = (r.place.x, r.place.y);
        let place = match &r.place.city {
            Some(c) => c.name.clone(),
            None => format!("({}, {})", r.place.x, r.place.y),
        };
        let (att, def) = (&r.sides[0], &r.sides[1]);
        let result = r.outcome.result;
        let walls = match (r.outcome.sentinels, r.outcome.sentinels_breached) {
            (0, _) => String::new(),
            (n, b) => format!(", {b} of {n} Sentinels breached"),
        };
        let took = match (&r.outcome.city_captured, r.outcome.plot_taken) {
            (Some(_), _) => format!("took {place}"),
            (None, true) => "took the plot".to_string(),
            _ => "cleared it".to_string(),
        };
        let mut msgs: Vec<(PlayerId, String)> = Vec::new();
        for &p in &att.players {
            let verb = match result {
                EngageResult::Taken => took.clone(),
                EngageResult::Cleared => "broke their line, but the plot holds".to_string(),
                EngageResult::Repulsed => "were repulsed".to_string(),
                EngageResult::Held => "could not break their line".to_string(),
            };
            let them = self.player_get_civilization_adjective(def.shown_player);
            msgs.push((p, format!("Battle at {place}: your {} attacked {} {them} and {verb}; {} fell, {} killed{walls} (report #{id})", att.committed, def.committed, att.fallen, def.fallen)));
        }
        for &p in &def.players {
            let verb = match result {
                EngageResult::Taken => "were overrun",
                EngageResult::Cleared => "lost their line, but the plot holds",
                EngageResult::Repulsed => "drove them off",
                EngageResult::Held => "held",
            };
            let them = self.player_get_civilization_adjective(att.shown_player);
            msgs.push((p, format!("Battle at {place}: {} {them} attacked your {}, which {verb}; {} fell, {} killed{walls} (report #{id})", att.committed, def.committed, def.fallen, att.fallen)));
        }
        for (p, text) in msgs {
            self.log.msg_at(turn, Some(p), at, || text);
        }
    }

    /// The strongest committed unit among `units[range]` with Fear that fights (plan 6.3): by base
    /// strength, then level, then the lowest id.
    pub(crate) fn engage_fear_source(&self, units: &[UnitId], range: std::ops::Range<usize>, sat_out: &[bool]) -> Option<usize> {
        range.filter(|&i| !sat_out[i] && self.unit_is_fear(units[i])).max_by_key(|&i| {
            let u = units[i];
            (self.unit_base_combat_str(u), self.units[u].level, std::cmp::Reverse(u))
        })
    }

    /// The morale check's rolls (plan 6.3; module comment, step 2) on the committed `units` (the
    /// first `n_att` attack), drawing from `rng` ("Im afeared!"): a committed Fear defender makes
    /// each committed attacker not immune roll, then a Fear attacker that did not sit out does the
    /// same to the defenders. Marks the units that fail in `sat_out`. Changes nothing in the game.
    pub(crate) fn engage_morale_rolls(&self, units: &[UnitId], n_att: usize, sat_out: &mut [bool], rng: &mut impl formation::BattleRng) -> Vec<MoraleRoll> {
        let mut out = Vec::new();
        let n = units.len();
        for (src_side, rollers) in [(n_att..n, 0..n_att), (0..n_att, n_att..n)] {
            let Some(src) = self.engage_fear_source(units, src_side, sat_out) else { continue };
            let su = units[src];
            let (src_str, src_level) = (self.unit_base_combat_str(su), self.units[su].level);
            for i in rollers {
                let u = units[i];
                if sat_out[i] || self.unit_is_immune_to_fear(u) {
                    continue;
                }
                let chance = (self.unit_base_combat_str(u) + 20 + self.units[u].level - src_str - src_level).max(4);
                let roll = rng.rand(40, "Im afeared!");
                let passed = roll <= chance;
                if !passed {
                    sat_out[i] = true;
                }
                out.push(MoraleRoll { roller: i, src, chance, roll, passed });
            }
        }
        out
    }

    /// Plan 6.3 (module comment, step 2): the rolls on the sync RNG, then their messages and
    /// log events.
    fn engage_morale(&mut self, e: &mut Eng, sink: &mut Option<Sink>) {
        let d = self.d;
        let turn = self.game_turn();
        let at = self.map.xy(e.to);
        let n_att = e.setup.n_att;
        let rolls = self.with_sync_rng(|g, rng| g.engage_morale_rolls(&e.units, n_att, &mut e.sat_out, rng));
        if let (Some(s), false) = (sink.as_mut(), rolls.is_empty()) {
            s.stage(super::report::Stage::Morale);
        }
        for r in rolls {
            if let Some(s) = sink.as_mut() {
                s.morale(r.roller, r.src, r.chance, r.roll, r.passed);
            }
            if !r.passed {
                let u = e.units[r.roller];
                let (owner, ty) = (self.units[u].owner, self.units[u].unit_type);
                let verb = if e.side(r.roller) == Side::Attacker { "attack" } else { "fight" };
                self.log.msg_at(turn, Some(owner), at, || format!("{} is afraid to {verb}", d.text.name(&d.units[ty].base.description)));
            }
        }
    }

    /// The opening's rolls (Q5; module comment, step 3) for the committed `units` (the first
    /// `n_att` attack; those in `sat_out` never close in), over the units `on_plot` (the plot's
    /// own for the real fight; what the attacker sees for a preview), reading and updating `ov`,
    /// drawing from `rng` in the fight's order: each attacker's defensive strike (roll, then the
    /// strike's damage). Changes nothing in the game.
    pub(crate) fn engage_opening_rolls(&self, units: &[UnitId], n_att: usize, sat_out: &[bool], on_plot: &[UnitId], ov: &mut UnitOverlay, rng: &mut impl formation::BattleRng) -> OpeningRolls {
        let mut out = OpeningRolls::default();
        for i in 0..n_att {
            let a = units[i];
            if sat_out[i] || self.unit_is_immune_to_defensive_strike(a) {
                continue;
            }
            let roll = self.unit_defensive_strike_roll_on(on_plot, a, ov, rng);
            if let Some(best) = roll.best {
                let hit = self.unit_defensive_strike_damage(best, a, ov, rng);
                if hit.used_up {
                    ov.use_up(best);
                }
                if hit.damage > 0 {
                    let now = ov.damage(self, a);
                    ov.set_damage(a, crate::utils::range(now + hit.damage, 0, self.unit_max_hit_points(a)));
                }
                out.strikes.push((i, roll, hit));
            }
        }
        out
    }

    /// Q5 (module comment, step 3): the rolls on the sync RNG, then their writes (damage,
    /// strikers used up, XP, messages) and log events in the same order.
    fn engage_opening(&mut self, e: &mut Eng, sink: &mut Option<Sink>) {
        if let Some(s) = sink.as_mut() {
            s.stage(super::report::Stage::Opening);
        }
        let on_plot = self.map.plots[e.to].units.clone();
        let n_att = e.setup.n_att;
        let rolls = self.with_sync_rng(|g, rng| g.engage_opening_rolls(&e.units, n_att, &e.sat_out, &on_plot, &mut UnitOverlay::default(), rng));
        for (i, roll, hit) in rolls.strikes {
            let a = e.units[i];
            let Some(best) = roll.best else { continue };
            self.unit_defensive_strike_write(best, a, hit);
            let pct = hit.damage;
            if pct > 0 {
                e.strikes += 1;
            }
            if let Some(s) = sink.as_mut() {
                let striker = s.ensure(self, e, best, Side::Defender);
                s.def_strike(striker, i, &roll, pct, pct * e.setup.fighters[i].max_hp);
            }
        }
    }

    /// A missile strike (Q11; module comment): one hit on a unit of the defender's formation
    /// (`formation::missile_hit`), then its collateral; the missile dies. A kill gives it the kill
    /// (no advance). `Err`: no defender there, nothing changed.
    pub(crate) fn engage_missile_strike(&mut self, m: UnitId, to: PlotIdx) -> Result<(), &'static str> {
        if !self.telemetry.on() {
            return self.engage_missile_inner(m, to);
        }
        self.tel_ctx_push("combat");
        let r = self.engage_missile_inner(m, to);
        self.tel_ctx_pop();
        r
    }

    /// A missile's battle (Q11): missile `m` alone against the defender's auto formation on `to`,
    /// the plot's own defence in the defenders' Dv ([`HitCtx::missile`]). `Err`: no defender.
    pub(crate) fn engage_missile_setup(&self, m: UnitId, to: PlotIdx) -> Result<Setup, &'static str> {
        let p = self.units[m].owner;
        let ctx = HitCtx::missile(self.unit_plot(m), to);
        let candidates = self.engage_defenders(to, p, &[m], WarTest::AtWar);
        if candidates.is_empty() {
            return Err("there is nobody to fight there");
        }
        let slots = self.engage_slots(&ctx);
        self.engage_setup(ctx, slots, vec![Place { unit: m, lane: 0, row: 0 }], candidates, false)
    }

    fn engage_missile_inner(&mut self, m: UnitId, to: PlotIdx) -> Result<(), &'static str> {
        let d = self.d;
        let p = self.units[m].owner;
        let setup = self.engage_missile_setup(m, to)?;
        let hp: Vec<i32> = setup.fighters.iter().map(|f| f.hp).collect();
        let (f, map) = setup.battle(&vec![false; setup.units.len()], &hp);
        let hit = missile_hit(&f, 0, &mut self.soren_rand);
        let turn = self.game_turn();
        let at = self.map.xy(to);
        let mut killed = None;
        if let Some(h) = &hit {
            let t = map[h.target as usize];
            let tu = setup.units[t];
            let left = hp[t] - h.dmg;
            let (mty, tty) = (self.units[m].unit_type, self.units[tu].unit_type);
            if left <= 0 {
                self.unit_set_damage(tu, d.defines.max_hit_points, Some(p));
                killed = Some(tu);
            } else {
                self.unit_set_damage(tu, damage_from_hp(left, setup.fighters[t].max_hp), None);
            }
            let verb = if left <= 0 { "destroyed" } else { "struck" };
            for owner in [p, self.units[tu].owner] {
                self.log.msg_at(turn, Some(owner), at, || format!("{} {verb} {}", d.text.name(&d.units[mty].base.description), d.text.name(&d.units[tty].base.description)));
            }
        }
        self.unit_collateral_combat_ex(m, to, None, true);
        if let Some(v) = killed {
            self.engage_kill_bookkeeping(m, v, false, to);
            if !self.crusades.is_empty() && self.units.contains(m) && self.units.contains(v) {
                self.crusade_on_combat(m, v, false, true);
            }
            if self.telemetry.on() && self.units.contains(m) && self.units.contains(v) {
                self.tel_engage_kill(m, v);
            }
            if self.units.contains(m) && self.units.contains(v) {
                self.unit_combat_won_as(m, v, crate::unit::WonMode::Kill { roll_convert: false }, None);
                if d.defines.use_combat_result_callback != 0 && self.units.contains(m) && self.units.contains(v) {
                    self.ev_combat_result(m, v);
                }
            }
            if self.units.contains(v) {
                self.unit_kill(v, false, None);
            }
        }
        if self.units.contains(m) {
            self.units[m].made_attack = true;
            self.units[m].attacks_made += 1;
            self.unit_kill(m, true, None);
        }
        if self.telemetry.on() {
            self.tel_missile_strike(p, hit.is_some(), killed.is_some());
        }
        Ok(())
    }

    /// A kill's bookkeeping outside `combatWon` (plan 6.7 step 3, today's two branches): the
    /// barbarian win counter, war weariness and war success (none for hidden nationality,
    /// summons or `bNoWarWeariness`). `victim_attacked`: the victim was on the attacking side.
    pub(crate) fn engage_kill_bookkeeping(&mut self, killer: UnitId, victim: UnitId, victim_attacked: bool, plot: PlotIdx) {
        let d = self.d;
        if self.unit_is_barbarian(victim) {
            let ko = self.units[killer].owner;
            self.players[ko].wins_vs_barbs += 1;
        }
        let vty = self.units[victim].unit_type;
        if !self.unit_is_hidden_nationality(killer)
            && !self.unit_is_hidden_nationality(victim)
            && self.units[victim].duration == 0
            && !d.units[vty].no_war_weariness
        {
            let (vt, kt) = (self.unit_get_team(victim), self.unit_get_team(killer));
            if victim_attacked {
                self.team_change_war_weariness_at_plot(vt, kt, plot, d.defines.ww_unit_killed_attacking);
                self.team_change_war_weariness_at_plot(kt, vt, plot, d.defines.ww_killed_unit_defending);
                self.teams[kt].ai_war_success[vt] += d.defines.war_success_defending;
            } else {
                self.team_change_war_weariness_at_plot(vt, kt, plot, d.defines.ww_unit_killed_defending);
                self.team_change_war_weariness_at_plot(kt, vt, plot, d.defines.ww_killed_unit_attacking);
                self.teams[kt].ai_war_success[vt] += d.defines.war_success_attacking;
            }
        }
    }

    /// `unit_attack` under the dispatch rule (plan section 1): a missile strikes; a land unit with
    /// a defender to fight fights a one-unit engagement (forced: whatever its moves and attacks
    /// this turn, as today's `attack`). Returns false when the duel should go on: no defender
    /// there (non-combatants are captured by today's attack).
    pub(crate) fn engage_unit_attack(&mut self, u: UnitId, to: PlotIdx) -> bool {
        let p = self.units[u].owner;
        let nobody = self.engage_defenders(to, p, &[u], WarTest::AtWar).is_empty();
        if nobody && (self.unit_is_suicide(u) || !self.plot_has_enemy_sentinels(to, self.players[p].team)) {
            return false;
        }
        if self.unit_is_suicide(u) {
            let _ = self.engage_missile_strike(u, to);
            return true;
        }
        let Some(from) = self.unit_plot_opt(u) else { return true };
        let launcher = self.units[u].group.filter(|&g| self.groups.contains(g));
        let _ = self.engage(EngageOrder { p, from, to, launcher, formation: None, forced: Some(vec![u]), log: None });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::GameSetup;

    /// Two flat plots side by side, players 0 and 1 at war; `att` units of player 0 on `from`,
    /// `def` units of player 1 on `to`.
    fn world(att: &[&str], def: &[&str]) -> (Game, PlotIdx, PlotIdx) {
        let d = ffh_data::global();
        let mut setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5);
        setup.options[GameOptionType::GAMEOPTION_ARMIES] = true;
            let mut g = Game::new(d, setup);
        g.game_init();
        let flat = |g: &Game, p: PlotIdx| {
            let plot = &g.map.plots[p];
            !g.plot_is_water(p) && !g.plot_is_peak(p) && plot.units.is_empty() && plot.owner.is_none() && plot.improvement_type.is_none()
        };
        let (from, to) = g
            .map
            .plot_ids()
            .filter(|&p| flat(&g, p))
            .find_map(|p| {
                let (x, y) = g.map.xy(p);
                let q = g.map.plot_xy(x + 1, y)?;
                flat(&g, q).then_some((p, q))
            })
            .unwrap();
        let (t0, t1) = (g.players[PlayerId(0)].team, g.players[PlayerId(1)].team);
        g.team_declare_war(t0, t1, false, None);
        for (owner, names, at) in [(PlayerId(0), att, from), (PlayerId(1), def, to)] {
            let (x, y) = g.map.xy(at);
            for n in names {
                let ty = d.t::<UnitType>(n);
                g.player_init_unit(owner, ty, x, y, None, None);
            }
            g.player_army_sync(owner);
        }
        (g, from, to)
    }

    fn order(from: PlotIdx, to: PlotIdx, log: bool) -> EngageOrder {
        EngageOrder { p: PlayerId(0), from, to, launcher: None, formation: None, forced: None, log: Some(log) }
    }

    fn clone(g: &Game) -> Game {
        Game::load(g.d, &g.save()).expect("a save of this build")
    }

    /// The battle log sink draws no random number and changes no state: the same engagement with
    /// and without it leaves the same RNG and the same game (spec section 2).
    #[test]
    fn the_sink_changes_nothing() {
        let (g, from, to) = world(
            &["UNIT_AXEMAN", "UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_HORSEMAN", "UNIT_CATAPULT", "UNIT_ADEPT"],
            &["UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_ARCHER", "UNIT_WARRIOR", "UNIT_DISCIPLE_THE_ORDER"],
        );
        let mut logged = 0;
        for seed in 0..12u32 {
            let mut a = clone(&g);
            let mut b = clone(&g);
            a.soren_rand = crate::rng::Rng::new(seed);
            b.soren_rand = crate::rng::Rng::new(seed);
            let ra = a.engage(order(from, to, true)).expect("an engagement");
            let rb = b.engage(order(from, to, false)).expect("an engagement");
            assert_eq!((ra.result, ra.plot_taken), (rb.result, rb.plot_taken));
            assert_eq!(a.soren_rand, b.soren_rand, "seed {seed}");
            assert_eq!(a.calculate_sync_checksum(), b.calculate_sync_checksum(), "seed {seed}");
            let (la, lb) = (a.battle_report(ra.report).unwrap(), b.battle_report(rb.report).unwrap());
            assert!(la.log.is_some() && lb.log.is_none());
            logged += la.log.as_ref().unwrap().events.len();
            // Everything else is the same state.
            for r in a.battle_reports.iter_mut() {
                r.log = None;
            }
            assert!(a.save() == b.save(), "seed {seed}");
        }
        assert!(logged > 100, "{logged}");
    }

    /// An engagement on a cloned game replays identically.
    #[test]
    fn an_engagement_replays_on_a_clone() {
        let (g, from, to) = world(&["UNIT_AXEMAN", "UNIT_AXEMAN", "UNIT_ARCHER"], &["UNIT_AXEMAN", "UNIT_ARCHER"]);
        let mut a = clone(&g);
        let mut b = clone(&g);
        let ra = a.engage(order(from, to, true)).unwrap();
        let rb = b.engage(order(from, to, true)).unwrap();
        assert_eq!(ra, rb);
        assert!(a.save() == b.save());
        assert_eq!(a.battle_report(ra.report), b.battle_report(rb.report));
    }

    /// A fighter carries Barrage's collateral extra and Drill's collateral protection, which
    /// scale a collateral splash (`formation::Formation::expected`).
    #[test]
    fn a_fighter_reads_barrage_and_drill() {
        let (mut g, from, to) = world(&["UNIT_CATAPULT"], &["UNIT_AXEMAN"]);
        let d = g.d;
        let (catapult, axeman) = (g.map.plots[from].units[0], g.map.plots[to].units[0]);
        let ctx = HitCtx::new(from, to);
        let params = Params::from_defines(&d.defines);
        let plain = (g.engage_fighter(catapult, Side::Attacker, (0, 2), false, &ctx, &params), g.engage_fighter(axeman, Side::Defender, (0, 0), false, &ctx, &params));
        assert_eq!((plain.0.collateral_extra, plain.1.collateral_protection), (0, 0));
        assert!(plain.0.collateral_limit.is_some(), "a catapult splashes");
        g.unit_set_has_promotion(catapult, d.t::<PromotionType>("PROMOTION_BARRAGE3"), true);
        g.unit_set_has_promotion(axeman, d.t::<PromotionType>("PROMOTION_DRILL3"), true);
        let att = g.engage_fighter(catapult, Side::Attacker, (0, 2), false, &ctx, &params);
        let def = g.engage_fighter(axeman, Side::Defender, (0, 0), false, &ctx, &params);
        assert_eq!((att.collateral_extra, def.collateral_protection), (100, 20));
    }

    /// A missile strikes at the strength it had before formation battles (Q11): half a hit
    /// (`ENGAGEMENT_MISSILE_PERCENT`), the plot's own defence (here the hills) in its target's Dv,
    /// and no ground rule (on hills a formation battle's attacker deals 75%).
    #[test]
    fn a_missile_strikes_at_its_old_strength() {
        let (mut g, from, to) = world(&["UNIT_FIREBALL"], &["UNIT_WARRIOR"]);
        let d = g.d;
        g.map.plots[to].plot_type = PlotType::PLOT_HILLS;
        let (fireball, warrior) = (g.map.plots[from].units[0], g.map.plots[to].units[0]);
        assert_eq!(d.defines.engagement_missile_percent, 50);
        let setup = g.engage_missile_setup(fireball, to).expect("a defender");
        assert_eq!(setup.units, [fireball, warrior]);
        let field = g.engage_hit(fireball, warrior, Side::Attacker, &HitCtx::new(from, to));
        let pair = setup.pair(0, 1);
        assert_eq!(pair.a, field.a);
        let team = g.players[PlayerId(1)].team;
        let hills = g.plot_defense_modifier(to, Some(team), false);
        assert!(hills > 0);
        // The warrior's Dv: its base under the hills' defence (the field's Dv has none of it).
        assert_eq!(pair.dv, field.dv * (100 + hills) / 100);
        let hp: Vec<i32> = setup.fighters.iter().map(|f| f.hp).collect();
        let (f, _) = setup.battle(&[false, false], &hp);
        let (a, dv) = (pair.a as i64, pair.dv as i64);
        let expected = (d.defines.engagement_base_damage as i64 * 100 * a / (a + dv) / 2) as i32;
        assert_eq!(f.missile_expected(0, 1), expected);
        assert!(f.expected(0, 1, ActionKind::Strike, 1, false) < 2 * expected, "a field strike takes the hills' 75%");
        let hit = missile_hit(&f, 0, &mut crate::rng::Rng::new(7)).expect("a hit");
        assert_eq!((hit.target, hit.expected), (1, expected));
        assert!(hit.dmg >= expected * 80 / 100 && hit.dmg <= expected * 120 / 100, "{hit:?}");
    }
}
