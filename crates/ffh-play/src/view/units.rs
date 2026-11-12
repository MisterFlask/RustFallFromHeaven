//! Unit queries (design §4.6): `group` (unit details and the actions the selection can take,
//! enumerated as the exe's `canHandleAction` does), `path` (a preview through the interface
//! finder) and `combat` (attack odds and the strength breakdown, C++ `setCombatPlotHelp`).
//!
//! A selection is a list of the human's units on one plot (as for commands). The exe checks an
//! action against the selection (a `CvSelectionGroup` of the selected units), so do these, with
//! the ports in [`crate::selection`]. Each action carries the command that performs it (`cmd`,
//! the canonical request form), so a client (or the fuzz test) can send it as is.

use ffh_core::ids::*;
use ffh_core::path::PathFinder;
use ffh_core::unit::CombatDetails;
use ffh_data::*;

use crate::command::Command;
use crate::json::Json;
use crate::query::{invalid, QueryErr};
use crate::selection::{self, InterfaceMode};
use crate::view::map::plot_at;
use crate::view::{unit_json, Viewer};

/// A selection of the human's units: non-empty, distinct, its own, on one plot.
pub fn check_selection(v: &Viewer, units: &[UnitId]) -> Result<(), QueryErr> {
    let g = v.g;
    if units.is_empty() {
        return Err(invalid("field `units`: empty selection"));
    }
    for (i, &u) in units.iter().enumerate() {
        match g.units.get(u) {
            None => return Err(QueryErr::Gone),
            Some(unit) if unit.owner != v.h => return Err(QueryErr::NotOwner),
            Some(_) => {}
        }
        if units[..i].contains(&u) {
            return Err(invalid(format!("field `units`: unit {} twice", u.0)));
        }
    }
    let plot = g.unit_plot(units[0]);
    if units.iter().any(|&u| g.unit_plot(u) != plot) {
        return Err(invalid("field `units`: the selected units are on different plots"));
    }
    Ok(())
}

/// The missions whose data is a plot the client picks: the exe sends them from an interface mode
/// ([`selection::InterfaceMode`]), never from a plain mission button.
pub const TARGET_MISSIONS: [MissionType; 8] = [
    MissionType::MISSION_MOVE_TO,
    MissionType::MISSION_ROUTE_TO,
    MissionType::MISSION_AIRLIFT,
    MissionType::MISSION_NUKE,
    MissionType::MISSION_RECON,
    MissionType::MISSION_PARADROP,
    MissionType::MISSION_AIRBOMB,
    MissionType::MISSION_RANGE_ATTACK,
];

fn mission_cmd(units: &[UnitId], m: MissionType, data1: i32, data2: i32) -> Json {
    Command::Mission { units: units.to_vec(), mission: m, data1, data2, flags: 0, append: false }.to_json()
}

fn command_cmd(units: &[UnitId], c: CommandType, data1: i32, data2: i32) -> Json {
    Command::DoCommand { units: units.to_vec(), command: c, data1, data2, alt: false, confirmed: false }.to_json()
}

