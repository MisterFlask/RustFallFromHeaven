//! The AI's use of the card world spells of batch B3 (docs/CARDS.md; the rules are
//! `ffh_core::ffh::cards::world_spells`): when to cast each once-a-game spell, and at what. As for
//! the civilizations' world spells (`world_spells.rs`), the moment is the one an expert would pick;
//! the choices are ad hoc (CLAUDE.md, "AI changes": the cards are a rules change).
//!
//! - **Mammon's Bargain:** an evil civilization casts it at once when broke or at war with a
//!   civilization; another only when broke.
//! - **Call of the Faithful:** as soon as a hero of our state religion is missing.
//! - **Seal of Binding:** the strongest world unit or hero of a civilization at war with us within 3
//!   plots of our borders (a barbarian one within 2 plots of a city of ours: a lair's guardian far
//!   away is no threat), once it is at least as strong as our strongest unit.
//! - **Masque of the Red Death:** the largest enemy city (6 or more) our army is near (3 or more of
//!   its units within 3 plots), or any of 10 or more.
//! - **Astral Leap:** a city of ours in peril (the capital, or 5 or more) whose relief is 6 or more
//!   plots away gets the stack of a safe city; else a safe city's stack (3 or more attackers) leaps
//!   next to an enemy city it takes (no standing Sentinel; no visible defender, or three times their
//!   defence), 6 or more plots away. The next turn the army's own logic takes the city
//!   (`military::army`: an empty or clearly weaker city next to a stack).
//! - **Purchase Charter, Cultural Annexation:** the eligible city of 3 or more with the most
//!   population (then buildings), not from a civilization at peace with twice our power. The card is
//!   once a game: a fresh city of 1 is not worth it.
//! - **Transmutation Charter:** a resource we have twice into the one we lack that is worth most to
//!   us (`player_ai_bonus_val`).
//! - **Bazaar of Wonders:** a world wonder a city of ours builds, when 5 or more turns are left and
//!   the price leaves a reserve; else the best world wonder we can afford is started in the city that
//!   values it most, and bought.
//! - **Grand Order:** when two thirds of our cities (three or more) get 5 or more turns of
//!   production from it.

use std::sync::OnceLock;

use ffh_core::ffh::cards::world_spells as ws;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::view::{attack_power, defense_power, View};

/// What the AI does with one card's spell this turn.
type Decide = fn(&mut Game, &View, SpellType);

/// The batch's spells and their decisions, in the order they are considered (the gold of Mammon's
/// Bargain first, the Grand Order after the Bazaar has bought its wonder).
const CARDS: [(&str, Decide); 10] = [
    ("SPELL_MAMMONS_BARGAIN", mammons_bargain),
    ("SPELL_CALL_OF_THE_FAITHFUL", call_of_the_faithful),
    ("SPELL_SEAL_OF_BINDING", seal_of_binding),
    ("SPELL_MASQUE_OF_THE_RED_DEATH", masque),
    ("SPELL_ASTRAL_LEAP", astral_leap),
    ("SPELL_PURCHASE_CHARTER", purchase_charter),
    ("SPELL_CULTURAL_ANNEXATION", cultural_annexation),
    ("SPELL_TRANSMUTATION_CHARTER", transmutation_charter),
    ("SPELL_BAZAAR_OF_WONDERS", bazaar_of_wonders),
    ("SPELL_GRAND_ORDER", grand_order),
];

fn types(d: &GameData) -> &'static [Option<SpellType>] {
    static T: OnceLock<Vec<Option<SpellType>>> = OnceLock::new();
    T.get_or_init(|| CARDS.iter().map(|(name, _)| d.ty::<SpellType>(name)).collect())
}

/// The batch's turn (`cards::TURNS`): each spell whose card our team knows and that we have not
/// cast.
pub fn turn(g: &mut Game, v: &View) {
    if v.barbarian {
        return;
    }
    let types = types(g.d);
    for (i, &(_, decide)) in CARDS.iter().enumerate() {
        let Some(spell) = types[i] else { continue };
        if holds(g, v.p, spell) {
            decide(g, v, spell);
        }
    }
}

