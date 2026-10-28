//! Port of CvRandomEventInterface.py (part b): the trigger/event callbacks from
//! `canTriggerDustbowlCont` (Python line 3304) to the end of the module that the FFH2 XML names.
//! Each Python function becomes a private Rust `fn` (its doc comment names the Python function)
//! registered by its Python name in the `find_*` lookup of the XML slot it fills (see dispatch.rs).
//!
//! Most of that Python range is stock BTS content (modern-era events) that no FFH2 XML
//! references; it was ported and then removed (2026-10-28), since the rules could never run it.
//! The callbacks kept are `canTriggerDustbowlCont`, `canTriggerChampion(Unit)`/`applyChampion`,
//! `canTriggerAntelope`/`doAntelope2`, `canTriggerWhaleOfAThing`, `doAncientOlympics2`,
//! `canTriggerHeroicGesture`/`doHeroicGesture2`, `canTriggerGreatMediator`/`doGreatMediator2`,
//! `doAncientTexts2`, `canTriggerLiteracy`, `canTriggerEsteemedPlaywright`,
//! `canTriggerExperiencedCaptain`, `doGreatBeast3` and `canTriggerControversialPhilosopherCity`.
//! The two event callbacks the XML names but no Python defines (`applyHeresy2`,
//! `applyTowerShield`) are registered here as no-ops (see [`missing_python_callback`]).
//!
//! Porting notes:
//! - Trigger `PythonCanDo` functions get the triggered data being tested; the Python edits it
//!   through `player.getEventTriggered(kTriggeredData.iId)`, i.e. the same object, so the port
//!   writes `data`. Where the Python reads `kTriggeredData` (its copy) after such a write, the
//!   port reads the value before writing.
//! - Type names FFH2 does not define give -1 in the Python (`d.ty` -> `None`); comparisons with
//!   -1 are `Option` comparisons, and where the C++ would index an array with -1 the port answers
//!   0 / false / -1.
//! - `gc.getPlayer(p).getCity(id)` / `getUnit(id)` return a wrapper of NULL when the player has
//!   no such object (`isNone()`; getters answer -1/false, actions are no-ops); here they are
//!   `None` ([`player_city`], [`player_unit`]). C++ ids are per player, Rust ids are global, so
//!   "the player has it" is "it exists and the player owns it".
//! - A Python exception (`None.getX()`, `gc.getBuildingInfo(-1).getBuildingClassType()`, ...)
//!   ends the function and the C++ caller's `lResult` stays uninitialized; where the port can
//!   reach that case, can-do / expire checks answer false and actions stop there.
//! - UI-only Python (messages, popups, the wonder movie) is omitted. The human-only popups of
//!   `doHeroicGesture2` / `doGreatMediator2` (whose answers go to `heroicGesture2Callback` /
//!   `greatMediator2Callback`) are replaced by their AI branch, which equals the "yes" answer
//!   (for `doGreatMediator2` the popup's "yes" makes the other team call `makePeace`).

use crate::ffh::dispatch::*;
use crate::game::Game;
use crate::ids::*;
use crate::structs::EventTriggeredData;
use ffh_data::*;

// ---------------------------------------------------------------------------------------------
// Lookups
// ---------------------------------------------------------------------------------------------

/// Trigger PythonCanDo (`canTriggerX(argsList=[triggerData])`).
pub fn find_trigger_can_do(name: &str) -> Option<TriggerCanDoFn> {
    let f: TriggerCanDoFn = match name {
        "canTriggerDustbowlCont" => can_trigger_dustbowl_cont,
        "canTriggerChampion" => can_trigger_champion,
        "canTriggerAntelope" => can_trigger_antelope,
        "canTriggerWhaleOfAThing" => can_trigger_whale_of_a_thing,
        "canTriggerHeroicGesture" => can_trigger_heroic_gesture,
        "canTriggerGreatMediator" => can_trigger_great_mediator,
        "canTriggerLiteracy" => can_trigger_literacy,
        "canTriggerEsteemedPlaywright" => can_trigger_esteemed_playwright,
        "canTriggerExperiencedCaptain" => can_trigger_experienced_captain,
        _ => return None,
    };
    Some(f)
}

