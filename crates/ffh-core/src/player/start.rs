//! CvPlayer: game start -- initial gold/research state, free starting units, starting
//! area/plot selection.
//!
//! Also holds the `CvPlayerAI` valuations these rules depend on (marked `// AI:`; they live
//! in the AI class in C++, but their results decide rules outcomes):
//! - [`ai_found_value`] (`AI_foundValue`, with `AI_bonusVal`, `AI_baseBonusVal`,
//!   `AI_corporationBonusVal` and `AI_countDeadlockedBonuses`): the city-site value behind
//!   `CvPlot::getFoundValue`, which `findStartingPlot` maximizes;
//! - the starting-location reset of `AI_updateFoundValues(true)`;
//! - the part of `AI_unitValue` (with `AI_combatValue`, `AI_trueCombatValue`,
//!   `AI_unitImpassableCount`, `AI_unitAIDomainType`) that picks the free starting units.
//!
//! They are free functions (not `Game` methods) so they cannot clash with helpers of the
//! same C++ functions elsewhere.


use std::cmp::{max, min};

use crate::defs::*;
use crate::game::Game;
use crate::ids::*;
use crate::utils::is_world_unit_class;
use ffh_data::*;


/// C++ `GC.getCivilizationInfo(getCivilizationType())`: C++ indexes the info unchecked, so a
/// player without a civilization is an error.
fn civ_of(g: &Game, p: PlayerId) -> CivilizationType {
    g.player_get_civilization_type(p).expect("player has no civilization")
}

/// C++ `GC.getHandicapInfo(getHandicapType())` (indexed unchecked in C++).
fn handicap_of(g: &Game, p: PlayerId) -> HandicapType {
    g.player_get_handicap_type(p).expect("player has no handicap")
}

/// C++ `pPlot->area()` (never NULL once the map's areas are calculated).
fn area_of(g: &Game, plot: PlotIdx) -> AreaId {
    g.map.plots[plot].area.expect("plot has no area")
}

impl Game {
    // ------------------------------------------------------------------ initial state and units

    /// C++ `CvPlayer::initFreeState`: starting gold (handicap + FFH trait starting gold) and an
    /// empty research queue.
    pub fn player_init_free_state(&mut self, p: PlayerId) {
        let d = self.d;
        let handicap = handicap_of(self, p);
        // setGold/changeGold only have UI side effects.
        self.players[p].gold = 0;
        self.players[p].gold += d.handicaps[handicap].starting_gold;

        // FfH Traits: Added by Kael 08/02/2007
        let starting_gold = self.players[p].starting_gold;
        self.players[p].gold += starting_gold;
        // FfH: End Add

        self.player_clear_research_queue(p);
    }

    /// C++ `CvPlayer::initFreeUnits` (without its advanced-start branch: there is none): the
    /// civilization's free units plus the handicap's defense and explore units.
    pub fn player_init_free_units(&mut self, p: PlayerId) {
        let d = self.d;

        let civ = civ_of(self, p);
        let game_handicap = self.handicap;
        let handicap = handicap_of(self, p);

        for class in d.unit_classes.keys() {
            if let Some(unit) = d.civ_unit(civ, class) {
                let mut free_count = d.civilizations[civ].civilization_free_units_class[class];

                let ai_multiplier = if !self.players[p].human { d.handicaps[game_handicap].ai_starting_unit_multiplier } else { 0 };
                free_count *= 1 + ai_multiplier;

                for _ in 0..free_count {
                    self.player_add_free_unit(p, unit, None);
                }
            }
        }

        let mut free_count = d.handicaps[handicap].starting_defense_units;

        if !self.players[p].human {
            free_count += d.handicaps[game_handicap].ai_starting_defense_units;
        }

        if free_count > 0 {
            self.player_add_free_unit_ai(p, UnitAIType::UNITAI_CITY_DEFENSE, free_count);
        }

        // (No free workers, C++ `getStartingWorkerUnits`: there are none, docs/WORKS.md.)

        let mut free_count = d.handicaps[handicap].starting_explore_units;

        if !self.players[p].human {
            free_count += d.handicaps[game_handicap].ai_starting_explore_units;
        }

        if free_count > 0 {
            self.player_add_free_unit_ai(p, UnitAIType::UNITAI_EXPLORE, free_count);
        }
    }

