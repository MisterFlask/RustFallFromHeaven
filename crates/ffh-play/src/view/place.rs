//! World spells the player places with its ritual's charge, no unit casting them
//! (`place_on_charge`, docs/CARDS.md "World rituals"; docs/HUMAN.md): Wellspring. The top bar's
//! world action carries the placement ([`world_action`]): the valid plots and the prompts; the
//! client opens it by itself on the turn the ritual completes. The menu of a plot
//! ([`menu`], the `place_menu` query) lists its mana types, each with what owning that mana gives
//! ([`passive_lines`]) and the sphere promotions it opens, with the spells of each.

use ffh_core::ffh::events::{adept_mana_promotions, ADEPT_MANA_FREE, ADEPT_MANA_UNIT_COMBAT};
use ffh_core::ffh::world_ritual_spells::small::{wellspring_decode, wellspring_label};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::SpellTarget;
use ffh_data::*;

use crate::command::{Command, TargetRef};
use crate::json::Json;
use crate::names::display;
use crate::query::QueryErr;
use crate::view::Viewer;

/// What a placement puts on the map, for the prompts: `the mana node` (Wellspring).
fn noun(d: &GameData, spell: SpellType) -> String {
    match ffh_core::ffh::pycall::PyCall::parse(&d.spells[spell].py_targets).map(|c| c.name) {
        Some(n) if n == "targetsWellspring" => "the mana node".to_string(),
        _ => display(d, spell).into_owned(),
    }
}

/// The plot and the mana type of a placement target (Wellspring's choices).
fn target_parts(g: &Game, spell: SpellType, t: SpellTarget) -> Option<(PlotIdx, Option<BonusType>)> {
    match t {
        SpellTarget::Choice(i) => {
            let q = g.spell_choice_plot(spell, i)?;
            Some((q, wellspring_decode(g, i).map(|(_, b)| b)))
        }
        SpellTarget::Plot(q) => Some((q, None)),
        _ => None,
    }
}

/// The world action of placed spell `spell` while the human holds its charge: `place` (its
/// prompts and its button's words), `target_kind`, `targets` (each `{target, at, label}`: its plot
/// and its mana type's name), `enabled` (`why` when no plot is valid), a `place` command at the
/// first target (the client puts the picked one in) and `charge`.
pub fn world_action(v: &Viewer, spell: SpellType, ritual: ProjectType, j: &mut Json) {
    let g = v.g;
    let d = g.d;
    let name = display(d, spell).into_owned();
    let what = noun(d, spell);
    let targets = g.player_place_targets(v.h, spell);
    let list: Vec<Json> = targets
        .iter()
        .filter_map(|&t| {
            let (q, _) = target_parts(g, spell, t)?;
            let (x, y) = g.map.xy(q);
            let label = match t {
                SpellTarget::Choice(i) => wellspring_label(g, i),
                _ => crate::view::near_own_city(v, x, y).unwrap_or_default(),
            };
            Some(crate::jobj! { "target" => TargetRef::of(g, t).to_json(), "at" => crate::jarr![x, y], "label" => label })
        })
        .collect();
    let enabled = !list.is_empty();
    j.set("cast", false);
    j.set("caster", Json::Null);
    j.set("target_kind", d.spells[spell].target.map(crate::view::cards::target_kind_name));
    j.set("targets", Json::Arr(list));
    j.set("enabled", enabled);
    if !enabled {
        j.set("why", "Charge held. No valid plot now.");
    }
    j.set(
        "place",
        crate::jobj! {
            "label" => format!("Place {name}"),
            "completed" => format!("{} completed. Place {what}: click a marked plot.", display(d, ritual)),
            "pick" => format!("Place {what} of {name}: click a marked plot."),
            "later" => "Esc: later.",
            "hint" => "Opens the placement: a marked plot on the map, then a mana type.",
            "title" => name.clone(),
        },
    );
    let first = targets.first().map(|&t| TargetRef::of(g, t)).unwrap_or(TargetRef::Choice(0));
    j.set("cmd", Command::Place { spell, target: first }.to_json());
    if enabled {
        if let Some(line) = crate::view::rituals::charge_line_verb(v, ritual, "Placing") {
            j.set("charge", line);
        }
    }
}

