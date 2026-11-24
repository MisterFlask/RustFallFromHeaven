//! Why the rules refuse an order, as text for the player: one function per gate, from the
//! refusal the rules' own check returns (ffh-core's `*Refusal` enums), or, where the gate is not
//! one that returns its reason, from the gate's checks enumerated here in its order (hurry, draft,
//! the unit orders). Each says the check that failed, with the numbers of the moment
//! (`Anarchy: 2 turns left.`, `Gold: 40 of 120.`), bare (CLAUDE.md, "Rules text and tooltips").

use std::borrow::Cow;

use crate::names::display;
use crate::selection;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::player::cities::{FoundRefusal, GoodyRefusal};
use ffh_core::player::civics_religion::{CivicRefusal, ConvertRefusal, RevolutionRefusal};
use ffh_core::player::economy::EverResearchRefusal;
use ffh_core::player::events::EventRefusal;
use ffh_core::team::diplomacy::WarRefusal;
use ffh_core::unit::SpellRefusal;
use ffh_data::*;

/// `1 turn`, `3 turns`.
fn turns(n: i32) -> String {
    if n == 1 { "1 turn".to_string() } else { format!("{n} turns") }
}

/// `A, B or C`.
fn or_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {}", rest.join(", "), last),
    }
}

/// `Gold: 40 of 120.`
fn gold_line(have: i32, need: i32) -> String {
    format!("Gold: {} of {}.", have.max(0), need)
}

/// A team for people: the names of its players (`A & B`), as the client's `teamName`.
pub fn team_name(g: &Game, t: TeamId) -> String {
    let names: Vec<String> = g.player_ids().filter(|&p| g.players[p].team == t).map(|p| crate::view::player_name(g, p)).collect();
    if names.is_empty() { "that team".to_string() } else { names.join(" & ") }
}

/// A unit's type name.
fn unit_label(g: &Game, u: UnitId) -> Cow<'static, str> {
    display(g.d, g.units[u].unit_type)
}

// ---------------------------------------------------------------------------------- civics

/// One check of `canDoCivics` that civic adoption fails, as the civics screen lists it (no full
/// stop): `Requires Code of Laws`, `Only while at war`.
pub fn civic_refusal_text(g: &Game, r: CivicRefusal) -> String {
    let d = g.d;
    match r {
        CivicRefusal::Forced(o) => match forced_civic(g, o) {
            Some(c) => format!("A resolution of the council forces {}", display(d, c)),
            None => "A resolution of the council forces another civic".to_string(),
        },
        CivicRefusal::Tech(t) => format!("Requires {}", display(d, t)),
        CivicRefusal::War => "Only while at war".to_string(),
        CivicRefusal::BlockedAlignment(a) => format!("Not for {} civilizations", display(d, a)),
        CivicRefusal::Alignment(a) => format!("Only for {} civilizations", display(d, a)),
        CivicRefusal::Civilization(c) => format!("Only for the {}", display(d, c)),
        CivicRefusal::Religion(r) => format!("Requires {} as state religion", display(d, r)),
        CivicRefusal::Script => "Refused by the rules' civic script".to_string(),
    }
}

/// The civic a resolution forces in civic option `o` (the first forced one).
fn forced_civic(g: &Game, o: CivicOptionType) -> Option<CivicType> {
    g.d.civics.keys().find(|&c| g.d.civics[c].civic_option_type == Some(o) && g.is_force_civic(c))
}

/// Why civics cannot change now, as a sentence: `Anarchy: 2 turns left.`
pub fn revolution_refusal_text(g: &Game, r: RevolutionRefusal) -> String {
    let d = g.d;
    match r {
        RevolutionRefusal::Anarchy(n) => format!("Anarchy: {} left.", turns(n)),
        RevolutionRefusal::Timer(n) => format!("Revolution timer: {}.", turns(n)),
        RevolutionRefusal::Forced(o) => match forced_civic(g, o) {
            Some(c) => format!("A resolution of the council forces {} ({}).", display(d, c), display(d, o)),
            None => format!("A resolution of the council forces the {} civic.", display(d, o)),
        },
        RevolutionRefusal::NoChange => "No change.".to_string(),
    }
}

/// Why player `p` cannot switch to `civics` (merged over its current ones, as the
/// `update_civics` command merges them), in the command's order: a civic twice for one option, a
/// civic it cannot adopt (its first failing check), then `canRevolution` of the set. `None` when
/// the change can be made.
pub fn civics_change_refusal(g: &Game, p: PlayerId, civics: &[CivicType]) -> Option<String> {
    let d = g.d;
    let mut set = g.players[p].civics.clone();
    let mut seen: Vec<CivicOptionType> = Vec::new();
    for &c in civics {
        let Some(o) = d.civics[c].civic_option_type else {
            return Some(format!("{} belongs to no civic option.", display(d, c)));
        };
        if seen.contains(&o) {
            return Some(format!("Two civics for {}.", display(d, o)));
        }
        seen.push(o);
        if set[o] != Some(c) {
            if let Some(r) = g.player_civic_refusal(p, c) {
                return Some(format!("{}: {}.", display(d, c), civic_refusal_text(g, r)));
            }
        }
        set[o] = Some(c);
    }
    g.player_revolution_refusal(p, Some(&set)).map(|r| revolution_refusal_text(g, r))
}

