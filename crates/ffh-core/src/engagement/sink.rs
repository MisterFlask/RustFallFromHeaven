//! The battle log sink (spec section 2): while an engagement a human could see resolves, the rules
//! write its [`BattleLog`] here: the fighters on their grids with their A and Dv parts and their
//! values in the battle, the ground and its rules, the slots and lanes, the opening's rolls, every
//! action of the battle (the resolver's own events, renumbered to log indices), the pair table of
//! the pairs that hit and the post-battle events. Writing it draws no random number and changes
//! no state: it reads the game (`&Game`) and values the rules already computed.

use super::engage::{hp_from_damage, Eng, Fought};
use super::formation::{Event, FighterIdx, Formation, Outcome, Side};
use super::report::{BattleLog, FrozenForecast, Immunities, LogEvent, LogFighter, LogPair, SatOutWhy, Stage, Term};
use crate::game::Game;
use crate::ids::*;
use crate::unit::DefStrikeRoll;

/// The log being written, and which unit each of its fighters is.
pub(crate) struct Sink {
    log: BattleLog,
    /// `(unit, side)` of each log fighter (a Sentinel: no unit).
    index: Vec<(Option<UnitId>, Side)>,
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

/// `ev` with its fighters renumbered by `map` (resolver index to log index).
fn renumber(mut ev: Event, map: &[usize]) -> Event {
    let m = |k: FighterIdx| map[k as usize] as FighterIdx;
    ev.actor = m(ev.actor);
    ev.target = ev.target.map(m);
    for h in ev.hits.iter_mut() {
        h.target = m(h.target);
    }
    if let Some(h) = ev.heal.as_mut() {
        h.target = m(h.target);
    }
    if let Some(k) = ev.knocked.as_mut() {
        k.unit = m(k.unit);
    }
    ev
}

impl Sink {
    /// A log for engagement `e`: its committed units (in the engagement's order), the defended
    /// city's Sentinels (so a log index is the set-up index), then `reserves`; `ordered`: each
    /// side's formation was its order's.
    pub fn new(g: &Game, e: &Eng, reserves: &[(UnitId, Side)], ordered: [bool; 2]) -> Sink {
        let s = &e.setup;
        let mut sink = Sink {
            log: BattleLog {
                slots: s.slots.sides.clone(),
                lanes: s.lanes,
                ground: s.ground,
                rules: s.ground.rules(&s.params),
                ordered,
                fighters: Vec::new(),
                pairs: Vec::new(),
                events: Vec::new(),
                forecast: None,
            },
            index: Vec::new(),
        };
        for (i, &u) in e.units.iter().enumerate() {
            sink.push(g, e, u, e.side(i), Some(s.places[i]));
        }
        for i in s.sentinel_range() {
            sink.push_sentinel(g, e, i);
        }
        for &(u, side) in reserves {
            sink.push(g, e, u, side, None);
        }
        sink
    }

    fn push(&mut self, g: &Game, e: &Eng, u: UnitId, side: Side, place: Option<(u8, u8)>) -> FighterIdx {
        let d = g.d;
        let unit = &g.units[u];
        let max_hp = g.unit_engage_max_hp(u);
        let other_team = match side {
            Side::Attacker => e.units.get(e.setup.n_att).map(|&du| g.unit_get_team(du)),
            Side::Defender => Some(e.team),
        };
        let withdraw_pct = match side {
            Side::Attacker => g.unit_withdrawal_probability(u),
            Side::Defender if e.in_city() => 0,
            Side::Defender => g.unit_get_withdrawl_prob_defensive(u),
        };
        let immune = Immunities::default().with(Immunities::DEF_STRIKE, g.unit_is_immune_to_defensive_strike(u)).with(Immunities::FEAR, g.unit_is_immune_to_fear(u));
        let hp = hp_from_damage(unit.damage, max_hp);
        let first_strikes = g.unit_first_strikes(u);
        let f = LogFighter {
            unit: u,
            sentinel: false,
            unit_type: unit.unit_type,
            name: unit.name.clone(),
            owner: unit.owner,
            shown_owner: g.unit_get_visual_owner(u, other_team),
            side,
            place,
            large: g.unit_engage_large(u),
            level: unit.level,
            xp: unit.experience,
            hero: g.unit_is_hero(u),
            max_hp,
            hp_start: hp,
            hp_battle: hp,
            speed: g.unit_engage_speed(u) + d.defines.engagement_first_strike_speed * first_strikes.max(0),
            first_strikes,
            chance_first_strikes: g.unit_chance_first_strikes(u),
            rolled_first_strikes: 0,
            actions: g.unit_engage_actions(u),
            spell: g.unit_engage_spell(u),
            accuracy: g.unit_engage_accuracy(u),
            evasion: g.unit_engage_evasion(u),
            crit: g.unit_engage_crit(u),
            anti_flank: g.unit_engage_anti_flank(u),
            flanker: g.unit_engage_flanks(u),
            withdraw_pct,
            combat_limit: if side == Side::Attacker { unit.combat_limit } else { 100 },
            collateral_limit: (g.unit_collateral_damage(u) > 0).then(|| g.unit_collateral_damage_limit(u)),
            def_strike: (unit.defensive_strike_chance > 0).then_some((unit.defensive_strike_chance, unit.defensive_strike_damage)),
            immune,
            fear: g.unit_is_fear(u),
            a: g.engage_stat(u, None, side, true, e.ctx()),
            dv: g.engage_stat(u, None, side, false, e.ctx()),
            mag: g.engage_magic_stat(u, true),
            mdef: g.engage_magic_stat(u, false),
            fate: None,
            dealt: 0,
            taken: 0,
            healed: 0,
        };
        self.log.fighters.push(f);
        self.index.push((Some(u), side));
        (self.index.len() - 1) as FighterIdx
    }