/// The actions of a selection: every action the exe would show (`canHandleAction` with
/// `bTestVisible`), `enabled` when it can be done now. Kinds: `mission` (no data or a typed
/// one: build, spread, join, construct, lead), `target` (an interface mode, `mode`, whose
/// mission needs a plot: `targets` lists the valid plots in range, or `any` for go-to and
/// route-to), `command`, `spell`, `promotion`, `upgrade` (with `price`), `load` (with the
/// `transports`), `army` (this game's armies).
///
/// A `target` action's `cmd` carries its first listed target, or the selection's own plot as a
/// placeholder (with `any`, or when no target is in range): the client puts the picked plot in
/// (`x`, `y` of a `move`, `data1`, `data2` of a `mission`). GO_TO's command is `move`, as the exe
/// moves through `selectionListMove` (the declare-war question).
pub fn actions(v: &Viewer, units: &[UnitId]) -> Vec<Json> {
    let g = v.g;
    let d = g.d;
    let plot = selection::plot(g, units);
    let (px, py) = g.map.xy(plot);
    let mut out = Vec::new();
    let mut push = |kind: &str, enabled: bool, cmd: Json, extra: Json| {
        let mut j = crate::jobj! { "kind" => kind, "enabled" => enabled };
        if let Json::Obj(m) = extra {
            for (k, x) in m {
                j.set(&k, x);
            }
        }
        j.set("cmd", cmd);
        out.push(j);
    };
    let can = |m: MissionType, d1: i32, d2: i32, visible: bool| selection::can_start_mission(g, units, m, d1, d2, plot, visible);

    // Missions.
    for i in 0..MissionType::NUM {
        let m = MissionType::from_usize(i);
        if !selection::is_player_mission(m) || m == MissionType::MISSION_ESPIONAGE {
            continue;
        }
        // Civ4's Sea Patrol (a sentry for ships) does nothing for who holds a sea: the human is
        // offered Patrol (`MISSION_PLUNDER`) instead (docs/NAVAL.md, 4.8; the rules keep it).
        if m == MissionType::MISSION_SEAPATROL {
            continue;
        }
        let typed: Option<usize> = match m {
            MissionType::MISSION_BUILD => Some(d.builds.len()),
            MissionType::MISSION_SPREAD => Some(d.religions.len()),
            MissionType::MISSION_SPREAD_CORPORATION => Some(d.corporations.len()),
            MissionType::MISSION_JOIN => Some(d.specialists.len()),
            MissionType::MISSION_CONSTRUCT => Some(d.buildings.len()),
            _ => None,
        };
        if TARGET_MISSIONS.contains(&m) {
            continue; // the interface modes, below
        }
        match (m, typed) {
            (MissionType::MISSION_LEAD, _) => {
                // The LEADUNIT popup: one button per unit the great commander can lead.
                for &t in &g.map.plots[plot].units {
                    if g.units[t].owner == v.h && !units.contains(&t) && can(m, t.0 as i32, -1, false) {
                        push("mission", true, mission_cmd(units, m, t.0 as i32, -1), crate::jobj! { "mission" => Json::idx(m), "data1" => t });
                    }
                }
            }
            (_, Some(n)) => {
                for k in 0..n {
                    let k = k as i32;
                    if !can(m, k, -1, true) {
                        continue;
                    }
                    let enabled = can(m, k, -1, false);
                    let mut extra = crate::jobj! { "mission" => Json::idx(m), "data1" => k };
                    if m == MissionType::MISSION_BUILD {
                        let b = BuildType::from_usize(k as usize);
                        let (mut now, mut then) = (0, 0);
                        for &u in units {
                            if g.unit_get_build_type(u) != Some(b) {
                                now += g.unit_work_rate(u, false);
                                then += g.unit_work_rate(u, true);
                            }
                        }
                        extra.set("turns", g.plot_get_build_turns_left(plot, b, now, then));
                        extra.set("cost", g.player_get_build_cost(v.h, plot, b));
                    }
                    push("mission", enabled, mission_cmd(units, m, k, -1), extra);
                }
            }
            (MissionType::MISSION_PLUNDER, _) => {
                // Patrol (docs/NAVAL.md, 4.2 and 10): its rule as the tooltip, and the sea's state
                // after it.
                if can(m, -1, -1, true) {
                    let mut help = crate::view::seas::patrol_rule(g);
                    for line in crate::view::seas::patrol_preview(v, units, plot) {
                        help.push_str("[NEWLINE]");
                        help.push_str(&line);
                    }
                    push("mission", can(m, -1, -1, false), mission_cmd(units, m, -1, -1), crate::jobj! { "mission" => Json::idx(m), "help" => help });
                }
            }
            _ => {
                if can(m, -1, -1, true) {
                    push("mission", can(m, -1, -1, false), mission_cmd(units, m, -1, -1), crate::jobj! { "mission" => Json::idx(m) });
                }
            }
        }
        // This game's outposts (docs/NAVAL.md, 9.2): the founder's second action (the Colonist's
        // only one), with the site rule as its reason when it cannot.
        if m == MissionType::MISSION_FOUND {
            if let Some(&f) = units.iter().find(|&&u| g.unit_founds_outposts(u)) {
                let outpost = ffh_core::outposts::FOUND_OUTPOST;
                let enabled = can(m, outpost, -1, false);
                let rule = format!("Founds an outpost: on another landmass than your capital's, next to the ocean. Population {} at most, works the 8 plots around it, builds nothing, pays no maintenance; its resources reach you while its lane to a home port is open.", g.d.defines.outpost_max_pop);
                let help = match (enabled, g.unit_found_outpost_refusal(f, plot, false)) {
                    (false, Some(why)) => format!("{rule}[NEWLINE]{why}"),
                    _ => rule,
                };
                push("mission", enabled, mission_cmd(units, m, outpost, -1), crate::jobj! { "mission" => Json::idx(m), "data1" => outpost, "outpost" => true, "help" => help });
            }
        }
    }

    // Interface modes (the exe's go-to, route-to, range attack, air ... buttons): shown, and so
    // enabled, when `canDoInterfaceMode`; the plot is the client's pick, among those that pass
    // `canDoInterfaceModeAt` and the mission check (any plot for go-to and route-to).
    for mode in InterfaceMode::ALL {
        if !selection::can_do_interface_mode(g, units, mode) {
            continue;
        }
        let m = mode.mission();
        let ok = |t: PlotIdx| {
            let (tx, ty) = g.map.xy(t);
            selection::can_do_interface_mode_at(g, units, mode, t) && can(m, tx, ty, false)
        };
        let targets: Option<Vec<(i32, i32)>> = match mode {
            InterfaceMode::GoTo | InterfaceMode::RouteTo => None,
            InterfaceMode::Airlift => Some(g.players[v.h].cities.iter().map(|&c| g.city_plot(c)).filter(|&t| ok(t)).map(|t| g.map.xy(t)).collect()),
            _ => {
                let range = units.iter().map(|&u| g.unit_air_range(u).max(g.unit_nuke_range(u)).max(g.unit_get_drop_range(u))).max().unwrap_or(0).max(0);
                let mut out = Vec::new();
                for dy in -range..=range {
                    for dx in -range..=range {
                        if let Some(t) = g.plot_xy(px, py, dx, dy) {
                            if v.plot_revealed(t) && ok(t) && !out.contains(&g.map.xy(t)) {
                                out.push(g.map.xy(t));
                            }
                        }
                    }
                }
                Some(out)
            }
        };
        let (tx, ty) = targets.as_ref().and_then(|t| t.first().copied()).unwrap_or((px, py));
        let cmd = if mode == InterfaceMode::GoTo {
            Command::Move { units: units.to_vec(), x: tx, y: ty, append: false, declare_war: None }.to_json()
        } else {
            mission_cmd(units, m, tx, ty)
        };
        let mut extra = crate::jobj! { "mode" => mode.name(), "mission" => Json::idx(m) };
        match targets {
            None => {
                extra.set("any", true);
            }
            Some(t) => {
                extra.set("targets", Json::Arr(t.into_iter().map(|(x, y)| crate::jarr![x, y]).collect()));
            }
        }
        push("target", true, cmd, extra);
    }

    // Commands without data.
    for i in 0..CommandType::NUM {
        let c = CommandType::from_usize(i);
        if matches!(
            c,
            CommandType::COMMAND_PROMOTION
                | CommandType::COMMAND_UPGRADE
                | CommandType::COMMAND_AUTOMATE
                | CommandType::COMMAND_AUTOMATE_SPELL
                | CommandType::COMMAND_HOTKEY
                | CommandType::COMMAND_CAST
                | CommandType::COMMAND_LOAD_UNIT
        ) {
            continue;
        }
        if !selection::can_do_command(g, units, c, -1, -1, true) {
            continue;
        }
        let enabled = selection::can_do_command(g, units, c, -1, -1, false);
        let mut extra = crate::jobj! { "command" => Json::idx(c), "confirm" => d.commands[c].confirm_command, "all" => d.commands[c].all };
        if c == CommandType::COMMAND_LOAD {
            let transports = if enabled { selection::load_targets(g, units) } else { Vec::new() };
            extra.set("transports", transports);
            push("load", enabled, command_cmd(units, c, -1, -1), extra);
        } else {
            push("command", enabled, command_cmd(units, c, -1, -1), extra);
        }
    }

    // Automation (the exe's automate buttons, one per CvAutomateInfo with `bVisible`): shown when
    // `canDoCommand(COMMAND_AUTOMATE, eAutomate, bTestVisible)`. FfH's automatic terraforming
    // (Python `AI_MageTurn`) has no AI here, so it is not offered.
    for a in d.automates.keys() {
        if a == AutomateType::AUTOMATE_TERRAFORMING {
            continue;
        }
        let k = a.idx() as i32;
        if !selection::can_do_command(g, units, CommandType::COMMAND_AUTOMATE, k, -1, true) {
            continue;
        }
        let enabled = selection::can_do_command(g, units, CommandType::COMMAND_AUTOMATE, k, -1, false);
        push("automate", enabled, command_cmd(units, CommandType::COMMAND_AUTOMATE, k, -1), crate::jobj! { "command" => Json::idx(CommandType::COMMAND_AUTOMATE), "automate" => Json::idx(a) });
    }

    // Spells. World spells are the empire's, not the selection's: they are cast from the top
    // bar's world actions ([`world_actions`]).
    let discover = d.ty::<SpellType>("SPELL_DISCOVER_CARD");
    for s in d.spells.keys() {
        let k = s.idx() as i32;
        // A card's world spell (`once`) is the empire's too (docs/CARDS.md), unless the unit
        // casting it matters (`cast_from_selection`: Astral Leap; the Khazad's The Delve, a world
        // spell, also stays among the world actions).
        if (d.spells[s].global && !d.spells[s].cast_from_selection) || d.spells[s].world_action || (d.spells[s].once && !d.spells[s].cast_from_selection) {
            continue;
        }
        let info = &d.spells[s];
        let help = crate::names::text(d, &info.base.help);
        let mut extra = crate::jobj! { "spell" => Json::idx(s), "global" => info.global, "once" => info.once, "causes_war" => info.causes_war, "help" => if help == info.base.help { "" } else { help } };
        // A spell cast at a distance (docs/CARDS.md): its targets, for the first selected unit
        // that can cast it; the command is `cast_at` (at its first target, for the client to
        // replace).
        if info.target.is_some() {
            if selection::is_busy(g, units) || !units.iter().any(|&u| g.unit_can_cast_untargeted(u, s, true)) {
                continue;
            }
            let caster = units.iter().copied().find(|&u| g.unit_can_cast_untargeted(u, s, false));
            let targets = caster.map(|u| g.unit_spell_targets(u, s)).unwrap_or_default();
            let (enabled, cmd) = targeted_action(v, caster.unwrap_or(units[0]), s, caster.is_some(), &targets, &mut extra);
            push("spell", enabled, cmd, extra);
            continue;
        }
        // Discovering a great person's card (docs/CARDS.md): shown for a selection holding a card
        // even while its needs are missing, named for its card, with what it lacks.
        let holder = (Some(s) == discover).then(|| units.iter().copied().find(|&u| g.units[u].card.is_some())).flatten();
        // Wheeled City's Pack Up in an outpost of ours that holds its building (docs/NAVAL.md, 9.2):
        // shown disabled, with the reason.
        let pack_why = (info.py_requirement.starts_with("reqPackUp"))
            .then(|| g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c) && g.cities[c].owner == v.h && g.cities[c].outpost))
            .flatten()
            .filter(|&c| info.building_prereq.is_none_or(|b| g.city_num_building(c, b) > 0))
            .and_then(|c| g.city_pack_up_refusal(c));
        if let Some(why) = pack_why {
            extra.set("why", why);
            push("spell", false, command_cmd(units, CommandType::COMMAND_CAST, k, -1), extra);
            continue;
        }
        if !selection::can_do_command(g, units, CommandType::COMMAND_CAST, k, -1, true) && holder.is_none() {
            continue;
        }
        let enabled = selection::can_do_command(g, units, CommandType::COMMAND_CAST, k, -1, false);
        if let Some(card) = holder.and_then(|u| g.units[u].card) {
            extra.set("label", format!("Discover: {}", crate::names::display(d, card)));
            extra.set("card", Json::idx(card));
            if !enabled {
                extra.set("why", crate::view::cards::card_why_not(v, card).unwrap_or_else(|| "Not possible now.".into()));
            }
        }
        // Explore Lair: the odds for the selected units that can cast it (each explores in turn).
        if info.py_result.starts_with("spellExploreLair") {
            let casters: Vec<UnitId> = units.iter().copied().filter(|&u| g.unit_can_cast(u, s, true)).collect();
            if let Some(explore) = crate::view::explore::explore_json(v, plot, &casters) {
                extra.set("explore", explore);
            }
        }
        push("spell", enabled, command_cmd(units, CommandType::COMMAND_CAST, k, -1), extra);
    }

    // Promotions.
    for p in d.promotions.keys() {
        let k = p.idx() as i32;
        if selection::can_do_command(g, units, CommandType::COMMAND_PROMOTION, k, -1, true) {
            let enabled = selection::can_do_command(g, units, CommandType::COMMAND_PROMOTION, k, -1, false);
            push("promotion", enabled, command_cmd(units, CommandType::COMMAND_PROMOTION, k, -1), crate::jobj! { "promotion" => Json::idx(p) });
        }
    }

    // Upgrades (with the head's price).
    for t in d.units.keys() {
        let k = t.idx() as i32;
        if selection::can_do_command(g, units, CommandType::COMMAND_UPGRADE, k, -1, true) {
            let enabled = selection::can_do_command(g, units, CommandType::COMMAND_UPGRADE, k, -1, false);
            let price: i32 = units.iter().filter(|&&u| g.unit_can_upgrade(u, Some(t), true)).map(|&u| g.unit_upgrade_price(u, t)).sum();
            push("upgrade", enabled, command_cmd(units, CommandType::COMMAND_UPGRADE, k, -1), crate::jobj! { "unit" => Json::idx(t), "price" => price });
        }
    }

    // This game's armies (docs/ARMIES.md): form, split, join or merge, garrison, disband,
    // upgrade all. Only for a selection of army units (heroes, mages, recon units and summons
    // move on their own), or of ships that serve in fleets (docs/NAVAL.md, section 7).
    if g.army_units_plot(v.h, units).is_ok() {
        for (enabled, cmd, extra) in army_actions(v, units, plot) {
            push("army", enabled, cmd, extra);
        }
    }

    out
}