/// Why the state religion cannot change, as a sentence.
pub fn convert_refusal_text(g: &Game, r: ConvertRefusal) -> String {
    let d = g.d;
    match r {
        ConvertRefusal::Favorite(r) => format!("{} is the leader's favorite religion.", display(d, r)),
        ConvertRefusal::Disliked(r) => format!("The leader does not adopt {}.", display(d, r)),
        ConvertRefusal::Barbarian => "The barbarians have no state religion.".to_string(),
        ConvertRefusal::Anarchy(n) => format!("Anarchy: {} left.", turns(n)),
        ConvertRefusal::Timer(n) => format!("Conversion timer: {}.", turns(n)),
        ConvertRefusal::NoStateReligion => "The civics allow no state religion.".to_string(),
        ConvertRefusal::Already(Some(r)) => format!("{} is the state religion already.", display(d, r)),
        ConvertRefusal::Already(None) => "There is no state religion.".to_string(),
        ConvertRefusal::NotPresent(r) => format!("No city of the team has {}.", display(d, r)),
        ConvertRefusal::Agnostic => "An agnostic leader adopts no religion.".to_string(),
    }
}

/// Why team `us` cannot declare war on `other`, as a sentence.
pub fn war_refusal_text(g: &Game, us: TeamId, other: TeamId, r: WarRefusal) -> String {
    match r {
        WarRefusal::Own => "That is your own team.".to_string(),
        WarRefusal::CrusadeHost => "A crusade's Host is at war with its crusade's enemies only.".to_string(),
        WarRefusal::Dead => format!("{} is not alive.", team_name(g, other)),
        WarRefusal::AtWar => format!("At war with {} already.", team_name(g, other)),
        WarRefusal::NotMet => format!("You have not met {}.", team_name(g, other)),
        WarRefusal::Peace(t) => format!("A peace treaty with {} holds.", team_name(g, t)),
        WarRefusal::Truce(t) => format!("A truce with {} holds until turn {}.", team_name(g, t), g.teams[us].truce_until[t]),
        WarRefusal::NoChangingWarPeace => "War and peace do not change in this game (No Changing War or Peace).".to_string(),
        WarRefusal::Permanent(t) => format!("Permanent war or peace with {}.", team_name(g, t)),
        WarRefusal::WeAreVassal => "A vassal's wars are its master's.".to_string(),
        WarRefusal::OurVassal => format!("{} is your vassal.", team_name(g, other)),
        WarRefusal::TheyAreVassal => format!("{} is a vassal: war is declared on its master.", team_name(g, other)),
        WarRefusal::AlwaysPeace => "Always Peace is on in this game.".to_string(),
        WarRefusal::Script => "Refused by the rules' war script.".to_string(),
    }
}

/// Why a goody hut outcome cannot be received, lower case without a full stop (it is shown in
/// parentheses: `(1/20, requires a city)`).
pub fn goody_refusal_text(g: &Game, r: GoodyRefusal) -> String {
    match r {
        GoodyRefusal::NoUnit => "needs a unit".to_string(),
        GoodyRefusal::NoPromotion => "the unit can take no promotion".to_string(),
        GoodyRefusal::TooEarly(n) => format!("after {n} turns of play, now {}", g.elapsed_game_turns),
        GoodyRefusal::NotDamaged(pct) => format!("the unit must have lost {pct}% of its hit points"),
        GoodyRefusal::NoTech => "no tech left to learn".to_string(),
        GoodyRefusal::NoBadGoodies => "the unit avoids bad results".to_string(),
        GoodyRefusal::NoCivUnit => "your civilization has no such unit".to_string(),
        GoodyRefusal::MultiPlayer => "no attack unit in a multiplayer game".to_string(),
        GoodyRefusal::OneCityChallenge => "no settler in One City Challenge".to_string(),
        GoodyRefusal::NoBarbarians => "barbarians are off in this game".to_string(),
        GoodyRefusal::NoCity => "requires a city".to_string(),
        GoodyRefusal::NearOnlyCity(n) => format!("within {n} plots of your only city"),
    }
}

/// Why player `p` can never research `tech`, as a sentence.
pub fn ever_research_text(g: &Game, p: PlayerId, tech: TechType, r: EverResearchRefusal) -> String {
    let d = g.d;
    let t = display(d, tech);
    match r {
        EverResearchRefusal::Disabled => format!("{t} is disabled."),
        EverResearchRefusal::Civilization(c) => format!("{t}: not for the {}.", display(d, c)),
        EverResearchRefusal::StateReligion(r) => format!("{t} requires {} as state religion.", display(d, r)),
        // The checks of the rules' `cannotResearch` (ffh-core's `py_cannot_research`).
        EverResearchRefusal::Script => {
            let agnostic = d.ty::<TraitType>("TRAIT_AGNOSTIC").is_some_and(|tr| g.players[p].traits[tr]);
            if d.ty::<TechType>("TECH_SEAFARING") == Some(tech) {
                format!("{t}: for the Lanun only.")
            } else if agnostic {
                format!("{t}: an agnostic leader researches no religion tech.")
            } else {
                format!("{t}: its religion is off in this game.")
            }
        }
    }
}