/// Trigger PythonCanDoCity (`canTriggerXCity(eTrigger, ePlayer, iCity)`).
pub fn find_trigger_city(name: &str) -> Option<TriggerCityFn> {
    let f: TriggerCityFn = match name {
        "canTriggerControversialPhilosopherCity" => can_trigger_controversial_philosopher_city,
        _ => return None,
    };
    Some(f)
}

/// Trigger PythonCanDoUnit (`canTriggerXUnit(eTrigger, ePlayer, iUnit)`).
pub fn find_trigger_unit(name: &str) -> Option<TriggerUnitFn> {
    let f: TriggerUnitFn = match name {
        "canTriggerChampionUnit" => can_trigger_champion_unit,
        _ => return None,
    };
    Some(f)
}

/// Trigger PythonCallback (`doX(triggerData, ePlayer)`). None in this part of the module.
pub fn find_trigger_callback(_name: &str) -> Option<TriggerCallbackFn> {
    None
}

/// Event PythonCanDo (`canApplyX(eEvent, triggerData)`). None in this part of the module.
pub fn find_event_can_do(_name: &str) -> Option<EventCanDoFn> {
    None
}

/// Event PythonCallback (`applyX(eEvent, triggerData)`).
pub fn find_event_callback(name: &str) -> Option<EventCallbackFn> {
    let f: EventCallbackFn = match name {
        "applyChampion" => apply_champion,
        "doAntelope2" => do_antelope2,
        "doAncientOlympics2" => do_ancient_olympics2,
        "doHeroicGesture2" => do_heroic_gesture2,
        "doGreatMediator2" => do_great_mediator2,
        "doAncientTexts2" => do_ancient_texts2,
        "doGreatBeast3" => do_great_beast3,
        // Named by the XML but defined nowhere in the FFH2 Python (see `missing_python_callback`).
        "applyHeresy2" => missing_python_callback,
        "applyTowerShield" => missing_python_callback,
        _ => return None,
    };
    Some(f)
}

/// Event PythonExpireCheck (`expireX(eEvent, triggerData)`). None in this part of the module.
pub fn find_event_expire(_name: &str) -> Option<EventExpireFn> {
    None
}

// ---------------------------------------------------------------------------------------------
// Helpers (Python idioms)
// ---------------------------------------------------------------------------------------------

/// `gc.getPlayer(p).getCity(iID)`: the city if `p` has it; `None` is the NULL `CyCity`.
fn player_city(g: &Game, p: PlayerId, id: Option<CityId>) -> Option<CityId> {
    let id = id?;
    match g.cities.get(id) {
        Some(city) if city.owner == p => Some(id),
        _ => None,
    }
}

/// `gc.getPlayer(p).getUnit(iID)`: the unit if `p` has it; `None` is the NULL `CyUnit`.
fn player_unit(g: &Game, p: PlayerId, id: Option<UnitId>) -> Option<UnitId> {
    let id = id?;
    match g.units.get(id) {
        Some(unit) if unit.owner == p => Some(id),
        _ => None,
    }
}

/// `city.isHasReligion(eReligion)`: false for the NULL city; for NO_RELIGION the C++ would
/// index its array at -1 (taken as false).
fn city_has_religion(g: &Game, city: Option<CityId>, religion: Option<ReligionType>) -> bool {
    match (city, religion) {
        (Some(c), Some(r)) => g.cities[c].has_religion[r],
        _ => false,
    }
}

/// `player.getBuildingClassCount(eClass)` (0 for a class FFH2 does not define).
fn building_class_count(g: &Game, p: PlayerId, class: Option<BuildingClassType>) -> i32 {
    class.map_or(0, |bc| g.players[p].building_class_count[bc])
}

