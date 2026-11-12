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
/// also queued with none yet: `queued`), `pending` (completed, its world spell's cast pending) or
/// `idle`.
pub fn row_state(v: &Viewer, r: &WorldRitual) -> &'static str {
    let g = v.g;
    if r.pending {
        return "pending";
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

/// The city a row names: the one building it (none while idle or pending).
fn row_city(r: &WorldRitual, state: &str) -> Option<CityId> {
    match state {
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
        let city = row_city(&r, state);
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
            "pending" => r.pending,
            "completed" => r.completed,
            "next_cost" => r.next_cost,
            "note" => if r.player == v.h { tithe_note(g, r.player, r.project) } else { None },
            // Its keywords' reminder texts (World Ritual, Ritual), shown as chips.
            "reminders" => crate::view::keywords::reminders(d, crate::view::keywords::of_project(d, r.project)),
        });
    }
    Json::Arr(out)
}

/// The production dialog's rules line of world ritual `project` in city `c`: what it leaves or
/// does and its next cost, the entity's own facts only (its keywords' rules are its reminders,
/// [`crate::view::keywords`]): `At completion: Warcry takes effect. Next cost: 400.`, `At
/// completion: cast The Cull. Next cost: 400.` for a targeted spell (`place Wellspring`), `Acts at
/// completion. Next cost: 400.` for a ritual no spell names.
pub fn rules_line(g: &Game, c: CityId, project: ProjectType) -> String {
    let p = g.cities[c].owner;
    let next = g.player_project_production_needed_at(p, project, 1);
    let does = match g.world_ritual_spell(project) {
        Some(s) if g.d.spells[s].place_on_charge => format!("At completion: place {}.", crate::names::display(g.d, s)),
        Some(s) if g.world_spell_targeted(s) => format!("At completion: cast {}.", crate::names::display(g.d, s)),
        Some(s) => format!("At completion: {} takes effect.", crate::names::display(g.d, s)),
        None => "Acts at completion.".to_string(),
    };
    if g.d.projects[project].repeat_cost_percent > 0 {
        format!("{does} Next cost: {next}.")
    } else {
        does
    }
}

/// The Calabim's ritual (Ritual: The Tithe) and the living rival player `p` has named for it.
fn tithe_named(g: &Game, p: PlayerId) -> Option<PlayerId> {
    g.players[p].tithe_target.filter(|&r| g.players[r].alive)
}

/// Is `project` Ritual: The Tithe?
fn is_tithe(g: &Game, project: ProjectType) -> bool {
    ffh_core::ffh::world_ritual_spells::small::tithe_project(g.d) == Some(project)
}

/// The tech city `c`'s civilization lacks for its world ritual `project` (`tech_prereq`, e.g.
/// Ritual: Wellspring's Knowledge of the Ether): the production list shows the ritual, disabled,
/// instead of hiding it as the rules' visibility test does. `None`: not a world ritual of the
/// city's civilization, or the tech is known.
pub fn ritual_needs_tech(g: &Game, c: CityId, project: ProjectType) -> Option<TechType> {
    let info = &g.d.projects[project];
    let city = &g.cities[c];
    if !info.world_ritual || g.is_option(GameOptionType::GAMEOPTION_NO_WORLD_SPELLS) || city.settlement || city.outpost || info.prereq_civilization.is_some_and(|civ| civ != g.city_civilization_type(c)) {
        return None;
    }
    info.tech_prereq.filter(|&t| !g.team_is_has_tech(g.players[city.owner].team, Some(t)))
}

/// Why city `c` cannot start world ritual `project` now (the production list shows it disabled
/// with this): `Requires <tech>.`, `Cast pending: cast <spell> first.`, `In progress in <city>.`, `Sanctuary holds: <N>
/// turns.`, `No rival named: use Name the Tithe.` (Ritual: The Tithe), else `Not available now.`
/// (a reason of the rules this list does not know).
pub fn cannot_reason(g: &Game, c: CityId, project: ProjectType) -> String {
    let d = g.d;
    let p = g.cities[c].owner;
    if let Some(t) = ritual_needs_tech(g, c, project) {
        return format!("Requires {}.", crate::names::display(d, t));
    }
    if g.player_world_ritual_pending(p, project) {
        if let Some(s) = g.world_ritual_spell(project) {
            return format!("Cast pending: cast {} first.", crate::names::display(d, s));
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

/// A ritual's entry of the production list (`buildable`; every project is a ritual, ffh-core's
/// `ffh::rituals`): `ritual: {rules, why, note, reminders}`. A world ritual's `rules` is its
/// [`rules_line`] (markup), `why` why it cannot be chosen (null while it can), `note` its own
/// state (Ritual: The Tithe's `Named rival: <Civ>.`); another ritual has them null. `reminders`:
/// its keywords' reminder texts ([`crate::view::keywords`]), shown as chips.
pub fn buildable_fields(g: &Game, c: CityId, project: ProjectType, enabled: bool, j: &mut Json) {
    let d = g.d;
    if !g.project_is_ritual(project) {
        return;
    }
    let reminders = crate::view::keywords::reminders(d, crate::view::keywords::of_project(d, project));
    if !g.project_is_world_ritual(project) {
        j.set("ritual", crate::jobj! { "rules" => Json::Null, "why" => Json::Null, "note" => Json::Null, "reminders" => reminders });
        return;
    }
    let note = tithe_note(g, g.cities[c].owner, project);
    j.set(
        "ritual",
        crate::jobj! {
            "rules" => rules_line(g, c, project),
            "why" => if enabled { Json::Null } else { Json::from(cannot_reason(g, c, project)) },
            "note" => note,
            "reminders" => reminders,
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

/// The city screen's ritual field `ritual_head`: the ritual (any project, ffh-core's
/// `ffh::rituals`) in progress at the head of the queue, with the question asked before an action
/// takes it off: `Abandon <ritual>? Its <N> production is lost.`
pub fn city_fields(g: &Game, c: CityId, j: &mut Json) {
    let d = g.d;
    let head = g.city_ritual_in_progress(c).map(|project| {
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
}

/// Why city `c` cannot use hurry `hurry` although it is shown, when the reason is a ritual's:
/// `Cannot be hurried with gold (Ritual).` (a gold hurry while the city's first build is a
/// ritual, ffh-core's `ffh::rituals`).
pub fn hurry_reason(g: &Game, c: CityId, hurry: HurryType) -> Option<&'static str> {
    (g.city_head_ritual(c).is_some() && g.hurry_is_gold(hurry)).then_some("Cannot be hurried with gold (Ritual).")
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
