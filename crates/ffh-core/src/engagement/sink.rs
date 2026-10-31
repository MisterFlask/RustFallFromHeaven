//! The battle log sink (plan 15 item 1; the battle UI design's log, DESIGN.md section 10): while
//! an engagement a human could see resolves, the rules write its [`BattleLog`] here: the
//! fighters with their A and Dv parts, the picks, the opening's rolls, the round model's events
//! (the resolver's own log is the source; this formats it), the pair table of the pairs that hit
//! and the post-battle events. Writing it draws no random number and changes no state: it reads
//! the game (`&Game`) and values the rules already computed.

use super::battle::{hp_from_damage, Battle, FighterIdx, Side, TargetClass};
use super::engage::Eng;
use super::report::{BattleLog, Immunities, LogDeath, LogEvent, LogFighter, LogPair, Pick, Position, SatOutWhy, SidePick, Stage, Term, TargetRule};
use super::resolve::{BattleEvent, Resolution};
use crate::game::Game;
use crate::ids::*;
use crate::unit::DefStrikeRoll;
use ffh_data::*;

/// The log being written, and which unit each of its fighters is.
pub(crate) struct Sink {
    log: BattleLog,
    /// `(unit, side)` of each log fighter.
    index: Vec<(UnitId, Side)>,
}

/// The step of the hitter's order that chose its target, as the log names it.
fn rule_of(b: &Battle, h: usize, class: TargetClass, step: u8) -> TargetRule {
    let f = &b.fighters[h];
    let flanker = f.order.contains(&TargetClass::Screen);
    match class {
        TargetClass::HeroOrSupport => TargetRule::AssassinSupportFirst,
        TargetClass::Screen => TargetRule::FlankerScreensFirst,
        TargetClass::Back if flanker => TargetRule::FlankerAround,
        TargetClass::Back if f.ignores_shield => TargetRule::AssassinBack,
        TargetClass::Back => TargetRule::FrontRankDown,
        TargetClass::Front if step == 0 => TargetRule::FrontFirst,
        TargetClass::Front if flanker => TargetRule::FlankerFrontLast,
        TargetClass::Front if f.ignores_shield => TargetRule::AssassinFrontLast,
        TargetClass::Front => TargetRule::FrontFirst,
    }
}

/// `terms` less the terms of `headline` (a multiset difference): the pair-only terms.
fn extra(terms: Vec<Term>, headline: &[Term]) -> Vec<Term> {
    let mut left: Vec<&Term> = headline.iter().collect();
    terms
        .into_iter()
        .filter(|t| match left.iter().position(|h| *h == t) {
            Some(i) => {
                left.swap_remove(i);
                false
            }
            None => t.pct != 0,
        })
        .collect()
}

impl Sink {
    /// A log for engagement `e`: its committed fighters (in the round model's order), then
    /// `reserves`; the sides' stance picks `auto` and whether the attacker's force is the
    /// player's own.
    pub fn new(g: &Game, e: &Eng, reserves: &[(UnitId, Side)], auto: [&[UnitId]; 2], by_player: bool) -> Sink {
        let mut s = Sink {
            log: BattleLog {
                slots: e.slots.sides.clone(),
                picks: [SidePick { auto_pick: Vec::new(), picked_by: Pick::Auto }, SidePick { auto_pick: Vec::new(), picked_by: Pick::Auto }],
                fighters: Vec::new(),
                pairs: Vec::new(),
                events: Vec::new(),
                forecast: None,
            },
            index: Vec::new(),
        };
        let mut slot = [0, 0];
        for (i, &u) in e.units.iter().enumerate() {
            let side = e.side(i);
            let sl = slot[side.index()];
            slot[side.index()] += 1;
            s.push(g, e, u, side, Some(sl), g.units[u].damage);
        }
        for &(u, side) in reserves {
            s.push(g, e, u, side, None, g.units[u].damage);
        }
        for side in [Side::Attacker, Side::Defender] {
            s.log.picks[side.index()].auto_pick = auto[side.index()].iter().filter_map(|&u| s.idx(u, side)).collect();
        }
        if by_player {
            let auto_att: Vec<FighterIdx> = s.log.picks[0].auto_pick.clone();
            let force: Vec<FighterIdx> = (0..e.n_att as FighterIdx).collect();
            let outs: Vec<FighterIdx> = auto_att.iter().copied().filter(|b| !force.contains(b)).collect();
            let ins: Vec<FighterIdx> = force.iter().copied().filter(|b| !auto_att.contains(b)).collect();
            s.log.picks[0].picked_by = Pick::Player { swaps: outs.into_iter().zip(ins).collect() };
        }
        s
    }

