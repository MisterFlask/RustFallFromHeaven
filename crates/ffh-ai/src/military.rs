//! Field armies: stacks that hunt barbarians and animals near our lands, clear lairs (an AI
//! unit entering a lair destroys it, FfH `CvUnit::setXY`), and take barbarian and enemy cities
//! in coordinated operations (`campaign.rs`).
//!
//! A stack is a selection group led by an army unit; army units meeting on a plot merge (up
//! to [`MAX_STACK`]). Attacks go through the rules' group attack (`MISSION_MOVE_TO` with
//! `MOVE_DIRECT_ATTACK`: the best attacker fights, or a sacrifice when the odds are poor),
//! repeated while the odds of the best attacker against the best defender, given how many
//! attackers remain for how many defenders, make the assault worth it (C++
//! `AI_groupAttack`/`AI_attackCityMove` follow-through).

use ffh_core::defs::*;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::production::garrison;
use crate::units::{alive, hold, move_to, nearest_city, skip};
use crate::view::{attack_power, defense_power, View};

/// Largest stack we build.
pub const MAX_STACK: usize = 8;

/// A target for a field army (computed once per turn in the view).
#[derive(Copy, Clone, Debug)]
pub struct Target {
    pub plot: PlotIdx,
    pub value: i32,
    /// Enemy (non-animal or animal) units visible there.
    pub units: i32,
    pub city: bool,
    pub lair: bool,
}

/// A field unit of ours the AI's stacks command (attack, city attack, collateral, counter, reserve
/// and pillage roles, on land): not a marching recruit, which goes where the rules take it
/// (docs/ARMIES.md), so no stack takes it in, no transport loads it and no count of our stacks has
/// it.
pub fn is_army_unit(g: &Game, u: UnitId) -> bool {
    let unit = &g.units[u];
    let info = &g.d.units[unit.unit_type];
    unit.march.is_none()
        && info.combat > 0
        && !info.found
        && g.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
        && matches!(
            unit.ai_type,
            Some(UnitAIType::UNITAI_ATTACK) | Some(UnitAIType::UNITAI_ATTACK_CITY) | Some(UnitAIType::UNITAI_COLLATERAL) | Some(UnitAIType::UNITAI_COUNTER) | Some(UnitAIType::UNITAI_RESERVE) | Some(UnitAIType::UNITAI_PILLAGE)
        )
}

/// Units of the group able to attack now (not the casters that march with it without fighting
/// for it, `magic::spared_caster`).
fn attackers(g: &Game, grp: GroupId) -> Vec<UnitId> {
    g.groups[grp].units.iter().copied().filter(|&u| crate::campaign::fights(g, u) && g.unit_moves_left(u) > 0).collect()
}

/// Fight estimate against a plot.
#[derive(Copy, Clone, Debug)]
pub struct Assessment {
    /// Best odds (per mille) of one of our attackers against their best defender.
    pub best: i32,
    /// Defenders there.
    pub n_def: i32,
    /// Our attackers able to fight now.
    pub n_att: i32,
    /// Expected "defender kills" of the whole stack, per mille: each attacker counts its odds
    /// plus a third of the rest (a lost fight still wounds the defender); collateral
    /// attackers count more against stacks.
    pub effective: i32,
}

/// Fight estimate against plot `q`.
pub(crate) fn assess(g: &Game, grp: GroupId, q: PlotIdx) -> Option<Assessment> {
    let mut atts = attackers(g, grp);
    if atts.is_empty() {
        return None;
    }
    atts.sort_by_key(|&u| (std::cmp::Reverse(attack_power(g, u)), u));
    let owner = g.groups[grp].owner;
    let mut best = 0;
    let mut n_def = 0;
    let mut effective = 0;
    let mut any = false;
    for &u in atts.iter().take(MAX_STACK) {
        if !g.unit_can_move_or_attack_into(u, q, false) {
            continue;
        }
        any = true;
        let Some(def) = g.plot_get_best_defender(q, None, Some(owner), Some(u), false, false, false) else {
            return Some(Assessment { best: 1000, n_def: 0, n_att: atts.len() as i32, effective: 1000 * atts.len() as i32 });
        };
        let odds = g.get_combat_odds(u, def);
        best = best.max(odds);
        n_def = n_def.max(g.plot_get_num_visible_enemy_defenders(q, u));
        effective += odds + (1000 - odds) / 3;
        if g.d.units[g.units[u].unit_type].collateral_damage > 0 {
            effective += 250;
        }
    }
    if !any {
        return None;
    }
    Some(Assessment { best, n_def: n_def.max(1), n_att: atts.len() as i32, effective })
}

/// The group attack of `grp` on plot `q` played in expected values as the rules will fight it
/// (`combat_sim::simulate`, [`Order::Group`](crate::combat_sim::Order::Group)): the attackers
/// that can attack there, against the defenders we see. Raiders on our land
/// or within three plots of one of our cities (any enemy but animals, which stay out of our
/// borders) count four times what they are worth: left alone they take the city. (Valued at face
/// value, the stacks let them be, and more of our cities fell to the barbarians: docs/AI.md,
/// round 7g.)
pub(crate) fn field_sim(g: &Game, grp: GroupId, q: PlotIdx) -> Option<crate::combat_sim::Sim> {
    let atts: Vec<UnitId> = attackers(g, grp).into_iter().filter(|&u| g.unit_can_move_or_attack_into(u, q, false)).collect();
    if atts.is_empty() {
        return None;
    }
    let p = g.groups[grp].owner;
    let team = g.players[p].team;
    let defs = crate::combat_sim::visible_defenders(g, team, q);
    let animals = !defs.is_empty() && defs.iter().all(|&d| g.d.units[g.units[d].unit_type].animal);
    let ours = g.map.plots[q].owner.is_some_and(|o| g.players[o].team == team);
    let (x, y) = g.map.xy(q);
    let near_city = ours || g.players[p].cities.iter().any(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) <= 3);
    let kill_pct = if !animals && near_city { 400 } else { 100 };
    Some(crate::combat_sim::simulate(g, q, &defs, &atts, 0, crate::combat_sim::Order::Group, kill_pct, 200))
}

/// Is the group attack [`field_sim`] played worth starting (or, `fought`, going on with)? As an
/// expert decides, on the simulation's numbers. Nothing to fight (a worker to capture, our own
/// units there): yes. The attacker the rules send has good odds (65%; 75% for a `hurt` stack):
/// yes -- a raider, an animal, a shot at a barbarian town's garrison. Wearing the defenders down
/// at worse odds: only when the stack clears the plot (every defender 80% sure to fall) and the
/// kills are worth more than the losses (`gain`). A city to `capture` (an opportune capture, a
/// landing): when it falls; once the fighting has started, also on good odds or a winning trade.
/// (Before round 7g -- switch `FIELD_SIM` off, removed once kept -- `attack_ok`, which counted a
/// third of a kill for every lost fight whatever the odds,
/// the defender's strength or its first strikes -- it sent four Warriors at a Griffon at 1% -- and
/// took the odds against a defender at full health for each attacker. Measured first as a pure
/// trade of what kills and losses are worth, the stacks traded lone units at 35-50% odds and the
/// player fell behind in units by turn 100: docs/AI.md, round 7g.)
pub(crate) fn field_go(s: &crate::combat_sim::Sim, city: bool, capture: bool, fought: bool, hurt: bool) -> bool {
    if s.n_def == 0 {
        return true;
    }
    let falls = s.used.is_some();
    if city && capture && !fought {
        return falls;
    }
    let good = s.first >= if hurt { 750 } else { 650 };
    good || falls && s.gain > 0
}

// ----------------------------------------------------------------------------
// The field rule on the rules' battle forecast (switch `FORECAST`)
// ----------------------------------------------------------------------------

/// Attack when the plot falls this turn with this chance (per mille; the forecast's `p_take`,
/// calibrated: docs/CALIBRATION.md) ...
pub const TAKE_BAR: i32 = 650;
/// ... with this chance for a hurt stack (a unit over 35% damage; round 7g's 75%) ...
pub const TAKE_BAR_HURT: i32 = 750;
/// ... or with this chance when the trade is favourable: more enemy units are expected to fall
/// than ours, the enemy's counted at [`RAIDER_KILL_PCT`] when they are raiders near our cities
/// (round 7g's `gain > 0`, in units: what kills and losses are worth on each side).
pub const TRADE_BAR: i32 = 500;
/// A city to capture (an opportune capture, a landing) is attacked when it falls with this chance.
pub const CAPTURE_BAR: i32 = 650;
/// Raiders (any enemy but animals) on our land or within three plots of one of our cities count
/// this percent in the trade: left alone they take the city (round 7g).
pub const RAIDER_KILL_PCT: i32 = 400;
/// The forecast is skipped, and the attack declined, when the attack power of our units that
/// would fight is under this percent of the defence power of the defenders we see (the rules'
/// strengths without the ground, the city or fortification, which only help the defender).
pub const HOPELESS_PCT: i32 = 25;