/// Does `p` hold `spell`: its team knows its card, and `p` has not cast it?
fn holds(g: &Game, p: PlayerId, spell: SpellType) -> bool {
    let team = g.players[p].team;
    g.d.spells[spell].tech_prereq.is_some_and(|t| g.team_is_has_tech(team, Some(t))) && !g.player_cast_once(p, spell)
}

/// Our units that may cast a card's spell, in the order tried: not marching, not having cast this
/// turn, a city defender first (the army keeps its turn).
fn casters(g: &Game, p: PlayerId) -> Vec<UnitId> {
    let mut units: Vec<UnitId> =
        g.players[p].units.iter().copied().filter(|&u| g.units.contains(u) && g.unit_march(u).is_none() && !g.units[u].has_casted && g.unit_plot_opt(u).is_some()).collect();
    units.sort_by_key(|&u| (g.units[u].ai_type != Some(UnitAIType::UNITAI_CITY_DEFENSE), u));
    units
}

/// The valid targets of `spell` for `p`, from the first unit that could cast it if it had a target
/// (`unit_can_cast` with `test_visible`: the batch's spells are `display_when_disabled`, so that
/// stops before the targets), and that unit. The targets of these spells but Astral Leap do not
/// depend on the caster. `None`: no unit of ours could cast it.
fn spell_targets(g: &Game, p: PlayerId, spell: SpellType) -> Option<(UnitId, Vec<SpellTarget>)> {
    let u = casters(g, p).into_iter().find(|&u| g.unit_can_cast(u, spell, true))?;
    Some((u, g.unit_spell_targets(u, spell)))
}

/// `FFH_AI_TRACE_CAST` is set (read once).
fn trace_cast_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("FFH_AI_TRACE_CAST").is_ok())
}

/// Casts an untargeted card spell with the first of our units that can; true if cast.
fn cast_now(g: &mut Game, p: PlayerId, spell: SpellType, ctx: &str, value: i32) -> bool {
    let Some(u) = casters(g, p).into_iter().find(|&u| g.unit_can_cast(u, spell, false)) else { return false };
    crate::magic::cast(g, u, spell, ctx, value);
    true
}

/// Casts a targeted card spell at `target` with the first of `units` that can (the trace line of
/// `magic::cast`, with the target); true if cast.
fn cast_at(g: &mut Game, units: &[UnitId], spell: SpellType, target: SpellTarget, ctx: &str, value: i32) -> bool {
    let Some(u) = units.iter().copied().find(|&u| g.units.contains(u) && g.unit_can_cast_at(u, spell, target)) else { return false };
    if trace_cast_on() {
        let p = g.units[u].owner;
        let civ = g.player_get_civilization_type(p).map_or("NONE", |c| g.d.civilizations[c].base.type_.as_str());
        eprintln!(
            "[cast t{} p{}] civ={} unit={} spell={} ctx={} value={} target={:?}",
            g.game_turn(),
            p.0,
            civ.trim_start_matches("CIVILIZATION_"),
            g.d.units[g.units[u].unit_type].base.type_.trim_start_matches("UNIT_"),
            g.d.spells[spell].base.type_.trim_start_matches("SPELL_"),
            ctx,
            value,
            target
        );
    }
    g.tel_cast_reason(u, ctx, value as i64);
    g.unit_cast_at(u, spell, target);
    true
}

/// At war with a civilization (not only the barbarians).
fn civ_war(v: &View) -> bool {
    !v.civ_enemies.is_empty()
}

/// Our army units within `range` plots of `plot`.
fn army_near(g: &Game, p: PlayerId, plot: PlotIdx, range: i32) -> i32 {
    let (x, y) = g.map.xy(plot);
    let mut n = 0;
    for dx in -range..=range {
        for dy in -range..=range {
            let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
            n += g.map.plots[q].units.iter().filter(|&&u| g.units[u].owner == p && crate::military::is_army_unit(g, u)).count() as i32;
        }
    }
    n
}

