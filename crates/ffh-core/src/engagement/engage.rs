//! The engagement on `Game` (plan section 6): the set-up from the units onto the round model, the
//! morale check, the opening, the battle on the game's RNG, then the post-battle steps
//! ([`super::apply`]) and the report. Reached only through `group_attack_force` (every order of
//! a group into an enemy land plot under the option) and `unit_attack` (Taunt, the Shrine of
//! Camulos, Gift of Kylorin: a one-unit force).
//!
//! **Order** (plan 6.1 to 6.7; RNG order, plan 7):
//! 1. *Set-up*: slots, the attacker's pool and force (its stance's proposal, or the force given,
//!    checked), the defender's candidates and its stance's pick. Nothing has changed yet: an
//!    empty force or no defender is an `Err`. The [`Battle`] value is built here (fighters in
//!    slot order, attackers first; the pair table from [`Game::engage_hit`], which nothing
//!    before the rounds changes), and phase 7's pre-battle hook ([`Game::engage_pre_battle`])
//!    reads it.
//! 2. *Morale* (Q8): a committed Fear defender (the strongest by base strength, level, then
//!    the lowest id) makes each committed attacker not immune roll today's check
//!    (`rand(40) > max(4, str + 20 + level - str(src) - level(src))`, "Im afeared!"): one that
//!    fails sits out (its attack is spent, its slot stays empty); then a Fear attacker that did not
//!    sit out does the same to the committed defenders, which hold the plot but do not fight.
//! 3. *Opening*: defensive strikes (Q5): for each committed attacker that did not sit out, in slot
//!    order and not immune, every unit on the plot with a strike chance, at war and not used up
//!    (committed or not) rolls, and the best success strikes (today's damage, never past 95%);
//!    collateral (Q4): each committed attacker with `iCollateralDamage` that did not sit out,
//!    today's formula and limit, against every visible enemy defender on the plot. Neither kills.
//!    Then phase 7's opening hook ([`Game::engage_opening_spells`]).
//! 4. *The battle*: the [`Battle`]'s HP from the damage fields after the opening and its
//!    sat-out fighters from the morale check, the hooks' effects added, resolved by
//!    [`resolve_sampled`] on `soren_rand` (opening volleys, rounds, withdrawal).
//! 5. *Post-battle* ([`super::apply`], with phase 7's post-battle hook,
//!    [`Game::engage_post_battle_spells`]) and the report ([`super::report`]), telemetry.
//!
//! **Phase 7 hooks** (plan 6.2, 14): the three hooks are empty in phase 6. Each reads the
//! [`Battle`] and returns [`BattleEffect`]s; none writes `Game` or draws a random number, so the
//! estimator can call the first two on its own `Battle` and see the same in-battle spells. The
//! writes a cast makes to `Game` come after the battle, through its `Cast` record.
//!
//! It never sets `combat_unit`, `attack_plot` or the combat timers, so `plot_is_fighting` stays
//! false and nothing is left between calls (plan 6.9).
//!
//! **Missiles** (6.11, Q11): a missile (`bSuicide`: Fireball, Meteor) ordered into an enemy plot
//! does not start an engagement: [`Game::engage_missile_strike`] takes the defender's pick, hits
//! one of them by the missile's role order (one hit of the round model), then its collateral,
//! and the missile dies.
//!
//! **The battle log** is written through an optional sink ([`super::sink`]) for a battle a human
//! could see; it draws no random number and changes no state (test
//! `the_sink_changes_nothing`).

use ffh_data::*;

use super::battle::{damage_from_hp, hp_from_damage, Battle, BattleEffect, Fighter, Params, Side, TargetClass};
use super::pick::{StanceChoice, WarTest};
use super::resolve::{resolve_sampled, resolve_sampled_quiet, targets, EngageResult, TargetState, Targets};
use super::apply::Post;
use super::report::{BattleLog, BattleReport, CitySnap, Outcome, PlotSnap, ReportSide};
use super::resolve::{BattleLog as RoundLog, Fate};
use super::sink::Sink;
use super::slots::Slots;
use super::stats::HitCtx;
use crate::game::Game;
use crate::ids::*;

