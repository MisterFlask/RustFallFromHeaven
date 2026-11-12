//! World rituals as the human sees them (docs/CARDS.md, "World rituals"; docs/HUMAN.md): the
//! view's `rituals` (every civilization's ritual, as the human may know it), the words of the
//! top bar's world-spell button in each state, the production dialog's ritual line and the reason
//! a ritual cannot be chosen, the city's abandon question, and the `kind` of the ritual news in
//! the message log. The rules are ffh-core's (`ffh::world_rituals`); this file only reads them,
//! and every sentence the page shows is written here.

use ffh_core::ffh::world_rituals::WorldRitual;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::view::{city_name, Viewer};

/// `1 turn`, `3 turns`.
pub fn turns_text(n: i32) -> String {
    if n == 1 {
        "1 turn".to_string()
    } else {
        format!("{n} turns")
    }
}

/// A ritual's name (`Ritual: Warcry`).
pub fn ritual_name(d: &GameData, project: ProjectType) -> String {
    crate::names::display(d, project).into_owned()
}

/// A civilization's short name as the log writes it (`Hippus`).
fn civ_name(g: &Game, p: PlayerId) -> String {
    crate::names::resolve_keys(g.d, &g.player_get_civilization_short_description(p)).into_owned()
}

/// What a row of the rituals list shows: `in_progress` (production put into it; for the human
/// also queued with none yet: `queued`), `charged` (a city holds its charge) or `idle`.
pub fn row_state(v: &Viewer, r: &WorldRitual) -> &'static str {
    let g = v.g;
    if r.charged {
        return "charged";
    }
    let Some(c) = r.city else { return "idle" };
    if r.progress > 0 {
        "in_progress"
    } else if r.player == v.h || v.reveal {
        // Another player's ritual is news only once production goes into it (the announcement).
        if g.cities[c].order_queue.first().is_some_and(|o| o.order_type == OrderType::ORDER_CREATE && o.data1 == r.project.i()) {
            "queued"
        } else {
            "idle"
        }
    } else {
        "idle"
    }
}

/// The city a row names: the one holding the charge, else the one building it (none while idle).
fn row_city(g: &Game, r: &WorldRitual, state: &str) -> Option<CityId> {
    match state {
        "charged" => g.player_world_ritual_charge_city(r.player, r.project),
        "in_progress" | "queued" => r.city,
        _ => None,
    }
}

/// `progress` in percent of `cost` (0-100).
pub fn percent(progress: i32, cost: i32) -> i32 {
    if cost <= 0 {
        return 0;
    }
    ((progress as i64 * 100 / cost as i64) as i32).clamp(0, 100)
}

/// Ritual: The Tithe's `Named rival: <Civ>.` for player `p` (none for another ritual or with no
/// living rival named).
fn tithe_note(g: &Game, p: PlayerId, project: ProjectType) -> Option<String> {
    is_tithe(g, project).then(|| tithe_named(g, p)).flatten().map(|r| format!("Named rival: {}.", civ_name(g, r)))
}

/// The view's `rituals`: one row per living civilization with a world ritual (player order), as
/// the human may know it. A civilization the human's team has not met is `Unknown civilization`
/// (no `civ`); the city and its plot are shown for every row, met or not: the start and the
/// completion are announced to everyone. Another player's ritual queued with no production yet is
/// idle (not announced). The human's own row adds its ritual's state (`note`: The Tithe's named
/// rival), else null.
pub fn rituals(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let mut out = Vec::new();
    for r in g.world_rituals() {
        let met = v.player_met(r.player);
        let state = row_state(v, &r);
        let city = row_city(g, &r, state);
        let (progress, turns) = if matches!(state, "in_progress" | "queued") { (r.progress, r.turns_left) } else { (0, None) };
        out.push(crate::jobj! {
            "player" => r.player,
            "met" => met,
            "mine" => r.player == v.h,
            "civ" => if met { Json::opt_idx(g.player_get_civilization_type(r.player)) } else { Json::Null },
            "name" => if met { civ_name(g, r.player) } else { "Unknown civilization".to_string() },
            "ritual" => Json::idx(r.project),
            "ritual_name" => ritual_name(d, r.project),
            "spell" => Json::opt_idx(r.spell),
            "state" => state,
            "city" => city,
            "city_name" => city.map(|c| city_name(g, c)),
            "x" => city.map(|c| g.cities[c].x),
            "y" => city.map(|c| g.cities[c].y),
            "progress" => progress,
            "cost" => r.cost,
            "percent" => percent(progress, r.cost),
            "turns_left" => turns,
            "charged" => r.charged,
            "completed" => r.completed,
            "next_cost" => r.next_cost,
            "note" => if r.player == v.h { tithe_note(g, r.player, r.project) } else { None },
        });
    }
    Json::Arr(out)
}