/// `place_menu`: the mana types human may place with `spell` on plot `(x, y)`, each a row
/// `{target, bonus, name, owned, passive, promotions}`: `passive` the lines of what owning it
/// gives ([`passive_lines`]; empty: none), `owned` `You own: N.`, `promotions` the sphere
/// promotions it opens, tier 1 to 3 ([`promotion_json`]). `title` is the spell's name.
pub fn menu(v: &Viewer, spell: SpellType, x: i32, y: i32) -> Result<Json, QueryErr> {
    let g = v.g;
    let d = g.d;
    if !d.spells[spell].place_on_charge {
        return Err(crate::query::invalid(format!("field `spell`: {} is not placed", crate::names::name(d, spell))));
    }
    let plot = g.map.plot_xy(x, y).ok_or_else(|| crate::query::invalid("not a plot of the map"))?;
    let mut rows = Vec::new();
    for t in g.player_place_targets(v.h, spell) {
        let Some((q, bonus)) = target_parts(g, spell, t) else { continue };
        if q != plot {
            continue;
        }
        let mut row = crate::jobj! { "target" => TargetRef::of(g, t).to_json() };
        if let Some(b) = bonus {
            row.set("bonus", Json::idx(b));
            row.set("name", display(d, b).into_owned());
            row.set("owned", format!("You own: {}.", g.player_get_num_available_bonuses(v.h, b)));
            row.set("passive", Json::Arr(passive_lines(v, b).into_iter().map(Json::from).collect()));
            let promos = adept_mana_promotions(d, b).map(|ps| ps.iter().enumerate().map(|(i, &p)| promotion_json(d, b, i, p)).collect()).unwrap_or_default();
            row.set("promotions", Json::Arr(promos));
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(QueryErr::Invalid(format!("({x}, {y}) is not a valid plot for {} now", display(d, spell))));
    }
    Ok(crate::jobj! {
        "spell" => Json::idx(spell),
        "title" => display(d, spell).into_owned(),
        "x" => x,
        "y" => y,
        "where" => crate::view::near_own_city(v, x, y),
        "rows" => Json::Arr(rows),
    })
}

/// `A`, `A and B`, `A, B and C`.
fn and_list(items: &[String], or: bool) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} {} {}", items[..n - 1].join(", "), if or { "or" } else { "and" }, items[n - 1]),
    }
}

/// The free sphere promotions rule for mana `b` (FfH `onUnitCreated`, [`ADEPT_MANA_FREE`]): `New
/// Arcane Units get Fire I with 2 or more Fire Mana, Fire II with 3 or more and Channeling II, Fire
/// III with 4 or more and Channeling III.`
pub fn free_promotions_line(d: &GameData, b: BonusType) -> Option<String> {
    let promos = adept_mana_promotions(d, b)?;
    let combat = d.ty::<UnitCombatType>(ADEPT_MANA_UNIT_COMBAT)?;
    let mana = display(d, b);
    let parts: Vec<String> = promos
        .iter()
        .zip(ADEPT_MANA_FREE)
        .enumerate()
        .map(|(i, (&p, (n, channeling)))| {
            let with = match channeling.and_then(|c| d.ty::<PromotionType>(c)) {
                Some(c) => format!(" and {}", display(d, c)),
                None => String::new(),
            };
            let of = if i == 0 { format!(" {mana}") } else { String::new() };
            format!("{} with {n} or more{of}{with}", display(d, p))
        })
        .collect();
    Some(format!("New {} get {}.", display(d, combat), parts.join(", ")))
}

