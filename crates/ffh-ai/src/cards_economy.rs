//! The AI's use of the economy and content batch of cards (docs/CARDS.md, section 5). Ad hoc, as a
//! rules change's AI may be (CLAUDE.md, "AI changes"): each card's content is used where an expert
//! would see it pay.
//!
//! - Buildings ([`building_value`]): the Beast Market where an animal beats the city's best
//!   attacker; the Elemental Foundry, the Spire of the Archmagi, the Admiralty and Agares's Well in
//!   the top production city that can use them; the Font of Rebirth near the front; Maponos's
//!   Cradle while city sites are free. (The Academy of War's land experience, the Cradle's food
//!   and the Pioneers' work rate are valued by the generic code.)
//! - Choices: a Spire caster's sphere ([`choose_free_promotion`], `magic::sphere_value`); the
//!   Well's mana ([`choose_free_bonus`], `mana::node_value`).
//! - Civics ([`civic_value`]): Gold Standard for a rich empire at war, Magocracy by the share of
//!   casters in the army, Stoicism by the war weariness it removes.
//! - Muster ([`process_value`]): an idle city with units, at war or preparing one.
//! - The Paymaster ([`production`], [`unit`]): one, trained at war; it follows the largest stack
//!   and buys the strongest enemy unit next to it that the treasury can spare (above the reserve
//!   `empire::hurry` keeps). The Court of Mammon's land units (the Infernal Gate's civilization,
//!   `SPELL_COURT_BUY_OUT`) buy the same way, at the start of the turn and once they have moved
//!   ([`end_turn`]).
//! - Mana Grafting ([`unit`]): a caster on a node of a type the empire has several of grafts it
//!   into the type worth most.
//! - Trading Posts ([`turn`]): a spare soldier of a garrison claims the best free site near our
//!   cities (a resource we lack, a unique feature, mana) and builds the road home (army units
//!   build the field works, docs/WORKS.md); the nearest city's crews improve the resource.

use std::sync::OnceLock;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::production::Choice;
use crate::units::{alive, move_to, push, skip};
use crate::view::View;

/// The cards' type names, looked up once.
struct Names {
    paymaster: Option<UnitClassType>,
    buy_out: Option<SpellType>,
    court: Option<CivilizationType>,
    court_buy_out: Option<SpellType>,
    graft: Option<SpellType>,
    graft_card: Option<TechType>,
    trading_post_build: Option<BuildType>,
    trading_post: Option<ImprovementType>,
    adept: Option<UnitCombatType>,
}

fn names(d: &GameData) -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| Names {
        paymaster: d.ty::<UnitClassType>("UNITCLASS_PAYMASTER"),
        buy_out: d.ty::<SpellType>("SPELL_BUY_OUT"),
        court: d.ty::<CivilizationType>("CIVILIZATION_COURT_OF_MAMMON"),
        court_buy_out: d.ty::<SpellType>("SPELL_COURT_BUY_OUT"),
        graft: d.ty::<SpellType>("SPELL_MANA_GRAFT"),
        graft_card: d.ty::<TechType>("TECH_CARD_MANA_GRAFTING"),
        trading_post_build: d.ty::<BuildType>("BUILD_TRADING_POST"),
        trading_post: d.ty::<ImprovementType>("IMPROVEMENT_TRADING_POST"),
        adept: d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT"),
    })
}

/// Gold a player keeps aside (as `empire::hurry`'s reserve).
fn reserve(g: &Game, p: PlayerId) -> i32 {
    30 + 5 * g.players[p].cities.len() as i32 + crate::civs::gold_floor(g, p)
}

fn at_war(g: &Game, p: PlayerId) -> bool {
    crate::production::civ_enemies_at_war(g, p) || crate::diplomacy::preparing_target(g, g.players[p].team).is_some()
}

// ---------------------------------------------------------------------------------------------
// The turn: Trading Posts.

/// Once a turn, after the empire decisions ([`crate::cards::turn`]).
pub fn turn(g: &mut Game, v: &View) {
    trading_posts(g, v);
    // A Paymaster in a stack (not its head) buys too, and so does every unit of the Court.
    buyers(g, v.p);
}