/// Is a civilization at peace with us at least twice our power (one we do not provoke)?
fn strong_neighbour(g: &Game, p: PlayerId, other: PlayerId) -> bool {
    let (team, ot) = (g.players[p].team, g.players[other].team);
    !g.player_is_barbarian(other) && !g.at_war(team, ot) && g.players[other].power as i64 > 2 * g.players[p].power.max(1) as i64
}

// ---------------------------------------------------------------------------------------------

/// Mammon's Bargain: an evil civilization at once when broke or at war with a civilization;
/// another (one that drew the card and changed alignment since) only when broke.
fn mammons_bargain(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    let broke = g.players[p].gold < 100 || crate::production::financial_trouble(g, p);
    let evil = g.d.ty::<AlignmentType>("ALIGNMENT_EVIL").is_some_and(|e| g.players[p].alignment == Some(e));
    if broke || evil && civ_war(v) {
        crate::trace!(g, p, "card: Mammon's Bargain (gold {}, broke {})", g.players[p].gold, broke);
        cast_now(g, p, spell, "card_mammon", ws::MAMMON_GOLD);
    }
}

/// Call of the Faithful: as soon as a hero of our state religion is missing (the requirement).
fn call_of_the_faithful(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    if let Some((_, heroes)) = ws::call_of_the_faithful(g, p) {
        crate::trace!(g, p, "card: Call of the Faithful ({} heroes)", heroes.len());
        cast_now(g, p, spell, "card_faithful", heroes.len() as i32);
    }
}

/// Seal of Binding: the strongest world unit or hero of a civilization at war with us within 3
/// plots of our borders (a barbarian one within 2 plots of a city of ours), once it is at least as
/// strong as our strongest unit.
fn seal_of_binding(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    if v.enemies.is_empty() {
        return;
    }
    let Some((_, targets)) = spell_targets(g, p, spell) else { return };
    let ours = g.players[p].units.iter().map(|&u| attack_power(g, u)).max().unwrap_or(0);
    let near = |u: UnitId| {
        let plot = g.unit_plot(u);
        if g.player_is_barbarian(g.units[u].owner) {
            let (x, y) = g.map.xy(plot);
            g.players[p].cities.iter().any(|&c| g.plot_distance(x, y, g.cities[c].x, g.cities[c].y) <= 2)
        } else {
            ffh_core::ffh::cards::world::within_range_of_borders(g, p, plot, 3)
        }
    };
    let best = targets
        .iter()
        .filter_map(|&t| match t {
            SpellTarget::Unit(u) => Some(u),
            _ => None,
        })
        .filter(|&u| near(u))
        .map(|u| (attack_power(g, u), std::cmp::Reverse(u)))
        .max();
    let Some((power, std::cmp::Reverse(u))) = best else { return };
    if power > 0 && power >= ours {
        crate::trace!(g, p, "card: Seal of Binding on {} at {:?}", g.d.units[g.units[u].unit_type].base.type_, g.map.xy(g.unit_plot(u)));
        cast_at(g, &casters(g, p), spell, SpellTarget::Unit(u), "card_seal", power);
    }
}

/// Masque of the Red Death: the largest enemy city of 6 or more our army is near (3 or more of its
/// units within 3 plots), or any of 10 or more.
fn masque(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    if !civ_war(v) {
        return;
    }
    let Some((_, targets)) = spell_targets(g, p, spell) else { return };
    let best = targets
        .iter()
        .filter_map(|&t| match t {
            SpellTarget::City(c) => Some(c),
            _ => None,
        })
        .filter(|&c| {
            let pop = g.cities[c].population;
            pop >= 10 || pop >= 6 && army_near(g, p, g.city_plot(c), 3) >= 3
        })
        .map(|c| (g.cities[c].population, std::cmp::Reverse(c)))
        .max();
    if let Some((pop, std::cmp::Reverse(c))) = best {
        crate::trace!(g, p, "card: Masque of the Red Death on {} ({})", g.cities[c].name, pop);
        cast_at(g, &casters(g, p), spell, SpellTarget::City(c), "card_masque", pop);
    }
}