/// An order to fight an engagement (`group_attack_force`, `unit_attack`).
#[derive(Clone, Debug)]
pub(crate) struct EngageOrder {
    /// The attacking player and the plots.
    pub p: PlayerId,
    pub from: PlotIdx,
    pub to: PlotIdx,
    /// The group that gave the order: it advances when the plot is cleared.
    pub launcher: Option<GroupId>,
    /// The force chosen (checked against the pool); `None`: the stance's proposal.
    pub force: Option<Vec<UnitId>>,
    /// A forced attack (`unit_attack`: Taunt, events): the force is the units given, whatever
    /// their moves and attacks this turn (as today's `attack`).
    pub forced: bool,
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

/// The engagement's working value (plan 6.1): lives for the call, never saved.
pub(crate) struct Eng {
    pub p: PlayerId,
    pub team: TeamId,
    pub from: PlotIdx,
    pub to: PlotIdx,
    pub launcher: Option<GroupId>,
    pub ctx: HitCtx,
    pub slots: Slots,
    /// `[attacker, defender]`.
    pub stances: [StanceChoice; 2],
    /// The committed units in the round model's order: the force, then the defender's pick.
    /// A unit converted after its kills (a Wolf into a Wolf Pack) is replaced by the new one.
    pub units: Vec<UnitId>,
    pub n_att: usize,
    /// Each fighter's side (by index).
    pub sides: Vec<Side>,
    /// Failed the morale check.
    pub sat_out: Vec<bool>,
    /// The defended plot holds a city (no defensive withdrawal, Q3).
    pub in_city: bool,
    /// Opening counts (telemetry): defensive strikes that hit, units hit by collateral.
    pub strikes: i32,
    pub collateral: i32,
    /// The defended plot as it was.
    pub place: PlotSnap,
    /// Per side: its player, the player the other side saw, every real owner in the battle.
    pub players: [(PlayerId, PlayerId, Vec<PlayerId>); 2],
    /// Each fighter's unit type and owner at the start.
    pub types: Vec<UnitType>,
    pub owners: Vec<PlayerId>,
    /// The defender's candidates it did not commit (the report's `defenders_left`).
    pub def_reserve: Vec<UnitId>,
}

impl Eng {
    pub fn side(&self, i: usize) -> Side {
        self.sides[i]
    }

    /// The fighters of side `s` (indices).
    pub fn of(&self, s: Side) -> std::ops::Range<usize> {
        match s {
            Side::Attacker => 0..self.n_att,
            Side::Defender => self.n_att..self.units.len(),
        }
    }
}

/// The battle-model target class of a role's XML class.
fn class_of(c: ffh_data::TargetClass) -> TargetClass {
    match c {
        ffh_data::TargetClass::Front => TargetClass::Front,
        ffh_data::TargetClass::Back => TargetClass::Back,
        ffh_data::TargetClass::Screen => TargetClass::Screen,
        ffh_data::TargetClass::HeroOrSupport => TargetClass::HeroOrSupport,
    }
}

impl Game {
    /// The round model's constants (the `ENGAGEMENT_*` defines).
    pub fn engage_params(&self) -> Params {
        let d = &self.d.defines;
        Params {
            base_damage: d.engagement_base_damage,
            rand_min: d.engagement_damage_rand_min,
            rand_max: d.engagement_damage_rand_max.max(d.engagement_damage_rand_min),
            max_rounds: d.engagement_max_rounds,
            withdraw_hp_pct: d.engagement_withdraw_hp_percent,
        }
    }

    /// Unit `u` as a fighter of side `side` (plan 6.1): its role's position, screen, shield and
    /// target order, its HP from the damage field, volleys, withdrawal chance (attackers'
    /// `withdrawalProbability`; defenders' defensive withdrawal outside a city, none in one, Q3),
    /// combat limit (read for attackers only) and first-strike immunity.
    pub(crate) fn engage_fighter(&self, u: UnitId, side: Side, in_city: bool) -> Fighter {
        let d = self.d;
        let max_hp = self.unit_engage_max_hp(u);
        let mut f = Fighter::new(side, max_hp);
        f.tag = u.0;
        if let Some(r) = self.unit_combat_role(u) {
            let role = &d.combat_roles[r];
            f.back = role.back;
            f.screen = role.screen;
            f.ignores_shield = role.ignores_shield;
            if !role.targets.is_empty() {
                f.order = role.targets.iter().map(|&c| class_of(c)).collect();
            }
            f.hero_or_support = self.unit_is_hero(u) || d.ty::<CombatRoleType>("COMBATROLE_SUPPORT") == Some(r);
        }
        f.hp = hp_from_damage(self.units[u].damage, max_hp);
        f.first_strikes = self.unit_first_strikes(u);
        f.chance_first_strikes = self.unit_chance_first_strikes(u);
        f.first_strike_immune = self.unit_immune_to_first_strikes(u);
        f.withdraw_pct = match side {
            Side::Attacker => self.unit_withdrawal_probability(u),
            Side::Defender if in_city => 0,
            Side::Defender => self.unit_get_withdrawl_prob_defensive(u),
        };
        f.combat_limit = if side == Side::Attacker { self.units[u].combat_limit.clamp(0, 100) } else { 100 };
        f
    }