/// Once our units have moved ([`crate::cards::end_turn`]): the Court of Mammon's units that came
/// next to an enemy buy.
pub fn end_turn(g: &mut Game, p: PlayerId) {
    if is_court(g, p) {
        buyers(g, p);
    }
}

/// Is `p` the Court of Mammon (whose land units may all Buy Out)?
fn is_court(g: &Game, p: PlayerId) -> bool {
    let court = names(g.d).court;
    court.is_some() && g.player_get_civilization_type(p) == court
}

/// Each of `p`'s units that may Buy Out (a Paymaster; every unit of the Court) buys ([`buy_out`]).
fn buyers(g: &mut Game, p: PlayerId) {
    let d = g.d;
    let class = names(d).paymaster;
    let court = is_court(g, p);
    if class.is_none() && !court {
        return;
    }
    for u in g.players[p].units.clone() {
        if g.units.contains(u) && (court || (class.is_some() && d.units[g.units[u].unit_type].unit_class_type == class)) {
            buy_out(g, p, u);
        }
    }
}

/// The Paymaster `u` of `p` (or a unit of the Court of Mammon) buys the strongest enemy unit next to
/// it that the treasury can spare (keeping the reserve). True when it bought one.
fn buy_out(g: &mut Game, p: PlayerId, u: UnitId) -> bool {
    let d = g.d;
    let n = names(d);
    let paymaster = n.paymaster.is_some() && d.units[g.units[u].unit_type].unit_class_type == n.paymaster;
    let Some(spell) = (if paymaster { n.buy_out } else { n.court_buy_out }) else { return false };
    if g.units[u].has_casted || g.unit_plot_opt(u).is_none() {
        return false;
    }
    let gold = g.players[p].gold;
    let keep = reserve(g, p);
    let best = g
        .unit_buy_out_targets(u)
        .into_iter()
        .filter(|&t| g.unit_buy_out_price(t).is_some_and(|price| gold - price >= keep))
        .max_by_key(|&t| (g.unit_base_combat_str(t) * 100 + g.units[t].experience, std::cmp::Reverse(t)));
    let Some(t) = best else { return false };
    if !g.unit_can_cast_at(u, spell, SpellTarget::Unit(t)) {
        return false;
    }
    crate::trace!(g, p, "{} buys {} for {}", if paymaster { "paymaster" } else { "court unit" }, d.units[g.units[t].unit_type].base.type_, g.unit_buy_out_price(t).unwrap_or(0));
    g.unit_cast_at(u, spell, SpellTarget::Unit(t));
    true
}

/// What a Trading Post at `site` would claim for `p`: resources we lack (more for one a worker
/// can improve), unique features, mana. 0 for a site with nothing.
fn post_site_value(g: &Game, p: PlayerId, site: PlotIdx) -> i32 {
    let d = g.d;
    let team = g.players[p].team;
    let (x, y) = g.map.xy(site);
    let mut v = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            let pl = &g.map.plots[q];
            if g.plot_is_water(q) || pl.owner.is_some() && q != site {
                continue;
            }
            if let Some(imp) = pl.improvement_type {
                if d.improvements[imp].unique {
                    v += 500;
                }
            }
            let Some(b) = pl.bonus_type else { continue };
            if d.bonuses[b].tech_reveal.is_some_and(|t| !g.team_is_has_tech(team, Some(t))) {
                continue;
            }
            if crate::mana::is_mana(d, b) || d.bonuses[b].base.type_ == "BONUS_MANA" {
                v += if g.player_has_bonus(p, b) { 100 } else { 300 };
            } else if improvable(g, p, q, b).is_some() {
                v += if g.player_has_bonus(p, b) { 40 } else { 100 + g.player_ai_bonus_val(p, b, 1).min(400) };
            }
        }
    }
    v
}

/// A build of the cities' crews that would connect resource `b` on plot `q` for `p` (its
/// improvement trades the resource and suits the plot), if any.
fn improvable(g: &Game, p: PlayerId, q: PlotIdx, b: BonusType) -> Option<BuildType> {
    let d = g.d;
    let team = g.players[p].team;
    d.builds.keys().find(|&bu| {
        let info = &d.builds[bu];
        info.city
            && info.improvement.is_some_and(|imp| d.improvements[imp].improvement_bonus[b].bonus_trade && g.plot_can_have_improvement(q, imp, Some(team), false))
            && g.team_is_has_tech(team, info.tech_prereq)
    })
}