/// A city of ours a stack may leave: no enemy within 3 plots and none that reaches it next turn,
/// not in peril, not the capital.
fn safe_source(g: &Game, v: &View, c: CityId) -> bool {
    let plot = g.city_plot(c);
    !g.city_is_capital(c) && v.danger_at(plot) == 0 && crate::production::local_threat(g, v.p, plot, 3) == 0 && !v.peril.iter().any(|&(q, _)| q == plot)
}

/// The units of `p` in its city `c`.
fn units_in(g: &Game, p: PlayerId, c: CityId) -> Vec<UnitId> {
    g.map.plots[g.city_plot(c)].units.iter().copied().filter(|&u| g.units[u].owner == p).collect()
}

/// Is `plot` 6 or more plots from city `c` (a leap, not a walk)?
fn far(g: &Game, c: CityId, plot: PlotIdx) -> bool {
    let (x, y) = g.map.xy(plot);
    g.plot_distance(g.cities[c].x, g.cities[c].y, x, y) >= 6
}

/// Astral Leap: relief for a city in peril, else a stack next to an enemy city it takes.
fn astral_leap(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    if !civ_war(v) {
        return;
    }
    // The stacks that may leap: a safe city's units (their attack and defence, the attackers).
    let mut sources: Vec<(CityId, i32, i32, i32)> = Vec::new();
    for &c in &g.players[p].cities {
        if !safe_source(g, v, c) {
            continue;
        }
        let units: Vec<UnitId> = g.map.plots[g.city_plot(c)].units.iter().copied().filter(|&u| g.units[u].owner == p).collect();
        let attack: i32 = units.iter().filter(|&&u| crate::military::is_army_unit(g, u)).map(|&u| attack_power(g, u)).sum();
        let defense: i32 = units.iter().map(|&u| defense_power(g, u)).sum();
        let attackers = units.iter().filter(|&&u| crate::military::is_army_unit(g, u) && attack_power(g, u) > 0).count() as i32;
        sources.push((c, attack, defense, attackers));
    }
    // Relief: the capital or a city of 5 or more in peril; the strongest defence that covers the
    // shortfall, from a city 6 or more plots away.
    let mut best: Option<(i32, CityId, PlotIdx)> = None;
    for &(q, shortfall) in &v.peril {
        let Some(c) = g.map.plots[q].plot_city.filter(|&c| g.cities.contains(c) && g.cities[c].owner == p) else { continue };
        if !g.city_is_capital(c) && g.cities[c].population < 5 {
            continue;
        }
        for &(s, _, defense, _) in &sources {
            if defense * 3 / 2 < shortfall || !far(g, s, q) {
                continue;
            }
            let Some(&u) = units_in(g, p, s).first() else { continue };
            if ws::astral_leap_ok(g, u, q) && best.map_or(true, |(b, bs, _)| defense > b || defense == b && s < bs) {
                best = Some((defense, s, q));
            }
        }
    }
    if let Some((defense, s, q)) = best {
        crate::trace!(g, p, "card: Astral Leap from {} to the city at {:?} in peril", g.cities[s].name, g.map.xy(q));
        let units = units_in(g, p, s);
        cast_at(g, &units, spell, SpellTarget::Plot(q), "card_leap_relief", defense);
        return;
    }
    // Attack: an enemy civilization's city we see, 6 or more plots from the stack, with no standing
    // Sentinel and no visible defender, or defenders a third of the stack's attack; the plot next to
    // it that the leap may reach. The capital and the most populous first.
    let team = v.team;
    let mut best: Option<((i32, i32), CityId, PlotIdx, CityId)> = None;
    for pl in g.players.iter() {
        if !pl.alive || !v.civ_enemies.contains(&pl.team) {
            continue;
        }
        for &e in &pl.cities {
            let ep = g.city_plot(e);
            if !g.plot_is_visible(ep, team) {
                continue;
            }
            // Standing Sentinels fight for the city: it is not undefended.
            if !g.city_sentinels_standing(e).is_empty() {
                continue;
            }
            let defenders = crate::combat_sim::visible_defenders(g, team, ep);
            let their: i32 = defenders.iter().map(|&u| defense_power(g, u)).sum();
            let value = (g.city_is_capital(e) as i32, g.cities[e].population);
            for &(s, attack, _, attackers) in &sources {
                if attackers < 3 || !far(g, s, ep) || attack < 3 * their {
                    continue;
                }
                let Some(&u) = units_in(g, p, s).first() else { continue };
                let (x, y) = g.map.xy(ep);
                let Some(q) = (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))).filter_map(|(dx, dy)| g.plot_xy(x, y, dx, dy)).find(|&q| q != ep && ws::astral_leap_ok(g, u, q)) else { continue };
                if best.map_or(true, |(b, be, _, _)| value > b || value == b && e < be) {
                    best = Some((value, e, q, s));
                }
            }
        }
    }
    if let Some((value, e, q, s)) = best {
        crate::trace!(g, p, "card: Astral Leap from {} next to {} at {:?}", g.cities[s].name, g.cities[e].name, g.map.xy(q));
        let units = units_in(g, p, s);
        cast_at(g, &units, spell, SpellTarget::Plot(q), "card_leap_attack", value.1);
    }
}