/// The command and the fields of a spell cast at a distance (docs/CARDS.md): `target_kind`, the
/// `targets` of `caster` ([`crate::view::cards::targets_json`]), `why` when it cannot be cast
/// (`can`: the caster can cast it now, whatever its target), and a `cast_at` command at its first
/// target (the client puts the picked one in). Returns whether it is enabled, and the command.
fn targeted_action(v: &Viewer, caster: UnitId, s: SpellType, can: bool, targets: &[ffh_core::structs::SpellTarget], extra: &mut Json) -> (bool, Json) {
    let g = v.g;
    let kind = g.d.spells[s].target.expect("a targeted spell");
    extra.set("target_kind", crate::view::cards::target_kind_name(kind));
    extra.set("targets", crate::view::cards::targets_json(v, caster, s, targets));
    let enabled = can && !targets.is_empty();
    if !can {
        extra.set("why", "No unit of yours can cast it now.");
    } else if targets.is_empty() {
        extra.set("why", "No valid target now.");
    }
    // A placeholder target of the kind (the client replaces it with the pick).
    let target = targets.first().map(|&t| crate::command::TargetRef::of(g, t)).unwrap_or(match kind {
        SpellTargetKind::Plot => {
            let (x, y) = (g.units[caster].x, g.units[caster].y);
            crate::command::TargetRef::Plot(x, y)
        }
        SpellTargetKind::City => crate::command::TargetRef::City(CityId(0)),
        SpellTargetKind::Unit => crate::command::TargetRef::Unit(caster),
        SpellTargetKind::Player => crate::command::TargetRef::Player(v.h),
        SpellTargetKind::Choice => crate::command::TargetRef::Choice(0),
    });
    (enabled, Command::CastAt { unit: caster, spell: s, target, confirmed: false }.to_json())
}