/// A Trading Post job every 10 turns: the best free site a spare soldier of ours reaches within
/// 15 turns (its value per turn of the trip), at most three posts; then the road to our nearest
/// city.
fn trading_posts(g: &mut Game, v: &View) {
    let d = g.d;
    let n = names(d);
    let (Some(build), Some(post)) = (n.trading_post_build, n.trading_post) else { return };
    let p = v.p;
    let team = g.players[p].team;
    if !d.builds[build].tech_prereq.is_some_and(|t| g.team_is_has_tech(team, Some(t))) || (g.game_turn() + p.0 as i32) % 10 != 0 {
        return;
    }
    if g.players[p].improvement_count[post] >= 3 {
        return;
    }
    // One post at a time: a unit with the job (on its way or building) keeps it.
    let ordered = |grp: GroupId| g.groups[grp].mission_queue.iter().any(|m| m.mission_type == MissionType::MISSION_BUILD && m.data1 == build.i());
    if g.players[p].groups.iter().any(|&grp| alive(g, grp) && ordered(grp)) {
        return;
    }
    // Spare soldiers: army units alone in their group, with nothing ordered, in a city of ours
    // that keeps three fighters without them; at most four, by id.
    let spare = |u: UnitId| {
        let here = g.unit_plot(u);
        g.map.plots[here].plot_city.is_some_and(|c| g.cities[c].owner == p)
            && g.map.plots[here].units.iter().filter(|&&o| g.units[o].owner == p && g.unit_is_army_unit(o)).count() > 3
    };
    let mut workers: Vec<UnitId> = g.players[p]
        .units
        .iter()
        .copied()
        .filter(|&u| g.unit_is_army_unit(u) && !g.unit_is_automated(u) && !g.unit_is_cargo(u) && spare(u))
        .filter(|&u| g.units[u].group.is_some_and(|grp| g.groups.contains(grp) && g.groups[grp].units.len() == 1 && g.groups[grp].mission_queue.is_empty()))
        .collect();
    workers.sort_unstable();
    workers.truncate(4);
    let mut flood = crate::reach::Flood::default();
    let mut best: Option<(i64, UnitId, PlotIdx)> = None;
    for &u in &workers {
        for r in flood.run(g, u, 15, false) {
            let q = r.plot;
            if v.danger_at(q) > 0 || !g.player_can_build(p, q, build, false) {
                continue;
            }
            let value = post_site_value(g, p, q);
            if value < 300 {
                continue;
            }
            let score = value as i64 * 100 / (r.turns as i64 + 3);
            if best.map_or(true, |(bs, bu, bq)| (score, std::cmp::Reverse(u), std::cmp::Reverse(q)) > (bs, std::cmp::Reverse(bu), std::cmp::Reverse(bq))) {
                best = Some((score, u, q));
            }
        }
    }
    let Some((score, u, site)) = best else { return };
    let Some(grp) = g.units[u].group else { return };
    let (sx, sy) = g.map.xy(site);
    crate::trace!(g, p, "trading post at {:?} (score {score}; {} built) by {:?}", (sx, sy), g.players[p].improvement_count[post], u);
    let start = g.unit_plot(u);
    if start != site {
        move_to(g, grp, site, 0);
        // (A move the path finder refuses leaves the unit to its usual orders.)
        if !alive(g, grp) || g.groups[grp].mission_queue.is_empty() && g.unit_plot(u) == start {
            return;
        }
    }
    push(g, grp, MissionType::MISSION_BUILD, build.i(), -1, 0, true);
    // Then the road home (the resource it claims is the nearest city's crews' to improve).
    if let Some(city) = crate::units::nearest_city(g, p, site) {
        let (cx, cy) = (g.cities[city].x, g.cities[city].y);
        push(g, grp, MissionType::MISSION_ROUTE_TO, cx, cy, 0, true);
    }
    // An idle group starts its turn flagged to re-plan (`group_do_turn`); these orders are its
    // plan (`units::play` would clear them).
    if alive(g, grp) {
        g.groups[grp].force_update = false;
    }
}