/// `player.isCivic(eCivic)`. C++ compares the civic of every civic option with `eCivic`, so for
/// -1 (a civic FFH2 does not define) it is true when some civic option has no civic.
fn is_civic(g: &Game, p: PlayerId, civic: Option<CivicType>) -> bool {
    match civic {
        Some(c) => g.player_is_civic(p, c),
        None => g.players[p].civics.iter().any(|c| c.is_none()),
    }
}

/// `trigger.getPrereqEvent(0)` (`None` for a trigger without prerequisite events, where the C++
/// would read past the end of its vector).
fn prereq_event0(g: &Game, trigger: EventTriggerType) -> Option<EventType> {
    g.d.event_triggers[trigger].prereq_events.first().copied()
}

/// `kOrigTriggeredData = player.getEventOccured(trigger.getPrereqEvent(0))` (a copy; `None` is
/// Python's `None`).
fn orig_triggered_data(g: &Game, p: PlayerId, trigger: EventTriggerType) -> Option<EventTriggeredData> {
    let event = prereq_event0(g, trigger)?;
    g.player_get_event_occured(p, event).cloned()
}

/// The "every civ that has met us likes us more" loop of `applySoloFlight`,
/// `applyModernOlympics`, `doAncientTexts2` and `applyBestDefenseDone2`:
/// `loopPlayer.AI_changeAttitudeExtra(ePlayer, change)` for every other living civ player whose
/// team has met `p`'s team.
fn change_attitude_of_contacts(g: &mut Game, p: PlayerId, change: i32) {
    let team = g.players[p].team;
    for other in g.civ_player_ids() {
        if g.players[other].alive && other != p {
            let other_team = g.players[other].team;
            if g.teams[other_team].has_met[team] {
                g.players[other].ai_attitude_extra[p] += change;
            }
        }
    }
}

/// `plot = gc.getMap().plot(iPlotX, iPlotY); if not plot.isNone(): plot.setImprovementType(e)`.
fn set_trigger_plot_improvement(g: &mut Game, data: &EventTriggeredData, improvement: Option<ImprovementType>) {
    if let Some(plot) = g.map.plot_xy(data.plot_x, data.plot_y) {
        g.plot_set_improvement_type(plot, improvement);
    }
}

/// The body shared by `canTriggerAntelope`, `canTriggerWhaleOfAThing` and
/// `canTriggerHiyoSilver`: the player has at most five happiness bonuses and not `bonus`, and the
/// trigger plot can have `bonus`.
fn can_trigger_new_happy_bonus(g: &Game, data: &EventTriggeredData, bonus: Option<BonusType>) -> bool {
    let p = data.player;
    let mut happy_bonuses = 0;
    for b in g.d.bonuses.keys() {
        let num = g.player_get_num_available_bonuses(p, b);
        if num > 0 {
            if g.d.bonuses[b].happiness > 0 {
                happy_bonuses += 1;
                if happy_bonuses > 5 {
                    return false;
                }
            }
            if Some(b) == bonus {
                return false;
            }
        }
    }

    // NULL plot: canHaveBonus() is false.
    match g.map.plot_xy(data.plot_x, data.plot_y) {
        Some(plot) => g.plot_can_have_bonus(plot, bonus, false),
        None => false,
    }
}

// ---------------------------------------------------------------------------------------------
// DUSTBOWL
// ---------------------------------------------------------------------------------------------

