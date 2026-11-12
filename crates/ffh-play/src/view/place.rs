//! World spells the player places when their ritual completes, no unit casting them
//! (`place_on_charge`, docs/CARDS.md "World rituals"; docs/HUMAN.md): Wellspring. The top bar's
//! world action carries the placement ([`world_action`]): the valid plots and the prompts; the
//! client opens it by itself while the cast is pending (the turn cannot end before it is placed
//! while a plot is valid). The menu of a plot
//! ([`menu`], the `place_menu` query) lists its mana types, each a compact row: the count before
//! and after placing it, the sphere's spells by tier, one line on what this placement changes and
//! effect chips; every sentence of what owning it gives ([`passive_lines`]) is in a tooltip.

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

/// The world action of placed spell `spell` while its cast is pending: `place` (its prompts and its
/// button's words), `target_kind`, `targets` (each `{target, at, label}`: its plot and its mana
/// type's name), `enabled` (`why` when no plot is valid), a `place` command at the first target
/// (the client puts the picked one in).
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
        j.set("why", crate::view::units::NO_TARGET_PENDING);
    }
    j.set(
        "place",
        crate::jobj! {
            "label" => format!("Place {name}"),
            "completed" => format!("{} completed. Place {what}: click a marked plot.", display(d, ritual)),
            "pick" => format!("Place {what} of {name}: click a marked plot."),
            "hint" => "Opens the placement: a marked plot on the map, then a mana type.",
            "title" => name.clone(),
        },
    );
    let first = targets.first().map(|&t| TargetRef::of(g, t)).unwrap_or(TargetRef::Choice(0));
    j.set("cmd", Command::Place { spell, target: first }.to_json());
}