    /// C++ `CvPlayer::addFreeUnitAI`: adds `count` units of the best trainable unit for
    /// `unit_ai` (best by `AI_unitValue`; see [`ai_unit_value`]).
    pub fn player_add_free_unit_ai(&mut self, p: PlayerId, unit_ai: UnitAIType, count: i32) {
        let d = self.d;
        let civ = civ_of(self, p);

        let mut best_unit = None;
        let mut best_value = 0;

        for class in d.unit_classes.keys() {
            let Some(unit) = d.civ_unit(civ, class) else {
                continue;
            };

            // FfH: Added by Kael 07/30/2009 (no world units as free units)
            if self.player_can_train(p, unit, false, false, false) && !is_world_unit_class(d, class) {
                let info = &d.units[unit];
                let mut valid = true;

                if info.prereq_and_bonus.is_some() {
                    valid = false;
                }

                // C++ scans all NUM_UNIT_PREREQ_OR_BONUSES slots for one != NO_BONUS; the data
                // keeps only the listed bonuses.
                if !info.prereq_or_bonuses.is_empty() {
                    valid = false;
                }

                if valid {
                    // AI: AI_unitValue(eLoopUnit, eUnitAI, NULL)
                    let value = ai_unit_value(self, p, unit, unit_ai);

                    if value > best_value {
                        best_unit = Some(unit);
                        best_value = value;
                    }
                }
            }
        }

        if let Some(unit) = best_unit {
            for _ in 0..count {
                self.player_add_free_unit(p, unit, Some(unit_ai));
            }
        }
    }