/// Does player `p` decide its attacks on the forecast (switch `FORECAST`; never the barbarians)?
pub(crate) fn forecast_on(g: &Game, p: PlayerId) -> bool {
    !g.player_is_barbarian(p) && crate::switches::FORECAST.on(g, p)
}

/// The percent the enemy units on `q` count in a trade for `p` ([`RAIDER_KILL_PCT`] for raiders
/// near our cities, else 100): `field_sim`'s weight of round 7g.
fn kill_pct(g: &Game, p: PlayerId, defs: &[UnitId], q: PlotIdx) -> i32 {
    let team = g.players[p].team;
    let animals = !defs.is_empty() && defs.iter().all(|&d| g.d.units[g.units[d].unit_type].animal);
    let ours = g.map.plots[q].owner.is_some_and(|o| g.players[o].team == team);
    let (x, y) = g.map.xy(q);
    let near_city = ours || g.players[p].cities.iter().any(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) <= 3);
    if !animals && near_city {
        RAIDER_KILL_PCT
    } else {
        100
    }
}

/// What a field attack is decided against.
#[derive(Copy, Clone, Debug)]
pub(crate) enum Rule {
    /// The field rule ([`field_rule`]).
    Field { city: bool, capture: bool, fought: bool, hurt: bool },
    /// A single unit's attack at a bar of its own (`units::attack_adjacent`: an explorer, a
    /// sortie), per mille of the chance the plot falls.
    Bar(i32),
}

impl Rule {
    /// The bars the decision reads (where a first look is checked again).
    fn bars(self) -> Vec<i32> {
        match self {
            Rule::Field { city: true, capture: true, fought: false, .. } => vec![CAPTURE_BAR],
            Rule::Field { hurt, .. } => vec![if hurt { TAKE_BAR_HURT } else { TAKE_BAR }, TRADE_BAR],
            Rule::Bar(b) => vec![b],
        }
    }
}

/// The field rule on the forecast `e` (round 7g's shape, the bars meaning what they say): a city
/// to capture when it falls ([`CAPTURE_BAR`]); else when the plot falls likely ([`TAKE_BAR`],
/// [`TAKE_BAR_HURT`]), or at worse odds ([`TRADE_BAR`]) when the trade is favourable (the enemy's
/// expected losses, at `kill_pct` percent, above ours). A capture once the fighting has started
/// goes on as any attack.
pub(crate) fn field_rule(e: &crate::forecast::Est, kill_pct: i32, rule: Rule) -> bool {
    let trade = e.enemy_lost_x100 as i64 * kill_pct as i64 > e.own_lost_x100 as i64 * 100;
    match rule {
        Rule::Field { city: true, capture: true, fought: false, .. } => e.p_take >= CAPTURE_BAR,
        Rule::Field { hurt, .. } => e.p_take >= if hurt { TAKE_BAR_HURT } else { TAKE_BAR } || e.p_take >= TRADE_BAR && trade,
        Rule::Bar(b) => e.p_take >= b,
    }
}

/// A hero or a veteran of ours (switch `PRECIOUS`, `campaign::precious_bar`) whose chance to fall
/// in the forecast `f` is above what its bar allows (a hero at 85% falls at most 15% of the
/// time): `(unit, fall percent, bar)`. Under engagements every unit of the plot fights, heroes
/// included, so the gate reads each one's fall instead of the first attacker's odds.
fn precious_falls(g: &Game, p: PlayerId, f: &ffh_core::engagement::Forecast) -> Option<(UnitId, i32, i32)> {
    if !crate::switches::PRECIOUS.on(g, p) {
        return None;
    }
    f.per.iter().filter(|x| x.side == ffh_core::engagement::Side::Attacker && g.units.contains(x.unit) && g.units[x.unit].owner == p).find_map(|x| {
        let bar = crate::campaign::precious_bar(g, x.unit);
        (bar > 0 && x.fall_pct * 10 > 1000 - bar).then_some((x.unit, x.fall_pct, bar))
    })
}

/// A field attack's answer on the forecast ([`field_call`]).
#[derive(Clone, Debug)]
pub(crate) struct Call {
    pub go: bool,
    /// Why: `no_defenders`, `no_attacker`, `hopeless` (the pre-filter), `refused` (the rules'
    /// forecast refused the battle), `precious`, `forecast`.
    pub why: &'static str,
    pub est: Option<crate::forecast::Est>,
    pub kill_pct: i32,
    pub precious: Option<(UnitId, i32, i32)>,
}

/// The real pool of `p`'s units on `from` that would fight an attack on `q` now, as one attack.
pub(crate) fn pool_attack(g: &Game, p: PlayerId, from: PlotIdx, q: PlotIdx) -> crate::forecast::Attacks {
    let pool = g.engage_attackers(p, from, q, ffh_core::engagement::WarTest::AtWar);
    if pool.is_empty() {
        Vec::new()
    } else {
        vec![(pool, from)]
    }
}

/// The attack of `attacks` (player `p`'s units, each group from its plot next to `q`) on plot
/// `q`, decided on the forecast ([`field_rule`] or a bar, `rule`): nothing to fight there is a go;
/// a hopeless attack ([`HOPELESS_PCT`]) is declined without a forecast; the forecast
/// ([`crate::forecast::decide_turn`]) decides, and a precious unit that would fall too likely
/// holds the attack. With telemetry on, the decision is recorded beside what round 7g's rule
/// says (`old`, computed by the caller) as decision `attack_rule` `<site>:<agree_go | agree_hold
/// | new_attack | new_decline>` (`site` `None`: not recorded).
pub(crate) fn field_call(g: &Game, v: &View, p: PlayerId, attacks: &crate::forecast::Attacks, q: PlotIdx, rule: Rule, site: Option<&'static str>, old: impl FnOnce() -> Option<(bool, crate::combat_sim::Sim)>) -> Call {
    let team = g.players[p].team;
    // The defenders the attack meets as the rules pick them (hidden nationality included, never
    // an invisible unit), where the AI knows the units (`view::sees_units`; else none, and the
    // stack walks in as before).
    let force: Vec<UnitId> = attacks.iter().flat_map(|a| a.0.iter().copied()).collect();
    let defs = if crate::view::sees_units(g, team, q) { g.engage_defenders(q, p, &force, ffh_core::engagement::WarTest::AtWar) } else { Vec::new() };
    let kp = kill_pct(g, p, &defs, q);
    let mut call = Call { go: false, why: "forecast", est: None, kill_pct: kp, precious: None };
    let checked = std::cell::Cell::new(-1);
    let tel = g.telemetry.on();
    if defs.is_empty() {
        call.go = true;
        call.why = "no_defenders";
    } else if attacks.iter().all(|a| a.0.is_empty()) {
        call.why = "no_attacker";
    } else {
        let ours: i64 = attacks.iter().flat_map(|a| a.0.iter()).map(|&u| attack_power(g, u) as i64).sum();
        let theirs: i64 = defs.iter().map(|&d| defense_power(g, d) as i64).sum();
        let hopeless = ours * 100 < theirs * HOPELESS_PCT as i64;
        if !hopeless || tel {
            match crate::forecast::decide_turn(g, Some(&v.forecasts), attacks, q, &rule.bars(), tel.then_some(&checked)) {
                None => call.why = "refused",
                Some(f) => {
                    let e = crate::forecast::Est::of(&f);
                    call.est = Some(e);
                    call.go = field_rule(&e, kp, rule);
                    if call.go {
                        call.precious = precious_falls(g, p, &f);
                        if call.precious.is_some() {
                            call.go = false;
                            call.why = "precious";
                        }
                    }
                }
            }
        }
        if hopeless {
            // (The ledger of the pre-filter: how often the forecast would have attacked.)
            if call.go {
                g.tel_decision(p, "attack_rule", "hopeless_contradicted", "", 1, String::new);
            }
            g.tel_decision(p, "attack_rule", "hopeless", "", 1, String::new);
            call.go = false;
            call.why = "hopeless";
        }
    }
    if tel {
        if let Some(site) = site {
            let o = old();
            let old_go = o.as_ref().is_some_and(|x| x.0);
            let class = match (old_go, call.go) {
                (true, true) => "agree_go",
                (false, false) => "agree_hold",
                (false, true) => "new_attack",
                (true, false) => "new_decline",
            };
            let (x, y) = g.map.xy(q);
            g.tel_decision(p, "attack_rule", &format!("{site}:{class}"), call.why, 1, || {
                let e = call.est.unwrap_or_default();
                let s = o.map(|x| x.1).unwrap_or_default();
                format!(
                    "{{\"site\":\"{site}\",\"class\":\"{class}\",\"x\":{x},\"y\":{y},\"go\":{},\"old\":{old_go},\"why\":\"{}\",\"p_take\":{},\"p_wipe\":{},\"p_repulsed\":{},\"own_lost_x100\":{},\"enemy_lost_x100\":{},\"n_att\":{},\"n_def\":{},\"kill_pct\":{kp},\"full\":{},\"old_first\":{},\"old_falls\":{},\"old_gain\":{},\"old_p_fall\":{},\"n_defs\":{},\"city\":{},\"from\":[{}]}}",
                    call.go,
                    call.why,
                    e.p_take,
                    e.p_wipe,
                    e.p_repulsed,
                    e.own_lost_x100,
                    e.enemy_lost_x100,
                    e.n_att,
                    e.n_def,
                    checked.get(),
                    s.first,
                    s.used.is_some(),
                    s.gain,
                    s.p_fall,
                    defs.len(),
                    g.map.plots[q].plot_city.is_some(),
                    attacks.iter().map(|a| { let (ax, ay) = g.map.xy(a.1); format!("[{ax},{ay},{}]", a.0.len()) }).collect::<Vec<_>>().join(",")
                )
            });
        }
        if let Some(e) = call.est.filter(|_| checked.get() >= 0) {
            // A first look the full forecast would have decided the other way.
            let full = crate::forecast::Est { p_take: checked.get(), ..e };
            if field_rule(&full, kp, rule) != field_rule(&e, kp, rule) {
                g.tel_decision(p, "attack_rule", "fast_overturned", "", 1, String::new);
            }
            g.tel_decision(p, "attack_rule", "fast_checked", "", 1, String::new);
        }
    }
    call
}