/// The least population of a city worth a once-a-game card that takes it.
const TAKE_MIN_POPULATION: i32 = 3;

/// The eligible city of `TAKE_MIN_POPULATION` or more with the most population (then buildings,
/// then the lower id), not from a civilization at peace with twice our power.
fn best_city(g: &Game, p: PlayerId, targets: &[SpellTarget]) -> Option<CityId> {
    targets
        .iter()
        .filter_map(|&t| match t {
            SpellTarget::City(c) => Some(c),
            _ => None,
        })
        .filter(|&c| g.cities[c].population >= TAKE_MIN_POPULATION && !strong_neighbour(g, p, g.cities[c].owner))
        .max_by_key(|&c| (g.cities[c].population, g.cities[c].num_buildings, std::cmp::Reverse(c)))
}

/// Purchase Charter: the best eligible city we can afford, keeping 50 gold.
fn purchase_charter(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    let Some((_, mut targets)) = spell_targets(g, p, spell) else { return };
    targets.retain(|&t| match t {
        SpellTarget::City(c) => g.players[p].gold - ws::purchase_charter_price(g, c) >= 50,
        _ => false,
    });
    if let Some(c) = best_city(g, p, &targets) {
        let price = ws::purchase_charter_price(g, c);
        crate::trace!(g, p, "card: Purchase Charter buys {} for {} gold", g.cities[c].name, price);
        cast_at(g, &casters(g, p), spell, SpellTarget::City(c), "card_purchase", price);
    }
}

/// Cultural Annexation: the best eligible city.
fn cultural_annexation(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    let Some((_, targets)) = spell_targets(g, p, spell) else { return };
    if let Some(c) = best_city(g, p, &targets) {
        crate::trace!(g, p, "card: Cultural Annexation takes {}", g.cities[c].name);
        cast_at(g, &casters(g, p), spell, SpellTarget::City(c), "card_annex", g.cities[c].population);
    }
}