    /// C++ `CvPlayer::addFreeUnit(eUnit, eUnitAI = NO_UNITAI)`: places a starting unit near
    /// the starting plot (FFH: settlers get STARTING_SETTLER_PROMOTION).
    pub fn player_add_free_unit(&mut self, p: PlayerId, unit: UnitType, unit_ai: Option<UnitAIType>) {
        let d = self.d;
        let info = &d.units[unit];

        if self.is_option(GameOptionType::GAMEOPTION_ONE_CITY_CHALLENGE) && self.players[p].human {
            if (unit_ai == Some(UnitAIType::UNITAI_SETTLE) || info.default_unit_ai_type == Some(UnitAIType::UNITAI_SETTLE))
                && self.players[p].ai_num_ai_units[UnitAIType::UNITAI_SETTLE] >= 1
            {
                return;
            }
        }

        let Some(starting_plot) = self.player_get_starting_plot(p) else {
            return;
        };

        let mut best_plot = None;

        if self.players[p].human {
            // Map script `startHumansOnSameTile` (lResult stays 0 unless it returns true).
            let ms = self.map_script.clone();
            if !ms.start_humans_on_same_tile(self).unwrap_or(false) && !info.found {
                // FfH: Modified by Kael 09/16/2008 (21 tries instead of NUM_CITY_PLOTS)
                let rand_offset = self.soren_rand_num(21, "Place Units (Player)");
                let (sx, sy) = self.map.xy(starting_plot);
                let starting_area = self.map.plots[starting_plot].area;
                for i in 0..21 {
                    // C++ quirk: the index still wraps at NUM_CITY_PLOTS (37), so offsets past
                    // the 21 inner plots look at the outer ring (and only 0..3 wrap around).
                    let index = ((i + rand_offset) as usize) % NUM_CITY_PLOTS;
                    let Some(loop_plot) = self.plot_city(sx, sy, index) else {
                        continue;
                    };
                    if self.map.plots[loop_plot].area == starting_area
                        && !self.plot_is_impassable(loop_plot)
                        && !self.plot_is_unit(loop_plot)
                        // FfH: Modified by Kael 09/16/2008 (was `!isGoody()`)
                        && self.map.plots[loop_plot].improvement_type.is_none()
                    {
                        best_plot = Some(loop_plot);
                        break;
                    }
                }
            }
        }

        let best_plot = best_plot.unwrap_or(starting_plot);
        let (x, y) = self.map.xy(best_plot);

        // FfH: Modified by Kael 08/13/2007
        let u = self.player_init_unit(p, unit, x, y, unit_ai, None);
        if info.default_unit_ai_type == Some(UnitAIType::UNITAI_SETTLE) {
            if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.starting_settler_promotion) {
                // C++ uses the new unit unconditionally; it only dies here if a creation hook
                // killed it.
                if self.units.contains(u) {
                    self.unit_set_has_promotion(u, promotion, true);
                }
            }
        }
        // FfH: End Add
    }

    // ------------------------------------------------------------------ starting plot

    /// C++ `CvPlayer::startingPlotRange`.
    pub fn player_starting_plot_range(&self, _p: PlayerId) -> i32 {
        let d = self.d;
        let civs_alive = self.count_civ_players_alive();

        let mut range = self.map.max_step_distance() + 10;

        range *= d.defines.starting_distance_percent;
        range /= 100;

        range *= self.map.land_plots / (d.worlds[self.setup.world_size].target_num_cities * civs_alive);

        // FfH: Modified by Kael 11/18/2007 (21 instead of NUM_CITY_PLOTS)
        range /= 21;

        range += min((self.map.areas.len() as i32 + 1) / 2, civs_alive);

        // Map script `minStartingDistanceModifier` (the default implementation returns 0,
        // which leaves the range unchanged).
        let ms = self.map_script.clone();
        let modifier = ms.min_starting_distance_modifier(self).unwrap_or(0);
        range *= max(0, modifier + 100);
        range /= 100;

        max(range, d.defines.min_civ_starting_distance)
    }

    /// C++ `CvPlayer::startingPlotDistanceFactor` (called on the player whose starting plot is
    /// measured, `p`, for the evaluating player `other`).
    pub fn player_starting_plot_distance_factor(&self, p: PlayerId, plot: PlotIdx, other: PlayerId, range: i32) -> i32 {
        debug_assert!(other != p);
        let d = self.d;
        let mut range = range;

        let mut value = 1000;

        if let Some(starting_plot) = self.player_get_starting_plot(p) {
            if self.is_team_game() {
                if self.players[other].team == self.players[p].team {
                    range *= d.defines.own_team_starting_modifier;
                    range /= 100;
                } else {
                    range *= d.defines.rival_team_starting_modifier;
                    range /= 100;
                }
            }

            let (x, y) = self.map.xy(plot);
            let (sx, sy) = self.map.xy(starting_plot);
            let mut distance = self.step_distance(x, y, sx, sy);
            if self.map.plots[starting_plot].area != self.map.plots[plot].area {
                distance *= 4;
                distance /= 3;
            }

            value *= distance;
            value /= range;
        }

        max(1, value)
    }

    /// C++ `CvPlayer::findStartingArea`: best land area id, None if it does not matter
    /// (C++ -1; the map script's `findStartingArea` override is consulted first).
    pub fn player_find_starting_area(&self, p: PlayerId) -> Option<AreaId> {
        let ms = self.map_script.clone();
        if let Some(result) = ms.find_starting_area(self, p) {
            // Python override
            if result == -1 {
                return None;
            }
            if result >= 0 && self.map.areas.contains(AreaId::from_usize(result as usize)) {
                return Some(AreaId::from_usize(result as usize));
            }
            // C++ FAssertMsg: "python findStartingArea() must return -1 or the ID of a valid
            // area"; falls through to the DLL search.
        }

        let mut best_value = 0;
        let mut best_area = None;

        // find best land area
        for (id, area) in self.map.areas.iter() {
            if !area.water {
                // iNumPlayersOnArea is the number of players starting on the area, plus this player
                let num_players_on_area = area.num_starting_plots + 1;
                let tile_value = (self.area_calculate_total_best_nature_yield(id)
                    + (self.area_count_coastal_land(id) * 2)
                    + area.num_river_edges
                    + area.num_tiles)
                    + 1;
                let mut value = tile_value / num_players_on_area;

                value *= min(NUM_CITY_PLOTS as i32 + 1, area.num_tiles + 1);
                value /= NUM_CITY_PLOTS as i32 + 1;

                if num_players_on_area <= 2 {
                    value *= 4;
                    value /= 3;
                }

                if value > best_value {
                    best_value = value;
                    best_area = Some(id);
                }
            }
        }

        best_area
    }

    /// C++ `CvPlayer::findStartingPlot(bRandomize = false)`: best plot by found value
    /// (C++ `AI_updateFoundValues(true)` + `CvPlot::getFoundValue`).
    pub fn player_find_starting_plot(&mut self, p: PlayerId, randomize: bool) -> Option<PlotIdx> {
        let ms = self.map_script.clone();
        if let Some(plot) = ms.find_starting_plot(self, p) {
            // Python override
            if plot.idx() < self.map.num_plots() {
                return Some(plot);
            }
            // C++ FAssertMsg: "python findStartingPlot() returned an invalid plot index!";
            // falls through to the DLL search.
        }

        // C++ iBestArea: -1 (None) = any area.
        let mut best_area: Option<AreaId> = None;

        let mut new = false;
        if let Some(starting_plot) = self.player_get_starting_plot(p) {
            best_area = self.map.plots[starting_plot].area;
            self.player_set_starting_plot(p, None, true);
            new = true;
        }

        // AI: AI_updateFoundValues(true) -- this sets all plots found values to -1
        ai_update_found_values_starting_loc(self, p);

        if !new {
            best_area = self.player_find_starting_area(p);
        }

        // C++ quirk: iRange is computed but no longer used (the distance factor is now done
        // inside the found value).
        let _range = self.player_starting_plot_range(p);

        // C++ quirk: every pass repeats the same search (found values are cached), so passes
        // after the first only run when no plot has a positive value.
        for _pass in 0..self.map.max_plot_distance() {
            let mut best_plot = None;
            let mut best_value = 0;

            for plot in self.map.plot_ids() {
                if best_area.is_none() || self.map.plots[plot].area == best_area {
                    // the distance factor is now done inside foundValue
                    let mut value = self.plot_get_found_value(plot, p);

                    if randomize && value > 0 {
                        value += self.soren_rand_num(10000, "Randomize Starting Location");
                    }

                    if value > best_value {
                        best_value = value;
                        best_plot = Some(plot);
                    }
                }
            }

            if best_plot.is_some() {
                return best_plot;
            }
            // C++ FAssertMsg(iPass != 0, "could not find starting plot in first pass.")
        }

        // C++ FAssertMsg(false, "Could not find starting plot.")
        None
    }

    /// C++ `CvPlayer::findBestFoundValue`: best `CvArea::getBestFoundValue` over all areas.
    pub fn player_find_best_found_value(&self, p: PlayerId) -> i32 {
        let mut best_value = 0;

        for (_, area) in self.map.areas.iter() {
            let value = area.best_found_value[p];

            if value > best_value {
                best_value = value;
            }
        }

        best_value
    }

    /// C++ `CvPlayer::getStartingPlot` (`plotSorenINLINE(m_iStartingX, m_iStartingY)`).
    pub fn player_get_starting_plot(&self, p: PlayerId) -> Option<PlotIdx> {
        self.map.plot_soren(self.players[p].starting_x, self.players[p].starting_y)
    }

    /// C++ `CvPlayer::setStartingPlot(pNewValue, bUpdateStartDist)`: updates the areas'
    /// starting plot counts (and min original start distances).
    pub fn player_set_starting_plot(&mut self, p: PlayerId, plot: Option<PlotIdx>, update_start_dist: bool) {
        let old_starting_plot = self.player_get_starting_plot(p);

        if old_starting_plot != plot {
            if let Some(old) = old_starting_plot {
                let old_area = area_of(self, old);
                self.map.areas[old_area].num_starting_plots -= 1;

                if update_start_dist {
                    // C++ quirk: m_iStartingX/Y still point at the old plot here, so the old
                    // area's distances are recomputed with this player's old start counted.
                    self.map_update_min_original_start_dist(old_area);
                }
            }

            match plot {
                None => {
                    self.players[p].starting_x = INVALID_PLOT_COORD;
                    self.players[p].starting_y = INVALID_PLOT_COORD;
                }
                Some(new_plot) => {
                    let (x, y) = self.map.xy(new_plot);
                    self.players[p].starting_x = x;
                    self.players[p].starting_y = y;

                    let new_area = area_of(self, new_plot);
                    self.map.areas[new_area].num_starting_plots += 1;

                    if update_start_dist {
                        self.map_update_min_original_start_dist(new_area);
                    }
                }
            }
        }
    }

    /// C++ `CvPlayer::getNewCityProductionValue`: production value of a new city's population
    /// (the settlers' extra cost). (C++ also counted the free buildings of a later
    /// start; this game has none.)
    pub fn player_get_new_city_production_value(&self, p: PlayerId) -> i32 {
        let d = self.d;
        let mut value = 0;

        value += (d.defines.advanced_start_city_cost * d.game_speeds[self.game_speed_type()].growth_percent) / 100;

        let population = d.defines.initial_city_population;
        for i in 1..=population {
            value += (self.player_get_growth_threshold(p, i) * d.defines.advanced_start_population_cost) / 100;
        }

        value
    }
}