/// The attacker the rules would send from `grp` into `q` (`AI_getBestGroupAttacker`, which already
/// puts heroes, veterans and war wizards last) is a hero or a veteran below its odds (switch
/// `PRECIOUS`, `campaign::precious_bar`), or a caster marching with the stack below war wizard
/// odds (switch `CASTER_STACKS`, `magic::spared_caster`): `(unit, odds, bar)`. The others are worse still, so the stack does
/// not attack there -- and does not pick the target either: a Balseraph stack whose best attacker
/// was a level-5 Warrior at 35% stood next to the same raider for 12 turns (docs/AI.md, "Forks of
/// the hero gate").
fn precious_holds(g: &Game, grp: GroupId, q: PlotIdx) -> Option<(UnitId, i32, i32)> {
    let p = g.units[g.group_get_head_unit(grp)?].owner;
    let precious = crate::switches::PRECIOUS.on(g, p);
    if !precious && !crate::switches::CASTER_STACKS.on(g, p) {
        return None;
    }
    let b = g.group_ai_get_best_group_attacker(grp, q, false, false, false).0?;
    // A caster that marches with the stack (`magic::spared_caster`): the rules send it below war
    // wizard odds only when no other unit of the stack can attack; then the stack stops too.
    let caster_bar = if crate::magic::spared_caster(g, b) { crate::magic::SPARED_CASTER_ODDS } else { 0 };
    let bar = if precious { crate::campaign::precious_bar(g, b) } else { 0 }.max(caster_bar);
    if bar == 0 {
        return None;
    }
    let odds = g.plot_get_best_defender(q, None, Some(p), Some(b), false, false, false).map_or(1000, |d| g.get_combat_odds(b, d));
    (odds < bar).then_some((b, odds, bar))
}

/// Round 7g's answer for the record beside the forecast's ([`field_call`]): `field_go` on
/// [`field_sim`], and with `precious` the hero gate ([`precious_holds`]).
fn old_field(g: &Game, grp: GroupId, q: PlotIdx, city: bool, capture: bool, fought: bool, hurt: bool, precious: bool) -> Option<(bool, crate::combat_sim::Sim)> {
    let s = field_sim(g, grp, q)?;
    Some((field_go(&s, city, capture, fought, hurt) && (!precious || precious_holds(g, grp, q).is_none()), s))
}

/// [`assault`] on the forecast (switch `FORECAST`): each attack is one engagement of the pool on
/// the group's plot (the formation, then its reserve on the next order), decided before each on
/// the forecast of the battles the units left there fight this turn ([`field_call`]).
fn assault_forecast(g: &mut Game, v: &View, grp: GroupId, q: PlotIdx) -> bool {
    let p = v.p;
    let mut fought = false;
    let city = g.map.plots[q].plot_city.is_some();
    let Some(head) = g.group_get_head_unit(grp) else { return false };
    let from = g.unit_plot(head);
    let rule = |fought: bool| Rule::Field { city, capture: false, fought, hurt: false };
    let first = field_call(g, v, p, &pool_attack(g, p, from, q), q, rule(false), Some("assault"), || old_field(g, grp, q, city, false, false, false, true));
    if g.telemetry.on() {
        if let Some(s) = field_sim(g, grp, q) {
            crate::predict::field(g, p, q, &s, city, first.go, "assault");
        }
        crate::predict::field2(g, Some(&v.forecasts), p, from, q, city, first.go, "assault");
    }
    if first.go {
        let atts = attackers(g, grp);
        crate::magic::before_attack(g, v, q, &atts);
    }
    for _ in 0..12 {
        if !alive(g, grp) {
            break;
        }
        let Some(head) = g.group_get_head_unit(grp) else { break };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) {
            break;
        }
        let from = g.unit_plot(head);
        let c = field_call(g, v, p, &pool_attack(g, p, from, q), q, rule(fought), fought.then_some("assault_more"), || old_field(g, grp, q, city, false, fought, false, true));
        crate::trace!(g, p, "  assault {:?} on {:?} forecast: {:?}", grp, g.map.xy(q), c);
        if !c.go {
            if let Some((b, fall, bar)) = c.precious {
                crate::trace!(g, p, "stack {:?} holds: {} would fall {}% (wants {})", grp, g.d.units[g.units[b].unit_type].base.type_, fall, bar);
                g.tel_decision(p, "military", "precious_held", "assault", 1, String::new);
            }
            break;
        }
        let pool = g.engage_attackers(p, from, q, ffh_core::engagement::WarTest::AtWar);
        // The first battle is the group's order; the next ones (the reserve the formation left)
        // are ordered by a unit of the pool that can still attack, out of its group: a group
        // whose other units have attacked cannot start a move (before, the forecast counted the
        // reserve and the group's order then did nothing).
        let launcher = if !fought {
            Some(grp)
        } else {
            pool.iter().copied().max_by_key(|&o| (g.units[o].group == Some(grp), g.units[o].combat_limit, attack_power(g, o), std::cmp::Reverse(o))).and_then(|o| {
                if g.units[o].group.is_some_and(|og| g.groups[og].units.len() > 1) {
                    g.unit_join_group(o, None, true);
                }
                g.units[o].group
            })
        };
        let Some(lg) = launcher else { break };
        if g.telemetry.on() {
            if let Some(b) = g.group_ai_get_best_group_attacker(lg, q, false, false, false).0 {
                let odds = g.plot_get_best_defender(q, None, Some(p), Some(b), false, false, false).map_or(1000, |d| g.get_combat_odds(b, d));
                crate::predict::attack(g, p, b, q, odds, "field");
                crate::predict::attack2(g, Some(&v.forecasts), p, b, q, "field");
            }
        }
        g.groups[lg].ai_group_attack = false;
        crate::tactics::note_attack(g, p, from, q);
        move_to(g, lg, q, MOVE_DIRECT_ATTACK);
        fought = true;
        if alive(g, lg) {
            g.groups[lg].ai_group_attack = false;
        }
        if !alive(g, grp) {
            break;
        }
        // Nothing happened (no attack possible): stop.
        if g.engage_attackers(p, from, q, ffh_core::engagement::WarTest::AtWar) == pool {
            break;
        }
    }
    if alive(g, grp) {
        g.groups[grp].ai_group_attack = false;
        if let Some(head) = g.group_get_head_unit(grp) {
            if g.unit_plot(head) != q && !g.plot_is_visible_enemy_unit_for_unit(q, head) && g.unit_moves_left(head) > 0 && g.group_can_move_into(grp, q, false) {
                move_to(g, grp, q, 0);
            }
        }
    }
    fought
}