    /// The [`Battle`] of the committed `units` (sides by `sides`; sat-out fighters by
    /// `sat_out`) on `ctx`'s plot, the pair table from [`Game::engage_hit`].
    pub(crate) fn engage_battle(&self, units: &[UnitId], sides: &[Side], sat_out: &[bool], in_city: bool, ctx: &HitCtx) -> Battle {
        let fighters: Vec<Fighter> = units
            .iter()
            .zip(sides)
            .zip(sat_out)
            .map(|((&u, &side), &out)| {
                let mut f = self.engage_fighter(u, side, in_city);
                f.sat_out = out;
                f
            })
            .collect();
        Battle::new(self.engage_params(), fighters, |h, t| self.engage_hit(units[h], units[t], sides[h], ctx))
    }

    /// Phase 7 hook (plan 6.2, 14): the pre-battle spells' effects on battle `b` (its fighters
    /// committed, before the morale check and the opening), applied to it by both integrators.
    /// None in phase 6.
    pub(crate) fn engage_pre_battle(&self, _b: &Battle) -> Vec<BattleEffect> {
        Vec::new()
    }

    /// Phase 7 hook (plan 6.4 step 3, 14): the opening spells' effects on battle `b` (its HP
    /// after the defensive strikes and collateral), applied to it before the opening volleys by
    /// both integrators: opening damage spells, Q11's missile casts. None in phase 6.
    pub(crate) fn engage_opening_spells(&self, _b: &Battle) -> Vec<BattleEffect> {
        Vec::new()
    }