// ============================================================================================
// CvPlayerAI valuations used by the rules above (AI: they live in the AI class in C++).
// ============================================================================================

/// AI: C++ `CvPlayerAI::AI_updateFoundValues(bStartingLoc = true)`: every area's best found
/// value for `p` back to 0 and every plot's found value to -1 (recomputed lazily by
/// `CvPlot::getFoundValue`). The `bStartingLoc = false` branch (city sites, the
/// `getCityFoundValue` callback) is AI-only.
fn ai_update_found_values_starting_loc(g: &mut Game, p: PlayerId) {
    for (_, area) in g.map.areas.iter_mut() {
        area.best_found_value[p] = 0;
    }

    for plot in g.map.plots.iter_mut() {
        plot.found_value[p] = -1;
    }
}


/// AI: C++ `CvPlayerAI::AI_unitValue(eUnit, eUnitAI, pArea = NULL)` for the UnitAIs rules
/// code asks for (`addFreeUnitAI` from `initFreeUnits`: CITY_DEFENSE, WORKER and EXPLORE).
/// Those cases read only static data, techs and bonuses. The other UnitAIs depend on AI
/// strategy state (`AI_isDoStrategy`, unit class/combat weights, missionary needs, area AI
/// types) and belong to the AI crate; here they are never valid (value 0).
fn ai_unit_value(g: &Game, p: PlayerId, unit: UnitType, unit_ai: UnitAIType) -> i32 {
    debug_assert!(
        matches!(unit_ai, UnitAIType::UNITAI_CITY_DEFENSE | UnitAIType::UNITAI_WORKER | UnitAIType::UNITAI_EXPLORE),
        "AI_unitValue is only ported for the free starting units"
    );
    let d = g.d;
    let info = &d.units[unit];

    if info.domain_type != ai_unit_ai_domain_type(unit_ai) && unit_ai != UnitAIType::UNITAI_ICBM {
        //XXX
        return 0;
    }

    if info.not_unit_ai_type[unit_ai] {
        return 0;
    }

    let mut valid = info.unit_ai_type[unit_ai];

    if !valid {
        match unit_ai {
            UnitAIType::UNITAI_WORKER => {
                for build in d.builds.keys() {
                    if info.builds[build] {
                        valid = true;
                        break;
                    }
                }
            }
            UnitAIType::UNITAI_CITY_DEFENSE => {
                if info.combat > 0 && !info.no_defensive_bonus && info.city_defense_modifier > 0 {
                    valid = true;
                }
            }
            UnitAIType::UNITAI_EXPLORE => {
                if info.combat > 0 && ai_unit_impassable_count(g, p, unit) == 0 {
                    valid = true;
                }
            }
            // AI: the other UnitAIs (not requested by rules code).
            _ => return 0,
        }
    }

    if !valid {
        return 0;
    }

    // FfH: Modified by Kael 05/07/2008 (CvPlayerAI::AI_combatValue, times 3)
    let combat_value = ai_combat_value(g, p, unit) * 3;

    let mut value = 1;

    value += info.ai_weight;

    match unit_ai {
        UnitAIType::UNITAI_WORKER => {
            for build in d.builds.keys() {
                if info.builds[build] {
                    value += 50;
                }
            }
            value += info.moves * 100;
        }
        UnitAIType::UNITAI_CITY_DEFENSE => {
            value += (combat_value * 2) / 3;
            value += (combat_value * info.city_defense_modifier) / 75;
        }
        UnitAIType::UNITAI_EXPLORE => {
            value += combat_value / 2;
            value += info.moves * 200;
            if info.no_bad_goodies {
                value += 100;
            }
        }
        // AI: the other UnitAIs (not requested by rules code).
        _ => return 0,
    }

    // (The ATTACK/ATTACK_CITY amphibious bonus needs an area; addFreeUnitAI passes NULL.)

    max(0, value)
}