/// Why an event option cannot be chosen, as a sentence.
pub fn event_refusal_text(g: &Game, p: PlayerId, r: EventRefusal) -> String {
    let d = g.d;
    match r {
        EventRefusal::TheirGold(n) => format!("The other player has less than {n} gold."),
        EventRefusal::Gold(n) => gold_line(g.players[p].gold, n),
        EventRefusal::NoSpaceRace => "No space race in this game.".to_string(),
        EventRefusal::Espionage => "There is no espionage.".to_string(),
        EventRefusal::NoTech => "No tech for it to advance.".to_string(),
        EventRefusal::Tech(t) => format!("Requires {}.", display(d, t)),
        EventRefusal::NoOtherPlayer => "It needs another player.".to_string(),
        EventRefusal::NoTrade => "No trade network with the other player.".to_string(),
        EventRefusal::TheyHaveBonus(b) => format!("The other player has {} already.", display(d, b)),
        EventRefusal::NoSpareBonus(b) => format!("Requires a second {} to give.", display(d, b)),
        EventRefusal::NoCivUnit => "Your civilization has no such unit.".to_string(),
        EventRefusal::City => "The city does not meet the option's conditions.".to_string(),
        EventRefusal::OtherCity => "The other player's city does not meet the option's conditions.".to_string(),
        EventRefusal::Plot => "The plot does not meet the option's conditions.".to_string(),
        EventRefusal::Unit => "The unit does not meet the option's conditions.".to_string(),
        EventRefusal::BonusKnown(b) => format!("{} is revealed already.", display(d, b)),
        EventRefusal::NoCityToConvert => "No city left for the religion to spread to.".to_string(),
        EventRefusal::SameTeam => "The other player is on your team.".to_string(),
        EventRefusal::HumanAttitude => "The other player is human.".to_string(),
        EventRefusal::NoWorstEnemy => "The other player has no worst enemy.".to_string(),
        EventRefusal::War => "War with the other player cannot be declared.".to_string(),
        EventRefusal::QuestDone => "The quest is done already.".to_string(),
        EventRefusal::Alignment(a) => format!("Only for {} civilizations.", display(d, a)),
        EventRefusal::Bonus(b) => format!("Requires {}.", display(d, b)),
        EventRefusal::Civilization(c) => format!("Only for the {}.", display(d, c)),
        EventRefusal::Corporation(c) => format!("The city must have {}.", display(d, c)),
        EventRefusal::Religion(r) => format!("The city must have {}.", display(d, r)),
        EventRefusal::StateReligion(r) => format!("Requires {} as state religion.", display(d, r)),
        EventRefusal::Script => "The option's own conditions are not met.".to_string(),
    }
}

// ---------------------------------------------------------------------------------- spells

/// Why unit `u` cannot cast `spell`, as a sentence.
pub fn spell_refusal_text(g: &Game, u: UnitId, spell: SpellType, r: SpellRefusal) -> String {
    let d = g.d;
    let unit = &g.units[u];
    let name = unit_label(g, u);
    let plot = g.unit_plot_opt(u);
    match r {
        SpellRefusal::Immobile(n) => format!("{name} is held in place: {}.", turns(n)),
        SpellRefusal::InBoats => "No casting in boats.".to_string(),
        SpellRefusal::BattleOnly => "Cast in battle only.".to_string(),
        SpellRefusal::Promotion(p) => format!("Requires the {} promotion.", display(d, p)),
        SpellRefusal::UnitClass(c) => format!("Only a {} casts it.", display(d, c)),
        SpellRefusal::Unit(t) => format!("Only a {} casts it.", display(d, t)),
        SpellRefusal::UnitCombat(c) => format!("Only a {} unit casts it.", display(d, c)),
        SpellRefusal::Building(b) => format!("Cast in a city with {}.", display(d, b)),
        SpellRefusal::BuildingOwned(c) => format!("Requires {}.", display(d, c)),
        SpellRefusal::Civilization(c) => format!("Only the {} cast it.", display(d, c)),
        SpellRefusal::Corporation(c) => format!("Cast in a city with {}.", display(d, c)),
        SpellRefusal::Improvement(i) => format!("Cast on {}.", display(d, i)),
        SpellRefusal::UnitReligion(r) => format!("The caster must follow {}.", display(d, r)),
        SpellRefusal::StateReligion(r) => format!("Requires {} as state religion.", display(d, r)),
        SpellRefusal::Tech(t) => format!("Requires {}.", display(d, t)),
        SpellRefusal::AlreadyConverted(t) => format!("{name} is a {} already.", display(d, t)),
        SpellRefusal::NoWorldSpells => "World spells are off in this game.".to_string(),
        SpellRefusal::CastOnce => "Cast already: once a game.".to_string(),
        SpellRefusal::Placed => "Placed from the world actions, not cast by a unit.".to_string(),
        SpellRefusal::RitualCharge(p) => format!("Requires a charge of {}.", display(d, p)),
        SpellRefusal::SlaveTrade => "Requires the slave trade.".to_string(),
        SpellRefusal::Equipment => "Equipment casts only its own spells.".to_string(),
        SpellRefusal::Level(n) => format!("Requires level {n}: {name} is level {}.", unit.level),
        SpellRefusal::NotAlive => "The caster must be alive.".to_string(),
        SpellRefusal::Duration => {
            if unit.duration != 0 {
                format!("Not for a unit with a duration: {name} has {}.", turns(unit.duration))
            } else {
                format!("{name} has no duration.")
            }
        }
        SpellRefusal::Disabled => "Spellcasting is disabled for your civilization.".to_string(),
        SpellRefusal::PromotionInStack(p) => {
            if unit.has_promotion[p] {
                format!("Not for a unit with {}.", display(d, p))
            } else {
                format!("Requires another unit of yours with {} on the plot.", display(d, p))
            }
        }
        SpellRefusal::UnitInStack(t) => {
            if unit.unit_type == t {
                format!("Not cast by a {}.", display(d, t))
            } else {
                format!("Requires a {} of yours on the plot.", display(d, t))
            }
        }
        SpellRefusal::Feature(f, None) => format!("Cast on {}.", display(d, f)),
        SpellRefusal::Feature(f, Some(f2)) => format!("Cast on {} or {}.", display(d, f), display(d, f2)),
        SpellRefusal::HasCast => format!("{name} has cast this turn."),
        SpellRefusal::Water => "Cast next to water.".to_string(),
        SpellRefusal::Borders => "Cast in your borders.".to_string(),
        SpellRefusal::City => "Cast in a city.".to_string(),
        SpellRefusal::Population(n) => format!("The city's population must be more than {n}."),
        SpellRefusal::Gold(n) => gold_line(g.players[unit.owner].gold, n),
        SpellRefusal::NotCast => format!("{name} has not cast this turn."),
        // `canCreateUnit`'s checks.
        SpellRefusal::CreateUnit => {
            if unit.duration > 0 && d.spells[spell].create_unit_type == Some(unit.unit_type) {
                "A summon does not summon its own kind.".to_string()
            } else if plot.is_some_and(|q| g.plot_is_visible_enemy_unit(q, unit.owner)) {
                "An enemy unit is in view on the plot.".to_string()
            } else {
                "The limit of permanent summons is reached.".to_string()
            }
        }
        SpellRefusal::CreateBuilding => format!("{} cannot be built in this city.", d.spells[spell].create_building_type.map_or(Cow::Borrowed("The building"), |b| display(d, b))),
        // The spell's own Python requirement, and a spell none of whose effects applies: the
        // gate says no more than that.
        SpellRefusal::Requirement => format!("{name} cannot cast {} here.", display(d, spell)),
        SpellRefusal::NoTarget => "No valid target.".to_string(),
        SpellRefusal::NoEffect => format!("Nothing here for {} to affect.", display(d, spell)),
    }
}