/// Attack plot `q` with the group until it falls or the odds turn bad. True if we fought.
pub fn assault(g: &mut Game, v: &View, grp: GroupId, q: PlotIdx) -> bool {
    if forecast_on(g, v.p) {
        return assault_forecast(g, v, grp, q);
    }
    let mut fought = false;
    let city = g.map.plots[q].plot_city.is_some();
    // Casters of ours around the target cast first when the assault goes ahead (magic.rs).
    let sim = field_sim(g, grp, q);
    let goes = sim.is_some_and(|s| field_go(&s, city, false, false, false));
    if let Some(s) = sim.filter(|_| g.telemetry.on()) {
        crate::predict::field(g, v.p, q, &s, city, goes, "assault");
        if let Some(h) = g.group_get_head_unit(grp) {
            crate::predict::field2(g, Some(&v.forecasts), v.p, g.unit_plot(h), q, city, goes, "assault");
        }
    }
    if goes {
        let atts = attackers(g, grp);
        crate::magic::before_attack(g, v, q, &atts);
    }
    for _ in 0..12 {
        if !alive(g, grp) {
            break;
        }
        let Some(head) = g.group_get_head_unit(grp) else { break };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) {
            break;
        }
        let Some(a) = assess(g, grp, q) else { break };
        let (n_def, n_att) = (a.n_def, a.n_att);
        crate::trace!(g, v.p, "  assault {:?} on {:?}: {:?}", grp, g.map.xy(q), a);
        if n_def == 0 {
            break;
        }
        let s = field_sim(g, grp, q);
        crate::trace!(g, v.p, "  assault {:?} on {:?} simulated: {:?} go {}", grp, g.map.xy(q), s, s.is_some_and(|s| field_go(&s, city, false, fought, false)));
        if !s.is_some_and(|s| field_go(&s, city, false, fought, false)) {
            break;
        }
        // The attacker the rules would send is a hero or a veteran below its odds: the stack stops
        // here.
        if let Some((b, odds, bar)) = precious_holds(g, grp, q) {
            crate::trace!(g, v.p, "stack {:?} holds: its next attacker {} has odds {} (wants {})", grp, g.d.units[g.units[b].unit_type].base.type_, odds, bar);
            g.tel_decision(v.p, "military", "precious_held", "assault", 1, String::new);
            break;
        }
        if g.telemetry.on() {
            if let (Some(b), Some(s)) = (g.group_ai_get_best_group_attacker(grp, q, false, false, false).0, s) {
                crate::predict::attack(g, v.p, b, q, s.first, "field");
                crate::predict::attack2(g, Some(&v.forecasts), v.p, b, q, "field");
            }
        }
        g.groups[grp].ai_group_attack = false;
        let before = g.groups[grp].units.len();
        crate::tactics::note_attack(g, v.p, g.unit_plot(head), q);
        move_to(g, grp, q, MOVE_DIRECT_ATTACK);
        fought = true;
        if !alive(g, grp) {
            break;
        }
        // Nothing happened (no attack possible): stop.
        if g.groups[grp].units.len() == before && attackers(g, grp).len() as i32 == n_att {
            break;
        }
    }
    if alive(g, grp) {
        g.groups[grp].ai_group_attack = false;
        // The defenders are gone: move in (capture a city, destroy a lair, take workers).
        if let Some(head) = g.group_get_head_unit(grp) {
            if g.unit_plot(head) != q && !g.plot_is_visible_enemy_unit_for_unit(q, head) && g.unit_moves_left(head) > 0 && g.group_can_move_into(grp, q, false) {
                move_to(g, grp, q, 0);
            }
        }
    }
    fought
}

/// Merge other army units on this plot into `grp` (up to [`MAX_STACK`]).
fn merge_here(g: &mut Game, v: &View, grp: GroupId, here: PlotIdx) {
    if g.groups[grp].units.len() >= MAX_STACK {
        return;
    }
    let others: Vec<UnitId> = g.map.plots[here].units.iter().copied().filter(|&o| g.units[o].owner == v.p && g.units[o].group != Some(grp) && is_army_unit(g, o)).collect();
    for o in others {
        if g.groups[grp].units.len() >= MAX_STACK {
            break;
        }
        // Only fresh units join (a group whose members have different moves left would keep
        // the turn from ending).
        if g.unit_plot(o) == here && g.unit_moves_left(o) == g.unit_max_moves(o) && g.unit_can_join_group(o, here, grp) {
            g.unit_join_group(o, Some(grp), true);
        }
    }
}

/// Enemy city our army marches on: this turn's war operation (`campaign.rs`), else the
/// closest known enemy city to any of our cities.
pub fn war_target(g: &Game, v: &View) -> Option<CityId> {
    // (The plan was made earlier this turn: the city may have fallen or been razed since.)
    if let Some(c) = v.campaign.war_target().filter(|&c| g.cities.contains(c) && g.cities[c].owner != v.p) {
        return Some(c);
    }
    let preparing = crate::diplomacy::preparing_target(g, v.team);
    if v.civ_enemies.is_empty() && preparing.is_none() {
        return None;
    }
    let ours: Vec<(i32, i32, Option<AreaId>)> = g.players[v.p].cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y, g.map.plots[g.city_plot(c)].area)).collect();
    let mut best: Option<(CityId, i32)> = None;
    for pl in g.players.iter() {
        if !pl.alive || !(v.civ_enemies.contains(&pl.team) || v.civ_enemies.is_empty() && preparing == Some(pl.team)) {
            continue;
        }
        for &c in &pl.cities {
            let cp = g.city_plot(c);
            if !g.map.plots[cp].revealed[v.team] {
                continue;
            }
            let (cx, cy) = (g.cities[c].x, g.cities[c].y);
            let area = g.map.plots[cp].area;
            let Some(dist) = ours.iter().filter(|o| o.2 == area).map(|&(x, y, _)| g.plot_distance(x, y, cx, cy)).min() else { continue };
            if best.map_or(true, |(bc, bd)| dist < bd || (dist == bd && c < bc)) {
                best = Some((c, dist));
            }
        }
    }
    best.map(|(c, _)| c)
}

/// Stack strength in rough power units (sum of attack power of its fighters).
fn stack_power(g: &Game, grp: GroupId) -> i32 {
    g.groups[grp].units.iter().map(|&u| attack_power(g, u)).sum()
}

/// One decision for an army stack led by `u`.
/// Diagnostic (`FFH_AI_EXPOSURE=1`): report army stacks of `p` left on a plot that a *much
/// stronger* visible enemy unit can reach next turn -- the single strongest enemy attacker in
/// reach out-powers our best defender by half again (`maxatt > 1.5x bestdef`), the "obviously
/// about to be killed" case. `natt` is how many enemy attackers reach the plot and `wipe` whether
/// there are enough to kill every unit in the stack. Skips a garrison in one of our cities and a
/// stack next to its target about to strike this turn; `ctx=op` marks a stack marching/striking in
/// an operation, `ctx=loose` one not in any. Compares single units (not summed power, which
/// over-counts many weak attackers a strong defender beats one at a time). Pure read, printed only
/// when the env var is set, so it has no cost or effect otherwise. Called once per player's turn.
pub fn report_exposure(g: &Game, v: &View, p: PlayerId) {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var("FFH_AI_EXPOSURE").is_ok()) {
        return;
    }
    let mut seen = std::collections::BTreeSet::new();
    for &u in &g.players[p].units {
        if !g.units.contains(u) {
            continue;
        }
        let Some(grp) = g.units[u].group else { continue };
        if !seen.insert(grp) || !g.groups[grp].units.iter().any(|&o| is_army_unit(g, o)) {
            continue;
        }
        let head = g.group_get_head_unit(grp).unwrap_or(u);
        let here = g.unit_plot(head);
        if g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p) || crate::campaign::striking(g, v, grp) {
            continue;
        }
        let (hx, hy) = g.map.xy(here);
        let best_def = g.groups[grp].units.iter().map(|&o| defense_power(g, o)).max().unwrap_or(0).max(1);
        // Strongest single visible enemy attacker that can reach this plot next turn, and how many.
        let mut max_att = 0;
        let mut n_att = 0;
        for pl in g.players.iter() {
            if !pl.alive || !v.enemies.contains(&pl.team) {
                continue;
            }
            for &e in &pl.units {
                if !g.units.contains(e) {
                    continue;
                }
                let eu = &g.units[e];
                let att = attack_power(g, e);
                if att == 0 {
                    continue;
                }
                let Some(ep) = g.map.plot_xy(eu.x, eu.y) else { continue };
                if !g.plot_is_visible(ep, v.team) {
                    continue;
                }
                let range = (g.d.units[eu.unit_type].moves.max(1) + 1).min(4);
                if g.plot_distance(eu.x, eu.y, hx, hy) <= range {
                    max_att = max_att.max(att);
                    n_att += 1;
                }
            }
        }
        if max_att * 2 > best_def * 3 {
            let ctx = if crate::campaign::on_the_move(v, grp) { "op" } else { "loose" };
            eprintln!(
                "[EXPOSED t{} p{} grp{}] bestdef={} maxatt={} ratio={} natt={} units={} wipe={} ctx={}",
                g.game_turn(), p.0, grp.0, best_def, max_att, max_att * 100 / best_def, n_att, g.groups[grp].units.len(), n_att >= g.groups[grp].units.len(), ctx
            );
        }
    }
}