/// AI: C++ `CvPlayerAI::AI_unitAIDomainType` (None = NO_DOMAIN).
fn ai_unit_ai_domain_type(unit_ai: UnitAIType) -> Option<DomainType> {
    match unit_ai {
        UnitAIType::UNITAI_UNKNOWN => None,

        UnitAIType::UNITAI_ANIMAL
        | UnitAIType::UNITAI_SETTLE
        | UnitAIType::UNITAI_WORKER
        | UnitAIType::UNITAI_ATTACK
        | UnitAIType::UNITAI_ATTACK_CITY
        | UnitAIType::UNITAI_COLLATERAL
        | UnitAIType::UNITAI_PILLAGE
        | UnitAIType::UNITAI_RESERVE
        | UnitAIType::UNITAI_COUNTER
        | UnitAIType::UNITAI_PARADROP
        | UnitAIType::UNITAI_CITY_DEFENSE
        | UnitAIType::UNITAI_CITY_COUNTER
        | UnitAIType::UNITAI_CITY_SPECIAL
        | UnitAIType::UNITAI_EXPLORE
        | UnitAIType::UNITAI_MISSIONARY
        | UnitAIType::UNITAI_PROPHET
        | UnitAIType::UNITAI_ARTIST
        | UnitAIType::UNITAI_SCIENTIST
        | UnitAIType::UNITAI_GENERAL
        | UnitAIType::UNITAI_MERCHANT
        | UnitAIType::UNITAI_ENGINEER
        | UnitAIType::UNITAI_SPY
        | UnitAIType::UNITAI_ATTACK_CITY_LEMMING
        // BETTER AI (New UNITAI) Sephi
        | UnitAIType::UNITAI_MAGE
        | UnitAIType::UNITAI_TERRAFORMER
        | UnitAIType::UNITAI_MANA_UPGRADE
        | UnitAIType::UNITAI_WARWIZARD
        | UnitAIType::UNITAI_BARBSMASHER
        | UnitAIType::UNITAI_HERO
        | UnitAIType::UNITAI_FEASTING
        | UnitAIType::UNITAI_MEDIC => Some(DomainType::DOMAIN_LAND),

        UnitAIType::UNITAI_ICBM => Some(DomainType::DOMAIN_IMMOBILE),

        UnitAIType::UNITAI_WORKER_SEA
        | UnitAIType::UNITAI_ATTACK_SEA
        | UnitAIType::UNITAI_RESERVE_SEA
        | UnitAIType::UNITAI_ESCORT_SEA
        | UnitAIType::UNITAI_EXPLORE_SEA
        | UnitAIType::UNITAI_ASSAULT_SEA
        | UnitAIType::UNITAI_SETTLER_SEA
        | UnitAIType::UNITAI_MISSIONARY_SEA
        | UnitAIType::UNITAI_SPY_SEA
        | UnitAIType::UNITAI_CARRIER_SEA
        | UnitAIType::UNITAI_MISSILE_CARRIER_SEA
        | UnitAIType::UNITAI_PIRATE_SEA => Some(DomainType::DOMAIN_SEA),

        UnitAIType::UNITAI_ATTACK_AIR | UnitAIType::UNITAI_DEFENSE_AIR | UnitAIType::UNITAI_CARRIER_AIR | UnitAIType::UNITAI_MISSILE_AIR => {
            Some(DomainType::DOMAIN_AIR)
        }

        // C++ FAssert(false)
        _ => None,
    }
}