    /// Phase 7 hook (plan 6.7 step 6, 14): the post-battle spells cast after battle `b` as it
    /// ended (`r`), once the medics have healed: `Cast` records whose writes to `Game` the
    /// post-battle steps make. None in phase 6.
    pub(crate) fn engage_post_battle_spells(&self, _b: &Battle, _r: &RoundLog) -> Vec<BattleEffect> {
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
        let EngageOrder { p, from, to, launcher, force, forced, log } = ord;
        let ctx = HitCtx::new(from, to);

        // 1. Set-up.
        let slots = self.engage_slots(&ctx);
        let att_stance = self.engage_attack_stance(p, from, launcher);
        let (pool, auto, force, by_player) = if forced {
            let f: Vec<UnitId> = force
                .unwrap_or_default()
                .into_iter()
                .filter(|&u| self.units.contains(u) && !self.unit_is_dead(u) && self.unit_can_attack(u) && self.unit_combat_role(u).is_some())
                .collect();
            (f.clone(), f.clone(), f, false)
        } else {
            let pool = self.engage_attackers(p, from, to, WarTest::AtWar);
            let auto = self.engage_auto_pick(&pool, att_stance.stance, &slots, Side::Attacker, &ctx);
            match force {
                Some(f) => {
                    self.engage_validate_force(&pool, &f, &slots)?;
                    let mut a = auto.clone();
                    let mut b = f.clone();
                    a.sort();
                    b.sort();
                    let by_player = a != b;
                    (pool, auto, f, by_player)
                }
                None => {
                    let f = self.engage_propose_force(&pool, att_stance.stance, &slots, &ctx);
                    (pool, auto, f, false)
                }
            }
        };
        if force.is_empty() {
            return Err("no unit can attack there");
        }
        let candidates = self.engage_defenders(to, p, &force, WarTest::AtWar);
        if candidates.is_empty() {
            return Err("there is nobody to fight there");
        }
        let def_stance = self.engage_defense_stance(to, &candidates);
        let def_pick = self.engage_auto_pick(&candidates, def_stance.stance, &slots, Side::Defender, &ctx);
        if def_pick.is_empty() {
            return Err("there is nobody to fight there");
        }

        debug_assert!(
            force.len() as i32 <= slots.max(Side::Attacker) && def_pick.len() as i32 <= slots.max(Side::Defender),
            "an engagement committed {} attackers and {} defenders with {:?}",
            force.len(),
            def_pick.len(),
            slots
        );
        let n_att = force.len();
        let units: Vec<UnitId> = force.iter().chain(&def_pick).copied().collect();
        let sides: Vec<Side> = (0..units.len()).map(|i| if i < n_att { Side::Attacker } else { Side::Defender }).collect();
        let mut e = Eng {
            p,
            team: self.players[p].team,
            from,
            to,
            launcher,
            ctx,
            slots,
            stances: [att_stance, def_stance],
            sat_out: vec![false; units.len()],
            units,
            n_att,
            sides,
            in_city: self.plot_is_city(to, false, None),
            strikes: 0,
            collateral: 0,
            place: self.engage_place(from, to),
            players: [
                self.engage_side_players(Side::Attacker, p, to, &force, &pool, &def_pick),
                self.engage_side_players(Side::Defender, p, to, &def_pick, &candidates, &force),
            ],
            types: Vec::new(),
            owners: Vec::new(),
            def_reserve: candidates.iter().copied().filter(|u| !def_pick.contains(u)).collect(),
        };
        e.types = e.units.iter().map(|&u| self.units[u].unit_type).collect();
        e.owners = e.units.iter().map(|&u| self.units[u].owner).collect();
        let wants_log = log.unwrap_or_else(|| self.engage_seen_by_human(&e.units, p, to));
        let mut sink = if wants_log {
            let reserves: Vec<(UnitId, Side)> = pool
                .iter()
                .filter(|u| !e.units.contains(u))
                .map(|&u| (u, Side::Attacker))
                .chain(candidates.iter().filter(|u| !e.units.contains(u)).map(|&u| (u, Side::Defender)))
                .collect();
            Some(Sink::new(self, &e, &reserves, [&auto, &def_pick], by_player))
        } else {
            None
        };

        // The battle value; phase 7's pre-battle effects.
        let mut battle = self.engage_battle(&e.units, &e.sides, &e.sat_out, e.in_city, &e.ctx);
        let mut effects = self.engage_pre_battle(&battle);

        // 2. Morale.
        self.engage_morale(&mut e, &mut sink);

        // 3. Opening: defensive strikes, then collateral (neither kills), then phase 7's opening
        // spells on the battle with its HP after them.
        self.engage_opening(&mut e, &mut sink);
        for (i, &u) in e.units.iter().enumerate() {
            battle.set_start(i, self.units[u].damage, e.sat_out[i]);
        }
        // (Checked builds: the opening changed nothing but HP, so the set-up's pair table holds.)
        debug_assert!(self.engage_battle(&e.units, &e.sides, &e.sat_out, e.in_city, &e.ctx).same_as(&battle), "the opening changed more than HP");
        effects.extend(self.engage_opening_spells(&battle));

        // 4. The battle.
        if !effects.is_empty() {
            battle.add_effects(effects);
        }
        let estimate = self.telemetry.on().then(|| super::expect::resolve_expected(&battle).win_permille);
        let rlog = if sink.is_some() { resolve_sampled(&battle, &mut self.soren_rand) } else { resolve_sampled_quiet(&battle, &mut self.soren_rand) };
        if let Some(s) = sink.as_mut() {
            s.rounds(self, &e, &battle, &rlog);
        }

        // 5. After the battle, the report, telemetry.
        let post = self.engage_post(&mut e, &battle, &rlog, &mut sink);
        let report = self.engage_report(&e, &rlog, &post, sink.map(|s| s.finish(&rlog)));
        let id = self.battle_report_add(report);
        if self.telemetry.on() {
            let r = self.battle_report(id).expect("the report just added").clone();
            self.tel_engagement(&r, &e, &battle, &rlog, estimate);
        }
        self.engage_messages(id);
        Ok(EngageOutcome { report: id, result: rlog.result, plot_taken: post.plot_taken })
    }