pub fn army(g: &mut Game, v: &mut View, mut grp: GroupId, mut u: UnitId) {
    let p = v.p;
    let here = g.unit_plot(u);
    if v.barbarian {
        crate::units::barbarian(g, v, grp, u);
        return;
    }

    // A group attack the rules queued (AI follow-through).
    if g.groups[grp].ai_group_attack {
        let (x, y) = (g.groups[grp].ai_group_attack_x, g.groups[grp].ai_group_attack_y);
        g.groups[grp].ai_group_attack = false;
        if let Some(q) = g.map.plot_xy(x, y) {
            if assault(g, v, grp, q) {
                finish(g, grp);
                return;
            }
        }
    }

    // An undefended city of ours (a fresh one, typically founded by the settler we escorted):
    // its first garrison.
    if let Some(c) = g.map.plots[here].plot_city.filter(|&c| g.cities[c].owner == p) {
        if garrison(g, p, here).0 == 0 && v.garrison.get(&c).copied().unwrap_or(0) == 0 {
            *v.garrison.entry(c).or_insert(0) += 1;
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_CITY_DEFENSE);
            if let Some(ng) = g.units[u].group {
                hold(g, ng);
            }
            return;
        }
    }

    merge_here(g, v, grp, here);
    crate::traits::before_army(g, v, grp); // trait-play
    if !alive(g, grp) {
        return;
    }
    // Its supply line cut (docs/SUPPLY.md): back toward home, unless hungry with the city it
    // attacks within two plots (supply.rs).
    if crate::supply::fall_back(g, v, grp, u) {
        return;
    }

    // Heal before anything but a sure kill.
    let max_hp = g.d.defines.max_hit_points.max(1);
    let hurt = g.groups[grp].units.iter().any(|&o| g.units[o].damage * 100 / max_hp > 35);

    // A stack landed next to the invasion's target city (switch `INVASION`): it takes the city or
    // holds for it, and does not turn on the field units around it (naval.rs).
    if crate::naval::landed_beside(g, v, u) && crate::naval::amphibious(g, v, grp, u) {
        return;
    }

    // Adjacent targets first (the operation's target city is left to its strike).
    crate::campaign::plan(g, v);
    // An enemy city next to us that is empty, or that this stack clearly takes: take it, plan or
    // not (C++ `AI_stackAttackCity(1, 160)`).
    if crate::switches::CAPTURE.on(g, p) && opportune(g, v, grp, u) {
        finish(g, grp);
        return;
    }
    let op_target = v.campaign.ops.first().map(|o| o.plot);
    if let Some(q) = best_adjacent(g, v, grp, here, hurt, op_target) {
        assault(g, v, grp, q);
        finish(g, grp);
        return;
    }
    if !alive(g, grp) {
        return;
    }
    // (Not a stack about to strike: the strike estimate counts its wounds, and the city's
    // defenders heal faster than we do.)
    match heal(g, v, grp, u) {
        Healed::Done => return,
        Healed::Continue(ng, nu) => {
            grp = ng;
            u = nu;
        }
    }

    // A city of ours in peril within a few turns: defend it (unless the stack is on its way to
    // a siege: campaign.rs).
    if !v.peril.is_empty() && !crate::campaign::on_the_move(v, grp) {
        let reach = v.flood.run(g, u, 5, false);
        if let Some(r) = v.peril.iter().filter_map(|&(q, _)| reach.iter().find(|r| r.plot == q).copied()).min_by_key(|r| (r.turns, r.plot)) {
            crate::trace!(g, p, "stack {:?} defends city at {:?}", grp, g.map.xy(r.plot));
            if r.plot == here {
                hold(g, grp);
            } else {
                move_to(g, grp, r.plot, 0);
                finish(g, grp);
            }
            return;
        }
    }

    // A settler waits for an escort.
    if !v.escort_requests.is_empty() && g.groups[grp].units.len() == 1 {
        let reach = v.flood.run(g, u, 6, false);
        if let Some((i, r)) = v.escort_requests.iter().enumerate().filter_map(|(i, &q)| reach.iter().find(|r| r.plot == q).map(|r| (i, *r))).min_by_key(|(_, r)| (r.turns, r.plot)) {
            v.escort_requests.remove(i);
            if r.plot != here {
                move_to(g, grp, r.plot, 0);
            }
            finish(g, grp);
            return;
        }
    }

    // An amphibious invasion (naval.rs): a committed stack boards the transport in port to
    // cross the water; a stack already landed on the enemy's landmass takes the overseas city
    // (which `campaign.rs` cannot, as it only targets landmasses where we hold a city).
    if crate::naval::amphibious(g, v, grp, u) {
        return;
    }

    // War (or its preparation): the operation against the chosen enemy city; stacks close to
    // a barbarian target act on its operation before hunting (campaign.rs).
    if crate::campaign::act(g, v, grp, u, true) {
        return;
    }

    // Hunt: the best target in reach that the stack can take.
    if hunt(g, v, grp, u) {
        return;
    }

    // A Raiders leader's spare stack raids the enemy's improvements (traits.rs).
    if crate::traits::raid(g, v, grp, u) { return; } // trait-play

    // Peacetime campaign against a barbarian city (campaign.rs).
    if crate::campaign::act(g, v, grp, u, false) {
        return;
    }

    // Gather: join the nearest other stack within a few turns, else hold in the nearest city.
    // (Posting lone units here to block barbarian spawns near our cities was tried and reverted:
    // the units held in cities fight for them. docs/AI_LOG.md, "Fog-busting posts".)
    if g.groups[grp].units.len() < MAX_STACK {
        let reach = v.flood.run(g, u, 3, false);
        let mut best: Option<(PlotIdx, i32)> = None;
        for r in &reach {
            if r.plot == here {
                continue;
            }
            let n = g.map.plots[r.plot].units.iter().filter(|&&o| g.units[o].owner == p && is_army_unit(g, o)).count() as i32;
            if n == 0 || n as usize >= MAX_STACK {
                continue;
            }
            // Smaller stacks walk to bigger ones (ties: lower plot index gathers).
            let mine = g.groups[grp].units.len() as i32;
            if n < mine || (n == mine && r.plot.idx() > here.idx()) {
                continue;
            }
            let score = n * 10 - r.turns * 5;
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((r.plot, score));
            }
        }
        // (Not beyond our supply line: supply.rs.)
        if let Some((q, _)) = best.filter(|&(q, _)| crate::supply::may_move(g, grp, q, None)) {
            move_to(g, grp, q, 0);
            finish(g, grp);
            return;
        }
    }
    crate::trace!(g, p, "stack {:?} ({} units, power {}) idle at {:?}; {} targets", grp, g.groups[grp].units.len(), stack_power(g, grp), g.map.xy(here), v.targets.len());
    if g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p) {
        hold(g, grp);
    } else if let Some(c) = nearest_city(g, p, here) {
        move_to(g, grp, g.city_plot(c), 0);
        finish(g, grp);
    } else {
        hold(g, grp);
    }
}

/// What [`heal`] did: the stack's turn is spent, or a stack (perhaps the healthy part of it,
/// with a new head) goes on with its turn.
enum Healed {
    Done,
    Continue(GroupId, UnitId),
}

/// A unit counts as hurt from this much damage (percent): in a field stack (most of it hurt sends
/// the stack to heal) and in one of our cities (it splits off and heals there). One threshold for
/// both, so a stack leaves a city only in a state the field rule accepts: with More Naval AI's 30%
/// in cities (`AI_heal(30, 1)`), units healed to 21-30% walked out and the next turn's field rule
/// sent them back, turn after turn (a fork of seed 26006, turn 239: docs/AI.md, "Forks of the
/// healing rule").
const HURT_PCT: i32 = 20;
/// A lone unit stops to heal from this much damage (the old rule for every stack).
const ALONE_PCT: i32 = 35;