/// AI: C++ `CvPlayerAI::AI_unitImpassableCount`: impassable terrains/features the team cannot
/// pass yet.
fn ai_unit_impassable_count(g: &Game, p: PlayerId, unit: UnitType) -> i32 {
    let d = g.d;
    let info = &d.units[unit];
    let team = g.players[p].team;
    let mut count = 0;

    for terrain in d.terrains.keys() {
        if info.terrain_impassable[terrain] {
            let tech = info.terrain_passable_tech[terrain];
            if tech.is_none() || !g.team_is_has_tech(team, tech) {
                count += 1;
            }
        }
    }

    for feature in d.features.keys() {
        if info.feature_impassable[feature] {
            let tech = info.feature_passable_tech[feature];
            if tech.is_none() || !g.team_is_has_tech(team, tech) {
                count += 1;
            }
        }
    }

    count
}

/// AI: C++ `CvPlayerAI::AI_combatValue` (FfH; replaces `CvGameAI::AI_combatValue`).
fn ai_combat_value(g: &Game, p: PlayerId, unit: UnitType) -> i32 {
    let d = g.d;
    let info = &d.units[unit];
    let mut value = 100 * ai_true_combat_value(g, p, unit);
    value *= (((info.first_strikes * 2) + info.chance_first_strikes) * (d.defines.combat_damage / 5)) + 100;
    value /= 100;
    value /= g.best_land_unit_combat();
    value
}