    /// The defended plot as it is now (the report's `place`).
    fn engage_place(&self, from: PlotIdx, to: PlotIdx) -> PlotSnap {
        let plot = &self.map.plots[to];
        let (x, y) = self.map.xy(to);
        let city = plot.plot_city.filter(|&c| self.cities.contains(c)).map(|c| {
            let city = &self.cities[c];
            CitySnap { id: c, name: city.name.clone(), owner: city.owner, size: city.population, defense_pct: self.city_defense_modifier(c, false) }
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

    /// The report of engagement `e` (plan 9.1, 15.5).
    fn engage_report(&self, e: &Eng, r: &RoundLog, post: &Post, log: Option<BattleLog>) -> BattleReport {
        let sides = [Side::Attacker, Side::Defender].map(|s| {
            let (player, shown_player, players) = e.players[s.index()].clone();
            let stance = e.stances[s.index()];
            let mut lost: Vec<(UnitType, i32)> = Vec::new();
            let (mut withdrew, mut sat_out) = (0, 0);
            for i in e.of(s) {
                match r.fighters[i].fate {
                    Fate::Killed { .. } => match lost.iter_mut().find(|(t, _)| *t == e.types[i]) {
                        Some((_, n)) => *n += 1,
                        None => lost.push((e.types[i], 1)),
                    },
                    Fate::Withdrew { .. } => withdrew += 1,
                    Fate::SatOut => sat_out += 1,
                    Fate::Survived => {}
                }
            }
            lost.sort();
            ReportSide {
                player,
                shown_player,
                players,
                army: stance.army,
                garrison_of: stance.garrison_of,
                stance: stance.stance,
                slots: e.slots.max(s),
                committed: e.of(s).len() as i32,
                withdrew,
                sat_out,
                fled: if s == Side::Defender { post.fled + post.routed } else { 0 },
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
            outcome: Outcome {
                result: r.result,
                volley_rounds: r.volley_rounds,
                rounds: r.rounds,
                plot_taken: post.plot_taken,
                city_captured: post.city_captured,
                moved_in: post.moved_in,
                defenders_left: post.defenders_left,
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
        let lost = |s: &ReportSide| s.lost.iter().map(|(_, n)| n).sum::<i32>();
        let (att, def) = (&r.sides[0], &r.sides[1]);
        let result = r.outcome.result;
        let took = match (&r.outcome.city_captured, r.outcome.plot_taken) {
            (Some(_), _) => format!(" and took {place}"),
            (None, true) => " and moved in".to_string(),
            _ => String::new(),
        };
        let mut msgs: Vec<(PlayerId, String)> = Vec::new();
        for &p in &att.players {
            let verb = match result {
                EngageResult::AttackerWon => format!("won{took}"),
                EngageResult::DefenderHeld => "were repelled".to_string(),
                EngageResult::Draw => "fought to a draw".to_string(),
                EngageResult::Mutual => "both sides were spent".to_string(),
            };
            let them = self.player_get_civilization_adjective(def.shown_player);
            msgs.push((p, format!("Battle at {place}: your {} attacked {} {them} and {verb}; {} lost, {} killed (report #{id})", att.committed, def.committed, lost(att), lost(def))));
        }
        for &p in &def.players {
            let verb = match result {
                EngageResult::AttackerWon => "were defeated",
                EngageResult::DefenderHeld => "held",
                EngageResult::Draw => "fought them to a draw",
                EngageResult::Mutual => "fought until both sides were spent",
            };
            let them = self.player_get_civilization_adjective(att.shown_player);
            msgs.push((p, format!("Battle at {place}: {} {them} attacked your {}, which {verb}; {} lost, {} killed (report #{id})", att.committed, def.committed, lost(def), lost(att))));
        }
        for (p, text) in msgs {
            self.log.msg_at(turn, Some(p), at, || text);
        }
    }

    /// The strongest committed unit of side `s` with Fear that fights (plan 6.3): by base
    /// strength, then level, then the lowest id.
    fn engage_fear_source(&self, e: &Eng, s: Side) -> Option<usize> {
        e.of(s)
            .filter(|&i| !e.sat_out[i] && self.unit_is_fear(e.units[i]))
            .max_by_key(|&i| {
                let u = e.units[i];
                (self.unit_base_combat_str(u), self.units[u].level, std::cmp::Reverse(u))
            })
    }

    /// Plan 6.3 (module comment, step 2).
    fn engage_morale(&mut self, e: &mut Eng, sink: &mut Option<Sink>) {
        let d = self.d;
        let turn = self.game_turn();
        let at = self.map.xy(e.to);
        let mut staged = false;
        for (src_side, rollers) in [(Side::Defender, Side::Attacker), (Side::Attacker, Side::Defender)] {
            let Some(src) = self.engage_fear_source(e, src_side) else { continue };
            let su = e.units[src];
            let (src_str, src_level) = (self.unit_base_combat_str(su), self.units[su].level);
            for i in e.of(rollers) {
                let u = e.units[i];
                if e.sat_out[i] || self.unit_is_immune_to_fear(u) {
                    continue;
                }
                let chance = (self.unit_base_combat_str(u) + 20 + self.units[u].level - src_str - src_level).max(4);
                let roll = self.soren_rand_num(40, "Im afeared!");
                let passed = roll <= chance;
                if let Some(s) = sink.as_mut() {
                    if !staged {
                        s.stage(super::report::Stage::Morale);
                        staged = true;
                    }
                    s.morale(i, src, chance, roll, passed);
                }
                if !passed {
                    e.sat_out[i] = true;
                    let (owner, ty) = (self.units[u].owner, self.units[u].unit_type);
                    let verb = if rollers == Side::Attacker { "attack" } else { "fight" };
                    self.log.msg_at(turn, Some(owner), at, || format!("{} is afraid to {verb}", d.text.name(&d.units[ty].base.description)));
                }
            }
        }
    }

    /// Plan 6.4 steps 1 and 2 (module comment, step 3).
    fn engage_opening(&mut self, e: &mut Eng, sink: &mut Option<Sink>) {
        if let Some(s) = sink.as_mut() {
            s.stage(super::report::Stage::Opening);
        }
        for i in e.of(Side::Attacker) {
            let a = e.units[i];
            if e.sat_out[i] || self.unit_is_immune_to_defensive_strike(a) {
                continue;
            }
            let roll = self.unit_defensive_strike_roll(e.to, a);
            if let Some(best) = roll.best {
                let pct = self.unit_defensive_strike_apply(best, a);
                if pct > 0 {
                    e.strikes += 1;
                }
                if let Some(s) = sink.as_mut() {
                    let damage = self.units[best].damage;
                    let striker = s.ensure(self, e, best, Side::Defender, damage);
                    s.def_strike(striker, i, &roll, pct.max(0), pct.max(0) * self.unit_engage_max_hp(a));
                }
            }
        }
        for i in e.of(Side::Attacker) {
            let a = e.units[i];
            if e.sat_out[i] || self.unit_collateral_damage(a) <= 0 {
                continue;
            }
            let hit = self.unit_collateral_combat_ex(a, e.to, None, true);
            e.collateral += hit.len() as i32;
            if let Some(s) = sink.as_mut() {
                let limit = self.unit_collateral_damage_limit(a);
                for (t, before, after) in hit {
                    let ti = s.ensure(self, e, t, Side::Defender, before);
                    let protection = self.units[t].collateral_damage_protection;
                    s.collateral(i, ti, after - before, limit, protection, (after - before) * self.unit_engage_max_hp(t));
                }
            }
        }
    }

    /// A missile strike (plan 6.11, Q11; module comment): one hit on a unit of the defender's
    /// pick, by the missile's role order, then its collateral; the missile dies. A kill gives it
    /// the kill (no advance). `Err`: no defender there, nothing changed.
    pub(crate) fn engage_missile_strike(&mut self, m: UnitId, to: PlotIdx) -> Result<(), &'static str> {
        if !self.telemetry.on() {
            return self.engage_missile_inner(m, to);
        }
        self.tel_ctx_push("combat");
        let r = self.engage_missile_inner(m, to);
        self.tel_ctx_pop();
        r
    }

    fn engage_missile_inner(&mut self, m: UnitId, to: PlotIdx) -> Result<(), &'static str> {
        let d = self.d;
        let p = self.units[m].owner;
        let from = self.unit_plot(m);
        let ctx = HitCtx::new(from, to);
        let candidates = self.engage_defenders(to, p, &[m], WarTest::AtWar);
        if candidates.is_empty() {
            return Err("there is nobody to fight there");
        }
        let slots = self.engage_slots(&ctx);
        let stance = self.engage_defense_stance(to, &candidates);
        let pick = self.engage_auto_pick(&candidates, stance.stance, &slots, Side::Defender, &ctx);
        let in_city = self.plot_is_city(to, false, None);
        let units: Vec<UnitId> = std::iter::once(m).chain(pick.iter().copied()).collect();
        let sides: Vec<Side> = (0..units.len()).map(|i| if i == 0 { Side::Attacker } else { Side::Defender }).collect();
        let sat_out = vec![false; units.len()];
        let b = self.engage_battle(&units, &sides, &sat_out, in_city, &ctx);
        let engaged = vec![true; units.len()];
        let hp: Vec<i32> = b.fighters.iter().map(|f| f.hp).collect();
        let mut cand = Vec::new();
        let st = TargetState { engaged: &engaged, hp: &hp, pending: &[] };
        let target = match targets(&b, 0, &st, false, &mut cand, None) {
            Targets::Class { .. } => {
                let n = cand.len();
                let t = if n > 1 { cand[self.soren_rand_num(n as i32, "Engagement target") as usize] } else { cand[0] };
                Some(t as usize)
            }
            Targets::Miss(_) => None,
        };
        let turn = self.game_turn();
        let at = self.map.xy(to);
        let mut killed = None;
        if let Some(t) = target {
            let prm = b.params;
            let r = prm.rand_min + self.soren_rand_num(prm.rand_max - prm.rand_min + 1, "Engagement hit");
            let mut dmg = ((b.mean_damage(0, t) as i64 * r as i64) / 100).max(1).min(i32::MAX as i64) as i32;
            if let Some(floor) = b.cap_floor(0, t) {
                dmg = dmg.min((hp[t] - floor).max(0));
            }
            let tu = units[t];
            let left = hp[t] - dmg;
            let max_hp = b.fighters[t].max_hp;
            let (mty, tty) = (self.units[m].unit_type, self.units[tu].unit_type);
            if left <= 0 {
                self.unit_set_damage(tu, d.defines.max_hit_points, Some(p));
                killed = Some(tu);
            } else {
                self.unit_set_damage(tu, damage_from_hp(left, max_hp), None);
            }
            let verb = if left <= 0 { "destroyed" } else { "struck" };
            for owner in [p, self.units[tu].owner] {
                self.log.msg_at(turn, Some(owner), at, || format!("{} {verb} {}", d.text.name(&d.units[mty].base.description), d.text.name(&d.units[tty].base.description)));
            }
        }
        self.unit_collateral_combat_ex(m, to, None, true);
        if let Some(v) = killed {
            self.engage_kill_bookkeeping(m, v, false, to);
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
            self.tel_missile_strike(p, target.is_some(), killed.is_some());
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
        if self.engage_defenders(to, p, &[u], WarTest::AtWar).is_empty() {
            return false;
        }
        if self.unit_is_suicide(u) {
            let _ = self.engage_missile_strike(u, to);
            return true;
        }
        let Some(from) = self.unit_plot_opt(u) else { return true };
        let launcher = self.units[u].group.filter(|&g| self.groups.contains(g));
        let _ = self.engage(EngageOrder { p, from, to, launcher, force: Some(vec![u]), forced: true, log: None });
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
        setup.options[GameOptionType::GAMEOPTION_ENGAGEMENTS] = true;
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
        EngageOrder { p: PlayerId(0), from, to, launcher: None, force: None, forced: false, log: Some(log) }
    }

    fn clone(g: &Game) -> Game {
        Game::load(g.d, &g.save()).expect("a save of this build")
    }

    /// The battle log sink draws no random number and changes no state: the same engagement with
    /// and without it leaves the same RNG and the same game (plan 15 item 1).
    #[test]
    fn the_sink_changes_nothing() {
        let (g, from, to) = world(
            &["UNIT_AXEMAN", "UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_HORSEMAN", "UNIT_CATAPULT"],
            &["UNIT_AXEMAN", "UNIT_ARCHER", "UNIT_ARCHER", "UNIT_WARRIOR"],
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
}