/// Transmutation Charter: a resource we have twice into the one we lack that is worth most to us.
fn transmutation_charter(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    let Some((_, targets)) = spell_targets(g, p, spell) else { return };
    let mut best: Option<(i32, i32)> = None;
    for &t in &targets {
        let SpellTarget::Choice(choice) = t else { continue };
        let Some((plot, new)) = ws::transmutation_decode(g, choice) else { continue };
        let Some(old) = g.map.plots[plot].bonus_type else { continue };
        if g.player_get_num_available_bonuses(p, old) < 2 || g.player_get_num_available_bonuses(p, new) > 0 {
            continue;
        }
        let value = g.player_ai_bonus_val(p, new, 1);
        if value > 0 && best.map_or(true, |(b, bc)| value > b || value == b && choice < bc) {
            best = Some((value, choice));
        }
    }
    if let Some((value, choice)) = best {
        crate::trace!(g, p, "card: Transmutation Charter, {}", ws::transmutation_charter_choice_label(g, choice));
        cast_at(g, &casters(g, p), spell, SpellTarget::Choice(choice), "card_transmute", value);
    }
}

/// The gold a wonder purchase leaves us at least.
const BAZAAR_RESERVE: i32 = 100;

/// Bazaar of Wonders: a world wonder one of our cities builds, with 5 or more turns left, that we
/// can buy keeping the reserve; else the best world wonder we can afford (`building_value`) is
/// started at the front of the queue of the city that values it most, and bought.
fn bazaar_of_wonders(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    let gold = g.players[p].gold;
    let Some((_, targets)) = spell_targets(g, p, spell) else { return };
    let building = targets
        .iter()
        .filter_map(|&t| match t {
            SpellTarget::City(c) => Some(c),
            _ => None,
        })
        .filter(|&c| gold - ws::bazaar_price(g, c) >= BAZAAR_RESERVE && g.city_production_turns_left(c) >= 5)
        .max_by_key(|&c| (g.city_production_turns_left(c), std::cmp::Reverse(c)));
    if let Some(c) = building {
        crate::trace!(g, p, "card: Bazaar of Wonders completes the wonder of {}", g.cities[c].name);
        cast_at(g, &casters(g, p), spell, SpellTarget::City(c), "card_bazaar", ws::bazaar_price(g, c));
        return;
    }
    // A wonder to start and buy: the one worth most to the city that values it most, among those
    // we can afford (3 gold per hammer of its full cost).
    let d = g.d;
    let mut best: Option<(i32, CityId, BuildingType)> = None;
    for &c in &g.players[p].cities {
        // (An army's requisition stays at the head of the queue: the wonder would not be.)
        if g.cities[c].order_queue.first().is_some_and(|o| o.requisition) {
            continue;
        }
        for b in d.buildings.keys() {
            let Some(class) = d.buildings[b].building_class_type else { continue };
            if !ffh_core::utils::is_world_wonder_class(d, class) || d.buildings[b].production_cost <= 0 || ws::is_victory_wonder(d, class) {
                continue;
            }
            if !g.city_can_construct(c, b, false, false, false) {
                continue;
            }
            let price = ws::BAZAAR_GOLD_PER_HAMMER * (g.city_production_needed_building(c, b) - g.cities[c].building_production[b]).max(0);
            if gold - price < BAZAAR_RESERVE {
                continue;
            }
            let value = crate::production::building_value(g, c, b, 0);
            if value > 0 && best.map_or(true, |(bv, bc, bb)| (value, std::cmp::Reverse(c), std::cmp::Reverse(b)) > (bv, std::cmp::Reverse(bc), std::cmp::Reverse(bb))) {
                best = Some((value, c, b));
            }
        }
    }
    let Some((_, c, b)) = best else { return };
    g.city_push_order(c, OrderType::ORDER_CONSTRUCT, b.i(), -1, false, false, false, false);
    let bought = ws::bazaar_wonder(g, c) == Some(b) && {
        crate::trace!(g, p, "card: Bazaar of Wonders buys {} in {}", d.buildings[b].base.type_, g.cities[c].name);
        let price = ws::bazaar_price(g, c);
        cast_at(g, &casters(g, p), spell, SpellTarget::City(c), "card_bazaar", price)
    };
    // Not bought after all: the order we added goes.
    if !bought {
        if let Some(i) = g.cities[c].order_queue.iter().position(|o| o.order_type == OrderType::ORDER_CONSTRUCT && o.data1 == b.i()) {
            g.city_pop_order(c, i as i32, false, false);
        }
    }
}