/// AI: C++ `CvPlayerAI::AI_trueCombatValue` (FfH): base strength plus damage-type strength,
/// bonus affinities and the weapon promotion the player's resources would give.
fn ai_true_combat_value(g: &Game, p: PlayerId, unit: UnitType) -> i32 {
    let d = g.d;
    let info = &d.units[unit];

    let mut combat = if info.domain_type == Some(DomainType::DOMAIN_AIR) { info.air_combat } else { info.combat };

    for damage_type in d.damages.keys() {
        combat += info.damage_type_combat[damage_type];
    }

    for bonus in d.bonuses.keys() {
        combat += info.bonus_affinity[bonus] * g.player_get_num_available_bonuses(p, bonus);
    }

    if info.weapon_tier > 0 {
        // (C++ indexes the WEAPON_REQ_BONUS_TIER* defines unchecked; they name bonuses.)
        let available = |bonus_define: i32| opt_from_i32::<BonusType>(bonus_define).map_or(0, |b| g.player_get_num_available_bonuses(p, b));

        let mut combat_bonus = 0;
        if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.weapon_promotion_tier1) {
            if available(d.defines.weapon_req_bonus_tier1) > 0 {
                combat_bonus = d.promotions[promotion].stat_changes[StatType::Attack.idx()].flat / 10;
            }
        }
        if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.weapon_promotion_tier2) {
            if info.weapon_tier > 1 && available(d.defines.weapon_req_bonus_tier2) > 0 {
                combat_bonus = d.promotions[promotion].stat_changes[StatType::Attack.idx()].flat / 10;
            }
        }
        if let Some(promotion) = opt_from_i32::<PromotionType>(d.defines.weapon_promotion_tier3) {
            if info.weapon_tier > 2 && available(d.defines.weapon_req_bonus_tier3) > 0 {
                combat_bonus = d.promotions[promotion].stat_changes[StatType::Attack.idx()].flat / 10;
            }
        }
        combat += combat_bonus;
    }

    combat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Map, MapInitData};
    use crate::player::Player;
    use crate::setup::GameSetup;

    /// Two AI civs on a `w` x `h` (non-wrapping, all ocean) map without areas; players are
    /// reset (`Player::new`) but not initialized.
    fn game(w: i32, h: i32) -> Game {
        let d = ffh_data::global();
        let setup = GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 7);
        let mut g = Game::new(d, setup);
        let (num_players, num_teams) = (g.num_players(), g.num_teams());
        for p in g.player_ids() {
            g.players[p] = Player::new(d, p, &g.setup.players[p.idx()], num_players, num_teams);
        }
        g.map = Map::new(d, &MapInitData { grid_w: w, grid_h: h, ..MapInitData::default() }, num_players, num_teams);
        g
    }

    #[test]
    fn starting_plot_tracks_area_counts() {
        let mut g = game(6, 4);
        let west = g.map_add_area(false);
        let east = g.map_add_area(false);
        for plot in g.map.plot_ids() {
            let (x, _) = g.map.xy(plot);
            g.map.plots[plot].area = Some(if x < 3 { west } else { east });
        }
        let p = PlayerId(0);
        assert_eq!(g.player_get_starting_plot(p), None);

        let first = g.map.plot_xy(1, 1).unwrap();
        g.player_set_starting_plot(p, Some(first), false);
        assert_eq!(g.player_get_starting_plot(p), Some(first));
        assert_eq!((g.players[p].starting_x, g.players[p].starting_y), (1, 1));
        assert_eq!(g.map.areas[west].num_starting_plots, 1);

        // Setting the same plot again changes nothing.
        g.player_set_starting_plot(p, Some(first), false);
        assert_eq!(g.map.areas[west].num_starting_plots, 1);

        let second = g.map.plot_xy(4, 2).unwrap();
        g.player_set_starting_plot(p, Some(second), false);
        assert_eq!(g.map.areas[west].num_starting_plots, 0);
        assert_eq!(g.map.areas[east].num_starting_plots, 1);

        g.player_set_starting_plot(p, None, false);
        assert_eq!(g.map.areas[east].num_starting_plots, 0);
        assert_eq!((g.players[p].starting_x, g.players[p].starting_y), (INVALID_PLOT_COORD, INVALID_PLOT_COORD));
        assert_eq!(g.player_get_starting_plot(p), None);
    }

    #[test]
    fn starting_loc_found_value_reset_and_best_found_value() {
        let mut g = game(4, 3);
        let land = g.map_add_area(false);
        let water = g.map_add_area(true);
        let (p, other) = (PlayerId(1), PlayerId(0));
        assert_eq!(g.player_find_best_found_value(p), 0);

        g.map.areas[land].best_found_value[p] = 120;
        g.map.areas[water].best_found_value[p] = 340;
        g.map.areas[water].best_found_value[other] = 999;
        g.map.plots[3].found_value[other] = 55;
        assert_eq!(g.player_find_best_found_value(p), 340);

        ai_update_found_values_starting_loc(&mut g, p);
        assert_eq!(g.player_find_best_found_value(p), 0);
        assert!(g.map.plots.iter().all(|plot| plot.found_value[p] == -1));
        // Other players' caches are untouched.
        assert_eq!(g.map.plots[3].found_value[other], 55);
        assert_eq!(g.player_find_best_found_value(other), 999);
    }

    #[test]
    fn init_free_state_sets_starting_gold_and_clears_research() {
        let mut g = game(2, 2);
        let d = g.d;
        let p = PlayerId(0);
        let tech = TechType(0);
        g.players[p].gold = 77;
        g.players[p].starting_gold = 25;
        g.players[p].research_queue.push(tech);
        g.players[p].researching_tech[tech] = true;

        g.player_init_free_state(p);

        let handicap = g.player_get_handicap_type(p).unwrap();
        let expected = d.handicaps[handicap].starting_gold + 25;
        assert_eq!(g.players[p].gold, expected);
        assert!(g.players[p].research_queue.is_empty());
        assert!(!g.players[p].researching_tech[tech]);
    }

    #[test]
    fn distance_factor_is_neutral_without_a_start() {
        let g = game(4, 4);
        let plot = g.map.plot_xy(2, 2).unwrap();
        assert_eq!(g.player_starting_plot_distance_factor(PlayerId(1), plot, PlayerId(0), 7), 1000);
    }

    #[test]
    fn starting_area_ignores_water() {
        let mut g = game(3, 3);
        let water = g.map_add_area(true);
        g.map.areas[water].num_tiles = 9;
        assert_eq!(g.player_find_starting_area(PlayerId(0)), None);
    }

    #[test]
    fn unit_ai_domains() {
        assert_eq!(ai_unit_ai_domain_type(UnitAIType::UNITAI_UNKNOWN), None);
        let land_ais = [
            UnitAIType::UNITAI_SETTLE,
            UnitAIType::UNITAI_WORKER,
            UnitAIType::UNITAI_CITY_DEFENSE,
            UnitAIType::UNITAI_EXPLORE,
            UnitAIType::UNITAI_MEDIC,
        ];
        for land in land_ais {
            assert_eq!(ai_unit_ai_domain_type(land), Some(DomainType::DOMAIN_LAND));
        }
        assert_eq!(ai_unit_ai_domain_type(UnitAIType::UNITAI_ICBM), Some(DomainType::DOMAIN_IMMOBILE));
        assert_eq!(ai_unit_ai_domain_type(UnitAIType::UNITAI_PIRATE_SEA), Some(DomainType::DOMAIN_SEA));
        assert_eq!(ai_unit_ai_domain_type(UnitAIType::UNITAI_MISSILE_AIR), Some(DomainType::DOMAIN_AIR));
    }

}