// ---------------------------------------------------------------------------------------------
// Units: the Paymaster, grafting casters.

/// A group this module moves itself (before the role's code): true when handled.
pub fn unit(g: &mut Game, v: &mut View, grp: GroupId, head: UnitId) -> bool {
    let d = g.d;
    let n = names(d);
    let info = &d.units[g.units[head].unit_type];
    if n.paymaster.is_some() && info.unit_class_type == n.paymaster {
        paymaster(g, v, grp, head);
        return true;
    }
    if n.adept.is_some() && info.unit_combat_type == n.adept && graft(g, head) {
        if alive(g, grp) && g.group_ready_to_move(grp, true) {
            skip(g, grp);
        }
        return true;
    }
    false
}

/// The Paymaster buys ([`buy_out`]), else walks with our largest stack in the field, else waits
/// in the capital.
fn paymaster(g: &mut Game, v: &View, grp: GroupId, u: UnitId) {
    let p = v.p;
    if buy_out(g, p, u) {
        if alive(g, grp) && g.group_ready_to_move(grp, true) {
            skip(g, grp);
        }
        return;
    }
    // Our largest stack of fighters outside our cities.
    let here = g.unit_plot(u);
    let mut stack: Option<(usize, PlotIdx)> = None;
    for &og in &g.players[p].groups {
        if og == grp || !alive(g, og) {
            continue;
        }
        let head = g.groups[og].units[0];
        let q = g.unit_plot(head);
        if g.map.plots[q].plot_city.is_some() {
            continue;
        }
        let fighters = g.groups[og].units.iter().filter(|&&x| g.unit_can_fight(x)).count();
        if fighters >= 3 && stack.map_or(true, |(n, sq)| (fighters, std::cmp::Reverse(q)) > (n, std::cmp::Reverse(sq))) {
            stack = Some((fighters, q));
        }
    }
    let target = stack.map(|(_, q)| q).or_else(|| g.player_get_capital_city(p).map(|c| g.city_plot(c)));
    if let Some(q) = target.filter(|&q| q != here) {
        move_to(g, grp, q, 0);
    }
    if alive(g, grp) && g.group_ready_to_move(grp, true) {
        skip(g, grp);
    }
}

/// A caster on a node of ours grafts it when it holds several nodes of its type and another type
/// is worth clearly more (`mana::node_value`). True when it cast.
fn graft(g: &mut Game, u: UnitId) -> bool {
    let d = g.d;
    let n = names(d);
    let (Some(spell), Some(card)) = (n.graft, n.graft_card) else { return false };
    let p = g.units[u].owner;
    if !g.team_is_has_tech(g.players[p].team, Some(card)) {
        return false;
    }
    let plot = g.unit_plot(u);
    let targets = g.plot_graft_targets(plot, p);
    if targets.is_empty() {
        return false;
    }
    let Some(current) = g.map.plots[plot].bonus_type else { return false };
    // Keep the sphere: only a type we hold more than one node of.
    let nodes_of = |b: BonusType| d.improvements.keys().filter(|&i| d.improvements[i].bonus_convert == Some(b)).map(|i| g.players[p].improvement_count[i]).sum::<i32>();
    if nodes_of(current) < 2 {
        return false;
    }
    let keep = crate::mana::node_value(g, p, current);
    let Some((best, value)) = targets.iter().map(|&b| (b, crate::mana::node_value(g, p, b))).max_by_key(|&(b, v)| (v, std::cmp::Reverse(b))) else { return false };
    if value <= keep + 100 || !g.unit_can_cast_at(u, spell, SpellTarget::Choice(best.i())) {
        return false;
    }
    crate::trace!(g, p, "grafts the {} node at {:?} into {}", d.bonuses[current].base.type_, g.map.xy(plot), d.bonuses[best].base.type_);
    g.tel_cast_reason(u, "graft", (value - keep) as i64);
    g.unit_cast_at(u, spell, SpellTarget::Choice(best.i()));
    true
}

// ---------------------------------------------------------------------------------------------
// Production, buildings, civics, processes.