    /// Set-up index `i`, a Sentinel of the defended city (docs/SIEGES.md), as a log fighter.
    fn push_sentinel(&mut self, g: &Game, e: &Eng, i: usize) {
        let s = &e.setup;
        let c = s.sentinel_city.expect("a Sentinel has a city");
        let lf = g.city_sentinel_log_fighter(c, s.places[i], s.fighters[i].hp);
        self.log.fighters.push(lf);
        self.index.push((None, Side::Defender));
    }

    /// The log index of `u` on side `side`.
    pub fn idx(&self, u: UnitId, side: Side) -> Option<FighterIdx> {
        self.index.iter().position(|&x| x == (Some(u), side)).map(|i| i as FighterIdx)
    }

    /// The log index of `u` on either side.
    pub fn index_of(&self, u: UnitId) -> Option<FighterIdx> {
        self.index.iter().position(|&(x, _)| x == Some(u)).map(|i| i as FighterIdx)
    }

    /// The log index of `u`, added as a reserve if it is not in the log yet (a defensive striker
    /// not among the candidates).
    pub fn ensure(&mut self, g: &Game, e: &Eng, u: UnitId, side: Side) -> FighterIdx {
        match self.idx(u, side) {
            Some(i) => i,
            None => self.push(g, e, u, side, None),
        }
    }

    /// The plan as the attacker saw it.
    pub fn set_forecast(&mut self, forecast: Option<FrozenForecast>) {
        self.log.forecast = forecast;
    }

    pub fn event(&mut self, ev: LogEvent) {
        self.log.events.push(ev);
    }

    pub fn stage(&mut self, stage: Stage) {
        self.event(LogEvent::Stage { stage });
    }

    /// A morale roll (set-up indices: log indices); a failure sits the unit out.
    pub fn morale(&mut self, unit: usize, source: usize, chance: i32, roll: i32, passed: bool) {
        let (unit, source) = (unit as FighterIdx, source as FighterIdx);
        self.event(LogEvent::Morale { unit, source, chance, roll, passed });
        if !passed {
            self.event(LogEvent::SatOut { unit, why: SatOutWhy::Afraid });
        }
    }

    /// A defensive strike of log fighter `striker` on attacker `target` (set-up index).
    pub fn def_strike(&mut self, striker: FighterIdx, target: usize, roll: &DefStrikeRoll, pct: i32, dmg: i32) {
        self.event(LogEvent::DefStrike { striker, target: target as FighterIdx, rolled: roll.rolled, chance: roll.chance, roll: roll.roll, pct, dmg });
    }

    /// The battle as the resolver fought it (`f`, its fighters' log indices `map`, `out`): every
    /// action, the end, the fighters' values in the battle (HP when it began, speed, the first
    /// strikes rolled, damage dealt, taken and healed) and the pairs that hit.
    pub fn battle(&mut self, g: &Game, e: &Eng, f: &Formation, map: &[usize], out: &Outcome) {
        self.stage(Stage::Battle);
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for ev in &out.events {
            for h in &ev.hits {
                let (hk, tk) = (ev.actor as usize, h.target as usize);
                if !h.miss && f.fighters[hk].side != f.fighters[tk].side && !pairs.contains(&(hk, tk)) {
                    pairs.push((hk, tk));
                }
            }
            self.event(LogEvent::Action(renumber(ev.clone(), map)));
        }
        self.event(LogEvent::End { rounds: out.rounds as i32, down: out.down });
        for (k, end) in out.fighters.iter().enumerate() {
            let lf = &mut self.log.fighters[map[k]];
            lf.hp_battle = f.fighters[k].hp;
            lf.speed = end.speed;
            lf.rolled_first_strikes = end.rolled_first_strikes;
            lf.dealt = end.dealt;
            lf.taken = end.taken;
            lf.healed = end.healed;
        }
        for (hk, tk) in pairs {
            let (h, t) = (map[hk], map[tk]);
            let (side_h, side_t) = (e.side(h), e.side(t));
            let pr = f.pair(hk, tk);
            // A Sentinel has no unit terms: a pair with one has no pair-only terms.
            let (a_extra, dv_extra) = if e.setup.is_sentinel(h) || e.setup.is_sentinel(t) {
                (Vec::new(), Vec::new())
            } else {
                let a = g.engage_stat(e.units[h], Some(e.units[t]), side_h, true, e.ctx());
                let dv = g.engage_stat(e.units[t], Some(e.units[h]), side_t, false, e.ctx());
                (extra(a.terms, &self.log.fighters[h].a.terms), extra(dv.terms, &self.log.fighters[t].dv.terms))
            };
            let base = e.setup.params.base_damage.max(0) as i64 * 100;
            self.log.pairs.push(LogPair {
                h: h as FighterIdx,
                t: t as FighterIdx,
                a_x100: pr.a,
                dv_x100: pr.dv,
                mean: (base * pr.a as i64 / (pr.a as i64 + pr.dv as i64)) as i32,
                ma_x100: pr.ma,
                md_x100: pr.md,
                spell_pct: pr.spell_pct,
                magic_mean: (base * pr.ma as i64 / (pr.ma as i64 + pr.md as i64)) as i32,
                a_extra,
                dv_extra,
            });
        }
    }

    /// The log, with each committed unit's fate.
    pub fn finish(mut self, fought: &Fought) -> BattleLog {
        for (f, fate) in self.log.fighters.iter_mut().zip(&fought.fate) {
            f.fate = Some(*fate);
        }
        self.log
    }
}