/// `place_menu`: the mana types human may place with `spell` on plot `(x, y)`, each a compact row
/// (docs/HUMAN.md, "World rituals"):
/// - `owned` and `after` (how many the human owns now and after placing it here: [`node_counts`]),
///   `count` (`1 → 2`) and `count_tip`;
/// - `promotions`: the sphere's promotions, tier I to III ([`promotion_json`]), each with the
///   spells it opens;
/// - `now`: at most one line on what this placement changes (`{text, warn}`; absent: nothing to
///   say), [`now_line`];
/// - `chips`: the effect chips, each `{kind, label, tip, aria}` (+ `building`, `new`), [`chips`];
/// - `passive`: every line of what owning it gives ([`passive_lines`]), the `info` chip's tooltip.
///
/// `title` is the spell's name, `intro` the line under it.
pub fn menu(v: &Viewer, spell: SpellType, x: i32, y: i32) -> Result<Json, QueryErr> {
    let g = v.g;
    let d = g.d;
    if !d.spells[spell].place_on_charge {
        return Err(crate::query::invalid(format!("field `spell`: {} is not placed", crate::names::name(d, spell))));
    }
    let plot = g.map.plot_xy(x, y).ok_or_else(|| crate::query::invalid("not a plot of the map"))?;
    let nodes = ffh_core::ffh::world_ritual_spells::small::wellspring_manas(d);
    let mut rows = Vec::new();
    for t in g.player_place_targets(v.h, spell) {
        let Some((q, bonus)) = target_parts(g, spell, t) else { continue };
        if q != plot {
            continue;
        }
        let mut row = crate::jobj! { "target" => TargetRef::of(g, t).to_json() };
        if let Some(b) = bonus {
            let name = display(d, b).into_owned();
            let owned = g.player_get_num_available_bonuses(v.h, b);
            let counts = nodes.iter().find(|&&(m, _)| m == b).is_some_and(|&(_, node)| node_counts(g, v.h, q, b, node));
            let after = owned + i32::from(counts);
            row.set("bonus", Json::idx(b));
            row.set("name", name.clone());
            row.set("owned", owned);
            row.set("after", after);
            row.set("count", format!("{owned} → {after}"));
            row.set("count_tip", format!("You own {owned} {name}. After placing: {after}."));
            let promos = adept_mana_promotions(d, b).map(|ps| ps.iter().enumerate().map(|(i, &p)| promotion_json(d, b, i, p)).collect()).unwrap_or_default();
            row.set("promotions", Json::Arr(promos));
            if let Some(now) = now_line(v, b, owned, after) {
                row.set("now", now);
            }
            row.set("chips", Json::Arr(chips(v, b, owned, after)));
            row.set("passive", Json::Arr(passive_lines(v, b).into_iter().map(Json::from).collect()));
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(QueryErr::Invalid(format!("({x}, {y}) is not a valid plot for {} now", display(d, spell))));
    }
    // By name (the data lists Water before Shadow and Sun).
    rows.sort_by(|a, b| a.get("name").and_then(Json::as_str).unwrap_or("").cmp(b.get("name").and_then(Json::as_str).unwrap_or("")));
    let combat = d.ty::<UnitCombatType>(ADEPT_MANA_UNIT_COMBAT).map(|c| display(d, c).into_owned()).unwrap_or_default();
    Ok(crate::jobj! {
        "spell" => Json::idx(spell),
        "title" => display(d, spell).into_owned(),
        "intro" => format!("Choose a mana type for the marked plot. I, II, III: the spells of each tier for {combat}."),
        "x" => x,
        "y" => y,
        "where" => crate::view::near_own_city(v, x, y),
        "rows" => Json::Arr(rows),
    })
}

/// Whether a node of mana `b` (improvement `node`) placed on `plot` would count among player
/// `p`'s available `b` (`player_get_num_available_bonuses`: its capital's plot group), by the
/// rule that counts a plot's resource (`plot_update_plot_group_bonus`): the plot in the capital's
/// plot group, the resource not obsolete, its trade tech known, the node trading it, the plot on
/// the bonus network; and no resolution banning it.
pub fn node_counts(g: &Game, p: PlayerId, plot: PlotIdx, b: BonusType, node: ImprovementType) -> bool {
    let d = g.d;
    let team = g.players[p].team;
    let Some(capital) = g.player_get_capital_city(p) else { return false };
    let group = g.plot_get_owner_plot_group(g.city_plot(capital));
    if group.is_none() || g.map.plots[plot].owner != Some(p) || g.plot_get_plot_group(plot, p) != group {
        return false;
    }
    if g.player_is_full_member(p, VoteSourceType(0)) && g.no_bonus[b] {
        return false;
    }
    !g.team_is_bonus_obsolete(team, b) && g.team_is_has_tech(team, d.bonuses[b].tech_city_trade) && d.improvements[node].improvement_bonus[b].bonus_trade && g.plot_is_bonus_network(plot, team)
}

/// The one line on what placing mana `b` changes (`owned` now, `after` placing), by priority:
/// no effect (the node would not count; the reason when it is the plot's connection), the first
/// node (tier I learnable), a free-promotion threshold crossed ([`ADEPT_MANA_FREE`]). `{text,
/// warn}`; `None`: nothing to say.
pub fn now_line(v: &Viewer, b: BonusType, owned: i32, after: i32) -> Option<Json> {
    let g = v.g;
    let d = g.d;
    let promos = adept_mana_promotions(d, b)?;
    let combat = display(d, d.ty::<UnitCombatType>(ADEPT_MANA_UNIT_COMBAT)?).into_owned();
    if after == owned {
        // A trade tech not known (an old charge: the ritual needs the tech now) says nothing.
        if !g.team_is_has_tech(v.team, d.bonuses[b].tech_city_trade) {
            return None;
        }
        let text = if g.player_is_full_member(v.h, VoteSourceType(0)) && g.no_bonus[b] {
            format!("No effect: a resolution bans {}.", display(d, b))
        } else {
            "No effect: the plot is not connected to your capital.".to_string()
        };
        return Some(crate::jobj! { "text" => text, "warn" => true });
    }
    if owned == 0 {
        let p1 = &d.promotions[promos[0]];
        let with: Vec<String> = [p1.prereq_promotion, p1.promotion_prereq_and].into_iter().flatten().map(|q| display(d, q).into_owned()).collect();
        let who = if with.is_empty() { combat } else { format!("{combat} with {}", and_list(&with, false)) };
        return Some(crate::jobj! { "text" => format!("First node: {who} can learn {}.", display(d, promos[0])), "warn" => false });
    }
    for (i, &(n, channeling)) in ADEPT_MANA_FREE.iter().enumerate().rev() {
        if owned < n && after >= n {
            let who = match channeling.and_then(|c| d.ty::<PromotionType>(c)) {
                Some(c) => format!("New {combat} with {}", display(d, c)),
                None => format!("New {combat}"),
            };
            return Some(crate::jobj! { "text" => format!("{who} start with {}.", display(d, promos[i])), "warn" => false });
        }
    }
    None
}

/// The effect chips of mana `b`'s row, each present only when it applies: `health` and
/// `happiness` (the resource's own, `+1`), `affinity` (only when the human owns or can build a
/// unit with an affinity for it), `buildings` (its civilization's buildings and projects the mana
/// allows or speeds; `new` when this node is what first meets one's mana requirement) and `info`
/// (every passive line). Each `{kind, label, tip: [lines], aria}`.
pub fn chips(v: &Viewer, b: BonusType, owned: i32, after: i32) -> Vec<Json> {
    let g = v.g;
    let d = g.d;
    let info = &d.bonuses[b];
    let parts = passive_parts(v, b);
    let mut out = Vec::new();
    let chip = |kind: &str, label: String, tip: Vec<String>| {
        let aria = tip.first().cloned().unwrap_or_default();
        crate::jobj! { "kind" => kind, "label" => label, "tip" => Json::Arr(tip.into_iter().map(Json::from).collect()), "aria" => aria }
    };
    if info.health != 0 {
        out.push(chip("health", format!("{:+}", info.health), parts.city.iter().filter(|l| l.contains(" health ")).cloned().collect()));
    }
    if info.happiness != 0 {
        out.push(chip("happiness", format!("{:+}", info.happiness), parts.city.iter().filter(|l| l.contains(" happiness ")).cloned().collect()));
    }
    // Affinity: the human's units with one for this mana (their own, with their promotions), then
    // the unit types it can build that have one.
    let mut have: Vec<(String, i32)> = Vec::new();
    let mut per = 0;
    for &u in &g.players[v.h].units {
        let n = g.units[u].bonus_affinity[b];
        if n == 0 {
            continue;
        }
        per = per.max(n);
        let name = display(d, g.units[u].unit_type).into_owned();
        match have.iter_mut().find(|(m, _)| *m == name) {
            Some((_, k)) => *k += 1,
            None => have.push((name, 1)),
        }
    }
    let can: Vec<String> = d.units.keys().filter(|&u| d.units[u].bonus_affinity[b] != 0 && g.player_can_train(v.h, u, false, false, false)).map(|u| display(d, u).into_owned()).collect();
    if per == 0 {
        per = d.units.keys().filter(|&u| can.contains(&display(d, u).into_owned())).map(|u| d.units[u].bonus_affinity[b]).max().unwrap_or(0);
    }
    if !have.is_empty() || !can.is_empty() {
        let mut tip = parts.affinity.clone();
        if tip.is_empty() {
            // A unit's affinity from a promotion of its own only (none the data lists for it).
            tip.push(format!("Affinity: {per:+} per {} you own.", display(d, b)));
        }
        if !have.is_empty() {
            let l: Vec<String> = have.iter().map(|(n, k)| if *k > 1 { format!("{n} ({k})") } else { n.clone() }).collect();
            tip.push(format!("Yours: {}.", l.join(", ")));
        }
        if !can.is_empty() {
            tip.push(format!("You can build: {}.", can.join(", ")));
        }
        out.push(chip("affinity", format!("{per:+}"), tip));
    }
    if !parts.buildings.is_empty() {
        let mut tip = parts.buildings.clone();
        let mut new = Vec::new();
        if owned == 0 && after > 0 {
            for (bl, others) in &parts.allows {
                let met = others.iter().any(|&o| g.player_get_num_available_bonuses(v.h, o) > 0);
                if !met {
                    new.push(display(d, *bl).into_owned());
                }
            }
        }
        if !new.is_empty() {
            tip.insert(0, format!("This node meets the mana requirement of {}.", and_list(&new, false)));
        }
        let mut j = chip("buildings", parts.buildings.len().to_string(), tip);
        if let Some(&(bl, _)) = parts.allows.first() {
            j.set("building", Json::idx(bl));
        } else if let Some(&bl) = parts.speeds.first() {
            j.set("building", Json::idx(bl));
        }
        j.set("new", !new.is_empty());
        out.push(j);
    }
    let all = passive_lines(v, b);
    if !all.is_empty() {
        out.push(chip("info", String::new(), all));
    }
    out
}

/// A mana's passive lines by kind ([`passive_lines`] is their concatenation, in this order).
pub struct PassiveParts {
    /// Its cities' health, happiness, free promotion, research and other modifiers.
    pub city: Vec<String>,
    /// The free sphere promotions rule ([`free_promotions_line`]).
    pub free: Option<String>,
    /// Affinity, one line per strength.
    pub affinity: Vec<String>,
    /// The buildings and projects of the human's civilization it allows or speeds.
    pub buildings: Vec<String>,
    /// The buildings whose mana requirement it meets, each with the other manas that would.
    pub allows: Vec<(BuildingType, Vec<BonusType>)>,
    /// The buildings it changes otherwise (happiness, health, production).
    pub speeds: Vec<BuildingType>,
}

/// What owning mana (or any resource) `b` gives the human, bare, one line each: the health and
/// happiness of its cities, the free sphere promotions of new arcane units, the units' affinity for
/// it, the buildings it allows and the projects it speeds (those of the human's civilization).
/// Empty: none.
pub fn passive_lines(v: &Viewer, b: BonusType) -> Vec<String> {
    let p = passive_parts(v, b);
    p.city.into_iter().chain(p.free).chain(p.affinity).chain(p.buildings).collect()
}

/// [`passive_lines`] by kind.
pub fn passive_parts(v: &Viewer, b: BonusType) -> PassiveParts {
    let g = v.g;
    let d = g.d;
    let info = &d.bonuses[b];
    let name = display(d, b).into_owned();
    let civ = g.player_get_civilization_type(v.h);
    let ours = |bl: BuildingType| match (civ, d.buildings[bl].building_class_type) {
        (Some(c), Some(class)) => d.civilizations[c].civilization_buildings(class, &d.building_classes) == Some(bl),
        _ => false,
    };
    let mut city = Vec::new();
    if info.health != 0 {
        city.push(format!("{:+} health in each of your cities that has it.", info.health));
    }
    if info.happiness != 0 {
        city.push(format!("{:+} happiness in each of your cities that has it.", info.happiness));
    }
    if let Some(p) = info.free_promotion {
        city.push(format!("Units built in a city that has it get {}.", display(d, p)));
    }
    for (n, what) in [(info.research_modifier, "research"), (info.great_people_rate_modifier, "great people birth rate"), (info.maintenance_modifier, "city maintenance")] {
        if n != 0 {
            city.push(format!("{n:+}% {what} in each of your cities that has it."));
        }
    }
    // Affinity (`unit_update_bonus_affinity`): per one owned, to strength, or to the damage of the
    // resource's damage type.
    let mut by_n: Vec<(i32, Vec<String>)> = Vec::new();
    let mut add = |n: i32, who: String| match by_n.iter_mut().find(|(m, _)| *m == n) {
        Some((_, l)) if !l.contains(&who) => l.push(who),
        Some(_) => {}
        None => by_n.push((n, vec![who])),
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
    let affinity = by_n.iter().map(|(n, who)| format!("Affinity: {n:+}{kind} per {name} you own, for {}.", and_list(who, false))).collect();
    let mut buildings = Vec::new();
    let mut allows = Vec::new();
    let mut speeds = Vec::new();
    for bl in d.buildings.keys().filter(|&bl| ours(bl)) {
        let bi = &d.buildings[bl];
        if bi.prereq_and_bonus == Some(b) {
            buildings.push(format!("Required by {}.", display(d, bl)));
            allows.push((bl, Vec::new()));
        }
        if bi.prereq_or_bonuses.contains(&Some(b)) {
            let others: Vec<BonusType> = bi.prereq_or_bonuses.iter().flatten().copied().filter(|&o| o != b).collect();
            let names: Vec<String> = others.iter().map(|&o| display(d, o).into_owned()).collect();
            buildings.push(format!("Allows {} (or {}).", display(d, bl), and_list(&names, true)));
            allows.push((bl, others));
        }
        let mut other = false;
        if bi.bonus_happiness_changes[b] != 0 {
            buildings.push(format!("{:+} happiness with {}.", bi.bonus_happiness_changes[b], display(d, bl)));
            other = true;
        }
        if bi.bonus_health_changes[b] != 0 {
            buildings.push(format!("{:+} health with {}.", bi.bonus_health_changes[b], display(d, bl)));
            other = true;
        }
        if bi.bonus_production_modifier[b] != 0 {
            buildings.push(format!("{:+}% production of {}.", bi.bonus_production_modifier[b], display(d, bl)));
            other = true;
        }
        if other {
            speeds.push(bl);
        }
    }
    for pr in d.projects.keys() {
        let pi = &d.projects[pr];
        if pi.bonus_production_modifier[b] != 0 && (pi.prereq_civilization.is_none() || pi.prereq_civilization == civ) {
            buildings.push(format!("{:+}% production of {}.", pi.bonus_production_modifier[b], display(d, pr)));
        }
    }
    PassiveParts { city, free: free_promotions_line(d, b), affinity, buildings, allows, speeds }
}

/// Spell help without its promotion requirement lines (`Requires the Fire I promotion`): the
/// menu's tooltip says `Needs Fire I.` first.
fn help_without_promotions(help: &str) -> String {
    help.split(NEWLINE)
        .filter(|l| !(l.starts_with("[ICON_BULLET]Requires the ") && (l.ends_with(" promotion") || l.ends_with(" promotions"))))
        .collect::<Vec<_>>()
        .join(NEWLINE)
}

const NEWLINE: &str = "[NEWLINE]";

/// A sphere promotion `p` (tier `tier`, 0-based) of mana `b`, as the menu shows it: `promotion`,
/// `name`, `tier` (`I`), `needs` (its requirements), `free` (when new arcane units get it free) and
/// `spells`: those it opens (`promotion_prereq1` or `2`; one entry per name), each `{spell, name,
/// needs, help}`:
/// `needs` `Needs Fire I.` (with the other promotion it may need), `help` the game's help for that
/// spell without its promotion lines.
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
        Some(ch) => format!("Free to new {} with {n} or more {} and {}.", display(d, c), display(d, b), display(d, ch)),
        None => format!("Free to new {} with {n} or more {}.", display(d, c), display(d, b)),
    });
    // Spells of one name (Life III's two Resurrections) are one entry, their helps one after the
    // other.
    let mut spells: Vec<Json> = Vec::new();
    for s in d.spells.keys().filter(|&s| d.spells[s].promotion_prereq1 == Some(p) || d.spells[s].promotion_prereq2 == Some(p)) {
        let si = &d.spells[s];
        let name = display(d, s).into_owned();
        let help = help_without_promotions(&crate::view::help::spell::effects(d, s));
        if let Some(j) = spells.iter_mut().find(|j| j.get("name").and_then(Json::as_str) == Some(name.as_str())) {
            let both = format!("{}{NEWLINE}{help}", j.get("help").and_then(Json::as_str).unwrap_or(""));
            j.set("help", both);
            continue;
        }
        let req: Vec<String> = [si.promotion_prereq1, si.promotion_prereq2].into_iter().flatten().map(|q| display(d, q).into_owned()).collect();
        spells.push(crate::jobj! {
            "spell" => Json::idx(s),
            "name" => name,
            "needs" => format!("Needs {}.", and_list(&req, false)),
            "help" => help,
        });
    }
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