/// What owning mana (or any resource) `b` gives the human, bare, one line each: the health and
/// happiness of its cities, the free sphere promotions of new arcane units, the units' affinity for
/// it, the buildings it allows and the projects it speeds (those of the human's civilization), and
/// the tech it waits for. Empty: none.
pub fn passive_lines(v: &Viewer, b: BonusType) -> Vec<String> {
    let g = v.g;
    let d = g.d;
    let info = &d.bonuses[b];
    let name = display(d, b).into_owned();
    let civ = g.player_get_civilization_type(v.h);
    let ours = |bl: BuildingType| match (civ, d.buildings[bl].building_class_type) {
        (Some(c), Some(class)) => d.civilizations[c].civilization_buildings(class, &d.building_classes) == Some(bl),
        _ => false,
    };
    let mut out = Vec::new();
    if info.health != 0 {
        out.push(format!("{:+} health in each of your cities that has it.", info.health));
    }
    if info.happiness != 0 {
        out.push(format!("{:+} happiness in each of your cities that has it.", info.happiness));
    }
    if let Some(p) = info.free_promotion {
        out.push(format!("Units built in a city that has it get {}.", display(d, p)));
    }
    for (n, what) in [(info.research_modifier, "research"), (info.great_people_rate_modifier, "great people birth rate"), (info.maintenance_modifier, "city maintenance")] {
        if n != 0 {
            out.push(format!("{n:+}% {what} in each of your cities that has it."));
        }
    }
    out.extend(free_promotions_line(d, b));
    // Affinity (`unit_update_bonus_affinity`): per one owned, to strength, or to the damage of the
    // resource's damage type.
    let mut affinity: Vec<(i32, Vec<String>)> = Vec::new();
    let mut add = |n: i32, who: String| match affinity.iter_mut().find(|(m, _)| *m == n) {
        Some((_, l)) if !l.contains(&who) => l.push(who),
        Some(_) => {}
        None => affinity.push((n, vec![who])),
    };
    for u in d.units.keys().filter(|&u| d.units[u].bonus_affinity[b] != 0) {
        add(d.units[u].bonus_affinity[b], display(d, u).into_owned());
    }
    for p in d.promotions.keys().filter(|&p| d.promotions[p].bonus_affinity[b] != 0) {
        add(d.promotions[p].bonus_affinity[b], format!("units with {}", display(d, p)));
    }
    let kind = match info.damage_type {
        Some(t) => format!(" {} damage", display(d, t)),
        None => " strength".to_string(),
    };
    for (n, who) in &affinity {
        out.push(format!("Affinity: {n:+}{kind} per {name} you own, for {}.", and_list(who, false)));
    }
    for bl in d.buildings.keys().filter(|&bl| ours(bl)) {
        let bi = &d.buildings[bl];
        if bi.prereq_and_bonus == Some(b) {
            out.push(format!("Required by {}.", display(d, bl)));
        }
        if bi.prereq_or_bonuses.contains(&Some(b)) {
            let others: Vec<String> = bi.prereq_or_bonuses.iter().flatten().filter(|&&o| o != b).map(|&o| display(d, o).into_owned()).collect();
            out.push(format!("Allows {} (or {}).", display(d, bl), and_list(&others, true)));
        }
        if bi.bonus_happiness_changes[b] != 0 {
            out.push(format!("{:+} happiness with {}.", bi.bonus_happiness_changes[b], display(d, bl)));
        }
        if bi.bonus_health_changes[b] != 0 {
            out.push(format!("{:+} health with {}.", bi.bonus_health_changes[b], display(d, bl)));
        }
        if bi.bonus_production_modifier[b] != 0 {
            out.push(format!("{:+}% production of {}.", bi.bonus_production_modifier[b], display(d, bl)));
        }
    }
    for pr in d.projects.keys() {
        let pi = &d.projects[pr];
        if pi.bonus_production_modifier[b] != 0 && (pi.prereq_civilization.is_none() || pi.prereq_civilization == civ) {
            out.push(format!("{:+}% production of {}.", pi.bonus_production_modifier[b], display(d, pr)));
        }
    }
    if let Some(t) = info.tech_city_trade.filter(|&t| !g.teams[v.team].has_tech[t]) {
        out.push(format!("Its nodes count once your team knows {}.", display(d, t)));
    }
    out
}

/// A sphere promotion `p` (tier `tier`, 0-based) of mana `b`, as the menu shows it: `promotion`,
/// `name`, `tier` (`I`), `needs` (its requirements), `free` (when new arcane units get it free) and
/// `spells`: those it opens (`promotion_prereq1` or `2`), each `{spell, name, help}` with the help
/// the game shows for that spell.
pub fn promotion_json(d: &GameData, b: BonusType, tier: usize, p: PromotionType) -> Json {
    let pi = &d.promotions[p];
    let mut needs: Vec<String> = Vec::new();
    for q in [pi.prereq_promotion, pi.promotion_prereq_and].into_iter().flatten() {
        needs.push(display(d, q).into_owned());
    }
    let ors: Vec<String> = [pi.prereq_or_promotion1, pi.prereq_or_promotion2, pi.promotion_prereq_or3, pi.promotion_prereq_or4].into_iter().flatten().map(|q| display(d, q).into_owned()).collect();
    if !ors.is_empty() {
        needs.push(and_list(&ors, true));
    }
    if let Some(m) = pi.bonus_prereq {
        needs.push(display(d, m).into_owned());
    }
    if let Some(t) = pi.tech_prereq {
        needs.push(display(d, t).into_owned());
    }
    let combat = d.ty::<UnitCombatType>(ADEPT_MANA_UNIT_COMBAT);
    let (n, channeling) = ADEPT_MANA_FREE[tier.min(2)];
    let free = combat.map(|c| match channeling.and_then(|ch| d.ty::<PromotionType>(ch)) {
        Some(ch) => format!("Free to new {} with {} and {n} or more {}.", display(d, c), display(d, ch), display(d, b)),
        None => format!("Free to new {} with {n} or more {}.", display(d, c), display(d, b)),
    });
    let spells: Vec<Json> = d
        .spells
        .keys()
        .filter(|&s| d.spells[s].promotion_prereq1 == Some(p) || d.spells[s].promotion_prereq2 == Some(p))
        .map(|s| {
            crate::jobj! {
                "spell" => Json::idx(s),
                "name" => display(d, s).into_owned(),
                "help" => crate::view::help::spell::effects(d, s),
            }
        })
        .collect();
    crate::jobj! {
        "promotion" => Json::idx(p),
        "type" => pi.base.type_.as_str(),
        "name" => display(d, p).into_owned(),
        "tier" => ["I", "II", "III"][tier.min(2)],
        "needs" => if needs.is_empty() { String::new() } else { format!("Needs {}.", and_list(&needs, false)) },
        "free" => free,
        "spells" => Json::Arr(spells),
    }
}