/// The actions of the whole empire, offered in the top bar rather than on a selection: its world
/// spell (once per charge of its world ritual: listed while the civilization has one, disabled
/// with no charge) and its cards' world spells (`once`, docs/CARDS.md). The rules cast a
/// spell through a unit, so each action names the unit it is cast through (`caster`): one that can
/// cast it now, preferring one that loses no promotion by casting (Hidden, Invisible), then the
/// oldest. The world spells' effects reach the whole empire, whoever casts them (the Basium's and
/// Hyborem's are theirs alone). An action is listed while some unit could cast it (`canCast` with
/// `bTestVisible`), `enabled` when one can now (`why`: why not); a card's world spell stays listed
/// once cast (`cast`, never enabled again). A spell cast at a distance carries its targets
/// (`target_kind`, `targets`) and a `cast_at` command.
pub fn world_actions(v: &Viewer) -> Json {
    let g = v.g;
    let d = g.d;
    let cast = CommandType::COMMAND_CAST;
    let mut out = Vec::new();
    for s in d.spells.keys() {
        let info = &d.spells[s];
        // A card's world spell whose caster matters is cast from a selection instead.
        if !info.global && !info.world_action && (!info.once || info.cast_from_selection) {
            continue;
        }
        let k = s.idx() as i32;
        let help = crate::names::text(d, &info.base.help);
        let mut j = crate::jobj! {
            "kind" => "spell",
            "spell" => Json::idx(s),
            "global" => info.global,
            "once" => info.once,
            "causes_war" => info.causes_war,
            "help" => if help == info.base.help { "" } else { help },
            // Placed by the player, no unit casting it (`place`: while its charge is held).
            "place_on_charge" => info.place_on_charge,
        };
        // A card's world spell cast already: still the empire's, spent.
        if info.once && g.player_cast_once(v.h, s) {
            if !g.team_is_has_tech(v.team, info.tech_prereq) {
                continue;
            }
            j.set("enabled", false);
            j.set("cast", true);
            j.set("why", "Cast already: once a game.");
            j.set("caster", Json::Null);
            j.set("cmd", Json::Null);
            out.push(j);
            continue;
        }
        let busy = |u: UnitId| selection::is_busy(g, &[u]);
        // The targets are listed once, for the caster: not for every unit tried.
        let can = |u: UnitId, visible: bool| if info.target.is_some() { !busy(u) && g.unit_can_cast_untargeted(u, s, visible) } else { selection::can_do_command(g, &[u], cast, k, -1, visible) };
        let visible: Vec<UnitId> = g.players[v.h].units.iter().copied().filter(|&u| can(u, true)).collect();
        // The civilization's world spell (docs/CARDS.md, "World rituals"): listed while it has
        // one, disabled while no city holds a charge of its ritual.
        let ritual = info.ritual_charge.filter(|&r| info.global && g.player_world_ritual(v.h) == Some(r) && !g.is_option(GameOptionType::GAMEOPTION_NO_WORLD_SPELLS));
        // Its ritual (the page writes no "once a game" line for it).
        j.set("ritual", Json::opt_idx(ritual));
        if let Some(r) = ritual.filter(|&r| g.player_world_ritual_charge_city(v.h, r).is_none()) {
            j.set("enabled", false);
            j.set("cast", false);
            j.set("why", crate::view::rituals::no_charge_why(v, r));
            j.set("caster", Json::Null);
            j.set("cmd", Json::Null);
            out.push(j);
            continue;
        }
        // A world spell the player places (`place_on_charge`: Wellspring), no unit casting it.
        if let Some(r) = ritual.filter(|_| info.place_on_charge) {
            crate::view::place::world_action(v, s, r, &mut j);
            out.push(j);
            continue;
        }
        if visible.is_empty() {
            // A charge held that no unit of the human can spend: the button stays, disabled.
            if ritual.is_some() {
                j.set("enabled", false);
                j.set("cast", false);
                j.set("why", "No unit of yours can cast it now.");
                j.set("caster", Json::Null);
                j.set("cmd", Json::Null);
                out.push(j);
            }
            continue;
        }
        let loses = |u: UnitId| d.promotions.keys().filter(|&p| g.units[u].has_promotion[p] && d.promotions[p].removed_by_casting).count();
        // Not a marching recruit (this game's armies): it takes no orders until it arrives.
        let caster = visible.iter().copied().filter(|&u| g.unit_march_refusal(u).is_none() && can(u, false)).min_by_key(|&u| (loses(u), u.0));
        j.set("cast", false);
        j.set("caster", caster);
        // Where the caster stands, by the human's nearest city (`in Prespur`, `near Prespur`;
        // null without a city): the tooltip's "Cast by your Adept near Prespur".
        j.set("caster_where", caster.and_then(|u| crate::view::near_own_city(v, g.units[u].x, g.units[u].y)));
        if info.target.is_some() {
            let targets = caster.map(|u| g.unit_spell_targets(u, s)).unwrap_or_default();
            let (enabled, cmd) = targeted_action(v, caster.unwrap_or(visible[0]), s, caster.is_some(), &targets, &mut j);
            j.set("enabled", enabled);
            j.set("cmd", cmd);
        } else {
            j.set("enabled", caster.is_some());
            if caster.is_none() {
                j.set("why", "No unit of yours can cast it now.");
            }
            j.set("cmd", command_cmd(&[caster.unwrap_or(visible[0])], cast, k, -1));
        }
        // A charge held and castable: where it is, and the ritual's cost after it is spent.
        if let Some(line) = ritual.filter(|_| j.get("enabled").and_then(Json::as_bool) == Some(true)).and_then(|r| crate::view::rituals::charge_line(v, r)) {
            j.set("charge", line);
        }
        out.push(j);
    }
    Json::Arr(out)
}