/// Healing as More Naval AI's stacks do (`CvUnitAI::AI_heal`), fitted to our small stacks: in one
/// of our cities the wounded split off and heal while the rest go on; in the field a stack
/// committed to a siege stops only when more than half of it is hurt, and only where healing is
/// better than 10% a turn (our land, a city: not enemy or neutral land, where it heals at 5-10%
/// beside defenders that heal faster), else it walks into a city of ours within two turns or onto
/// our land within one, else it carries on; a loose stack (a few units hunting raiders) heals where
/// it stands for one badly hurt unit, as before. A stack about to strike never stops. (Before: one unit over 35% damage
/// halted the whole stack wherever it stood, committed stacks next to their target included. More
/// Naval AI's rule for every stack -- stop only when most of it is hurt -- lost units and cities
/// here: our loose stacks are two to four units, and a badly hurt one is a large part of them.)
fn heal(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> Healed {
    let p = v.p;
    if crate::campaign::striking(g, v, grp) {
        return Healed::Continue(grp, u);
    }
    let here = g.unit_plot(u);
    let max_hp = g.d.defines.max_hit_points.max(1);
    let pct = |g: &Game, o: UnitId| g.units[o].damage * 100 / max_hp;
    let units = g.groups[grp].units.clone();
    let safe = |g: &Game, v: &View| v.danger_at(here) * 2 < g.groups[grp].units.iter().map(|&o| defense_power(g, o)).sum::<i32>().max(1) * 3;
    let can_heal = |g: &Game, grp: GroupId| g.group_can_start_mission(grp, MissionType::MISSION_HEAL, -1, -1, None, false);
    let our_city = g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p);
    if our_city {
        // The wounded (that can still heal this turn: they have not moved) split off and heal.
        let wounded: Vec<UnitId> = units.iter().copied().filter(|&o| pct(g, o) >= HURT_PCT && !g.unit_has_moved(o)).collect();
        if wounded.is_empty() {
            return Healed::Continue(grp, u);
        }
        if wounded.len() == units.len() {
            if can_heal(g, grp) {
                crate::trace!(g, p, "stack {:?} ({} units) heals in {:?}", grp, units.len(), g.map.xy(here));
                g.tel_decision(p, "military", "heal_city", "heal", units.len() as i64, String::new);
                crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
                return Healed::Done;
            }
            return Healed::Continue(grp, u);
        }
        // Move one part into a new group without taking the head out of the old one (a head
        // leaving its group marks the group for separation, and it would take no more units).
        let head_wounded = wounded.contains(&u);
        let leave: Vec<UnitId> = if head_wounded { units.iter().copied().filter(|o| !wounded.contains(o)).collect() } else { wounded.clone() };
        let first = leave[0];
        g.unit_join_group(first, None, true);
        let Some(ng) = g.units[first].group else { return Healed::Continue(grp, u) };
        for &o in &leave[1..] {
            g.unit_join_group(o, Some(ng), true);
        }
        let (heal_grp, go_grp, go_head) = if head_wounded { (grp, ng, first) } else { (ng, grp, u) };
        crate::trace!(g, p, "stack {:?}: {} wounded split off to heal in {:?}", grp, wounded.len(), g.map.xy(here));
        g.tel_decision(p, "military", "heal_split", "heal", wounded.len() as i64, String::new);
        if alive(g, heal_grp) && can_heal(g, heal_grp) {
            crate::units::push(g, heal_grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
        }
        if alive(g, go_grp) && g.units.contains(go_head) && g.units[go_head].group == Some(go_grp) && g.group_ready_to_move(go_grp, false) {
            return Healed::Continue(go_grp, go_head);
        }
        return Healed::Done;
    }
    // In the field. An army whose line is cut does not heal (docs/SUPPLY.md): no stop for it.
    if crate::supply::cut(g, grp) {
        return Healed::Continue(grp, u);
    }
    let n = units.len();
    let badly = units.iter().any(|&o| pct(g, o) > ALONE_PCT);
    let most = units.iter().filter(|&&o| pct(g, o) >= HURT_PCT).count() * 2 > n;
    // A loose stack (a few units hunting raiders near home) heals where it stands as soon as one
    // unit is badly hurt, as before this rule: making it walk home, or wait for most of it to be
    // hurt, let raiders run and lost cities to the barbarians (+29-43% on fresh seeds).
    let marching = crate::campaign::on_the_move(v, grp);
    if !marching {
        if badly && safe(g, v) && can_heal(g, grp) {
            crate::trace!(g, p, "stack {:?} ({} units) heals at {:?}", grp, n, g.map.xy(here));
            g.tel_decision(p, "military", "heal_stack", "heal", n as i64, String::new);
            crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
            return Healed::Done;
        }
        return Healed::Continue(grp, u);
    }
    // A stack committed to a siege past gathering stops only when most of it is hurt (C++).
    if !most {
        if badly && safe(g, v) {
            crate::trace!(g, p, "stack {:?} ({} units) marches on with its wounded at {:?}", grp, n, g.map.xy(here));
            g.tel_decision(p, "military", "heal_skipped", "few_hurt", 1, String::new);
        }
        return Healed::Continue(grp, u);
    }
    if g.unit_heal_rate(u, here) > 10 && safe(g, v) && can_heal(g, grp) {
        crate::trace!(g, p, "stack {:?} ({} units) heals at {:?}", grp, n, g.map.xy(here));
        g.tel_decision(p, "military", "heal_stack", "heal", n as i64, String::new);
        crate::units::push(g, grp, MissionType::MISSION_HEAL, -1, -1, 0, false);
        return Healed::Done;
    }
    // Poor healing here: into a city of ours within two turns, else onto our land within one.
    let ours = |g: &Game, q: PlotIdx| g.map.plots[q].plot_city.is_some_and(|c| g.cities[c].owner == p);
    let reach = v.flood.run(g, u, 2, false);
    let city = reach.iter().filter(|r| ours(g, r.plot)).min_by_key(|r| (r.turns, r.plot)).copied();
    let land = || reach.iter().filter(|r| r.turns <= 1 && g.map.plots[r.plot].owner == Some(p) && v.danger_at(r.plot) == 0).min_by_key(|r| (r.turns, std::cmp::Reverse(g.plot_defense_modifier(r.plot, Some(v.team), true)), r.plot)).copied();
    if let Some(r) = city.or_else(land) {
        crate::trace!(g, p, "stack {:?} ({} units) goes to heal at {:?}", grp, n, g.map.xy(r.plot));
        g.tel_decision(p, "military", "heal_retreat", "heal", n as i64, String::new);
        move_to(g, grp, r.plot, 0);
        finish(g, grp);
        return Healed::Done;
    }
    g.tel_decision(p, "military", "heal_skipped", "poor_healing", 1, String::new);
    Healed::Continue(grp, u)
}

/// Make sure the group's turn ends.
fn finish(g: &mut Game, grp: GroupId) {
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
}

/// Opportunistic captures (switch `CAPTURE`): an enemy city at war (a civ's or the barbarians')
/// that the stack can walk into this turn because it is empty, or one next to it whose defenders
/// it takes when the simulation of the group attack says the city falls ([`field_go`]) -- even
/// when no operation targets it (the targets of our operations are left to their strikes). An
/// expert takes the city the enemy has left open, wherever his army stands. Before, a stack next
/// to an empty enemy city walked on: the plan named another city, and the hunt only went for
/// visible units. True if the stack acted.
fn opportune(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    let p = v.p;
    let team = v.team;
    let here = g.unit_plot(u);
    let (x, y) = g.map.xy(here);
    if g.unit_moves_left(u) <= 0 {
        return false;
    }
    // Enemy cities around (two plots: as far as a unit with two moves gets this turn).
    let mut cities: Vec<(i32, CityId, PlotIdx)> = Vec::new();
    for dx in -2..=2 {
        for dy in -2..=2 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            let Some(c) = g.map.plots[q].plot_city else { continue };
            let owner_team = g.players[g.cities[c].owner].team;
            if owner_team == team || !g.at_war(team, owner_team) || !crate::view::sees_units(g, team, q) || v.campaign.ops.iter().any(|o| o.plot == q) {
                continue;
            }
            let (qx, qy) = g.map.xy(q);
            cities.push((g.plot_distance(x, y, qx, qy), c, q));
        }
    }
    if cities.is_empty() {
        return false;
    }
    cities.sort();
    let mut reach: Option<Vec<crate::reach::Reached>> = None;
    for (d, c, q) in cities {
        let defs = crate::combat_sim::visible_defenders(g, team, q);
        let name = g.cities[c].name.clone();
        if defs.is_empty() {
            // Walk in, if we get there this turn.
            let arrives = d == 1 && g.group_can_move_into(grp, q, false) || {
                let r = reach.get_or_insert_with(|| v.flood.run(g, u, 1, false));
                r.iter().any(|r| r.plot == q && r.turns == 0)
            };
            if !arrives {
                continue;
            }
            crate::trace!(g, p, "stack {:?} ({} units) walks into the empty city {}", grp, g.groups[grp].units.len(), name);
            g.tel_decision(p, "military", "capture_empty", "capture", 1, String::new);
            move_to(g, grp, q, 0);
            return true;
        }
        if d != 1 {
            continue;
        }
        // When the simulation of the group attack says the city falls (`field_go` with
        // `capture`; before round 7g, the siege estimate with a reserve and odds the assault goes
        // on with).
        let units = g.groups[grp].units.clone();
        if forecast_on(g, p) {
            // On the forecast: the battles the pool on our plot fights take the city.
            let rule = Rule::Field { city: true, capture: true, fought: false, hurt: false };
            let call = field_call(g, v, p, &pool_attack(g, p, here, q), q, rule, Some("capture"), || old_field(g, grp, q, true, true, false, false, false));
            if !call.go {
                if g.telemetry.on() {
                    if let Some(s) = field_sim(g, grp, q) {
                        crate::predict::field(g, p, q, &s, true, false, "capture");
                    }
                    crate::predict::field2(g, Some(&v.forecasts), p, here, q, true, false, "capture");
                }
                continue;
            }
            crate::trace!(g, p, "stack {:?} ({} units) takes its chance on {} ({} defenders): {:?}", grp, units.len(), name, defs.len(), call.est);
            g.tel_decision(p, "military", "capture_attack", "capture", defs.len() as i64, String::new);
            assault(g, v, grp, q);
            if !g.cities.contains(c) || g.cities[c].owner == p {
                g.tel_decision(p, "military", "capture_taken", "capture", 1, String::new);
            }
            return true;
        }
        let sim = field_sim(g, grp, q);
        let takes = sim.is_some_and(|s| field_go(&s, true, true, false, false));
        if !takes {
            if let Some(s) = sim.filter(|_| g.telemetry.on()) {
                crate::predict::field(g, p, q, &s, true, false, "capture");
                crate::predict::field2(g, Some(&v.forecasts), p, g.unit_plot(u), q, true, false, "capture");
            }
            continue;
        }
        crate::trace!(g, p, "stack {:?} ({} units) takes its chance on {} ({} defenders)", grp, units.len(), name, defs.len());
        g.tel_decision(p, "military", "capture_attack", "capture", defs.len() as i64, String::new);
        assault(g, v, grp, q);
        if !g.cities.contains(c) || g.cities[c].owner == p {
            g.tel_decision(p, "military", "capture_taken", "capture", 1, String::new);
        }
        return true;
    }
    false
}