/// The Paymaster: one, at war or preparing one, in our top production city, with gold to use it.
pub fn production(g: &Game, c: CityId, threat: i32, war: bool) -> Option<Choice> {
    let d = g.d;
    let p = g.cities[c].owner;
    let class = names(d).paymaster?;
    if !war || threat > 0 || g.players[p].gold < 4 * reserve(g, p) {
        return None;
    }
    if g.player_get_unit_class_count_plus_making(p, class) > 0 {
        return None;
    }
    let civ = g.player_get_civilization_type(p)?;
    let unit = d.civ_unit(civ, class)?;
    if !g.city_can_train(c, unit, false, false, false, false) || !crate::production::top_production_city(g, c, |o| g.city_can_train(o, unit, false, false, false, false)) {
        return None;
    }
    Some(Choice::Train(unit, UnitAIType::UNITAI_RESERVE))
}

/// One hammer a turn on `production::building_value`'s scale.
const HAMMER: i32 = 150;

/// What the cards' buildings are worth beyond their data (added to `production::building_value`).
pub fn building_value(g: &Game, c: CityId, b: BuildingType, v: i32) -> i32 {
    let d = g.d;
    let info = &d.buildings[b];
    let p = g.cities[c].owner;
    let mut v = v;
    let top = || crate::production::top_production_city(g, c, |o| g.city_can_construct(o, b, false, false, false));
    // The Beast Market, the Elemental Foundry: the units it lets the city train.
    if !info.train_units.is_empty() {
        let best_now = crate::production::best_unit(g, c, crate::production::Role::Attack).map_or(0, |u| crate::view::type_attack_power(d, u));
        let mut gain = 0;
        for t in &info.train_units {
            if t.bonus.is_some_and(|bn| !g.player_has_bonus(p, bn)) {
                continue;
            }
            // (A strength point of a better attacker, in `type_attack_power`'s hundredths, as a
            // hammer a turn.)
            let power = crate::view::type_attack_power(d, t.unit);
            if power > best_now {
                gain = gain.max((power - best_now) * HAMMER / 100);
            }
        }
        // Two training cities are enough: a third at a quarter; a national one in our top city.
        let have = info.building_class_type.map_or(0, |k| g.players[p].cities.iter().filter(|&&o| g.city_is_has_building_class(o, k)).count());
        let national = info.building_class_type.is_some_and(|k| d.building_classes[k].max_player_instances > 0);
        if (national && !top()) || have >= 2 {
            gain /= 4;
        }
        // Animals are an early army ("before bronze"): none once the early game is over.
        if info.train_units.iter().all(|t| d.units[t.unit].animal) && crate::research::late_game(g, p) {
            gain = 0;
        }
        v += if at_war(g, p) { gain } else { gain / 2 };
    }
    // The Spire of the Archmagi: a sphere for every caster trained there.
    if !info.free_promotion_choices.is_empty() {
        let casters = g.players[p].units.iter().filter(|&&u| g.unit_get_unit_combat_type(u) == names(d).adept).count() as i32;
        let spheres = info.free_promotion_choices.iter().filter(|&&pr| d.promotions[pr].bonus_prereq.is_some_and(|bn| g.player_has_bonus(p, bn))).count() as i32;
        if spheres > 0 && top() {
            v += (2 + casters.min(8)) * 60 + spheres * 30;
        }
    }
    // Agares's Well: the best mana node it can be.
    if !info.free_bonus_choices.is_empty() {
        let best = info.free_bonus_choices.iter().map(|&bn| crate::mana::node_value(g, p, bn)).max().unwrap_or(0);
        v += best.max(0);
    }
    // The Font of Rebirth: near the front.
    if info.rebirth_range > 0 {
        let plot = g.city_plot(c);
        let threat = crate::production::local_threat(g, p, plot, info.rebirth_range + 2);
        v += if threat > 0 { 400 } else if at_war(g, p) { 200 } else { 40 };
    }
    // Maponos's Cradle: free settlers while land is free.
    if info.free_unit_turns > 0 {
        v += if g.city_free_site_near(c, ffh_core::ffh::cards::economy::FREE_UNIT_SITE_RANGE) { 600 } else { 150 };
    }
    // The Admiralty: sea experience (the generic value counts land experience only).
    let sea = info.domain_free_experience[DomainType::DOMAIN_SEA];
    if sea > 0 && info.building_class_type.is_some_and(|k| d.building_classes[k].max_player_instances > 0) {
        let ships = g.players[p].units.iter().filter(|&&u| g.unit_get_domain_type(u) == DomainType::DOMAIN_SEA && g.unit_can_fight(u)).count() as i32;
        v += if top() { sea * (20 + 10 * ships.min(10)) } else { sea * 5 };
    }
    v
}