/// Why unit `u` cannot cast `spell` now, for the world actions and a targeted spell: the
/// command's checks ([`command_refusal`]: combat, a trade route, boats, the cast gate).
pub fn cast_refusal(g: &Game, u: UnitId, spell: SpellType) -> String {
    command_refusal(g, &[u], CommandType::COMMAND_CAST, spell.idx() as i32, -1)
}

// ---------------------------------------------------------------------------------- cities

/// Why city `c` cannot use hurry `hurry` now: the checks of `city_can_hurry` (ffh-core's
/// `CvCity::canHurry`) in its order; `None` when it can.
pub fn hurry_refusal(g: &Game, c: CityId, hurry: HurryType) -> Option<String> {
    let d = g.d;
    let city = &g.cities[c];
    let owner = city.owner;
    if !g.city_specialty_can_hurry(c, hurry) && !g.player_can_hurry(owner, hurry) && g.city_unit_rush_gold(c, hurry).is_none() {
        let civics: Vec<String> = d.civics.keys().filter(|&k| d.civics[k].hurry[hurry]).map(|k| display(d, k).into_owned()).collect();
        return Some(if civics.is_empty() { format!("No civic allows {}.", display(d, hurry)) } else { format!("Requires {}.", or_list(&civics)) });
    }
    if g.city_is_disorder(c) {
        return Some("The city is in disorder.".to_string());
    }
    if g.city_production(c) >= g.city_production_needed(c) {
        return Some("The build is complete.".to_string());
    }
    if !g.city_is_production_unit(c) && !g.city_is_production_building(c) && !g.city_is_production_project(c) {
        return Some("Only a unit, a building or a ritual is hurried.".to_string());
    }
    if let Some(why) = crate::view::rituals::hurry_reason(g, c, hurry) {
        return Some(why.to_string());
    }
    let gold = g.city_hurry_gold(c, hurry);
    if g.players[owner].gold < gold {
        return Some(gold_line(g.players[owner].gold, gold));
    }
    let pop = g.city_hurry_population(c, hurry);
    let max = g.city_max_hurry_population(c);
    if max < pop {
        return Some(format!("Population to spend: {pop}, at most {max} (half of {}).", city.population));
    }
    if city.outpost {
        return Some("An outpost hurries nothing.".to_string());
    }
    None
}

/// Why city `c` cannot draft now: the checks of `city_can_conscript` (ffh-core's
/// `CvCity::canConscript`) in its order; `None` when it can.
pub fn draft_refusal(g: &Game, c: CityId) -> Option<String> {
    let d = g.d;
    let city = &g.cities[c];
    let pl = &g.players[city.owner];
    if g.city_is_disorder(c) {
        return Some("The city is in disorder.".to_string());
    }
    if city.drafted {
        return Some("The city has drafted this turn.".to_string());
    }
    if !g.city_specialty_draft_ready(c) && pl.conscript_count >= pl.max_conscript {
        if pl.max_conscript <= 0 {
            let civics: Vec<String> = d.civics.keys().filter(|&k| d.civics[k].max_conscript > 0).map(|k| display(d, k).into_owned()).collect();
            return Some(if civics.is_empty() { "No civic allows drafting.".to_string() } else { format!("Requires {}.", or_list(&civics)) });
        }
        return Some(format!("Drafts this turn: {} of {}.", pl.conscript_count, pl.max_conscript));
    }
    let cost = g.city_conscript_population(c);
    if city.population <= cost {
        return Some(format!("The draft costs {cost} population; the city has {}.", city.population));
    }
    let min = g.city_conscript_min_city_population(c);
    if city.population < min {
        return Some(format!("Requires population {min}; the city has {}.", city.population));
    }
    let culture = g.plot_calculate_team_culture_percent(g.city_plot(c), g.city_team(c));
    if culture < d.defines.conscript_min_culture_percent {
        return Some(format!("Your culture in the city: {culture}% of the {}% the draft needs.", d.defines.conscript_min_culture_percent));
    }
    if g.city_conscript_unit(c).is_none() {
        return Some("No unit to draft.".to_string());
    }
    if city.outpost {
        return Some("An outpost drafts nothing.".to_string());
    }
    None
}

// ---------------------------------------------------------------------------------- units