/// Best adjacent enemy plot worth attacking now.
fn best_adjacent(g: &Game, v: &View, grp: GroupId, here: PlotIdx, hurt: bool, skip: Option<PlotIdx>) -> Option<PlotIdx> {
    let head = g.group_get_head_unit(grp)?;
    let (x, y) = g.map.xy(here);
    let mut best: Option<(PlotIdx, i64)> = None;
    for dir in 0..DirectionType::NUM {
        let Some(q) = g.plot_direction(x, y, DirectionType::from_usize(dir)) else { continue };
        if !g.plot_is_visible_enemy_unit_for_unit(q, head) || skip == Some(q) {
            continue;
        }
        // Do not attack into civ cities unless that is the plan (campaign.rs does).
        let city = g.map.plots[q].plot_city;
        if city.is_some_and(|c| !g.player_is_barbarian(g.cities[c].owner)) {
            continue;
        }
        if forecast_on(g, g.groups[grp].owner) {
            // On the forecast: the battles the pool on our plot fights there this turn; the best
            // target a city, else the most favourable trade.
            let p = g.groups[grp].owner;
            let rule = Rule::Field { city: city.is_some(), capture: false, fought: false, hurt };
            let attacks = pool_attack(g, p, here, q);
            if attacks.is_empty() {
                continue;
            }
            let c = field_call(g, v, p, &attacks, q, rule, Some("adjacent"), || old_field(g, grp, q, city.is_some(), false, false, hurt, true));
            if !c.go {
                if g.telemetry.on() {
                    if let Some(s) = field_sim(g, grp, q) {
                        crate::predict::field(g, p, q, &s, city.is_some(), false, "adjacent");
                    }
                    crate::predict::field2(g, Some(&v.forecasts), p, here, q, city.is_some(), false, "adjacent");
                }
                continue;
            }
            let e = c.est.unwrap_or_default();
            let score = if city.is_some() { 1_000_000 + e.p_take as i64 } else { e.enemy_lost_x100 as i64 * c.kill_pct as i64 / 100 - e.own_lost_x100 as i64 };
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((q, score));
            }
            continue;
        }
        let Some(s) = field_sim(g, grp, q) else { continue };
        let (ok, score) = (field_go(&s, city.is_some(), false, false, hurt), if city.is_some() { 1_000_000 + s.best as i64 } else { s.gain });
        if !ok || precious_holds(g, grp, q).is_some() {
            if g.telemetry.on() {
                crate::predict::field(g, g.groups[grp].owner, q, &s, city.is_some(), false, "adjacent");
                crate::predict::field2(g, Some(&v.forecasts), g.groups[grp].owner, here, q, city.is_some(), false, "adjacent");
            }
            continue;
        }
        if best.map_or(true, |(_, s)| score > s) {
            best = Some((q, score));
        }
    }
    best.map(|(q, _)| q)
}