/// The army buttons of a selection of army units on `plot` (`army`: the action, `label`, `help`);
/// for ships, the fleet's (docs/NAVAL.md, section 7: form, split, join or merge, disband, upgrade
/// all; no garrison).
fn army_actions(v: &Viewer, units: &[UnitId], plot: PlotIdx) -> Vec<(bool, Json, Json)> {
    let g = v.g;
    let h = v.h;
    let mut out = Vec::new();
    let mut add = |enabled: bool, cmd: Command, action: &str, label: String, help: &str| {
        out.push((enabled, cmd.to_json(), crate::jobj! { "army" => action, "label" => label, "help" => help }));
    };
    let fleet = g.unit_army_domain(units[0]) == Some(ffh_core::army::ArmyDomain::Sea);
    let noun = if fleet { "fleet" } else { "army" };
    let first = g.units[units[0]].army;
    // The army all the selected units are in, if one.
    let one = first.filter(|_| units.iter().all(|&u| g.units[u].army == first));
    let whole = one.is_some_and(|a| g.army_units(a).len() == units.len());
    let city = g.map.plots[plot].plot_city.filter(|&c| g.cities[c].owner == h);
    match one {
        Some(_) if !whole => add(
            true,
            Command::ArmySplit { units: units.to_vec() },
            "split",
            format!("Split into a new {noun}"),
            if fleet { "The selected ships leave their fleet and form a new one here." } else { "The selected units leave their army and form a new one here." },
        ),
        Some(_) => {}
        None => add(
            true,
            Command::ArmyForm { units: units.to_vec() },
            "form",
            format!("Form a new {noun}"),
            if fleet { "The selected ships leave their fleets and form a new fleet, which moves as one." } else { "The selected units leave their armies or garrison and form a new army, which moves as one." },
        ),
    }
    for &b in &g.players[h].armies {
        if !g.armies.contains(b) || Some(b) == one || g.army_plot(b) != Some(plot) || g.army_is_fleet(b) != fleet {
            continue;
        }
        let name = &g.armies[b].name;
        let (label, help) = if whole {
            (format!("Merge into the {name}"), if fleet { "The whole fleet joins the other; the ships on their way to it follow." } else { "The whole army joins the other; the units on their way to it follow." })
        } else {
            (format!("Join the {name}"), if fleet { "The selected ships join that fleet." } else { "The selected units join that army." })
        };
        add(true, Command::ArmyJoin { units: units.to_vec(), army: b }, "join", label, help);
    }
    if let (Some(c), false) = (city, fleet) {
        if units.iter().any(|&u| g.units[u].army.is_some()) {
            add(
                true,
                Command::ArmyGarrison { units: units.to_vec() },
                "garrison",
                format!("Join the garrison of {}", crate::view::city_name(g, c)),
                "The selected units stay in the city as its garrison; a garrison unit that leaves the city forms an army again.",
            );
        }
    }
    if let (Some(a), true) = (one, whole) {
        if fleet {
            // A fleet in port is disbanded there; at sea it sails to the nearest port first.
            let to = if city.is_some() { city } else { g.fleet_nearest_port(h, plot) };
            add(
                to.is_some(),
                Command::ArmyDisband { army: a },
                "disband",
                match (to, city) {
                    (Some(_), Some(_)) => "Disband the fleet".into(),
                    (Some(c), None) => format!("Disband: sail to {}", crate::view::city_name(g, c)),
                    _ => "Disband (no port to sail to)".into(),
                },
                "The fleet's ships sail to the nearest of your ports; there they are ships in no fleet.",
            );
        } else if city.is_none() {
            let to = g.army_nearest_city(h, plot);
            add(
                to.is_some(),
                Command::ArmyDisband { army: a },
                "disband",
                match to {
                    Some(c) => format!("Disband: march to {}", crate::view::city_name(g, c)),
                    None => "Disband (no city to go to)".into(),
                },
                "The army's units march to the nearest of your cities and join its garrison there; its reinforcements go to that garrison.",
            );
        }
        // Upgrade all: the units with an upgrade (with the gold), the most experienced first.
        let plan: Vec<(UnitId, UnitType)> = g.army_upgrade_order(a).into_iter().filter_map(|u| g.unit_best_upgrade(u, true).map(|t| (u, t))).collect();
        if !plan.is_empty() {
            let price: i32 = plan.iter().map(|&(u, t)| g.unit_upgrade_price(u, t)).sum();
            let now = plan.iter().any(|&(u, _)| g.unit_best_upgrade(u, false).is_some());
            add(
                now,
                Command::ArmyUpgrade { army: a },
                "upgrade",
                format!("Upgrade all: {} unit{} ({price} gold)", plan.len(), if plan.len() == 1 { "" } else { "s" }),
                "Each unit takes its best upgrade, the most experienced first, while the gold lasts.",
            );
        }
    }
    out
}