/// Why a city cannot be founded, as a sentence.
pub fn found_refusal_text(g: &Game, u: UnitId, r: FoundRefusal) -> String {
    let d = g.d;
    match r {
        FoundRefusal::OffMap => "Off the map.".to_string(),
        FoundRefusal::OneCityChallenge => "One City Challenge: one city only.".to_string(),
        FoundRefusal::Impassable => "No city on an impassable plot.".to_string(),
        FoundRefusal::SeaIce => "No city on Sea Ice.".to_string(),
        FoundRefusal::Feature(f) => format!("No city on {}.", display(d, f)),
        FoundRefusal::Owned(p) => format!("The plot is in the borders of {}.", crate::view::player_name(g, p)),
        FoundRefusal::Terrain(t) => {
            let info = &d.terrains[t];
            match (info.found_coast, info.found_fresh_water) {
                (true, true) => format!("{}: a city needs the coast or fresh water.", display(d, t)),
                (true, false) => format!("{}: a city needs the coast.", display(d, t)),
                (false, true) => format!("{}: a city needs fresh water.", display(d, t)),
                (false, false) => format!("No city on {}.", display(d, t)),
            }
        }
        FoundRefusal::NearCity(n) => format!("A city within {n} plots on this landmass."),
        FoundRefusal::Permanent(i) => format!("No city on {}.", display(d, i)),
        FoundRefusal::Mana(b) => format!("No city on {}.", display(d, b)),
        FoundRefusal::Enemy => "An enemy unit is in view on the plot.".to_string(),
        FoundRefusal::NotFounder => format!("{} founds no city.", unit_label(g, u)),
        FoundRefusal::InBoats => "Not from boats.".to_string(),
        FoundRefusal::Disabled => "No city can be founded on this plot.".to_string(),
        FoundRefusal::HasCast => format!("{} has cast this turn.", unit_label(g, u)),
    }
}

/// Why unit `u` cannot build `build` at `plot`: the check of `unit_can_build` that refuses (its
/// own, `player_can_build`'s and `plot_can_build`'s, ffh-core's `BuildRefusal`); `None` when
/// the gate allows it.
fn build_refusal(g: &Game, u: UnitId, plot: PlotIdx, build: BuildType) -> Option<String> {
    g.unit_build_refusal(u, plot, build, false).map(|r| crate::view::reasons_orders::build_refusal_text(g, u, plot, build, r))
}

/// Why the selection `units` cannot start `mission` (with its data, at `plot`): what
/// `selection::can_start_mission` refused, as a sentence. The selection-wide checks first, then
/// the failing check of the mission's gate for the unit explained: the first one the gate shows
/// the mission to (`test_visible`), else the first not held back by boats. For the missions no
/// unit of this game can do (air, espionage), `{Unit} cannot {Mission} here.`
pub fn mission_refusal(g: &Game, units: &[UnitId], mission: MissionType, data1: i32, data2: i32, plot: PlotIdx) -> String {
    use crate::view::reasons_orders::mission_refusal_text;
    let d = g.d;
    if selection::is_busy(g, units) {
        return "A selected unit is in combat.".to_string();
    }
    let boats_ok = ffh_core::naval::mission_allowed_in_boats(mission);
    let free = |x: UnitId| boats_ok || !g.unit_in_boats_at(x, plot);
    if !units.iter().any(|&x| free(x)) {
        return "In boats a unit only moves, waits or sleeps.".to_string();
    }
    let u = units
        .iter()
        .copied()
        .filter(|&x| free(x))
        .find(|&x| selection::can_start_mission(g, &[x], mission, data1, data2, plot, true))
        .or_else(|| units.iter().copied().find(|&x| free(x)))
        .unwrap_or(units[0]);
    let name = unit_label(g, u);
    let typed = |n: usize| (data1 >= 0 && (data1 as usize) < n).then_some(data1 as usize);
    let target = (data1 >= 0).then_some(UnitId(data1 as u32));
    let text = |r| mission_refusal_text(g, u, plot, target, r);
    let why = match mission {
        MissionType::MISSION_MOVE_TO if g.plot_at(plot, data1, data2) => Some("The units are already there.".to_string()),
        MissionType::MISSION_ROUTE_TO if g.plot_at(plot, data1, data2) => Some("The units are already there, and no route can be built here.".to_string()),
        MissionType::MISSION_SLEEP => g.unit_sleep_refusal(u, plot).map(text),
        MissionType::MISSION_FORTIFY => g.unit_fortify_refusal(u, plot).map(text),
        MissionType::MISSION_HEAL => g.unit_heal_refusal(u, plot).map(text),
        MissionType::MISSION_SENTRY => g.unit_sentry_refusal(u, plot).map(text),
        MissionType::MISSION_BOMBARD => g.unit_bombard_refusal(u, plot).map(text),
        MissionType::MISSION_RANGE_ATTACK => g.unit_range_strike_at_refusal(u, plot, data1, data2).map(text),
        MissionType::MISSION_PLUNDER => g.unit_plunder_refusal(u, plot, false).map(text),
        MissionType::MISSION_PILLAGE => g.unit_pillage_reason(u, plot).map(text),
        MissionType::MISSION_FOUND if data1 == ffh_core::outposts::FOUND_OUTPOST => g.unit_found_outpost_refusal(u, plot, false).map(|s| s.to_string()),
        MissionType::MISSION_FOUND => g.unit_found_refusal(u, plot, false).map(|r| found_refusal_text(g, u, r)),
        MissionType::MISSION_SPREAD => g.unit_spread_refusal(u, plot, typed(d.religions.len()).map(ReligionType::from_usize), false).map(text),
        MissionType::MISSION_SPREAD_CORPORATION => g.unit_spread_corporation_refusal(u, plot, typed(d.corporations.len()).map(CorporationType::from_usize), false).map(text),
        MissionType::MISSION_JOIN => g.unit_join_refusal(u, plot, typed(d.specialists.len()).map(SpecialistType::from_usize)).map(text),
        MissionType::MISSION_CONSTRUCT => g.unit_construct_refusal(u, plot, typed(d.buildings.len()).map(BuildingType::from_usize), false).map(text),
        MissionType::MISSION_DISCOVER => g.unit_discover_refusal(u, plot).map(text),
        MissionType::MISSION_HURRY => g.unit_hurry_refusal(u, plot, false).map(text),
        MissionType::MISSION_TRADE => g.unit_trade_refusal(u, plot, false).map(text),
        MissionType::MISSION_GREAT_WORK => g.unit_great_work_refusal(u, plot).map(text),
        // (data1 other than -1 plays the animation only: never refused.)
        MissionType::MISSION_GOLDEN_AGE => g.unit_golden_age_refusal(u, plot, false).map(text),
        MissionType::MISSION_BUILD => typed(d.builds.len()).and_then(|b| build_refusal(g, u, plot, BuildType::from_usize(b))),
        MissionType::MISSION_LEAD => g.unit_lead_refusal(u, plot, target).map(text),
        _ => None,
    };
    why.unwrap_or_else(|| format!("{name} cannot {} here.", display(d, mission)))
}