/// Peacetime (and barbarian) targets: move toward the best one we can take.
fn hunt(g: &mut Game, v: &mut View, grp: GroupId, u: UnitId) -> bool {
    if v.targets.is_empty() {
        return false;
    }
    let here = g.unit_plot(u);
    let power = stack_power(g, grp);
    let n_att = attackers(g, grp).len() as i32;
    if n_att == 0 {
        return false;
    }
    let reach = v.flood.run(g, u, 6, true);
    let mut cands: Vec<(i32, usize, i32)> = Vec::new(); // (score, target index, turns)
    for (i, t) in v.targets.iter().enumerate() {
        let Some(r) = reach.iter().find(|r| r.plot == t.plot) else { continue };
        if v.target_claims.iter().any(|&(q, _)| q == t.plot) && !t.city {
            continue;
        }
        // The operation's target is taken by the operation.
        if t.city && v.campaign.ops.iter().any(|o| o.plot == t.plot) {
            continue;
        }
        // Rough feasibility before exact odds: their strongest defender against our stack.
        let their: i32 = g.map.plots[t.plot].units.iter().filter(|&&e| g.units[e].owner != v.p).map(|&e| defense_power(g, e)).max().unwrap_or(0);
        if their > 0 && their * t.units.max(1) > power * 2 {
            continue;
        }
        // (An empty city needs no army: whoever gets there takes it, if it is still empty; not
        // worth a long march.)
        if t.city && n_att < 3 && t.units > 0 || t.city && t.units == 0 && r.turns > 2 {
            continue;
        }
        let score = t.value * 10 / (r.turns + 2);
        cands.push((score, i, r.turns));
    }
    cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, i, _) in cands.iter().take(4) {
        let t = v.targets[i];
        // Exact check where it matters: the best odds against what stands there.
        if t.units > 0 && forecast_on(g, v.p) {
            // On the forecast: the stack's battles from where it will attack (here, or the
            // staging plot next to the target below).
            let (tx, ty) = g.map.xy(t.plot);
            let (x, y) = g.map.xy(here);
            let from = if g.plot_distance(x, y, tx, ty) <= 1 {
                Some(here)
            } else {
                v.flood
                    .run(g, u, 6, false)
                    .iter()
                    .filter(|r| {
                        let (rx, ry) = g.map.xy(r.plot);
                        g.plot_distance(rx, ry, tx, ty) == 1 && g.map.plots[r.plot].plot_city.map_or(true, |c| g.cities[c].owner == v.p)
                    })
                    .min_by_key(|r| (r.turns, std::cmp::Reverse(g.plot_defense_modifier(r.plot, Some(v.team), true)), r.plot))
                    .map(|r| r.plot)
            };
            let Some(from) = from else { continue };
            // (The units that can attack there, as round 7g's estimate takes them.)
            let atts: Vec<UnitId> = attackers(g, grp).into_iter().filter(|&o| g.unit_can_move_or_attack_into(o, t.plot, false)).collect();
            if atts.is_empty() {
                continue;
            }
            let attacks: crate::forecast::Attacks = if from == here { pool_attack(g, v.p, here, t.plot) } else { vec![(atts, from)] };
            let rule = Rule::Field { city: t.city, capture: false, fought: false, hurt: false };
            if !field_call(g, v, v.p, &attacks, t.plot, rule, Some("hunt"), || old_field(g, grp, t.plot, t.city, false, false, false, true)).go {
                continue;
            }
        } else if t.units > 0 {
            if !field_sim(g, grp, t.plot).is_some_and(|s| field_go(&s, t.city, false, false, false)) {
                continue;
            }
            if precious_holds(g, grp, t.plot).is_some() {
                continue;
            }
        }
        // Where it goes: the target (an empty one), or next to it on the best defensive ground
        // we reach first; not beyond our supply line unless the city it marches on is near
        // (supply.rs).
        let (tx, ty) = g.map.xy(t.plot);
        let (x, y) = g.map.xy(here);
        let adjacent = g.plot_distance(x, y, tx, ty) <= 1 && t.units > 0;
        let staging = if adjacent || t.units == 0 {
            None
        } else {
            let reach = v.flood.run(g, u, 6, false);
            reach
                .iter()
                .filter(|r| {
                    let (rx, ry) = g.map.xy(r.plot);
                    g.plot_distance(rx, ry, tx, ty) == 1 && g.map.plots[r.plot].plot_city.map_or(true, |c| g.cities[c].owner == v.p)
                })
                .min_by_key(|r| (r.turns, std::cmp::Reverse(g.plot_defense_modifier(r.plot, Some(v.team), true)), r.plot))
                .copied()
        };
        let dest = if adjacent { None } else if t.units == 0 { Some(t.plot) } else { staging.map(|r| r.plot).filter(|&q| q != here) };
        let city = t.city.then_some(t.plot);
        if dest.is_some_and(|q| !crate::supply::may_move(g, grp, q, city)) {
            continue;
        }
        v.target_claims.push((t.plot, grp));
        crate::trace!(g, v.p, "stack {:?} ({} units) hunts {:?} (value {}, units {}, lair {}, city {})", grp, g.groups[grp].units.len(), g.map.xy(t.plot), t.value, t.units, t.lair, t.city);
        if adjacent {
            assault(g, v, grp, t.plot);
        } else if t.units == 0 {
            if t.city {
                g.tel_decision(v.p, "military", "capture_empty_hunt", "capture", 1, String::new);
            }
            move_to(g, grp, t.plot, 0);
        } else if let Some(q) = dest {
            move_to(g, grp, q, 0);
        }
        finish(g, grp);
        return true;
    }
    false
}

/// Targets for field armies near our lands: enemy units outside civ cities, lairs, barbarian
/// cities.
pub fn find_targets(g: &Game, v: &View) -> Vec<Target> {
    let p = v.p;
    let pl = &g.players[p];
    let ours: Vec<(i32, i32)> = pl.cities.iter().map(|&c| (g.cities[c].x, g.cities[c].y)).collect();
    if ours.is_empty() {
        return Vec::new();
    }
    let near = |x: i32, y: i32, r: i32| ours.iter().map(|&(cx, cy)| g.plot_distance(x, y, cx, cy)).min().unwrap_or(i32::MAX) <= r;
    let mut out: Vec<Target> = Vec::new();
    // Enemy units (visible).
    for opl in g.players.iter() {
        if !opl.alive || !v.enemies.contains(&opl.team) {
            continue;
        }
        for &e in &opl.units {
            let unit = &g.units[e];
            let Some(q) = g.map.plot_xy(unit.x, unit.y) else { continue };
            if !crate::view::sees_units(g, v.team, q) || g.unit_is_invisible(e, v.team, false) || g.plot_is_water(q) {
                continue;
            }
            if out.iter().any(|t| t.plot == q) {
                continue;
            }
            let city = g.map.plots[q].plot_city;
            if city.is_some_and(|c| !g.player_is_barbarian(g.cities[c].owner)) {
                continue; // civ cities: war logic
            }
            let in_ours = g.map.plots[q].owner == Some(p);
            if !in_ours && !near(unit.x, unit.y, if city.is_some() { 10 } else { 6 }) {
                continue;
            }
            let n = g.map.plots[q].units.iter().filter(|&&o| g.units[o].owner == unit.owner).count() as i32;
            let lair = g.map.plots[q].improvement_type.is_some_and(|i| g.d.improvements[i].spawn_unit_type.is_some());
            let mut value = 150 + 30 * n;
            if in_ours {
                value += 250;
            }
            if lair {
                value += 250;
            }
            if city.is_some() {
                value += 700;
            }
            out.push(Target { plot: q, value, units: n, city: city.is_some(), lair });
        }
    }
    // Empty enemy cities near our lands (switch `CAPTURE`): a stack walks in (`hunt`).
    if crate::switches::CAPTURE.on(g, p) {
        for opl in g.players.iter() {
            if !opl.alive || !v.enemies.contains(&opl.team) {
                continue;
            }
            for &c in &opl.cities {
                let q = g.city_plot(c);
                let (x, y) = (g.cities[c].x, g.cities[c].y);
                if !crate::view::sees_units(g, v.team, q) || out.iter().any(|t| t.plot == q) || !near(x, y, 10) {
                    continue;
                }
                if !crate::combat_sim::visible_defenders(g, v.team, q).is_empty() {
                    continue;
                }
                out.push(Target { plot: q, value: 900 + 100 * g.cities[c].population, units: 0, city: true, lair: false });
            }
        }
    }
    // Empty lairs near our lands.
    let barb_team = g.barbarian_team();
    if v.enemies.contains(&barb_team) {
        for &(cx, cy) in &ours {
            for dx in -6..=6 {
                for dy in -6..=6 {
                    let Some(q) = g.plot_xy(cx, cy, dx, dy) else { continue };
                    let plot = &g.map.plots[q];
                    if !plot.revealed[v.team] || plot.owner.is_some_and(|o| o != p) {
                        continue;
                    }
                    let Some(imp) = plot.improvement_type else { continue };
                    if g.d.improvements[imp].spawn_unit_type.is_none() || out.iter().any(|t| t.plot == q) {
                        continue;
                    }
                    out.push(Target { plot: q, value: 300, units: 0, city: false, lair: true });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forecast::Est;

    fn est(p_take: i32, own: i32, enemy: i32) -> Est {
        Est { p_take, own_lost_x100: own, enemy_lost_x100: enemy, n_att: 4, n_def: 2, ..Est::default() }
    }

    #[test]
    fn the_field_rule_reads_the_bars() {
        let field = |hurt| Rule::Field { city: false, capture: false, fought: false, hurt };
        // Likely to take the plot: go, whatever the trade.
        assert!(field_rule(&est(TAKE_BAR, 300, 0), 100, field(false)));
        assert!(!field_rule(&est(TAKE_BAR, 300, 0), 100, field(true)));
        assert!(field_rule(&est(TAKE_BAR_HURT, 300, 0), 100, field(true)));
        // At worse odds only on a favourable trade; raiders count four times.
        assert!(field_rule(&est(TRADE_BAR, 100, 150), 100, field(false)));
        assert!(!field_rule(&est(TRADE_BAR, 100, 90), 100, field(false)));
        assert!(field_rule(&est(TRADE_BAR, 100, 90), RAIDER_KILL_PCT, field(false)));
        assert!(!field_rule(&est(TRADE_BAR - 1, 0, 200), RAIDER_KILL_PCT, field(false)));
        // A city to capture: when it falls; once fought, as any attack.
        let capture = |fought| Rule::Field { city: true, capture: true, fought, hurt: false };
        assert!(!field_rule(&est(CAPTURE_BAR - 1, 0, 200), 100, capture(false)));
        assert!(field_rule(&est(CAPTURE_BAR, 0, 0), 100, capture(false)));
        assert!(field_rule(&est(TRADE_BAR, 0, 200), 100, capture(true)));
        // A bar of its own.
        assert!(field_rule(&est(800, 0, 0), 100, Rule::Bar(800)));
        assert!(!field_rule(&est(799, 0, 500), 100, Rule::Bar(800)));
    }
}