/// Grand Order: when two thirds of our cities (three or more) get 5 or more turns of production.
fn grand_order(g: &mut Game, v: &View, spell: SpellType) {
    let p = v.p;
    let n = g.players[p].cities.len();
    if n < 3 {
        return;
    }
    let mut big = 0;
    for &c in &g.players[p].cities {
        let Some((gain, _)) = ws::grand_order_gain(g, c) else { continue };
        if gain >= 5 * g.city_current_production_difference(c, false, false).max(1) {
            big += 1;
        }
    }
    if big >= 3 && big * 3 >= n * 2 {
        crate::trace!(g, p, "card: Grand Order ({big} of {n} cities with long builds)");
        cast_now(g, p, spell, "card_grand_order", big as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::one_landmass_game;
    use crate::view::View;

    const US: PlayerId = PlayerId(0);
    const THEM: PlayerId = PlayerId(1);

    /// `p`'s team learns the card of `spell`.
    fn learn(g: &mut Game, p: PlayerId, spell: &str) -> SpellType {
        let s = g.d.t::<SpellType>(spell);
        let team = g.players[p].team;
        g.team_set_has_tech(team, g.d.spells[s].tech_prereq.unwrap(), true, Some(p), false, false);
        s
    }

    fn unit(g: &mut Game, p: PlayerId, name: &str, x: i32, y: i32) -> UnitId {
        let t = g.d.t::<UnitType>(name);
        g.player_init_unit(p, t, x, y, None, None)
    }

    fn war(g: &mut Game) {
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_meet(t0, t1, false);
        g.team_declare_war(t0, t1, false, None);
    }

    /// Our cards' turn, on a fresh view.
    fn play(g: &mut Game) {
        let v = View::new(g, US);
        turn(g, &v);
    }

    #[test]
    fn mammons_bargain_waits_for_need() {
        let mut g = one_landmass_game(41);
        let spell = learn(&mut g, US, "SPELL_MAMMONS_BARGAIN");
        g.player_found(US, 4, 5);
        unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        g.players[US].gold = 500;
        play(&mut g);
        assert!(!g.players[US].once_spells.contains(&spell), "rich and at peace: kept");
        g.players[US].gold = 0;
        play(&mut g);
        assert!(g.players[US].once_spells.contains(&spell), "broke: cast");
        assert_eq!(g.players[US].gold, ws::MAMMON_GOLD);
    }

    #[test]
    fn seal_of_binding_holds_an_enemy_hero_stronger_than_our_army() {
        let mut g = one_landmass_game(42);
        let spell = learn(&mut g, US, "SPELL_SEAL_OF_BINDING");
        g.player_found(US, 4, 5);
        unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        unit(&mut g, US, "UNIT_SCOUT", 6, 5);
        let hero = unit(&mut g, THEM, "UNIT_AXEMAN", 7, 5);
        g.unit_set_has_promotion(hero, g.d.t::<PromotionType>("PROMOTION_HERO"), true);
        war(&mut g);
        play(&mut g);
        assert!(g.players[US].once_spells.contains(&spell));
        assert_eq!(g.unit_get_immobile_timer(hero), ws::SEAL_TURNS);
    }

    #[test]
    fn masque_strikes_a_large_enemy_city() {
        let mut g = one_landmass_game(43);
        let spell = learn(&mut g, US, "SPELL_MASQUE_OF_THE_RED_DEATH");
        g.player_found(US, 4, 5);
        unit(&mut g, US, "UNIT_WARRIOR", 4, 5);
        g.player_found(THEM, 15, 5);
        let c = g.players[THEM].cities[0];
        g.city_set_population(c, 6);
        unit(&mut g, US, "UNIT_SCOUT", 14, 5);
        war(&mut g);
        play(&mut g);
        assert!(!g.players[US].once_spells.contains(&spell), "a city of 6 our army is not near: kept");
        g.city_set_population(c, 10);
        play(&mut g);
        assert!(g.players[US].once_spells.contains(&spell));
        assert_eq!(g.cities[c].population, 5);
    }

    #[test]
    fn astral_leap_puts_a_stack_next_to_an_undefended_enemy_city() {
        let mut g = one_landmass_game(44);
        let spell = learn(&mut g, US, "SPELL_ASTRAL_LEAP");
        g.player_found(US, 3, 3);
        g.player_found(US, 6, 6);
        let source = g.players[US].cities[1];
        assert!(!g.city_is_capital(source));
        let stack: Vec<UnitId> = (0..3).map(|_| unit(&mut g, US, "UNIT_AXEMAN", 6, 6)).collect();
        for &u in &stack {
            g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_ATTACK);
        }
        g.player_found(THEM, 16, 6);
        let target = g.players[THEM].cities[0];
        unit(&mut g, US, "UNIT_SCOUT", 15, 7);
        war(&mut g);
        play(&mut g);
        assert!(g.players[US].once_spells.contains(&spell));
        let (tx, ty) = (g.cities[target].x, g.cities[target].y);
        for &u in &stack {
            let (x, y) = (g.units[u].x, g.units[u].y);
            assert_eq!(g.plot_distance(x, y, tx, ty), 1, "next to the enemy city");
        }
    }

    #[test]
    fn bazaar_of_wonders_starts_and_buys_a_wonder_it_can_afford() {
        let mut g = one_landmass_game(46);
        let d = g.d;
        let spell = learn(&mut g, US, "SPELL_BAZAAR_OF_WONDERS");
        g.player_found(US, 5, 5);
        let c = g.players[US].cities[0];
        unit(&mut g, US, "UNIT_WARRIOR", 5, 5);
        let team = g.players[US].team;
        let wonder = |b: BuildingType| d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_world_wonder_class(d, k)) && d.buildings[b].production_cost > 0;
        for b in d.buildings.keys().filter(|&b| wonder(b)) {
            if let Some(t) = d.buildings[b].prereq_and_tech.filter(|&t| !g.teams[team].has_tech[t]) {
                g.team_set_has_tech(team, t, true, Some(US), false, false);
            }
        }
        g.players[US].gold = 100;
        play(&mut g);
        assert!(!g.players[US].once_spells.contains(&spell), "no wonder it can afford");
        g.players[US].gold = 1_000_000;
        play(&mut g);
        assert!(g.players[US].once_spells.contains(&spell));
        assert!(d.buildings.keys().any(|b| wonder(b) && g.cities[c].num_real_building[b] > 0), "a wonder bought");
    }

    #[test]
    fn purchase_charter_buys_the_best_city_it_can_afford() {
        let mut g = one_landmass_game(45);
        let spell = learn(&mut g, US, "SPELL_PURCHASE_CHARTER");
        g.player_found(US, 3, 5);
        unit(&mut g, US, "UNIT_WARRIOR", 3, 5);
        g.player_found(THEM, 16, 5);
        g.player_found(THEM, 8, 5);
        g.player_found(THEM, 8, 8);
        let (small, smaller) = (g.players[THEM].cities[1], g.players[THEM].cities[2]);
        g.city_set_population(small, 4);
        g.city_set_population(smaller, 2);
        let (t0, t1) = (g.players[US].team, g.players[THEM].team);
        g.team_meet(t0, t1, false);
        g.players[US].gold = 5000;
        g.players[THEM].power = g.players[US].power;
        play(&mut g);
        assert!(g.players[US].once_spells.contains(&spell));
        let bought = g.map.plot_xy(8, 5).and_then(|q| g.map.plots[q].plot_city).unwrap();
        assert_eq!(g.cities[bought].owner, US, "the larger of the two");
    }
}
