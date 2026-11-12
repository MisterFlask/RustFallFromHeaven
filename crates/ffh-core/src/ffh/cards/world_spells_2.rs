//! Batch B7, the second batch of card world spells (docs/CARDS.md, section 5; the specs are
//! docs/DECKS_ROUND8.md with the C.4 changes, and docs/DECKS.md). Six cards are world spells cast
//! once a game, three are rituals, one a national wonder:
//!
//! | Card | Vehicle | Effect |
//! |---|---|---|
//! | B02 Sing the Stones | spell | every City Ruins within 3 plots of the borders becomes a city of size 3 |
//! | B13 Dragon's Oath | spell | Sylvarith, a dragon of the card's own, joins the caster's owner |
//! | P06 Call the Leviathan | spell at a city | the Leviathan (a Kraken while one lives) in a coastal city |
//! | Judgment of Junil | spell at a player | 20 turns with no war between the two teams |
//! | Earthshaper's Decree | spell | every unimproved land plot of the owner gets its best improvement |
//! | The Plagiarist's Lens | spell at a choice | the team learns a card a civilization it has met knows |
//! | Weather Engine | ritual, spells at a plot | a Blizzard, Storm or Fog over an area for 5 turns |
//! | Tidecall of Danalin | ritual, spell at a plot | a flat land plot by the water becomes Coast |
//! | P12 Pillar of Dawn | ritual | the owner's units near the city heal fully each turn for 10 turns |
//! | The Second Sky | national wonder | its team sees every mana node, unique feature, lair and goody hut |
//!
//! As in batch B3 (`world_spells.rs`), a spell cast at a distance has a predicate for one target
//! (`*_ok`): its `targets*` callback lists what passes it, and its result checks the target again.
//!
//! The rules that are not callbacks live here as `Game` methods and are called where the rule they
//! change lives, with a comment naming the card: the truce (`team_has_truce`: war declarations,
//! hidden units, False Colours), the rituals' charges (`ritual_charge`: casting, and the city's
//! queue), the cost of a repeated ritual (`repeat_cost_percent`), the sites a building reveals
//! (`reveals_sites`), and the area effects (`Game::area_effects`: healing, moves, battles, sight).

use super::world::near_borders;
use super::world_spells::own_city_at;
use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use crate::structs::{AreaEffect, AreaEffectKind, SpellTarget};
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellSingTheStones" => spell_sing_the_stones,
        "spellDragonsOath" => spell_dragons_oath,
        "spellCallTheLeviathan" => spell_call_the_leviathan,
        "spellJudgmentOfJunil" => spell_judgment_of_junil,
        "spellEarthshapersDecree" => spell_earthshapers_decree,
        "spellPlagiaristsLens" => spell_plagiarists_lens,
        "spellWeather" => spell_weather,
        "spellTidecall" => spell_tidecall,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqSingTheStones" => req_sing_the_stones,
        "reqDragonsOath" => req_dragons_oath,
        "reqEarthshapersDecree" => req_earthshapers_decree,
        _ => return None,
    };
    Some(f)
}

/// `py_targets` functions.
pub fn find_targets(name: &str) -> Option<TargetsFn> {
    let f: TargetsFn = match name {
        "targetsCallTheLeviathan" => targets_call_the_leviathan,
        "targetsJudgmentOfJunil" => targets_judgment_of_junil,
        "targetsPlagiaristsLens" => targets_plagiarists_lens,
        "targetsWeather" => targets_weather,
        "targetsTidecall" => targets_tidecall,
        _ => return None,
    };
    Some(f)
}

/// `py_target_label` functions: what a city or a player target adds to its name.
pub fn find_target_label(name: &str) -> Option<TargetLabelFn> {
    let f: TargetLabelFn = match name {
        "labelCallTheLeviathan" => label_call_the_leviathan,
        "labelJudgmentOfJunil" => label_judgment_of_junil,
        _ => return None,
    };
    Some(f)
}

// ---------------------------------------------------------------------------------------------
// Numbers.