/// One of the human's units in detail (the unit panel).
pub fn unit_details(v: &Viewer, u: UnitId) -> Json {
    let g = v.g;
    let d = g.d;
    let unit = &g.units[u];
    let info = &d.units[unit.unit_type];
    let mut j = unit_json(v, u);
    j.set("name", if unit.name.is_empty() { Json::Null } else { Json::from(crate::names::text(d, &unit.name)) });
    j.set("strength", g.unit_base_combat_str(u));
    j.set("defense_strength", g.unit_base_combat_str_defense(u));
    j.set("domain", Json::idx(g.unit_get_domain_type(u)));
    j.set("unit_combat", Json::opt_idx(info.unit_combat_type));
    j.set("race", Json::opt_idx(unit.race));
    j.set("religion", Json::opt_idx(unit.religion));
    j.set("promotions", Json::Arr(d.promotions.keys().filter(|&p| unit.has_promotion[p]).map(Json::idx).collect()));
    j.set("first_strikes", crate::jarr![g.unit_first_strikes(u), g.unit_chance_first_strikes(u)]);
    j.set("withdrawal", g.unit_withdrawal_probability(u));
    j.set("collateral", g.unit_collateral_damage(u));
    j.set("cargo", Json::Arr(g.map.plots[g.unit_plot(u)].units.iter().filter(|&&c| g.units[c].transport_unit == Some(u)).map(|&c| Json::from(c)).collect()));
    j.set("cargo_space", unit.cargo_capacity);
    j.set("has_casted", unit.has_casted);
    // A great person's card (docs/CARDS.md): what it is, its needs, whether it can be discovered.
    j.set("card_detail", unit.card.map(|c| crate::view::cards::card_json(v, c)));
    // The sea (docs/NAVAL.md, section 10): range, wear, in boats (null for a land unit on land).
    j.set("sea", crate::view::sea::unit_panel(v, u));
    j
}

/// `group {units}`: the selected units in detail and the selection's actions. A selection with a
/// marching recruit (this game's armies) has none: the recruit takes no orders until it arrives
/// (`marching`: why, naming where it goes; each unit's `march` has the rest).
pub fn group(v: &Viewer, units: &[UnitId]) -> Result<Json, QueryErr> {
    check_selection(v, units)?;
    let g = v.g;
    let plot = selection::plot(g, units);
    let (x, y) = g.map.xy(plot);
    let mirrored = selection::mirrors_group(g, units);
    let marching = units.iter().find_map(|&u| g.unit_march_refusal(u));
    Ok(crate::jobj! {
        "units" => Json::Arr(units.iter().map(|&u| unit_details(v, u)).collect()),
        "plot" => crate::jarr![x, y],
        "group" => mirrored,
        // The group waits for orders (`pending.ready_groups`).
        "ready" => mirrored.is_some_and(|grp| crate::decision::is_ready_group(g, grp)),
        "busy" => selection::is_busy(g, units),
        "actions" => Json::Arr(if marching.is_some() { Vec::new() } else { actions(v, units) }),
        "marching" => marching,
    })
}