    fn push(&mut self, g: &Game, e: &Eng, u: UnitId, side: Side, slot: Option<i32>, damage: i32) -> FighterIdx {
        let d = g.d;
        let unit = &g.units[u];
        let role = g.unit_combat_role(u);
        let info = role.map(|r| &d.combat_roles[r]);
        let max_hp = g.unit_engage_max_hp(u);
        let other_team = match side {
            Side::Attacker => e.units.get(e.n_att).map(|&du| g.unit_get_team(du)),
            Side::Defender => Some(e.team),
        };
        let withdraw_pct = match side {
            Side::Attacker => g.unit_withdrawal_probability(u),
            Side::Defender if e.in_city => 0,
            Side::Defender => g.unit_get_withdrawl_prob_defensive(u),
        };
        let immune = Immunities::default()
            .with(Immunities::FIRST_STRIKES, g.unit_immune_to_first_strikes(u))
            .with(Immunities::DEF_STRIKE, g.unit_is_immune_to_defensive_strike(u))
            .with(Immunities::FEAR, g.unit_is_immune_to_fear(u))
            .with(Immunities::COLLATERAL, unit.collateral_damage_protection >= 100);
        let f = LogFighter {
            unit: u,
            unit_type: unit.unit_type,
            name: unit.name.clone(),
            owner: unit.owner,
            shown_owner: g.unit_get_visual_owner(u, other_team),
            side,
            slot,
            level: unit.level,
            xp: unit.experience,
            hero: g.unit_is_hero(u),
            role: role.unwrap_or(CombatRoleType::from_usize(0)),
            pos: if info.is_some_and(|r| r.back) { Position::Back } else { Position::Front },
            screen: info.is_some_and(|r| r.screen),
            ignores_shield: info.is_some_and(|r| r.ignores_shield),
            assassin_profile: info.is_some_and(|r| r.ignores_shield && !r.targets.contains(&ffh_data::TargetClass::Screen)),
            guardsman: false,
            max_hp,
            hp_start: hp_from_damage(damage, max_hp),
            volleys: g.unit_first_strikes(u),
            volley_chance: g.unit_chance_first_strikes(u),
            withdraw_pct,
            def_strike: (unit.defensive_strike_chance > 0).then_some((unit.defensive_strike_chance, unit.defensive_strike_damage)),
            collateral: (g.unit_collateral_damage(u) > 0).then(|| (g.unit_collateral_damage(u), g.unit_collateral_damage_limit(u), g.unit_collateral_damage_max_units(u))),
            combat_limit: if side == Side::Attacker { unit.combat_limit } else { 100 },
            immune,
            fear: g.unit_is_fear(u),
            a: g.engage_stat(u, None, side, true, &e.ctx),
            dv: g.engage_stat(u, None, side, false, &e.ctx),
            fate: None,
        };
        self.log.fighters.push(f);
        self.index.push((u, side));
        (self.index.len() - 1) as FighterIdx
    }

    /// The log index of `u` on side `side`.
    pub fn idx(&self, u: UnitId, side: Side) -> Option<FighterIdx> {
        self.index.iter().position(|&x| x == (u, side)).map(|i| i as FighterIdx)
    }

    /// The log index of `u` on either side.
    pub fn index_of(&self, u: UnitId) -> Option<FighterIdx> {
        self.index.iter().position(|&(x, _)| x == u).map(|i| i as FighterIdx)
    }

    /// The log index of `u`, added as a reserve with damage `damage` at the start if it is not
    /// in the log yet (a defensive striker or a collateral target not among the candidates).
    pub fn ensure(&mut self, g: &Game, e: &Eng, u: UnitId, side: Side, damage: i32) -> FighterIdx {
        match self.idx(u, side) {
            Some(i) => i,
            None => self.push(g, e, u, side, None, damage),
        }
    }

    pub fn event(&mut self, ev: LogEvent) {
        self.log.events.push(ev);
    }

    pub fn stage(&mut self, stage: Stage) {
        self.event(LogEvent::Stage { stage });
    }

    /// A morale roll (fighter indices are the round model's); a failure sits the unit out.
    pub fn morale(&mut self, unit: usize, source: usize, chance: i32, roll: i32, passed: bool) {
        let (unit, source) = (unit as FighterIdx, source as FighterIdx);
        self.event(LogEvent::Morale { unit, source, chance, roll, passed });
        if !passed {
            self.event(LogEvent::SatOut { unit, why: SatOutWhy::Afraid });
        }
    }

    /// A defensive strike of log fighter `striker` on attacker `target` (round model index).
    pub fn def_strike(&mut self, striker: FighterIdx, target: usize, roll: &DefStrikeRoll, pct: i32, dmg: i32) {
        self.event(LogEvent::DefStrike { striker, target: target as FighterIdx, rolled: roll.rolled, chance: roll.chance, roll: roll.roll, pct, dmg });
    }

    pub fn collateral(&mut self, source: usize, target: FighterIdx, pct: i32, limit_pct: i32, protection_pct: i32, dmg: i32) {
        self.event(LogEvent::Collateral { source: source as FighterIdx, target, pct, limit_pct, protection_pct, dmg });
    }