/// Why the selection `units` cannot do `command` (with its data): what
/// `selection::can_do_command` refused, as a sentence. The selection-wide checks first, then the
/// failing check of the command's gate for the unit explained (the first one the gate shows the
/// command to, else the first). For the commands a client never sends (hot keys, automatic
/// spells), `{Unit} cannot {Command} now.`
pub fn command_refusal(g: &Game, units: &[UnitId], command: CommandType, data1: i32, data2: i32) -> String {
    use crate::view::reasons_orders as o;
    let d = g.d;
    if selection::is_busy(g, units) {
        return "A selected unit is in combat.".to_string();
    }
    if !selection::can_ever_do_command(g, units, command) {
        return match command {
            CommandType::COMMAND_LOAD => "No transport on this plot has room.".to_string(),
            _ => "No selected unit is loaded.".to_string(),
        };
    }
    let u = units.iter().copied().find(|&x| g.unit_can_do_command(x, command, data1, data2, true, false)).unwrap_or(units[0]);
    let name = unit_label(g, u);
    if g.unit_in_route(u) {
        return "A caravan of a trade route takes no orders.".to_string();
    }
    let boats_ok = matches!(
        command,
        CommandType::COMMAND_PROMOTION
            | CommandType::COMMAND_WAKE
            | CommandType::COMMAND_CANCEL
            | CommandType::COMMAND_CANCEL_ALL
            | CommandType::COMMAND_STOP_AUTOMATION
            | CommandType::COMMAND_DELETE
            | CommandType::COMMAND_LOAD
            | CommandType::COMMAND_LOAD_UNIT
            | CommandType::COMMAND_HOTKEY
    );
    if g.unit_is_in_boats(u) && !boats_ok {
        return "In boats a unit only moves, waits, loads or is promoted.".to_string();
    }
    let typed = |n: usize| (data1 >= 0 && (data1 as usize) < n).then_some(data1 as usize);
    let plot = g.unit_plot(u);
    let why = match command {
        CommandType::COMMAND_CAST => typed(d.spells.len()).map(SpellType::from_usize).and_then(|s| g.unit_cast_refusal(u, s, false).map(|r| spell_refusal_text(g, u, s, r))),
        CommandType::COMMAND_PROMOTION => {
            let leader = (data2 >= 0).then_some(UnitId(data2 as u32));
            g.unit_promote_refusal(u, typed(d.promotions.len()).map(PromotionType::from_usize), leader).map(|r| o::promote_refusal_text(g, u, r))
        }
        CommandType::COMMAND_UPGRADE => {
            let t = typed(d.units.len()).map(UnitType::from_usize);
            g.unit_upgrade_refusal(u, t, false).map(|r| match t {
                Some(t) => o::upgrade_refusal_text(g, u, t, r),
                None => "No unit type given.".to_string(),
            })
        }
        CommandType::COMMAND_AUTOMATE => g.unit_automate_refusal(u, typed(d.automates.len()).map(AutomateType::from_usize)).map(|r| o::automate_refusal_text(g, u, r)),
        CommandType::COMMAND_DELETE => g.unit_scrap_refusal(u).map(o::scrap_refusal_text),
        CommandType::COMMAND_GIFT => g.unit_gift_refusal(u, false, true).map(|r| o::gift_refusal_text(g, u, r)),
        CommandType::COMMAND_WAKE => Some(if g.unit_is_automated(u) { format!("{name} is automated.") } else { format!("{name} is not asleep or fortified.") }),
        CommandType::COMMAND_CANCEL | CommandType::COMMAND_CANCEL_ALL => Some(if g.unit_is_automated(u) { format!("{name} is automated.") } else { format!("{name} has no orders.") }),
        CommandType::COMMAND_STOP_AUTOMATION => Some(format!("{name} is not automated.")),
        // `canLoad`: some transport on the plot takes the unit; the first one with room says why
        // it does not.
        CommandType::COMMAND_LOAD => Some(
            g.map.plots[plot]
                .units
                .iter()
                .copied()
                .filter(|&t| t != u && !g.unit_is_full(t))
                .find_map(|t| g.unit_load_unit_refusal(u, t, plot))
                .map_or_else(|| "No transport on this plot has room.".to_string(), |r| o::load_refusal_text(g, u, r)),
        ),
        CommandType::COMMAND_LOAD_UNIT => {
            let t = (data1 >= 0 && (data1 as usize) < g.num_players() && data2 >= 0)
                .then_some(UnitId(data2 as u32))
                .filter(|&t| g.units.contains(t) && g.units[t].owner.idx() == data1 as usize);
            Some(match t {
                Some(t) => g.unit_load_unit_refusal(u, t, plot).map_or_else(|| "No transport on this plot has room.".to_string(), |r| o::load_refusal_text(g, u, r)),
                None => "No such transport.".to_string(),
            })
        }
        CommandType::COMMAND_UNLOAD => g.unit_unload_refusal(u).map(|r| o::unload_refusal_text(g, u, r)),
        CommandType::COMMAND_UNLOAD_ALL => (g.units[u].cargo == 0).then(|| format!("{name} carries no units.")),
        _ => None,
    };
    why.unwrap_or_else(|| format!("{name} cannot {} now.", display(d, command)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The client's `plainDetail` (assets/app/dom.js): a refusal's detail is shown only when it
    /// holds no JSON, code, request, engine type name (`SPELL_X`), decision id or field name.
    pub(crate) fn plain(t: &str) -> bool {
        if t.trim().is_empty() || t.contains(['{', '}', '`', '"', '<', '>']) || t.contains("/api") {
            return false;
        }
        let words: Vec<&str> = t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).collect();
        let type_name = |w: &str| {
            let b = w.as_bytes();
            b.first().is_some_and(|c| c.is_ascii_uppercase())
                && b.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_')
                && w.find('_').is_some_and(|i| i + 1 < w.len())
        };
        if words.iter().any(|w| matches!(*w, "GET" | "POST" | "HTTP" | "field") || type_name(w)) {
            return false;
        }
        !t.split("decision ").skip(1).any(|r| r.starts_with(|c: char| c.is_ascii_digit()))
    }

    use ffh_core::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    /// A land plot where `p` can found a city (any map has one), and the city founded there.
    fn city_of(g: &mut Game, p: PlayerId) -> CityId {
        let (x, y) = (0..g.map.plots.len())
            .map(|i| g.map.xy(PlotIdx(i as u32)))
            .find(|&(x, y)| g.player_can_found(p, x, y, false) && g.map.plot_xy(x, y).is_some_and(|q| g.map.plots[q].units.is_empty()))
            .expect("a site");
        g.player_init_city(p, x, y, true, true)
    }

    #[test]
    fn civics_say_the_check_that_refuses() {
        let mut g = game();
        let d = g.d;
        let p = g.player_ids().next().unwrap();
        // Every civic that cannot be adopted names at least one check.
        for c in d.civics.keys() {
            let lines = crate::view::player::civic_requires(&g, p, c);
            assert_eq!(lines.is_empty(), g.player_can_do_civics(p, Some(c)), "{}", d.civics[c].base.type_);
            for l in &lines {
                assert!(plain(l), "{l}");
            }
        }
        // A civic whose tech is unknown.
        let locked = d.civics.keys().find(|&c| matches!(g.player_civic_refusal(p, c), Some(CivicRefusal::Tech(_)))).expect("a civic behind a tech");
        let Some(CivicRefusal::Tech(t)) = g.player_civic_refusal(p, locked) else { unreachable!() };
        assert_eq!(civics_change_refusal(&g, p, &[locked]), Some(format!("{}: Requires {}.", display(d, locked), display(d, t))));
        // The current civics: no change.
        let current: Vec<CivicType> = g.players[p].civics.iter().flatten().copied().collect();
        assert_eq!(civics_change_refusal(&g, p, &current), Some("No change.".to_string()));
        // The revolution's own checks, in order.
        g.players[p].revolution_timer = 3;
        assert_eq!(g.player_revolution_refusal(p, None).map(|r| revolution_refusal_text(&g, r)), Some("Revolution timer: 3 turns.".to_string()));
        g.players[p].anarchy_turns = 2;
        assert_eq!(g.player_revolution_refusal(p, None).map(|r| revolution_refusal_text(&g, r)), Some("Anarchy: 2 turns left.".to_string()));
        g.players[p].human = true;
        assert_eq!(convert_refusal_text(&g, g.player_convert_refusal(p, None).unwrap()), "Anarchy: 2 turns left.");
        g.players[p].anarchy_turns = 0;
        g.players[p].conversion_timer = 4;
        assert_eq!(g.player_convert_refusal(p, None), Some(ConvertRefusal::Timer(4)));
        assert_eq!(convert_refusal_text(&g, ConvertRefusal::Timer(4)), "Conversion timer: 4 turns.");
    }

    #[test]
    fn war_and_goodies_name_their_check() {
        let g = game();
        let p = g.player_ids().next().unwrap();
        let t = g.players[p].team;
        assert_eq!(g.team_declare_war_refusal(t, t), Some(WarRefusal::Own));
        assert_eq!(war_refusal_text(&g, t, t, WarRefusal::Own), "That is your own team.");
        let other = g.player_ids().find(|&q| g.players[q].team != t && !g.player_is_barbarian(q)).unwrap();
        let ot = g.players[other].team;
        if !g.teams[t].has_met[ot] {
            let r = g.team_declare_war_refusal(t, ot).unwrap();
            assert_eq!(r, WarRefusal::NotMet);
            let s = war_refusal_text(&g, t, ot, r);
            assert!(s.starts_with("You have not met ") && plain(&s), "{s}");
        }
        // Without a unit, a goody that needs one says so.
        let plot = g.unit_plot(g.players[p].units[0]);
        for goody in g.d.goodies.keys() {
            let r = g.player_goody_refusal(p, plot, goody, None);
            assert_eq!(r.is_none(), g.player_can_receive_goody(p, plot, goody, None));
            if g.d.goodies[goody].experience > 0 {
                assert_eq!(r, Some(GoodyRefusal::NoUnit));
            }
            if let Some(r) = r {
                assert!(plain(&goody_refusal_text(&g, r)));
            }
        }
        assert_eq!(goody_refusal_text(&g, GoodyRefusal::NoCity), "requires a city");
        assert_eq!(goody_refusal_text(&g, GoodyRefusal::NotDamaged(25)), "the unit must have lost 25% of its hit points");
    }

    #[test]
    fn hurry_and_draft_name_the_gate_check() {
        let mut g = game();
        let d = g.d;
        let p = g.player_ids().next().unwrap();
        let c = city_of(&mut g, p);
        // The enumeration agrees with the gate in every state tried.
        let check = |g: &Game| {
            for h in d.hurries.keys() {
                let why = hurry_refusal(g, c, h);
                assert_eq!(why.is_none(), g.city_can_hurry(c, h, false), "{}: {why:?}", d.hurries[h].base.type_);
                if let Some(w) = why {
                    assert!(plain(&w), "{w}");
                }
            }
            let why = draft_refusal(g, c);
            assert_eq!(why.is_none(), g.city_can_conscript(c), "{why:?}");
            if let Some(w) = why {
                assert!(plain(&w), "{w}");
            }
        };
        check(&g);
        // Every hurry and the draft allowed by the civics, with no gold, then with gold and
        // population.
        for h in d.hurries.keys() {
            g.players[p].hurry_count[h] = 1;
        }
        g.players[p].max_conscript = 1;
        g.players[p].gold = 0;
        check(&g);
        let gold = d.hurries.keys().find(|&h| d.hurries[h].gold_per_production > 0).expect("a gold hurry");
        if !g.city_is_disorder(c)
            && g.city_production(c) < g.city_production_needed(c)
            && (g.city_is_production_unit(c) || g.city_is_production_building(c))
            && g.city_hurry_gold(c, gold) > 0
        {
            assert_eq!(hurry_refusal(&g, c, gold), Some(format!("Gold: 0 of {}.", g.city_hurry_gold(c, gold))));
        }
        g.players[p].gold = 100_000;
        g.city_set_population(c, 9);
        check(&g);
        g.cities[c].drafted = true;
        if !g.city_is_disorder(c) {
            assert_eq!(draft_refusal(&g, c), Some("The city has drafted this turn.".to_string()));
        }
        check(&g);
    }

    #[test]
    fn unit_orders_name_their_check() {
        let mut g = game();
        let d = g.d;
        let p = g.player_ids().next().unwrap();
        let c = city_of(&mut g, p);
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let settler = d.t::<UnitType>("UNIT_SETTLER");
        let u = g.player_init_unit(p, settler, x, y, None, None);
        let plot = g.unit_plot(u);
        let name = display(d, settler);
        // Founding where a city stands.
        let r = g.unit_found_refusal(u, plot, false).expect("a city stands there");
        let s = mission_refusal(&g, &[u], MissionType::MISSION_FOUND, -1, -1, plot);
        assert_eq!(s, found_refusal_text(&g, u, r));
        assert!(plain(&s), "{s}");
        // Healing an unhurt unit.
        assert!(!g.unit_is_hurt(u));
        assert_eq!(mission_refusal(&g, &[u], MissionType::MISSION_HEAL, -1, -1, plot), format!("{name} is not hurt."));
        assert_eq!(command_refusal(&g, &[u], CommandType::COMMAND_STOP_AUTOMATION, -1, -1), format!("{name} is not automated."));
        // A spell whose promotion the unit lacks.
        let spell = d.spells.keys().find(|&s| {
            let i = &d.spells[s];
            !i.battle_only && !i.battle_summon && i.promotion_prereq1.is_some_and(|q| !g.units[u].has_promotion[q])
        });
        if let Some(s) = spell {
            let q = d.spells[s].promotion_prereq1.unwrap();
            assert_eq!(g.unit_cast_refusal(u, s, false), Some(SpellRefusal::Promotion(q)));
            let t = command_refusal(&g, &[u], CommandType::COMMAND_CAST, s.idx() as i32, -1);
            assert_eq!(t, format!("Requires the {} promotion.", display(d, q)));
            assert_eq!(cast_refusal(&g, u, s), t);
        }
        // Every spell's refusal exists exactly when the gate refuses, and reads as a sentence.
        for s in d.spells.keys() {
            let r = g.unit_cast_refusal(u, s, false);
            assert_eq!(r.is_none(), g.unit_can_cast(u, s, false), "{}", d.spells[s].base.type_);
            if let Some(r) = r {
                let t = spell_refusal_text(&g, u, s, r);
                assert!(plain(&t), "{t}");
            }
        }
    }

    #[test]
    fn plain_mirrors_the_client() {
        assert!(plain("Gold: 40 of 120."));
        assert!(!plain("SPELL_RAISE_SKELETON is not possible"));
        assert!(!plain("field `units`: empty selection"));
        assert!(!plain("decision 4 must be answered"));
    }

    #[test]
    fn words() {
        assert_eq!(turns(1), "1 turn");
        assert_eq!(turns(3), "3 turns");
        assert_eq!(or_list(&["A".into()]), "A");
        assert_eq!(or_list(&["A".into(), "B".into(), "C".into()]), "A, B or C");
        assert_eq!(gold_line(40, 120), "Gold: 40 of 120.");
        assert_eq!(gold_line(-5, 10), "Gold: 0 of 10.");
    }
}