/// `path {units,x,y,flags}`: the interface finder's path for the selection's group (the head's
/// group when the selection is not a whole group: `approximate`), with each node's turn of
/// arrival (1 = this turn) and moves left, the war it would declare and whether the last step
/// attacks.
pub fn path(v: &Viewer, finder: &mut PathFinder, units: &[UnitId], x: i32, y: i32, flags: i32) -> Result<Json, QueryErr> {
    check_selection(v, units)?;
    let to = plot_at(v, x, y)?;
    let g = v.g;
    let mirrored = selection::mirrors_group(g, units);
    let Some(grp) = mirrored.or(g.units[units[0]].group) else { return Err(invalid("the unit has no group")) };
    let from = selection::plot(g, units);
    let (fx, fy) = g.map.xy(from);
    finder.set_group(Some(grp));
    let ok = finder.generate_path(g, fx, fy, x, y, false, flags);
    let mut nodes = Vec::new();
    let mut steps: Vec<(i32, i32, i32)> = Vec::new();
    if ok {
        let mut node = finder.last_node().cloned();
        while let Some(n) = node {
            nodes.push(crate::jobj! { "x" => n.x, "y" => n.y, "turn" => n.data2, "moves" => n.data1 });
            steps.push((n.x, n.y, n.data2));
            node = finder.parent_of(&n).cloned();
        }
        nodes.reverse();
        steps.reverse();
    }
    // The sea (docs/NAVAL.md, section 10): where the path goes in boats or out of range.
    let mut sea = crate::view::sea::path_json(v, units, &steps);
    let war = units.iter().find_map(|&u| g.unit_get_declare_war_move(u, to));
    // Card: False Colours (docs/CARDS.md): a move onto units the group's false colours let it
    // attack is an attack (its war is declared at the battle, without a question).
    let attack = v.plot_visible(to) && (units.iter().any(|&u| g.plot_is_visible_enemy_defender(to, u)) || g.group_false_colours_team(grp, to).is_some());
    // Named seas (docs/NAVAL.md, 10): the seas it enters where an enemy patrols, and an attack on
    // a coastal city's harbour.
    let mut more = crate::view::seas::interception_lines(v, g.group_get_head_unit(grp).unwrap_or(units[0]), &steps);
    if attack {
        if let Some(c) = g.map.plots[to].plot_city.filter(|&c| v.city_revealed(c)) {
            more.extend(crate::view::seas::attack_harbour_lines(v, c));
        }
    }
    if !more.is_empty() {
        if sea.is_null() {
            sea = crate::jobj! { "lines" => Json::Arr(Vec::new()) };
        }
        let mut lines: Vec<Json> = sea.get("lines").and_then(Json::as_arr).map(|a| a.to_vec()).unwrap_or_default();
        lines.extend(more.into_iter().map(Json::from));
        sea.set("lines", Json::Arr(lines));
    }
    // Supply lines (docs/SUPPLY.md, section 5): with the units of one army of the human's
    // selected, whether the army would be supplied on the plot (a revealed one): `ok`, the line's
    // `cost` and the `budget`; null otherwise. It answers from what the human knows (the known
    // search, the human's team as viewer; "Second pass", A): an enemy unit the human does not see
    // does not cut it, so it can be wrong.
    let army = g.units[units[0]].army.filter(|&a| g.armies.contains(a) && g.armies[a].owner == v.h && units.iter().all(|&u| g.units[u].army == Some(a)));
    let supply = match army {
        Some(_) if v.plot_revealed(to) => {
            let line = g.plot_supply_line_known(v.h, to, v.team);
            // At sea (docs/NAVAL.md, 3.3): the port it would be in range of.
            let port = line.as_ref().and_then(|l| match l.source {
                ffh_core::army::SupplySource::Sea(c) => Some(crate::view::sea::port_name(v, c)),
                _ => None,
            });
            // No line: why a supply ship of the human's there is no source (docs/NAVAL.md, 4.6).
            let why = if line.is_none() { crate::view::seas::transport_refusal(v, to) } else { None };
            crate::jobj! { "ok" => line.is_some(), "cost" => line.as_ref().map(|l| l.cost), "budget" => g.d.defines.supply_line_budget, "sea_port" => port, "sea_why" => why }
        }
        _ => Json::Null,
    };
    Ok(crate::jobj! {
        "ok" => ok,
        "turns" => if ok { finder.path_turns().map(Json::from).unwrap_or(Json::Null) } else { Json::Null },
        "nodes" => Json::Arr(nodes),
        "declare_war_team" => war,
        "attack" => attack,
        "approximate" => mirrored.is_none(),
        "supply" => supply,
        "sea" => sea,
    })
}

/// The attacker the selection would send against `plot`: the group's (C++
/// `AI_getBestGroupAttacker`, the human's rule: the best odds, the first of equals); for a
/// selection that is not a whole group, the selected unit with the best exact odds.
fn best_attacker(v: &Viewer, units: &[UnitId], plot: PlotIdx, force: bool) -> Option<UnitId> {
    let g = v.g;
    if let Some(grp) = selection::mirrors_group(g, units) {
        return g.group_ai_get_best_group_attacker(grp, plot, false, force, false).0;
    }
    let mut best: Option<(i32, UnitId)> = None;
    for &u in units {
        if g.unit_is_dead(u) {
            continue;
        }
        let can_attack = if g.unit_get_domain_type(u) == DomainType::DOMAIN_AIR { g.unit_can_air_attack(u) } else { g.unit_can_attack(u) };
        if !can_attack || !(force || g.unit_can_move(u)) || !(force || g.unit_can_move_into(u, plot, true, false, false)) {
            continue;
        }
        let odds = g.plot_get_best_defender(plot, None, Some(v.h), Some(u), false, false, false).map_or(1000, |def| g.get_combat_odds(u, def));
        let value = odds.max(1);
        if best.is_none_or(|(b, _)| value > b) {
            best = Some((value, u));
        }
    }
    best.map(|(_, u)| u)
}