/// Sing the Stones: how far from its caster's borders a City Ruins may be.
pub const STONES_RANGE: i32 = 3;
/// Sing the Stones: the size of a city it refounds.
pub const STONES_POPULATION: i32 = 3;
/// Judgment of Junil: the turns the truce holds.
pub const TRUCE_TURNS: i32 = 20;
/// The Plagiarist's Lens: what a card's holders think of its copier (`ai_attitude_extra`).
pub const LENS_ATTITUDE: i32 = -2;
/// The Weather Engine: how far from its city a weather's centre may be.
pub const WEATHER_RANGE: i32 = 10;
/// The Weather Engine: the radius of a weather around its centre.
pub const WEATHER_RADIUS: i32 = 5;
/// The Weather Engine: the turns a weather lasts after the turn it is cast.
pub const WEATHER_TURNS: i32 = 5;
/// Storm: the percent less damage SHOOT and BOMBARD deal in its battles.
pub const STORM_RANGED_PERCENT: i32 = 50;
/// Fog: within how many plots a team must have a unit or a city to see the units in it.
pub const FOG_SIGHT: i32 = 1;
/// Tidecall of Danalin: how far from its city the plot may be.
pub const TIDECALL_RANGE: i32 = 3;
/// Pillar of Dawn: the radius around its city.
pub const PILLAR_RADIUS: i32 = 5;
/// Pillar of Dawn: the turns its units heal.
pub const PILLAR_TURNS: i32 = 10;