/// `canTriggerDustbowlCont(argsList)`: moves the trigger to the player's plains farm nearest to
/// the first dust bowl; without one, forgets the first event and fails.
fn can_trigger_dustbowl_cont(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let p = data.player;
    let prereq_event = prereq_event0(g, data.trigger);

    let Some(orig) = orig_triggered_data(g, p, data.trigger) else {
        return false;
    };

    let farm_type = g.d.ty::<ImprovementType>("IMPROVEMENT_FARM");
    let plains_type = g.d.ty::<TerrainType>("TERRAIN_PLAINS");

    let mut best_value = g.map.grid_width + g.map.grid_height;
    let mut best_plot = None;
    for plot in g.map.plot_ids() {
        let pl = &g.map.plots[plot];
        if pl.owner == Some(p) && pl.improvement_type == farm_type && pl.terrain_type == plains_type {
            let value = g.plot_distance(orig.plot_x, orig.plot_y, pl.x, pl.y);
            if value < best_value {
                best_value = value;
                best_plot = Some(plot);
            }
        }
    }

    match best_plot {
        Some(plot) => {
            let (x, y) = g.map.xy(plot);
            data.plot_x = x;
            data.plot_y = y;
        }
        None => {
            // (orig is Some, so the prerequisite event exists)
            if let Some(event) = prereq_event {
                g.player_reset_event_occured(p, event, true);
            }
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------------------------
// SALTPETER
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// GREAT DEPRESSION
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// CHAMPION
// ---------------------------------------------------------------------------------------------

/// `canTriggerChampion(argsList)`: only at peace.
fn can_trigger_champion(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let team = g.players[data.player].team;
    if g.team_get_at_war_count(team, true) > 0 {
        return false;
    }
    true
}

/// `canTriggerChampionUnit(argsList)`: an undamaged unit with 3+ experience that is not a hero.
fn can_trigger_champion_unit(g: &Game, _trigger: EventTriggerType, p: PlayerId, u: UnitId) -> bool {
    let Some(u) = player_unit(g, p, Some(u)) else {
        return false;
    };
    let unit = &g.units[u];

    if unit.damage > 0 {
        return false;
    }

    if unit.experience < 3 {
        return false;
    }

    // FfH: PROMOTION_HERO replaces BTS PROMOTION_LEADERSHIP.
    let leadership = g.d.t::<PromotionType>("PROMOTION_HERO");
    if unit.has_promotion[leadership] {
        return false;
    }

    true
}

/// `applyChampion(argsList)`: the unit becomes a hero.
fn apply_champion(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    let unit = player_unit(g, data.player, data.unit);
    let leadership = g.d.t::<PromotionType>("PROMOTION_HERO");
    // NULL unit: setHasPromotion is a no-op.
    if let Some(u) = unit {
        g.unit_set_has_promotion(u, leadership, true);
    }
}

// ---------------------------------------------------------------------------------------------
// ELECTRIC COMPANY / GOLD RUSH / INFLUENZA
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// SOLO FLIGHT
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// ANTELOPE / WHALE OF A THING / HIYO SILVER / WINING MONKS / INDEPENDENT FILMS
// ---------------------------------------------------------------------------------------------

/// `canTriggerAntelope(argsList)`.
fn can_trigger_antelope(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let deer = g.d.ty::<BonusType>("BONUS_DEER");
    can_trigger_new_happy_bonus(g, data, deer)
}

/// `doAntelope2(argsList)`: a camp on the plot ("camps are not normally allowed unless there is
/// already deer").
fn do_antelope2(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    let camp = g.d.ty::<ImprovementType>("IMPROVEMENT_CAMP");
    set_trigger_plot_improvement(g, data, camp);
}

/// `canTriggerWhaleOfAThing(argsList)`.
fn can_trigger_whale_of_a_thing(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let whale = g.d.ty::<BonusType>("BONUS_WHALE");
    can_trigger_new_happy_bonus(g, data, whale)
}

// ---------------------------------------------------------------------------------------------
// ANCIENT OLYMPICS / MODERN OLYMPICS / INTERSTATE
// ---------------------------------------------------------------------------------------------

/// `doAncientOlympics2(argsList)`: every other living major civ with land next to ours gets a
/// "good event" memory of us.
fn do_ancient_olympics2(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    let p = data.player;
    for j in g.civ_player_ids() {
        if j != p && g.players[j].alive && !g.player_is_minor_civ(j) {
            for plot in g.map.plot_ids() {
                if !g.plot_is_water(plot) && g.map.plots[plot].owner == Some(p) && g.plot_is_adjacent_player(plot, j, true) {
                    g.players[j].ai_memory_count[p][MemoryType::MEMORY_EVENT_GOOD_TO_US] += 1;
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// EARTH DAY / FREEDOM CONCERT
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// HEROIC GESTURE / GREAT MEDIATOR / ANCIENT TEXTS
// ---------------------------------------------------------------------------------------------

/// `canTriggerHeroicGesture(argsList)`: at war with the other player, both sides having had
/// success. (No other player: `gc.getPlayer(-1)` is None and the Python raises.)
fn can_trigger_heroic_gesture(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let Some(dest_player) = data.other_player else {
        return false;
    };
    let team = g.players[data.player].team;
    let dest_team = g.players[dest_player].team;

    if !g.team_can_change_war_peace(dest_team, team, false) {
        return false;
    }

    if g.teams[dest_team].ai_war_success[team] <= 0 {
        return false;
    }

    if g.teams[team].ai_war_success[dest_team] <= 0 {
        return false;
    }

    true
}

/// `doHeroicGesture2(argsList)`: peace with the other player and +1 attitude both ways. A human
/// other player is asked by popup in the Python (`heroicGesture2Callback`, whose "yes" does the
/// same); here everyone takes the AI branch.
fn do_heroic_gesture2(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    let p = data.player;
    let Some(dest_player) = data.other_player else { return };

    g.player_force_peace(dest_player, p);
    g.players[dest_player].ai_attitude_extra[p] += 1;
    g.players[p].ai_attitude_extra[dest_player] += 1;
}

/// `canTriggerGreatMediator(argsList)`: at war with the other player for 10+ turns.
fn can_trigger_great_mediator(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let Some(dest_player) = data.other_player else {
        return false;
    };
    let team = g.players[data.player].team;
    let dest_team = g.players[dest_player].team;

    if !g.team_can_change_war_peace(team, dest_team, false) {
        return false;
    }

    if g.teams[team].ai_at_war_counter[dest_team] < 10 {
        return false;
    }

    true
}

/// `doGreatMediator2(argsList)`: peace with the other player and +1 attitude both ways (AI
/// branch for everyone; the Python asks a human other player by popup, `greatMediator2Callback`).
fn do_great_mediator2(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    let p = data.player;
    let Some(dest_player) = data.other_player else { return };

    let team = g.players[p].team;
    let dest_team = g.players[dest_player].team;
    g.team_make_peace(team, dest_team, true);
    g.players[dest_player].ai_attitude_extra[p] += 1;
    g.players[p].ai_attitude_extra[dest_player] += 1;
}

/// `doAncientTexts2(argsList)`: +1 attitude from every civ that has met us.
fn do_ancient_texts2(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    change_attitude_of_contacts(g, data.player, 1);
}

// ---------------------------------------------------------------------------------------------
// IMPACT CRATER
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// BARBARIAN INVASIONS (THE HUNS, THE VANDALS, THE GOTHS, THE PHILISTINES, THE VEDIC ARYANS)
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// SECURITY TAX / LITERACY / TEA
// ---------------------------------------------------------------------------------------------

/// `canTriggerLiteracy(argsList)`: a library in every city.
fn can_trigger_literacy(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let p = data.player;
    let library = g.d.ty::<BuildingClassType>("BUILDINGCLASS_LIBRARY");
    if g.players[p].num_cities() > building_class_count(g, p, library) {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------------------------
// HORSE WHISPERING / HARBORMASTER / CLASSIC LITERATURE (quests)
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// MASTER BLACKSMITH
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// THE BEST DEFENSE / NATIONAL SPORTS LEAGUE
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// CRUSADE
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// ESTEEMED PLAYWRIGHT / SECRET KNOWLEDGE / HIGH WARLORD / EXPERIENCED CAPTAIN
// ---------------------------------------------------------------------------------------------

/// `canTriggerEsteemedPlaywright(argsList)`: not under Slavery.
fn can_trigger_esteemed_playwright(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    // If source civ is operating this Civic, disallow the event to trigger.
    if is_civic(g, data.player, g.d.ty::<CivicType>("CIVIC_SLAVERY")) {
        return false;
    }
    true
}

/// `canTriggerExperiencedCaptain(argsList)`: the unit has 7+ experience.
fn can_trigger_experienced_captain(g: &mut Game, data: &mut EventTriggeredData) -> bool {
    let Some(u) = player_unit(g, data.player, data.unit) else {
        return false;
    };

    if g.units[u].experience < 7 {
        return false;
    }

    true
}

// ---------------------------------------------------------------------------------------------
// PARTISANS
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// GREED
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// WAR CHARIOTS / ELITE SWORDSMEN / WARSHIPS / GUNS NOT BUTTER / NOBLE KNIGHTS / OVERWHELM
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// CORPORATE EXPANSION / HOSTILE TAKEOVER
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// GREAT BEAST / COMET FRAGMENT / IMMIGRANTS / CONTROVERSIAL PHILOSOPHER
// ---------------------------------------------------------------------------------------------

/// `doGreatBeast3(argsList)`: 40 turns of extra happiness in our cities with the religion.
fn do_great_beast3(g: &mut Game, _event: EventType, data: &EventTriggeredData) {
    let p = data.player;
    for c in g.players[p].cities.clone() {
        if city_has_religion(g, Some(c), data.religion) {
            g.city_change_happiness_timer(c, 40);
        }
    }
}

/// `canTriggerControversialPhilosopherCity(argsList)`: the capital, with 35+ research.
fn can_trigger_controversial_philosopher_city(g: &Game, _trigger: EventTriggerType, p: PlayerId, c: CityId) -> bool {
    let Some(city) = player_city(g, p, Some(c)) else {
        return false;
    };
    if !g.city_is_capital(city) {
        return false;
    }
    if g.city_commerce_rate_times100(city, CommerceType::COMMERCE_RESEARCH) < 3500 {
        return false;
    }

    true
}

// ---------------------------------------------------------------------------------------------
// SPY DISCOVERED / NUCLEAR PROTEST
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// BTS civilization-specific events (none of these civilizations exists in FFH2)
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// XML callback names with no Python definition
// ---------------------------------------------------------------------------------------------

/// `applyHeresy2` (EVENT_HERESY_2) and `applyTowerShield` (EVENT_TOWER_SHIELD_1): the XML names
/// these event callbacks, but CvRandomEventInterface.py defines neither. The DLL's
/// `callFunction` then fails (Python AttributeError) and `CvPlayer::applyEvent` ignores the
/// result, so the callback does nothing.
fn missing_python_callback(_g: &mut Game, _event: EventType, _data: &EventTriggeredData) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The callbacks of this part that the FFH2 XML names, each in its slot (the rest of the
    /// Python module's range is stock BTS content no XML references, so it is not ported).
    #[test]
    fn referenced_callbacks_are_registered() {
        for name in ["canTriggerDustbowlCont", "canTriggerChampion", "canTriggerAntelope", "canTriggerWhaleOfAThing", "canTriggerHeroicGesture", "canTriggerGreatMediator", "canTriggerLiteracy", "canTriggerEsteemedPlaywright", "canTriggerExperiencedCaptain"] {
            assert!(find_trigger_can_do(name).is_some(), "{name}");
        }
        for name in ["canTriggerControversialPhilosopherCity"] {
            assert!(find_trigger_city(name).is_some(), "{name}");
        }
        for name in ["canTriggerChampionUnit"] {
            assert!(find_trigger_unit(name).is_some(), "{name}");
        }
        for name in ["applyChampion", "doAntelope2", "doAncientOlympics2", "doHeroicGesture2", "doGreatMediator2", "doAncientTexts2", "doGreatBeast3", "applyHeresy2", "applyTowerShield"] {
            assert!(find_event_callback(name).is_some(), "{name}");
        }
        // Callbacks never cross slots.
        assert!(find_event_callback("canTriggerDustbowlCont").is_none());
        assert!(find_trigger_can_do("applyChampion").is_none());
        assert!(find_trigger_callback("doGreatBeast3").is_none());
    }
}