fn details_json(c: &CombatDetails) -> Json {
    let mut j = crate::jobj! {};
    for (k, x) in [
        ("extra_combat_percent", c.extra_combat_percent),
        ("animal_combat_modifier_ta", c.animal_combat_modifier_ta),
        ("animal_combat_modifier_aa", c.animal_combat_modifier_aa),
        ("barbarian_combat_modifier_tb", c.barbarian_combat_modifier_tb),
        ("barbarian_combat_modifier_ab", c.barbarian_combat_modifier_ab),
        ("plot_defense_modifier", c.plot_defense_modifier),
        ("fortify_modifier", c.fortify_modifier),
        ("city_defense_modifier", c.city_defense_modifier),
        ("hills_attack_modifier", c.hills_attack_modifier),
        ("hills_defense_modifier", c.hills_defense_modifier),
        ("feature_attack_modifier", c.feature_attack_modifier),
        ("feature_defense_modifier", c.feature_defense_modifier),
        ("terrain_attack_modifier", c.terrain_attack_modifier),
        ("terrain_defense_modifier", c.terrain_defense_modifier),
        ("city_attack_modifier", c.city_attack_modifier),
        ("domain_defense_modifier", c.domain_defense_modifier),
        ("city_barbarian_defense_modifier", c.city_barbarian_defense_modifier),
        ("class_defense_modifier", c.class_defense_modifier),
        ("class_attack_modifier", c.class_attack_modifier),
        ("combat_modifier_t", c.combat_modifier_t),
        ("combat_modifier_a", c.combat_modifier_a),
        ("domain_modifier_a", c.domain_modifier_a),
        ("domain_modifier_t", c.domain_modifier_t),
        ("animal_combat_modifier_a", c.animal_combat_modifier_a),
        ("animal_combat_modifier_t", c.animal_combat_modifier_t),
        ("river_attack_modifier", c.river_attack_modifier),
        ("amphib_attack_modifier", c.amphib_attack_modifier),
        ("kamikaze_modifier", c.kamikaze_modifier),
        ("modifier_total", c.modifier_total),
        ("base_combat_str", c.base_combat_str),
        ("combat", c.combat),
        ("max_combat_str", c.max_combat_str),
        ("curr_hit_points", c.curr_hit_points),
        ("max_hit_points", c.max_hit_points),
        ("curr_combat_str", c.curr_combat_str),
    ] {
        j.set(k, x);
    }
    j
}

/// `combat {units,x,y}`: the attack preview of C++ `setCombatPlotHelp` on a plot the human sees:
/// the attacker the selection would send, the defender it would meet, the odds of winning (per
/// mille), of retreating (per mille), both current strengths (x100), and each side's strength
/// breakdown (`CombatDetails` from `maxCombatStr`). Under engagements, an attack of land units
/// on land adds `engagement: true` and decides `valid` by the pool and the defenders
/// (`battles::combat_fields`); the duel's odds stay beside it (a battle's forecast is the plan's).
pub fn combat(v: &Viewer, units: &[UnitId], x: i32, y: i32) -> Result<Json, QueryErr> {
    let j = duel_combat(v, units, x, y)?;
    let g = v.g;
    let plot = plot_at(v, x, y)?;
    if g.engage_applies(units[0], plot) {
        return Ok(crate::view::battles::combat_fields(v, units, plot, j));
    }
    Ok(j)
}

/// The duel's `combat` preview (C++ `setCombatPlotHelp`).
fn duel_combat(v: &Viewer, units: &[UnitId], x: i32, y: i32) -> Result<Json, QueryErr> {
    check_selection(v, units)?;
    let plot = plot_at(v, x, y)?;
    let g = v.g;
    let none = || crate::jobj! { "valid" => false };
    if !v.plot_visible(plot) {
        return Ok(none());
    }
    let first = best_attacker(v, units, plot, false);
    // The selection's domain (its head's) decides where it can fight.
    let valid = match g.unit_get_domain_type(units[0]) {
        DomainType::DOMAIN_SEA => g.plot_is_water(plot),
        DomainType::DOMAIN_AIR => true,
        DomainType::DOMAIN_LAND => !g.plot_is_water(plot) || first.is_some_and(|a| g.unit_can_move_all_terrain(a) || g.unit_is_boarding(a)),
        _ => false,
    };
    if !valid {
        return Ok(none());
    }
    let Some(att) = first.or_else(|| best_attacker(v, units, plot, true)) else { return Ok(none()) };
    let def = if g.unit_is_always_hostile(att, Some(plot)) {
        g.plot_get_best_defender(plot, None, Some(v.h), Some(att), false, false, false)
    } else {
        g.plot_get_best_defender(plot, None, Some(v.h), Some(att), false, g.unit_get_declare_war_move(att, plot).is_none(), false)
    };
    let Some(def) = def.filter(|&d| d != att && v.unit_visible(d) && g.unit_can_defend(d, Some(plot)) && g.unit_can_attack_unit(att, d)) else {
        return Ok(none());
    };
    let air = g.unit_get_domain_type(att) == DomainType::DOMAIN_AIR;
    let odds = (!air).then(|| g.get_combat_odds(att, def));
    let max_hp = g.d.defines.max_hit_points;
    let withdraw = odds.map(|o| {
        let mut w = 0;
        if g.units[att].combat_limit < max_hp {
            w += 100 * o;
        }
        w += g.unit_withdrawal_probability(att).min(100) * (1000 - o);
        w / 100
    });
    let mut ad = CombatDetails::default();
    let mut dd = CombatDetails::default();
    let att_str = if air { g.unit_air_curr_combat_str(att, Some(def)) } else { g.unit_curr_combat_str(att, None, Some(def), Some(&mut ad)) };
    let def_str = g.unit_curr_combat_str(def, Some(plot), Some(att), Some(&mut dd));
    Ok(crate::jobj! {
        "valid" => true,
        "attacker" => att,
        "defender" => def,
        "defender_owner" => v.unit_owner(def),
        "defender_type" => Json::idx(g.units[def].unit_type),
        "odds" => odds,
        "withdraw" => withdraw,
        "attacker_strength" => att_str,
        "defender_strength" => def_str,
        "attacker_first_strikes" => crate::jarr![g.unit_first_strikes(att), g.unit_chance_first_strikes(att)],
        "defender_first_strikes" => crate::jarr![g.unit_first_strikes(def), g.unit_chance_first_strikes(def)],
        "collateral" => g.unit_collateral_damage(att),
        // The sea (docs/NAVAL.md, 3.2 and 3.4): a warship's wear as one more term of its
        // strength (`sea_wear`, negative), and a defender in boats (strength 1, no modifier).
        "attacker_details" => details_json(&ad).with("sea_wear", -g.unit_sea_wear_percent(att)),
        "defender_details" => details_json(&dd).with("sea_wear", -g.unit_sea_wear_percent(def)),
        "defender_in_boats" => g.unit_in_boats_at(def, plot),
        "embarked_strength" => g.d.defines.embarked_strength,
        "declare_war_team" => g.unit_get_declare_war_move(att, plot),
    })
}