/// The plots within `range` plots (plot distance) of `(x, y)`, each once, in plot order.
fn plots_within(g: &Game, x: i32, y: i32, range: i32) -> Vec<PlotIdx> {
    let mut out = Vec::new();
    for dy in -range..=range {
        for dx in -range..=range {
            if let Some(t) = g.plot_xy(x, y, dx, dy) {
                let (tx, ty) = g.map.xy(t);
                if g.plot_distance(x, y, tx, ty) <= range {
                    out.push(t);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The city target being cast at, if it is still a city.
fn target_city(g: &Game) -> Option<CityId> {
    match g.spell_target {
        Some(SpellTarget::City(c)) if g.cities.contains(c) => Some(c),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// B02 Sing the Stones: every City Ruins its team has revealed, within 3 plots of the caster's
// owner's borders, where the owner could found a city (not in another civilization's land, not
// too close to a city, no enemy unit seen on it), becomes a city of the owner of size 3.

/// The City Ruins player `p`'s Sing the Stones would refound now, in plot order.
pub fn sing_the_stones_ruins(g: &Game, p: PlayerId) -> Vec<PlotIdx> {
    let Some(ruins) = g.d.ty::<ImprovementType>("IMPROVEMENT_CITY_RUINS") else { return Vec::new() };
    let team = g.players[p].team;
    let near = near_borders(g, p, STONES_RANGE);
    g.map
        .plot_ids()
        .filter(|&t| {
            let plot = &g.map.plots[t];
            near[t.idx()] && plot.improvement_type == Some(ruins) && plot.revealed[team] && {
                let (x, y) = g.map.xy(t);
                g.player_can_found(p, x, y, false)
            }
        })
        .collect()
}

/// `reqSingTheStones(pCaster)`: a City Ruins to refound.
fn req_sing_the_stones(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    !sing_the_stones_ruins(g, g.units[caster].owner).is_empty()
}

/// `spellSingTheStones(pCaster)`: each ruin, in plot order, becomes a city of size 3 (a ruin too
/// close to one founded just before stays a ruin).
fn spell_sing_the_stones(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let p = g.units[caster].owner;
    let mut names = Vec::new();
    for t in sing_the_stones_ruins(g, p) {
        let (x, y) = g.map.xy(t);
        if !g.player_can_found(p, x, y, false) {
            continue;
        }
        g.player_found(p, x, y);
        let Some(c) = g.map.plots[t].plot_city.filter(|&c| g.cities.contains(c) && g.cities[c].owner == p) else { continue };
        if g.cities[c].population < STONES_POPULATION {
            g.city_set_population(c, STONES_POPULATION);
        }
        names.push(g.d.text.name(&g.cities[c].name).to_string());
    }
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Sing the Stones: {} cities rise from their ruins ({})", names.len(), names.join(", ")));
}

// ---------------------------------------------------------------------------------------------
// B13 Dragon's Oath: Sylvarith joins the caster's owner in its capital, while no Sylvarith lives.

/// The capital where Sylvarith would join `p`: none while a Sylvarith lives, or without a capital.
pub fn dragons_oath_city(g: &Game, p: PlayerId) -> Option<CityId> {
    let d = g.d;
    let unit = d.ty::<UnitType>("UNIT_SYLVARITH")?;
    let class = d.units[unit].unit_class_type?;
    if g.player_ids().any(|o| g.players[o].unit_class_count[class] > 0) {
        return None;
    }
    g.player_get_capital_city(p)
}

/// `reqDragonsOath(pCaster)`.
fn req_dragons_oath(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    dragons_oath_city(g, g.units[caster].owner).is_some()
}

/// `spellDragonsOath(pCaster)`.
fn spell_dragons_oath(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let p = g.units[caster].owner;
    let Some(c) = dragons_oath_city(g, p) else { return };
    let unit = g.d.t::<UnitType>("UNIT_SYLVARITH");
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    g.player_init_unit(p, unit, x, y, None, None);
    let turn = g.game_turn();
    let name = g.d.text.name(&g.cities[c].name).to_string();
    g.log.msg_at(turn, Some(p), (x, y), || format!("Dragon's Oath: Sylvarith joins you in {name}"));
}

// ---------------------------------------------------------------------------------------------
// P06 Call the Leviathan: the Leviathan joins the caster's owner in a coastal city of its (on the
// ocean, not a lake), or a Kraken while a Leviathan lives.

/// The beast Call the Leviathan brings now: the Leviathan, or a Kraken while one lives.
pub fn leviathan_beast(g: &Game) -> Option<UnitType> {
    let d = g.d;
    let leviathan = d.ty::<UnitType>("UNIT_LEVIATHAN")?;
    let class = d.units[leviathan].unit_class_type?;
    if g.player_ids().any(|o| g.players[o].unit_class_count[class] > 0) {
        d.ty::<UnitType>("UNIT_KRAKEN")
    } else {
        Some(leviathan)
    }
}

/// May `caster` call the beast to city `c` (a coastal city of its owner)?
pub fn leviathan_city_ok(g: &Game, caster: UnitId, c: CityId) -> bool {
    g.cities[c].owner == g.units[caster].owner && g.city_is_coastal(c, g.d.defines.min_water_size_for_ocean)
}

/// `targetsCallTheLeviathan(pCaster)`: the owner's coastal cities, in city-list order.
fn targets_call_the_leviathan(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    if leviathan_beast(g).is_none() {
        return Vec::new();
    }
    let p = g.units[caster].owner;
    g.players[p].cities.iter().copied().filter(|&c| g.cities.contains(c) && leviathan_city_ok(g, caster, c)).map(SpellTarget::City).collect()
}

/// `labelCallTheLeviathan(pCaster)`: the beast that would come.
fn label_call_the_leviathan(g: &Game, _caster: UnitId, _target: SpellTarget, _: &PyCall) -> String {
    leviathan_beast(g).map_or_else(String::new, |u| g.d.text.name(&g.d.units[u].base.description).to_string())
}

/// `spellCallTheLeviathan(pCaster)`.
fn spell_call_the_leviathan(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(c) = target_city(g) else { return };
    if !leviathan_city_ok(g, caster, c) {
        return;
    }
    let Some(beast) = leviathan_beast(g) else { return };
    let p = g.units[caster].owner;
    let (x, y) = (g.cities[c].x, g.cities[c].y);
    g.player_init_unit(p, beast, x, y, None, None);
    let turn = g.game_turn();
    let (what, name) = (g.d.text.name(&g.d.units[beast].base.description).to_string(), g.d.text.name(&g.cities[c].name).to_string());
    g.log.msg_at(turn, Some(p), (x, y), || format!("Call the Leviathan: the {what} joins you in {name}"));
}

// ---------------------------------------------------------------------------------------------
// Judgment of Junil: a player of another team the caster's team has met (a civilization: not the
// barbarians, a minor civilization or a crusade Host), with no truce already. A war between the
// two teams ends at once; for 20 turns there is none (`Team::truce_until`, `team_has_truce`).

impl Game {
    /// Card: Judgment of Junil (docs/CARDS.md): does a truce hold between teams `t` and `other`?
    /// While it does there is no war between them: neither can declare it, the declarations other
    /// wars drag in skip it, and their always-hostile units (hidden nationality) are not enemies.
    pub fn team_has_truce(&self, t: TeamId, other: TeamId) -> bool {
        t != other && self.teams[t].truce_until[other] > self.game_turn()
    }
}

/// May `caster` name player `o` with Judgment of Junil?
pub fn junil_ok(g: &Game, caster: UnitId, o: PlayerId) -> bool {
    let team = g.unit_get_team(caster);
    let ot = g.players[o].team;
    g.players[o].alive
        && ot != team
        && !g.player_is_barbarian(o)
        && !g.team_is_minor_civ(ot)
        && !g.team_is_crusade_host(ot)
        && g.teams[team].has_met[ot]
        && !g.teams[team].permanent_war_peace[ot]
        && !g.team_has_truce(team, ot)
}

/// `targetsJudgmentOfJunil(pCaster)`: the players it may name, in player order.
fn targets_judgment_of_junil(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    g.player_ids().filter(|&o| junil_ok(g, caster, o)).map(SpellTarget::Player).collect()
}

/// `labelJudgmentOfJunil(pCaster)`: a player at war with the caster's team says so (the war ends).
fn label_judgment_of_junil(g: &Game, caster: UnitId, target: SpellTarget, _: &PyCall) -> String {
    match target {
        SpellTarget::Player(o) if g.units.contains(caster) && g.at_war(g.unit_get_team(caster), g.players[o].team) => "at war: the war ends".to_string(),
        _ => String::new(),
    }
}

/// Judgment of Junil between teams `t` and `other` from this turn: a war between them ends, and
/// the truce holds `TRUCE_TURNS` turns.
pub fn impose_truce(g: &mut Game, t: TeamId, other: TeamId) {
    if g.at_war(t, other) {
        g.team_make_peace(t, other, true);
    }
    let until = g.game_turn() + TRUCE_TURNS;
    g.teams[t].truce_until[other] = until;
    g.teams[other].truce_until[t] = until;
}

/// `spellJudgmentOfJunil(pCaster)`.
fn spell_judgment_of_junil(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Player(o)) = g.spell_target else { return };
    if !junil_ok(g, caster, o) {
        return;
    }
    let p = g.units[caster].owner;
    let (team, ot) = (g.players[p].team, g.players[o].team);
    impose_truce(g, team, ot);
    let turn = g.game_turn();
    let (ours, theirs) = (g.d.text.name(&g.player_get_name(p)).to_string(), g.d.text.name(&g.player_get_name(o)).to_string());
    g.log.msg(turn, Some(p), || format!("Judgment of Junil: no war with {theirs} for {TRUCE_TURNS} turns"));
    g.log.msg(turn, Some(o), || format!("Judgment of Junil: no war with {ours} for {TRUCE_TURNS} turns"));
}

// ---------------------------------------------------------------------------------------------
// Earthshaper's Decree: every land plot of the caster's owner without an improvement or a city gets
// the improvement that suits it best, among those its cities' crews could build there
// now without removing its feature and without changing its resource: the one that connects the
// plot's resource (the most food and production among those), else the one that adds the most food
// and production, then commerce (none that adds nothing).

/// The builds of the cities' crews (docs/WORKS.md) that make an improvement which changes no
/// resource, in build order.
fn crew_builds(g: &Game) -> Vec<BuildType> {
    let d = g.d;
    d.builds.keys().filter(|&b| d.builds[b].city && d.builds[b].improvement.is_some_and(|i| d.improvements[i].bonus_convert.is_none())).collect()
}

/// The improvement Earthshaper's Decree gives plot `t` of player `p`, from `builds`
/// ([`crew_builds`]), if any.
fn decree_improvement_with(g: &Game, p: PlayerId, t: PlotIdx, builds: &[BuildType]) -> Option<ImprovementType> {
    let d = g.d;
    let plot = &g.map.plots[t];
    if plot.owner != Some(p) || g.plot_is_water(t) || plot.improvement_type.is_some() || plot.plot_city.is_some() {
        return None;
    }
    let team = g.players[p].team;
    let bonus = g.plot_get_bonus_type(t, Some(team));
    let mut best: Option<((bool, i32, i32), ImprovementType)> = None;
    for &b in builds {
        let Some(imp) = d.builds[b].improvement else { continue };
        if plot.feature_type.is_some_and(|f| d.builds[b].feature_remove[f]) || !g.player_can_build(p, t, b, false) {
            continue;
        }
        let connects = bonus.is_some_and(|bn| d.improvements[imp].improvement_bonus[bn].bonus_trade);
        let y = |yt: YieldType| g.plot_calculate_improvement_yield_change(t, imp, yt, Some(p), false, false);
        let (food_prod, commerce) = (y(YieldType::YIELD_FOOD) + y(YieldType::YIELD_PRODUCTION), y(YieldType::YIELD_COMMERCE));
        if !connects && food_prod <= 0 && commerce <= 0 {
            continue;
        }
        let key = (connects, food_prod, commerce);
        if best.map_or(true, |(k, _)| key > k) {
            best = Some((key, imp));
        }
    }
    best.map(|(_, imp)| imp)
}

/// The plots Earthshaper's Decree improves for `p` and their improvements, in plot order.
pub fn decree_plots(g: &Game, p: PlayerId) -> Vec<(PlotIdx, ImprovementType)> {
    let builds = crew_builds(g);
    if builds.is_empty() {
        return Vec::new();
    }
    g.map.plot_ids().filter_map(|t| decree_improvement_with(g, p, t, &builds).map(|i| (t, i))).collect()
}

/// `reqEarthshapersDecree(pCaster)`: a plot to improve.
fn req_earthshapers_decree(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    !decree_plots(g, g.units[caster].owner).is_empty()
}

/// `spellEarthshapersDecree(pCaster)`.
fn spell_earthshapers_decree(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let p = g.units[caster].owner;
    let plots = decree_plots(g, p);
    for &(t, imp) in &plots {
        g.plot_set_improvement_type(t, Some(imp));
    }
    let turn = g.game_turn();
    g.log.msg(turn, Some(p), || format!("Earthshaper's Decree: {} plots improved", plots.len()));
}

// ---------------------------------------------------------------------------------------------
// The Plagiarist's Lens: the caster's team learns a card (a tech with a suit) known by the team of
// a living player it has met (not the barbarians), that it lacks. Each player of a team that knows
// the card thinks less of the caster's owner (`LENS_ATTITUDE`).

/// The cards `p`'s team may copy with the Lens, in tech order.
pub fn lens_cards(g: &Game, p: PlayerId) -> Vec<TechType> {
    let d = g.d;
    let team = g.players[p].team;
    let others: Vec<TeamId> = g
        .player_ids()
        .filter(|&o| g.players[o].alive && !g.player_is_barbarian(o) && g.players[o].team != team && g.teams[team].has_met[g.players[o].team])
        .map(|o| g.players[o].team)
        .collect();
    d.techs.keys().filter(|&t| d.techs[t].card.is_some() && !g.team_is_has_tech(team, Some(t)) && others.iter().any(|&o| g.team_is_has_tech(o, Some(t)))).collect()
}

/// `targetsPlagiaristsLens(pCaster)`: each card, by its index (`labelInfo(pCaster, iChoice,
/// 'TechType')` names it).
fn targets_plagiarists_lens(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    lens_cards(g, g.units[caster].owner).into_iter().map(|t| SpellTarget::Choice(t.idx() as i32)).collect()
}

/// `spellPlagiaristsLens(pCaster)`.
fn spell_plagiarists_lens(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Choice(i)) = g.spell_target else { return };
    let p = g.units[caster].owner;
    let Some(card) = lens_cards(g, p).into_iter().find(|t| t.idx() as i32 == i) else { return };
    let team = g.players[p].team;
    let holders: Vec<PlayerId> = g
        .player_ids()
        .filter(|&o| g.players[o].alive && !g.player_is_barbarian(o) && g.players[o].team != team && g.teams[team].has_met[g.players[o].team] && g.team_is_has_tech(g.players[o].team, Some(card)))
        .collect();
    g.team_set_has_tech(team, card, true, Some(p), false, false);
    for &o in &holders {
        g.players[o].ai_attitude_extra[p] += LENS_ATTITUDE;
    }
    let turn = g.game_turn();
    let name = g.d.text.name(&g.d.techs[card].base.description).to_string();
    let copier = g.d.text.name(&g.player_get_name(p)).to_string();
    g.log.msg(turn, Some(p), || format!("The Plagiarist's Lens: you learn {name}"));
    for o in holders {
        g.log.msg(turn, Some(o), || format!("The Plagiarist's Lens: {copier} copies {name}"));
    }
}

// ---------------------------------------------------------------------------------------------
// The rituals' charges (`SpellInfo::ritual_charge`, docs/CARDS.md): a completion of a ritual that a
// spell names leaves its city a charge; a unit in the city casts the spell, which spends it. A city
// holding a ritual's charge does not build it again.

/// The city of its owner unit `u` stands in, if it holds a charge of card ritual `ritual`. (A
/// world ritual leaves no charge: its cast is its owner's, pending: `Player::world_ritual_pending`.)
pub fn ritual_charge_city(g: &Game, u: UnitId, ritual: ProjectType) -> Option<CityId> {
    own_city_at(g, u).filter(|&c| g.cities[c].ritual_charges.contains(&ritual))
}

/// Does a spell spend charges of `project` (a ritual with a charge)?
fn charges(d: &GameData, project: ProjectType) -> bool {
    d.spells.iter().any(|s| s.ritual_charge == Some(project))
}

/// Card rituals, at their completion in city `c` (`ev_project_built`): the charge of a ritual a
/// spell spends; the Pillar of Dawn's healing.
pub fn project_built(g: &mut Game, c: CityId, project: ProjectType) {
    let d = g.d;
    let p = g.cities[c].owner;
    let turn = g.game_turn();
    let name = d.text.name(&d.projects[project].base.description).to_string();
    // (A world ritual's charge: `world_rituals::project_built`.)
    if charges(d, project) && !d.projects[project].world_ritual {
        g.cities[c].ritual_charges.push(project);
        let city = d.text.name(&g.cities[c].name).to_string();
        g.log.msg(turn, Some(p), || format!("{name}: ready in {city}"));
    }
    if d.ty::<ProjectType>("PROJECT_PILLAR_OF_DAWN") == Some(project) {
        let plot = g.city_plot(c);
        g.area_effects.push(AreaEffect { kind: AreaEffectKind::PillarOfDawn, owner: p, plot, radius: PILLAR_RADIUS, until: turn + PILLAR_TURNS + 1, moved: -1 });
        let (x, y) = g.map.xy(plot);
        g.log.msg_at(turn, Some(p), (x, y), || format!("{name}: your units within {PILLAR_RADIUS} plots heal fully for {PILLAR_TURNS} turns"));
    }
}

/// The Tidecall of Danalin's ritual (looked up once).
fn tidecall_project(d: &GameData) -> Option<ProjectType> {
    static P: std::sync::OnceLock<Option<ProjectType>> = std::sync::OnceLock::new();
    *P.get_or_init(|| d.ty::<ProjectType>("PROJECT_TIDECALL_OF_DANALIN"))
}

/// Card rituals that city `c` cannot build (`cannotCreate`): one whose charge it holds; the
/// Tidecall of Danalin away from the ocean.
pub fn cannot_create(g: &Game, c: CityId, project: ProjectType) -> bool {
    // (A world ritual's held charge is `world_rituals::cannot_create`'s: the ritual stays listed,
    // refused, in every city of its owner.)
    if !g.d.projects[project].world_ritual && g.cities[c].ritual_charges.contains(&project) {
        return true;
    }
    tidecall_project(g.d) == Some(project) && !g.city_is_coastal(c, g.d.defines.min_water_size_for_ocean)
}

impl Game {
    /// `ritual_charge`: may unit `u` cast a spell of `ritual`: a card ritual's, it stands in a city
    /// of its owner holding a charge; a world ritual's, its owner's cast is pending.
    pub fn unit_has_ritual_charge(&self, u: UnitId, ritual: ProjectType) -> bool {
        if self.d.projects[ritual].world_ritual {
            return self.player_world_ritual_pending(self.units[u].owner, ritual);
        }
        ritual_charge_city(self, u, ritual).is_some()
    }

    /// `ritual_charge`: unit `u` casts a spell of `ritual`: a card ritual's city loses a charge; a
    /// world ritual's pending cast is spent.
    pub fn unit_spend_ritual_charge(&mut self, u: UnitId, ritual: ProjectType) {
        if self.d.projects[ritual].world_ritual {
            let p = self.units[u].owner;
            if self.players[p].world_ritual_pending == Some(ritual) {
                self.players[p].world_ritual_pending = None;
            }
            return;
        }
        if let Some(c) = ritual_charge_city(self, u, ritual) {
            if let Some(i) = self.cities[c].ritual_charges.iter().position(|&r| r == ritual) {
                self.cities[c].ritual_charges.remove(i);
            }
        }
    }

    /// `repeat_cost_percent` (docs/CARDS.md; the Pillar of Dawn): `cost` grown by that percent
    /// once for each time team `t` has completed `project` (at most 16 times).
    pub fn team_repeat_project_cost(&self, t: TeamId, project: ProjectType, cost: i32) -> i32 {
        let pct = self.d.projects[project].repeat_cost_percent;
        if pct <= 0 {
            return cost;
        }
        let mut c = cost as i64;
        for _ in 0..self.teams[t].project_count[project].clamp(0, 16) {
            c = c * pct as i64 / 100;
        }
        c.min(i32::MAX as i64 / 4) as i32
    }
}

// ---------------------------------------------------------------------------------------------
// The Weather Engine: its spells are cast by a unit in a city of its owner holding the ritual's
// charge, at a plot its team has revealed within 10 plots of the city. The weather covers the
// plots within 5 plots of it for the rest of the turn and the next 5.

/// The weather of a `spellWeather(pCaster, 'KIND')` call.
fn weather_kind(call: &PyCall) -> Option<AreaEffectKind> {
    match call.str_arg(1)? {
        "BLIZZARD" => Some(AreaEffectKind::Blizzard),
        "STORM" => Some(AreaEffectKind::Storm),
        "FOG" => Some(AreaEffectKind::Fog),
        _ => None,
    }
}

/// May `caster` centre a weather on `t` (within 10 plots of the city it stands in, revealed)?
/// The charge is checked by the spell (`ritual_charge`).
pub fn weather_ok(g: &Game, caster: UnitId, t: PlotIdx) -> bool {
    let Some(c) = own_city_at(g, caster) else { return false };
    let (x, y) = g.map.xy(t);
    g.map.plots[t].revealed[g.unit_get_team(caster)] && g.plot_distance(g.cities[c].x, g.cities[c].y, x, y) <= WEATHER_RANGE
}

/// `targetsWeather(pCaster)`: the revealed plots within 10 plots of the caster's city.
fn targets_weather(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let Some(c) = own_city_at(g, caster) else { return Vec::new() };
    let team = g.unit_get_team(caster);
    plots_within(g, g.cities[c].x, g.cities[c].y, WEATHER_RANGE).into_iter().filter(|&t| g.map.plots[t].revealed[team]).map(SpellTarget::Plot).collect()
}

/// `spellWeather(pCaster, 'KIND')`.
fn spell_weather(g: &mut Game, caster: UnitId, call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    let Some(kind) = weather_kind(call) else { return };
    if !weather_ok(g, caster, t) {
        return;
    }
    let p = g.units[caster].owner;
    let turn = g.game_turn();
    g.area_effects.push(AreaEffect { kind, owner: p, plot: t, radius: WEATHER_RADIUS, until: turn + WEATHER_TURNS + 1, moved: -1 });
    let (x, y) = g.map.xy(t);
    let what = match kind {
        AreaEffectKind::Blizzard => "Blizzard",
        AreaEffectKind::Storm => "Storm",
        _ => "Fog",
    };
    // Every player is told (big effects are visible to opponents).
    g.log.msg_at(turn, None, (x, y), || format!("Weather Engine: {what} within {WEATHER_RADIUS} plots of ({x}, {y}) for {WEATHER_TURNS} turns"));
}

impl Game {
    /// The area effects of kind `kind` that hold on plot `t` this turn (docs/CARDS.md).
    pub fn plot_area_effects(&self, t: PlotIdx, kind: AreaEffectKind) -> impl Iterator<Item = &AreaEffect> + '_ {
        let turn = self.game_turn();
        let (x, y) = self.map.xy(t);
        self.area_effects.iter().filter(move |e| {
            e.kind == kind && e.until > turn && {
                let (ex, ey) = self.map.xy(e.plot);
                self.plot_distance(x, y, ex, ey) <= e.radius
            }
        })
    }

    /// Card: the Weather Engine (docs/CARDS.md): is plot `t` under a weather of kind `kind`?
    pub fn plot_has_weather(&self, t: PlotIdx, kind: AreaEffectKind) -> bool {
        !self.area_effects.is_empty() && self.plot_area_effects(t, kind).next().is_some()
    }

    /// Card: the Weather Engine's Fog (docs/CARDS.md): are the units on plot `t` hidden from team
    /// `team` (the plot is in fog, and the team has no unit or city within 1 plot of it)?
    pub fn plot_fog_hides(&self, t: PlotIdx, team: TeamId) -> bool {
        if self.area_effects.is_empty() || !self.plot_has_weather(t, AreaEffectKind::Fog) {
            return false;
        }
        let (x, y) = self.map.xy(t);
        for dy in -FOG_SIGHT..=FOG_SIGHT {
            for dx in -FOG_SIGHT..=FOG_SIGHT {
                let Some(q) = self.plot_xy(x, y, dx, dy) else { continue };
                let plot = &self.map.plots[q];
                if plot.plot_city.is_some_and(|c| self.cities.contains(c) && self.city_team(c) == team) {
                    return false;
                }
                if plot.units.iter().any(|&u| self.unit_get_team(u) == team) {
                    return false;
                }
            }
        }
        true
    }

    /// Card: the Pillar of Dawn (docs/CARDS.md): does a Pillar of Dawn of `u`'s owner cover the plot
    /// it stands on (and no Blizzard)?
    pub fn unit_in_pillar_of_dawn(&self, u: UnitId) -> bool {
        if self.area_effects.is_empty() {
            return false;
        }
        let Some(t) = self.unit_plot_opt(u) else { return false };
        let owner = self.units[u].owner;
        self.plot_area_effects(t, AreaEffectKind::PillarOfDawn).any(|e| e.owner == owner) && !self.plot_has_weather(t, AreaEffectKind::Blizzard)
    }

    /// The area effects at the end of a game turn: those that have ended are dropped.
    pub fn area_effects_do_turn(&mut self) {
        let turn = self.game_turn();
        self.area_effects.retain(|e| e.until > turn);
    }
}

// ---------------------------------------------------------------------------------------------
// Tidecall of Danalin: its spell is cast by a unit in a city of its owner holding the ritual's
// charge, at a flat land plot its team sees within 3 plots of the city, next to water, with no
// city and no unique feature. The plot becomes Coast for good.

/// May `caster` drown plot `t`? The charge is checked by the spell (`ritual_charge`).
pub fn tidecall_ok(g: &Game, caster: UnitId, t: PlotIdx) -> bool {
    let d = g.d;
    let Some(c) = own_city_at(g, caster) else { return false };
    let plot = &g.map.plots[t];
    let (x, y) = g.map.xy(t);
    !g.plot_is_water(t)
        && !g.plot_is_hills(t)
        && !g.plot_is_peak(t)
        && plot.plot_city.is_none()
        && plot.improvement_type.map_or(true, |i| !d.improvements[i].unique && !d.improvements[i].permanent)
        && g.plot_is_visible(t, g.unit_get_team(caster))
        && g.plot_distance(g.cities[c].x, g.cities[c].y, x, y) <= TIDECALL_RANGE
        && g.plot_is_adjacent_to_water(t)
}

/// `targetsTidecall(pCaster)`: the plots within 3 plots of the caster's city it may drown.
fn targets_tidecall(g: &Game, caster: UnitId, _call: &PyCall) -> Vec<SpellTarget> {
    let Some(c) = own_city_at(g, caster) else { return Vec::new() };
    plots_within(g, g.cities[c].x, g.cities[c].y, TIDECALL_RANGE).into_iter().filter(|&t| tidecall_ok(g, caster, t)).map(SpellTarget::Plot).collect()
}

/// Plot `t` becomes Coast for good: its improvement, feature, route and land resource go with
/// the land (`plot_set_plot_type`, which erases what was on the plot). Its units leave it first,
/// each to the nearest other plot it may stand on (a land unit to land, a ship in a canal to
/// water; its cargo with it).
pub fn drown_plot(g: &mut Game, t: PlotIdx) {
    let coast = g.d.t::<TerrainType>("TERRAIN_COAST");
    let units: Vec<UnitId> = g.map.plots[t].units.iter().copied().filter(|&u| g.units.contains(u) && !g.units[u].death_delay && !g.unit_is_cargo(u)).collect();
    for &u in &units {
        move_off(g, u, t);
    }
    g.plot_set_plot_type(t, PlotType::PLOT_OCEAN, true);
    g.plot_set_terrain_type(t, coast, true);
}

/// Moves `u` to the nearest plot other than `avoid` it may stand on and enter (ties: the lower
/// plot index), its cargo with it; it dies if there is none (as a unit left with no valid plot).
fn move_off(g: &mut Game, u: UnitId, avoid: PlotIdx) {
    let (x, y) = (g.units[u].x, g.units[u].y);
    let mut best: Option<(i32, PlotIdx)> = None;
    for q in g.map.plot_ids() {
        if q == avoid || !g.plot_is_valid_domain_for_location(q, u) || !g.unit_can_move_into(u, q, false, false, false) {
            continue;
        }
        let (qx, qy) = g.map.xy(q);
        let dist = g.plot_distance(x, y, qx, qy);
        if best.map_or(true, |(b, _)| dist < b) {
            best = Some((dist, q));
        }
    }
    match best {
        Some((_, q)) => {
            let (qx, qy) = g.map.xy(q);
            g.unit_set_xy(u, qx, qy, false);
        }
        None => g.unit_kill(u, false, None),
    }
}

/// `spellTidecall(pCaster)`.
fn spell_tidecall(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(SpellTarget::Plot(t)) = g.spell_target else { return };
    if !tidecall_ok(g, caster, t) {
        return;
    }
    drown_plot(g, t);
    let p = g.units.get(caster).map(|u| u.owner);
    let turn = g.game_turn();
    let (x, y) = g.map.xy(t);
    g.log.msg_at(turn, p, (x, y), || format!("Tidecall of Danalin: ({x}, {y}) becomes Coast"));
}

// ---------------------------------------------------------------------------------------------
// The Second Sky (`reveals_sites`): when built, its owner's team has every plot with a mana node,
// a unique feature, a lair or a goody hut revealed.

/// Does plot `t` hold a site the Second Sky reveals: a mana resource, or a unique feature, a lair
/// (an improvement that spawns units) or a goody hut?
pub fn is_site(g: &Game, t: PlotIdx) -> bool {
    let d = g.d;
    let plot = &g.map.plots[t];
    let mana = plot.bonus_type.is_some_and(|b| {
        let class = opt_to_i32(d.bonuses[b].bonus_class_type);
        class != -1 && (class == d.defines.bonusclass_mana || class == d.defines.bonusclass_mana_raw)
    });
    mana || plot.improvement_type.is_some_and(|i| {
        let info = &d.improvements[i];
        info.unique || info.goody || info.spawn_unit_type.is_some()
    })
}

/// Team `team` has every site revealed (the Second Sky).
pub fn team_reveal_sites(g: &mut Game, team: TeamId) {
    let sites: Vec<PlotIdx> = g.map.plot_ids().filter(|&t| is_site(g, t)).collect();
    for t in sites {
        if !g.map.plots[t].revealed[team] {
            g.plot_set_revealed(t, team, true, false, None, true);
        }
    }
}

#[cfg(test)]
mod tests;