    /// The round model's events (volley rolls, the volley rounds and rounds with every hit, the
    /// withdrawals, the deaths, the shield falling, the end) and the pairs that hit.
    pub fn rounds(&mut self, g: &Game, e: &Eng, b: &Battle, r: &Resolution) {
        let n = b.len();
        // The HP each withdrawal roll saw: the next round end's.
        let mut next_hp: Vec<Option<&Vec<i32>>> = vec![None; r.events.len()];
        let mut upcoming: Option<&Vec<i32>> = None;
        for (k, ev) in r.events.iter().enumerate().rev() {
            if let BattleEvent::RoundEnd { hp, .. } = ev {
                upcoming = Some(hp);
            }
            next_hp[k] = upcoming;
        }
        let mut engaged: Vec<bool> = b.fighters.iter().map(|f| !f.sat_out).collect();
        let mut leaving: Vec<bool> = vec![false; n];
        let mut pairs: Vec<(FighterIdx, FighterIdx)> = Vec::new();
        for (k, ev) in r.events.iter().enumerate() {
            match ev {
                // Phase 7's opening damage spells (none in phase 6).
                BattleEvent::Effect { .. } | BattleEvent::NoTarget { .. } => {}
                BattleEvent::Volleys { fighter, first_strikes, rolled } => {
                    let f = &b.fighters[*fighter as usize];
                    if f.chance_first_strikes > 0 {
                        self.event(LogEvent::VolleyRoll { unit: *fighter, chance: f.chance_first_strikes, roll: *rolled, total: first_strikes + rolled });
                    }
                }
                BattleEvent::RoundStart { phase } => self.stage((*phase).into()),
                BattleEvent::Hit(h) => {
                    if !pairs.contains(&(h.hitter, h.target)) {
                        pairs.push((h.hitter, h.target));
                    }
                    self.event(LogEvent::Hit {
                        h: h.hitter,
                        t: h.target,
                        rule: rule_of(b, h.hitter as usize, h.class, h.step),
                        step: h.step as i32,
                        blocked: h.blocked.iter().map(|&(s, why)| (s as i32, why)).collect(),
                        eligible: h.candidates.clone(),
                        r: h.r,
                        dmg: h.dmg,
                        capped: h.capped,
                    });
                }
                BattleEvent::Withdraw { fighter, chance, roll, ok, fatal } => {
                    let f = *fighter as usize;
                    let after = next_hp[k].map_or(b.fighters[f].hp, |hp| hp[f]);
                    let hp_pct = if *fatal { 0 } else { after.max(0) / b.fighters[f].max_hp.max(1) };
                    if *ok {
                        leaving[f] = true;
                    }
                    self.event(LogEvent::Withdraw { unit: *fighter, hp_pct, chance: *chance, roll: *roll, ok: *ok, fatal: *fatal, hp_kept: (*fatal && *ok).then_some(after) });
                }
                BattleEvent::RoundEnd { hp, deaths, left, .. } => {
                    let front_before = [Side::Attacker, Side::Defender].map(|s| b.front(s).iter().any(|&j| engaged[j as usize]));
                    let deaths: Vec<LogDeath> = deaths.iter().map(|dth| LogDeath { b: dth.fighter, killer: dth.killer, assists: dth.assists.clone(), overkill: (-hp[dth.fighter as usize]).max(0) }).collect();
                    for dth in &deaths {
                        engaged[dth.b as usize] = false;
                    }
                    for f in 0..n {
                        if leaving[f] {
                            engaged[f] = false;
                            leaving[f] = false;
                        }
                    }
                    for &l in left {
                        engaged[l as usize] = false;
                    }
                    self.event(LogEvent::RoundEnd { deaths });
                    for s in [Side::Attacker, Side::Defender] {
                        if front_before[s.index()] && !b.front(s).iter().any(|&j| engaged[j as usize]) {
                            self.event(LogEvent::ShieldDown { side: s });
                        }
                    }
                    for &l in left {
                        self.event(LogEvent::SatOut { unit: l, why: SatOutWhy::EveryTargetCapped });
                    }
                }
                BattleEvent::End { result, volley_rounds, rounds } => self.event(LogEvent::End { result: *result, volley_rounds: *volley_rounds, rounds: *rounds }),
            }
        }
        for (h, t) in pairs {
            let (hu, tu) = (h as usize, t as usize);
            let (side_h, side_t) = (b.fighters[hu].side, b.fighters[tu].side);
            let pr = b.pair(hu, tu);
            let a = g.engage_stat(e.units[hu], Some(e.units[tu]), side_h, true, &e.ctx);
            let dv = g.engage_stat(e.units[tu], Some(e.units[hu]), side_t, false, &e.ctx);
            self.log.pairs.push(LogPair {
                h,
                t,
                a_x100: pr.a,
                dv_x100: pr.dv,
                mean: b.mean_damage(hu, tu),
                a_extra: extra(a.terms, &self.log.fighters[hu].a.terms),
                dv_extra: extra(dv.terms, &self.log.fighters[tu].dv.terms),
            });
        }
    }

    /// The log, with each committed fighter's fate.
    pub fn finish(mut self, r: &Resolution) -> BattleLog {
        for (f, end) in self.log.fighters.iter_mut().zip(&r.fighters) {
            f.fate = Some(end.fate);
        }
        self.log
    }
}