/// The human's own row of [`Game::world_rituals`], if its civilization has a world ritual.
fn own(v: &Viewer) -> Option<WorldRitual> {
    v.g.world_rituals().into_iter().find(|r| r.player == v.h)
}

/// The world-spell button's `why` while the human holds no charge of its ritual `project`:
/// `No charge. Build <ritual> in a city (cost <cost>).`, or with the ritual queued
/// `No charge. <ritual>: <progress>/<cost> in <city>, <N> turns.`
pub fn no_charge_why(v: &Viewer, project: ProjectType) -> String {
    let g = v.g;
    let name = ritual_name(g.d, project);
    match own(v).filter(|r| r.project == project) {
        Some(WorldRitual { city: Some(c), progress, cost, turns_left, .. }) => {
            let turns = turns_left.map_or(String::new(), |n| format!(", {}", turns_text(n)));
            format!("No charge. {name}: {progress}/{cost} in {}{turns}.", city_name(g, c))
        }
        Some(r) => format!("No charge. Build {name} in a city (cost {}).", r.cost),
        None => format!("No charge. Build {name} in a city (cost {}).", g.player_get_production_needed_project(v.h, project)),
    }
}

/// The world-spell button's line while the human holds a charge of `project` and can cast it:
/// `Charge held in <city>. Casting spends it. Next ritual cost: <cost>.` (the cost the ritual has
/// now: the completion that left the charge doubled it already).
pub fn charge_line(v: &Viewer, project: ProjectType) -> Option<String> {
    let g = v.g;
    let c = g.player_world_ritual_charge_city(v.h, project)?;
    let cost = g.player_get_production_needed_project(v.h, project);
    Some(format!("Charge held in {}. Casting spends it. Next ritual cost: {cost}.", city_name(g, c)))
}

/// The production dialog's line for world ritual `project` in city `c`: `World ritual. One in
/// progress per player. Taken off the head of the queue, its production is lost. Cannot be
/// hurried. Each completion doubles the next cost (next: <next_cost>).`
pub fn rules_line(g: &Game, c: CityId, project: ProjectType) -> String {
    let p = g.cities[c].owner;
    let next = g.player_project_production_needed_at(p, project, 1);
    let mut s = String::from("World ritual. One in progress per player. Taken off the head of the queue, its production is lost. Cannot be hurried.");
    match g.d.projects[project].repeat_cost_percent {
        200 => s.push_str(&format!(" Each completion doubles the next cost (next: {next}).")),
        n if n > 0 => s.push_str(&format!(" Each completion sets the next cost to {n}% of the last (next: {next}).")),
        _ => {}
    }
    s
}

/// The Calabim's ritual (Ritual: The Tithe) and the living rival player `p` has named for it.
fn tithe_named(g: &Game, p: PlayerId) -> Option<PlayerId> {
    g.players[p].tithe_target.filter(|&r| g.players[r].alive)
}

/// Is `project` Ritual: The Tithe?
fn is_tithe(g: &Game, project: ProjectType) -> bool {
    ffh_core::ffh::world_ritual_spells::small::tithe_project(g.d) == Some(project)
}

/// Why city `c` cannot start world ritual `project` now (the production list shows it disabled
/// with this): `Charge held: cast <spell> first.`, `In progress in <city>.`, `Sanctuary holds: <N>
/// turns.`, `No rival named: use Name the Tithe.` (Ritual: The Tithe), else `Not available now.`
/// (a reason of the rules this list does not know).
pub fn cannot_reason(g: &Game, c: CityId, project: ProjectType) -> String {
    let d = g.d;
    let p = g.cities[c].owner;
    if g.player_world_ritual_charge_city(p, project).is_some() {
        if let Some(s) = g.world_ritual_spell(project) {
            return format!("Charge held: cast {} first.", crate::names::display(d, s));
        }
    }
    if let Some(o) = g.player_world_ritual_city(p, project).filter(|&o| o != c) {
        return format!("In progress in {}.", city_name(g, o));
    }
    if g.player_world_ritual_paused(p, project) {
        return format!("Sanctuary holds: {}.", turns_text(g.players[p].sanctuary_timer));
    }
    if is_tithe(g, project) && tithe_named(g, p).is_none() {
        if let Some(s) = d.ty::<SpellType>("SPELL_NAME_THE_TITHE") {
            return format!("No rival named: use {}.", crate::names::display(d, s));
        }
    }
    "Not available now.".to_string()
}