/// What the cards' civics are worth beyond the generic terms (added to `empire::civic_value`).
pub fn civic_value(g: &Game, p: PlayerId, civic: CivicType, v: i32) -> i32 {
    let d = g.d;
    let info = &d.civics[civic];
    let pl = &g.players[p];
    let n_cities = pl.cities.len() as i32;
    let mut v = v;
    // Gold Standard: units bought at a flat rate whatever the Labor civic (`unit_rush_gold`),
    // cheaper than the Labor civics' gold hurry.
    if info.unit_rush_gold > 0 {
        let cheaper = d.hurries.keys().filter(|&o| g.player_can_hurry(p, o) && d.hurries[o].gold_per_production > 0).map(|o| d.hurries[o].gold_per_production).min();
        let rich = pl.gold > 6 * reserve(g, p);
        let gain = match cheaper {
            Some(rate) if rate <= info.unit_rush_gold => 0,
            Some(_) => 150,
            None => 400,
        };
        v += if rich && at_war(g, p) { gain + pl.gold / 10 } else if rich { gain / 2 } else { -100 };
    }
    // Magocracy: casters cheaper, other units dearer.
    if info.unit_cost_modifier != 0 || !info.unit_combat_cost_modifiers.is_empty() {
        let mut casters = 0;
        let mut others = 0;
        for &u in &pl.units {
            if !g.unit_can_fight(u) {
                continue;
            }
            let m = info.unit_cost_modifier_for(g.unit_get_unit_combat_type(u));
            if m < 0 {
                casters += 1;
            } else if m > 0 {
                others += 1;
            }
        }
        let mage = pl.personality_type.is_some_and(|l| d.leader_heads[l].ai_value_mage > 100);
        v += casters * 60 - others * 25 + if mage { 200 } else { 0 };
    }
    // Stoicism: war weariness without unhappiness.
    if info.no_war_weariness {
        let anger = pl.war_weariness_percent_anger;
        v += anger * n_cities / 5 + if at_war(g, p) { 60 * n_cities } else { 0 };
    }
    v
}

/// What Muster (`production_to_experience`) is worth in city `c`, on `production::decide`'s
/// process scale (research at 50% is 100): with two units or more to drill, at war or preparing
/// one, more than research; 0 otherwise.
pub fn process_value(g: &Game, c: CityId, pr: ProcessType) -> i32 {
    if g.d.processes[pr].production_to_experience <= 0 {
        return 0;
    }
    let p = g.cities[c].owner;
    let plot = g.city_plot(c);
    let units = g.map.plots[plot].units.iter().filter(|&&u| g.units[u].owner == p && g.unit_can_fight(u) && g.unit_can_acquire_promotion_any(u)).count();
    if units >= 2 && at_war(g, p) {
        150
    } else {
        0
    }
}

// ---------------------------------------------------------------------------------------------
// Choices.

/// The Spire of the Archmagi: the sphere worth most to a fresh caster (`magic::sphere_value`).
pub fn choose_free_promotion(g: &Game, p: PlayerId, options: &[PromotionType]) -> Option<PromotionType> {
    options.iter().copied().max_by_key(|&pr| (crate::magic::sphere_value(g, p, pr), std::cmp::Reverse(pr)))
}

/// Agares's Well: the mana node worth most (`mana::node_value`).
pub fn choose_free_bonus(g: &Game, p: PlayerId, options: &[BonusType]) -> Option<BonusType> {
    options.iter().copied().max_by_key(|&b| (crate::mana::node_value(g, p, b), std::cmp::Reverse(b)))
}