/// A world ritual's entry of the production list (`buildable`): `ritual: {rules, why, note}`
/// (`why` null while it can be chosen; `note` the ritual's own state, else null: Ritual: The
/// Tithe's `Named rival: <Civ>.`).
pub fn buildable_fields(g: &Game, c: CityId, project: ProjectType, enabled: bool, j: &mut Json) {
    if !g.project_is_world_ritual(project) {
        return;
    }
    let note = tithe_note(g, g.cities[c].owner, project);
    j.set(
        "ritual",
        crate::jobj! {
            "rules" => rules_line(g, c, project),
            "why" => if enabled { Json::Null } else { Json::from(cannot_reason(g, c, project)) },
            "note" => note,
        },
    );
}

/// The question asked before player `h` casts `spell` at `target`, when the cast has one: Name the
/// Tithe's `{title: "Name the Tithe", text: "Name <Civ>? This declares war on <Civ>.", ok: "Name"}`
/// (`Name <Civ>?` when at war with it already).
pub fn target_confirm(g: &Game, h: PlayerId, spell: SpellType, target: ffh_core::structs::SpellTarget) -> Option<Json> {
    let d = g.d;
    let ffh_core::structs::SpellTarget::Player(rival) = target else { return None };
    if d.ty::<SpellType>("SPELL_NAME_THE_TITHE") != Some(spell) {
        return None;
    }
    let civ = civ_name(g, rival);
    let (t, o) = (g.players[h].team, g.players[rival].team);
    let text = if g.teams[t].at_war[o] { format!("Name {civ}?") } else { format!("Name {civ}? This declares war on {civ}.") };
    Some(crate::jobj! { "title" => crate::names::display(d, spell).into_owned(), "text" => text, "ok" => "Name" })
}

/// The city screen's ritual fields: `ritual_head` (the world ritual in progress at the head of the
/// queue, with the question asked before an action takes it off: `Abandon <ritual>? Its <N>
/// production is lost.`) and `hurry_note` (a world ritual at the head: `A world ritual cannot be
/// hurried.`).
pub fn city_fields(g: &Game, c: CityId, j: &mut Json) {
    let d = g.d;
    let head = g.city_world_ritual_in_progress(c).map(|project| {
        let name = ritual_name(d, project);
        let progress = g.cities[c].project_production[project];
        crate::jobj! {
            "project" => Json::idx(project),
            "name" => name.as_str(),
            "progress" => progress,
            "confirm" => format!("Abandon {name}? Its {progress} production is lost."),
        }
    });
    j.set("ritual_head", head);
    j.set("hurry_note", g.city_head_world_ritual(c).map(|_| "A world ritual cannot be hurried."));
}

/// The world rituals' names, for [`tag_news`].
fn ritual_names() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        let d = ffh_data::global();
        d.projects.keys().filter(|&p| d.projects[p].world_ritual).map(|p| ritual_name(d, p)).collect()
    })
}

/// The verb of a ritual's news (`<Civ> began|abandoned|completed <ritual> in <city>.`, ffh-core's
/// `world_ritual_news`), if `text` is one.
pub fn news_verb(text: &str) -> Option<&'static str> {
    if !text.ends_with('.') {
        return None;
    }
    for verb in ["began", "abandoned", "completed"] {
        for name in ritual_names() {
            if text.contains(&format!(" {verb} {name} in ")) {
                return Some(verb);
            }
        }
    }
    None
}

/// A message's JSON with `kind: "ritual"` and `verb` when it is a ritual's news, so that the page
/// shows it as an alert without reading its wording.
pub fn tag_news(j: Json, text: &str) -> Json {
    match news_verb(text) {
        Some(verb) => j.with("kind", "ritual").with("verb", verb),
        None => j,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        assert_eq!(turns_text(1), "1 turn");
        assert_eq!(turns_text(4), "4 turns");
        assert_eq!(percent(34, 100), 34);
        assert_eq!(percent(150, 100), 100);
        assert_eq!(percent(5, 0), 0);
        assert_eq!(news_verb("Hippus began Ritual: Warcry in Lycus."), Some("began"));
        assert_eq!(news_verb("Unknown completed Ritual: Rally in X."), Some("completed"));
        assert_eq!(news_verb("Hippus began Ritual: Warcry in Lycus"), None);
        assert_eq!(news_verb("Your Warrior was killed."), None);
    }
}
