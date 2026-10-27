//! Port of the other FFH2 0.41 AI code outside the AI classes: the Python hooks
//! `CvGameUtils.AI_unitUpdate` (CvGameUtils.py:2957-2995), `AI_MageTurn` (3190-3561),
//! `AI_MagePromotion` (3562-3968), `AI_Mage_UPGRADE_MANA` (3969-4357),
//! `CustomFunctions.warScript`/`warScriptAllow`/`dogpile`/`dogPileAllow` (CustomFunctions.py:
//! 1169-1265), and the AI functions of the rules classes: `CvUnit::chooseSpell`
//! (CvUnit.cpp:15917-16088), `CvUnit::AI_canJoinGroup` (CvUnit.cpp:18499),
//! `CvPlot::AI_sumStrength` (CvPlot.cpp:2792-2831), `CvPlot::AI_neededPermDefenseReserve`
//! (CvPlot.cpp:10960-10990), `CvPlot::AI_neededBetterDefender` (CvPlot.cpp:10992-11097).
//! (`CvPlayer::AI_isSummonSuicideMode` is `Player::sum_sui_mode`; `AI_doFavoriteReligion` and
//! `AI_doTowerMastery` are ported in ffh-core.)
//!
//! Porting notes (Python):
//! - `CyMap().plot(x, y)` is `g.map.plot_xy(x, y)` (wraps; `None` is the Python NULL plot,
//!   whose `isNone()` is true and `getNumUnits()` is -1, i.e. an empty `range`).
//! - `pUnit.getGroup().pushMission(MISSION_MOVE_TO, iiX, iiY, ..)`: the Python passes the
//!   unwrapped loop coordinates; the port passes the coordinates of the plot `CyMap().plot`
//!   returned (PORT ADAPT: the same plot, and `CvSelectionGroup::at` then matches it).
//! - `for icity in range(pPlayer.getNumCities()): pCity = pPlayer.getCity(icity)` reads the
//!   cities in free-list slots `0..getNumCities()` (`getCity` takes an id; a small id is a slot
//!   index); the port iterates the owner's city list in order (PORT ADAPT: the same cities as long
//!   as none was lost, which is the usual case).
//! - AI random numbers (`CyGame().getSorenRandNum` in AI Python) come from `self.rand`.
//!
//! SIGNATURE NOTES: `plot_ai_sum_strength` takes `owner: Option<PlayerId>` (C++ `eOwner` may
//! be NO_PLAYER, e.g. from `AI_bombardCity`), as the integrator asked; callers pass
//! `Some(owner)` / `None`.
#![allow(unused_variables, unused_mut, unused_imports, unused_assignments, clippy::all)]
use crate::prelude::*;
use ffh_core::ffh::game_utils::py_div;

/// `pUnit.getGroup().pushMission(eMission, iData1, iData2, 0, bAppend, False, eMissionAI,
/// pMissionAIPlot, pUnit)`.
fn push_mission(
    g: &mut Game,
    u: UnitId,
    mission: MissionType,
    data1: i32,
    data2: i32,
    append: bool,
    mission_ai: Option<MissionAIType>,
    mission_ai_plot: Option<PlotIdx>,
) {
    if let Some(grp) = g.units[u].group {
        if g.groups.contains(grp) {
            g.group_push_mission(grp, mission, data1, data2, 0, append, false, mission_ai, mission_ai_plot, Some(u));
        }
    }
}

/// `pUnit.getGroup().pushMission(MISSION_MOVE_TO, x, y, 0, False, False, NO_MISSIONAI,
/// pUnit.plot(), pUnit)` to plot `to`.
fn push_move_to(g: &mut Game, u: UnitId, to: PlotIdx) {
    let (x, y) = g.map.xy(to);
    let here = g.unit_plot(u);
    push_mission(g, u, MissionType::MISSION_MOVE_TO, x, y, false, None, Some(here));
}

/// `pPlot.getPlotCity()` (the live city on the plot).
fn plot_city_of(g: &Game, plot: PlotIdx) -> Option<CityId> {
    g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c))
}

/// `not (pPlot2.isImpassable() or pPlot2.isVisibleEnemyUnit(owner))` and
/// `pPlot2.getOwner() == owner` (the common test of the terraformer searches).
fn plot_ok(g: &Game, p2: PlotIdx, owner: PlayerId) -> bool {
    !(g.plot_is_impassable(p2) || g.plot_is_visible_enemy_unit(p2, owner)) && g.map.plots[p2].owner == Some(owner)
}

/// `not (pPlot2.getImprovementType() != -1 and gc.getImprovementInfo(..).isUnique() == true)`.
fn not_unique_improvement(g: &Game, p2: PlotIdx) -> bool {
    match g.map.plots[p2].improvement_type {
        Some(imp) => !g.d.improvements[imp].unique,
        None => true,
    }
}

/// The flags and types `AI_MageTurn` works with.
struct Terra {
    owner: PlayerId,
    smokeb: bool,
    desertb: bool,
    snowb: bool,
    tundrab: bool,
    marshb: bool,
    grassb: bool,
    hellterrb: bool,
    treesb: bool,
    /// Python `treesimpb` (only set for Devout/Priest of Leaves; read only when `treesb`).
    treesimpb: bool,
    /// `pPlot.isRiver()` of the unit's plot at the start of the turn.
    river_here: bool,
    water1: bool,
    sun1: bool,
    life1: bool,
    desert: TerrainType,
    snow: TerrainType,
    tundra: TerrainType,
    marsh: TerrainType,
    grass: TerrainType,
    plains: TerrainType,
    hell: [TerrainType; 4],
    smoke: ImprovementType,
    flood_plains: FeatureType,
    oasis: FeatureType,
}

/// Work found on one plot by the counting loops of `AI_MageTurn` ("LOOK FOR WORK" and the
/// "move on to another City" count use the same tests).
#[derive(Default)]
struct TerraWork {
    desert: i32,
    snow: i32,
    tundra: i32,
    marsh: i32,
    grass: i32,
    hellterr: i32,
    trees: i32,
}

impl TerraWork {
    fn total(&self) -> i32 {
        self.desert + self.snow + self.tundra + self.marsh + self.grass + self.hellterr + self.trees
    }
}

impl Terra {
    fn is_hell(&self, t: Option<TerrainType>) -> bool {
        match t {
            Some(t) => self.hell.contains(&t),
            None => false,
        }
    }

    /// The counting tests on `p2` (a non-NULL plot).
    fn count(&self, g: &Game, p2: PlotIdx) -> TerraWork {
        let mut w = TerraWork::default();
        if !plot_ok(g, p2, self.owner) || !not_unique_improvement(g, p2) {
            return w;
        }
        let pl = &g.map.plots[p2];
        let terrain = pl.terrain_type;
        if self.water1 {
            if self.smokeb && pl.improvement_type == Some(self.smoke) {
                w.desert += 1;
            }
            if self.desertb || self.river_here {
                if terrain == Some(self.desert) && !(pl.feature_type == Some(self.flood_plains)) {
                    if !(pl.feature_type == Some(self.oasis)) {
                        w.desert += 1;
                    }
                }
            }
        }
        if self.sun1 {
            if self.snowb && terrain == Some(self.snow) {
                w.snow += 1;
            }
            if self.tundrab && terrain == Some(self.tundra) {
                w.tundra += 1;
            }
            if self.marshb && terrain == Some(self.marsh) {
                w.marsh += 1;
            }
            if self.grassb && terrain == Some(self.grass) {
                w.grass += 1;
            }
        }
        if self.hellterrb && self.life1 && self.is_hell(terrain) {
            w.hellterr += 1;
        }
        if self.treesb && self.tree_target(g, p2) {
            w.trees += 1;
        }
        w
    }

    /// The Bloom target test: no feature, grass/plains/tundra, no city, no improvement (or
    /// `treesimpb`).
    fn tree_target(&self, g: &Game, p2: PlotIdx) -> bool {
        let pl = &g.map.plots[p2];
        pl.feature_type.is_none()
            && (pl.terrain_type == Some(self.grass) || pl.terrain_type == Some(self.plains) || pl.terrain_type == Some(self.tundra))
            && !g.plot_is_city(p2, false, None)
            && (pl.improvement_type.is_none() || self.treesimpb)
    }
}

/// The 16 mana types `AI_Mage_UPGRADE_MANA` chooses from (`BONUS_MANA_x` / `BUILD_MANA_x`), in
/// Python order.
const MANA_NAMES: [&str; 16] =
    ["AIR", "BODY", "CHAOS", "DEATH", "EARTH", "ENCHANTMENT", "ENTROPY", "FIRE", "LAW", "LIFE", "MIND", "NATURE", "SHADOW", "SPIRIT", "SUN", "WATER"];

fn mana_index(name: &str) -> usize {
    MANA_NAMES.iter().position(|&n| n == name).expect("mana type")
}

/// The mana the Tower victory flag asks for (`getArcaneTowerVictoryFlag()` 1..4).
fn tower_mana(flag: i32) -> &'static [&'static str] {
    match flag {
        1 => &["BODY", "LIFE", "ENCHANTMENT", "NATURE"],
        2 => &["LAW", "SUN", "SPIRIT", "MIND"],
        3 => &["CHAOS", "DEATH", "ENTROPY", "SHADOW"],
        4 => &["EARTH", "FIRE", "AIR", "WATER"],
        _ => &[],
    }
}

impl Cai {
    /// Python `CvGameUtils.AI_unitUpdate(argsList = (pUnit,))`: true = the Python moved the
    /// unit (the DLL's `AI_update` returns).
    pub fn py_ai_unit_update(&self, g: &mut Game, u: UnitId) -> bool {
        let d = g.d;
        let plot = g.unit_plot(u);
        let owner = g.units[u].owner;

        if g.unit_get_unit_class_type(u) == d.t::<UnitClassType>("UNITCLASS_GIANT_SPIDER") {
            let ix = g.units[u].x;
            let iy = g.units[u].y;
            for iix in (ix - 1)..(ix + 2) {
                for iiy in (iy - 1)..(iy + 2) {
                    if let Some(loop_plot) = g.map.plot_xy(iix, iiy) {
                        for &loop_unit in &g.map.plots[loop_plot].units {
                            if g.units[loop_unit].owner != owner {
                                return false;
                            }
                        }
                    }
                }
            }
            let here = g.unit_plot(u);
            push_mission(g, u, MissionType::MISSION_SKIP, 0, 0, false, None, Some(here));
            return true;
        }

        if let Some(improvement) = g.map.plots[plot].improvement_type {
            let num_units = g.map.plots[plot].units.len() as i32;
            let num_animals = g.plot_get_num_animal_units(plot);
            let is = |name: &str| improvement == d.t::<ImprovementType>(name);
            if is("IMPROVEMENT_BARROW") || is("IMPROVEMENT_RUINS") || is("IMPROVEMENT_HELLFIRE") {
                if !g.unit_is_animal(u) {
                    if num_units - num_animals == 1 {
                        push_mission(g, u, MissionType::MISSION_SKIP, 0, 0, false, None, Some(plot));
                        return true;
                    }
                }
            }
            if is("IMPROVEMENT_BEAR_DEN") || is("IMPROVEMENT_LION_DEN") {
                if g.unit_is_animal(u) {
                    if num_animals == 1 {
                        push_mission(g, u, MissionType::MISSION_SKIP, 0, 0, false, None, Some(plot));
                        return true;
                    }
                }
            }
            if is("IMPROVEMENT_GOBLIN_FORT") {
                if g.unit_get_unit_combat_type(u) == Some(d.t::<UnitCombatType>("UNITCOMBAT_ARCHER")) {
                    push_mission(g, u, MissionType::MISSION_SKIP, 0, 0, false, None, Some(plot));
                    return true;
                }
                if !g.unit_is_animal(u) {
                    if num_units - num_animals <= 2 {
                        push_mission(g, u, MissionType::MISSION_SKIP, 0, 0, false, None, Some(plot));
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Python `CvGameUtils.AI_MageTurn(argsList = (pUnit,))`: the `lResult` the DLL reads
    /// (1 = the unit acted).
    pub fn py_ai_mage_turn(&self, g: &mut Game, u: UnitId) -> i32 {
        let d = g.d;
        let plot = g.unit_plot(u);
        let owner = g.units[u].owner;
        let civ = g.player_get_civilization_type(owner);
        let ix = g.units[u].x;
        let iy = g.units[u].y;
        let is_civ = |name: &str| civ == Some(d.t::<CivilizationType>(name));

        if g.units[u].ai_type != Some(UnitAIType::UNITAI_TERRAFORMER) {
            // Python falls off the end (None): the DLL's lResult stays -1.
            return -1;
        }

        // TERRAFORMING: SETTING FLAGS
        let searchdistance = 3;

        // INIT
        let mut smokeb = true; // terraformer tries to put out smoke
        let mut desertb = true; // terraformer tries to spring deserts
        let mut snowb = true; // terraformer tries to scorch snow to tundra
        let mut tundrab = true; // terraformer tries to scorch tundra to plains
        let mut marshb = true; // terraformer tries to scorch marsh to grassland
        let mut grassb = false; // terraformer tries to scorch grassland to plains
        let mut hellterrb = true; // terraformer tries to remove hell terrain
        let mut treesb = false; // terraformer tries to Create Trees
        let mut treesimpb = false; // (Python: only defined for the units below)
        let mut canupgrademana = false; // terraformer tries to upgrade mana nodes

        let has_tech = |g: &Game, name: &str| g.player_is_has_tech(owner, d.t::<TechType>(name));
        if has_tech(g, "TECH_SORCERY")
            || has_tech(g, "TECH_ALTERATION")
            || has_tech(g, "TECH_DIVINATION")
            || has_tech(g, "TECH_ELEMENTALISM")
            || has_tech(g, "TECH_NECROMANCY")
        {
            canupgrademana = true;
        }

        let mut desert = 0;
        let mut snow = 0;
        let mut tundra = 0;
        let mut marsh = 0;
        let mut grass = 0;
        let mut hellterr = 0;
        let floodplain = 0;
        let mut trees = 0;

        // CIV SPECIFICS
        if is_civ("CIVILIZATION_INFERNAL") {
            smokeb = false;
        }
        if is_civ("CIVILIZATION_INFERNAL") {
            desertb = false;
        }
        if is_civ("CIVILIZATION_ILLIANS") {
            snowb = false;
        }
        if is_civ("CIVILIZATION_DOVIELLO") || is_civ("CIVILIZATION_ILLIANS") {
            tundrab = false;
        }
        if is_civ("CIVILIZATION_SHEAIM") {
            hellterrb = false;
        }

        // UNIT SPECIFIC
        let unit_type = g.units[u].unit_type;
        if unit_type == d.t::<UnitType>("UNIT_DEVOUT") {
            desertb = false;
            snowb = false;
            tundrab = false;
            marshb = false;
            grassb = false;
            hellterrb = true;
            treesb = false;
            treesimpb = false;
            canupgrademana = false;
        }

        if unit_type == d.t::<UnitType>("UNIT_PRIEST_OF_LEAVES") {
            desertb = false;
            snowb = false;
            tundrab = false;
            marshb = false;
            grassb = false;
            hellterrb = false;
            treesb = true;
            treesimpb = false;
            if is_civ("CIVILIZATION_LJOSALFAR") || is_civ("CIVILIZATION_SVARTALFAR") {
                treesimpb = true;
            }
            if !treesimpb && g.player_get_state_religion(owner) != Some(d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES")) {
                if !g.players[owner].human {
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_RESERVE);
                    return 0;
                }
            }
            canupgrademana = false;
        }

        // prefer to upgrade mana rather than terraform?
        if canupgrademana {
            if self.rand(100, "Upgrademana") < 10 {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_MANA_UPGRADE);
                return 0;
            }
            if g.players[owner].tower_vic_flag > 0 {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_MANA_UPGRADE);
                return 0;
            }
        }

        let ctx = Terra {
            owner,
            smokeb,
            desertb,
            snowb,
            tundrab,
            marshb,
            grassb,
            hellterrb,
            treesb,
            treesimpb,
            river_here: g.plot_is_river(plot),
            water1: g.units[u].has_promotion[d.t::<PromotionType>("PROMOTION_WATER1")],
            sun1: g.units[u].has_promotion[d.t::<PromotionType>("PROMOTION_SUN1")],
            life1: g.units[u].has_promotion[d.t::<PromotionType>("PROMOTION_LIFE1")],
            desert: d.t("TERRAIN_DESERT"),
            snow: d.t("TERRAIN_SNOW"),
            tundra: d.t("TERRAIN_TUNDRA"),
            marsh: d.t("TERRAIN_MARSH"),
            grass: d.t("TERRAIN_GRASS"),
            plains: d.t("TERRAIN_PLAINS"),
            hell: [d.t("TERRAIN_BROKEN_LANDS"), d.t("TERRAIN_BURNING_SANDS"), d.t("TERRAIN_FIELDS_OF_PERDITION"), d.t("TERRAIN_SHALLOWS")],
            smoke: d.t("IMPROVEMENT_SMOKE"),
            flood_plains: d.t("FEATURE_FLOOD_PLAINS"),
            oasis: d.t("FEATURE_OASIS"),
        };
        let spring = d.t::<SpellType>("SPELL_SPRING");
        let scorch = d.t::<SpellType>("SPELL_SCORCH");
        let sanctify = d.t::<SpellType>("SPELL_SANCTIFY");
        let bloom = d.t::<SpellType>("SPELL_BLOOM");

        // TERRAFORMING
        if g.map.plots[plot].owner == Some(owner) {
            let terrain = g.map.plots[plot].terrain_type;
            if desertb || ctx.river_here {
                if terrain == Some(ctx.desert) {
                    if crate::ai_can_cast(g, u, spring, false) {
                        g.tel_cast_reason(u, "ai_mage_turn", 0);
                        g.unit_cast(u, spring);
                        return 0;
                    }
                }
            } else if smokeb {
                if g.map.plots[plot].improvement_type == Some(ctx.smoke) {
                    if crate::ai_can_cast(g, u, spring, false) {
                        g.tel_cast_reason(u, "ai_mage_turn", 0);
                        g.unit_cast(u, spring);
                        return 0;
                    }
                }
            }

            if snowb && terrain == Some(ctx.snow) {
                if crate::ai_can_cast(g, u, scorch, false) {
                    g.tel_cast_reason(u, "ai_mage_turn", 0);
                    g.unit_cast(u, scorch);
                    return 0;
                }
            }

            if tundrab && terrain == Some(ctx.tundra) {
                if crate::ai_can_cast(g, u, scorch, false) {
                    g.tel_cast_reason(u, "ai_mage_turn", 0);
                    g.unit_cast(u, scorch);
                    return 0;
                }
            }

            if marshb && terrain == Some(ctx.marsh) {
                if crate::ai_can_cast(g, u, scorch, false) {
                    g.tel_cast_reason(u, "ai_mage_turn", 0);
                    g.unit_cast(u, scorch);
                    return 0;
                }
            }

            if grassb && terrain == Some(ctx.grass) {
                if crate::ai_can_cast(g, u, scorch, false) {
                    g.tel_cast_reason(u, "ai_mage_turn", 0);
                    g.unit_cast(u, scorch);
                    return 0;
                }
            }

            if hellterrb {
                if crate::ai_can_cast(g, u, sanctify, false) {
                    g.tel_cast_reason(u, "ai_mage_turn", 0);
                    g.unit_cast(u, sanctify);
                    return 0;
                }
            }

            if treesb {
                if g.map.plots[plot].feature_type.is_none() {
                    if crate::ai_can_cast(g, u, bloom, false) {
                        g.tel_cast_reason(u, "ai_mage_turn", 0);
                        g.unit_cast(u, bloom);
                        return 0;
                    }
                }
            }
        }

        // LOOK FOR WORK
        let no_plot_counter = g.is_option(GameOptionType::GAMEOPTION_NO_PLOT_COUNTER);
        for isearch in 1..searchdistance {
            // The (non-NULL) plots of the search square, x outside, y inside.
            let square = |g: &Game| -> Vec<PlotIdx> {
                let mut v = Vec::new();
                for iix in (ix - isearch)..(ix + isearch + 1) {
                    for iiy in (iy - isearch)..(iy + isearch + 1) {
                        if let Some(p2) = g.map.plot_xy(iix, iiy) {
                            v.push(p2);
                        }
                    }
                }
                v
            };

            // Python `if 1==1:` (count the work in the square; the counts add up over the
            // searches).
            for p2 in square(g) {
                let w = ctx.count(g, p2);
                desert += w.desert;
                snow += w.snow;
                tundra += w.tundra;
                marsh += w.marsh;
                grass += w.grass;
                hellterr += w.hellterr;
                trees += w.trees;
            }

            // Remove some deserts/smoke etc.?
            if desert > 0 {
                for p2 in square(g) {
                    if plot_ok(g, p2, owner) && not_unique_improvement(g, p2) {
                        let pl = &g.map.plots[p2];
                        if smokeb && pl.improvement_type == Some(ctx.smoke) {
                            push_move_to(g, u, p2);
                            return 1;
                        }
                        let pl = &g.map.plots[p2];
                        if desertb || ctx.river_here {
                            if pl.terrain_type == Some(ctx.desert) && !(pl.feature_type == Some(ctx.flood_plains)) {
                                if !(pl.feature_type == Some(ctx.oasis)) {
                                    push_move_to(g, u, p2);
                                    return 1;
                                }
                            }
                        }
                    }
                }
            }

            // snow, tundra, marsh, grass (in this order)
            let terrain_work = [(snow, ctx.snow), (tundra, ctx.tundra), (marsh, ctx.marsh), (grass, ctx.grass)];
            for (count, terrain) in terrain_work {
                if count > 0 {
                    for p2 in square(g) {
                        if plot_ok(g, p2, owner) && not_unique_improvement(g, p2) {
                            if g.map.plots[p2].terrain_type == Some(terrain) {
                                push_move_to(g, u, p2);
                                return 1;
                            }
                        }
                    }
                }
            }

            // Hell terrain to sanctify?
            if hellterr > 0 && ctx.life1 {
                for p2 in square(g) {
                    if !g.units.contains(u) {
                        // (the CyUnit would point to a dead unit)
                        return 1;
                    }
                    if !(g.plot_is_impassable(p2) || g.plot_is_visible_enemy_unit(p2, owner) || g.plot_is_water(p2)) {
                        if g.map.plots[p2].owner == Some(owner) {
                            // Python quirk: no `return` after these pushes; every matching plot
                            // pushes a new (non-appended) move and the search goes on.
                            if no_plot_counter {
                                if ctx.is_hell(g.map.plots[p2].terrain_type) {
                                    push_move_to(g, u, p2);
                                }
                            } else if g.map.plots[p2].plot_counter > 7 {
                                push_move_to(g, u, p2);
                            }
                        }
                    }
                }
                if !g.units.contains(u) {
                    return 1;
                }
            }

            if floodplain > 0 {
                for p2 in square(g) {
                    if plot_ok(g, p2, owner) && not_unique_improvement(g, p2) {
                        if g.plot_is_river(p2) && g.map.plots[p2].feature_type.is_none() {
                            push_move_to(g, u, p2);
                            return 1;
                        }
                    }
                }
            }

            if trees > 0 {
                for p2 in square(g) {
                    if plot_ok(g, p2, owner) && not_unique_improvement(g, p2) {
                        if ctx.tree_target(g, p2) {
                            push_move_to(g, u, p2);
                            return 1;
                        }
                    }
                }
            }
        }

        // Nothing to do, lets move on to another City!
        let mut best_count = 0;
        let mut best_city: Option<CityId> = None;
        // PORT ADAPT: `pPlayer.getCity(icity)` for icity < getNumCities() (see the header).
        for c in g.players[owner].cities.clone() {
            if !g.cities.contains(c) {
                continue;
            }
            let mut count = 0;
            for i in 1..21 {
                if let Some(p2) = g.city_index_plot(c, i) {
                    count += ctx.count(g, p2).total();
                }
            }
            if count > best_count {
                best_city = Some(c);
                best_count = count;
            }
        }
        if let Some(c) = best_city {
            let cplot = g.city_plot(c);
            push_move_to(g, u, cplot);
            return 1;
        }
        0
    }

    /// Python `CvGameUtils.AI_MagePromotion(argsList = (pUnit,))`: the promotion to take
    /// (`None` = Python -1).
    pub fn py_ai_mage_promotion(&self, g: &Game, u: UnitId) -> Option<PromotionType> {
        let d = g.d;
        let plot = g.unit_plot(u);
        let owner = g.units[u].owner;
        let civ = g.player_get_civilization_type(owner);
        let is_civ = |name: &str| civ == Some(d.t::<CivilizationType>(name));
        let promo = |name: &str| d.t::<PromotionType>(name);
        let has = |name: &str| g.units[u].has_promotion[promo(name)];
        let sorcery = g.player_is_has_tech(owner, d.t::<TechType>("TECH_SORCERY"));
        let is_city = g.plot_is_city(plot, false, None);
        let city = if is_city { plot_city_of(g, plot) } else { None };
        let plot_units = g.map.plots[plot].units.clone();
        let is_adept = g.unit_get_unit_class_type(u) == d.t::<UnitClassType>("UNITCLASS_ADEPT");
        let experience = g.units[u].experience;
        let level = g.units[u].level;
        let ai_type = g.units[u].ai_type;

        // Choose the best spell: the first highest value the unit can take.
        let choose = |types: &[&str], values: &[i32]| -> Option<PromotionType> {
            let mut best_spell: Option<usize> = None;
            let mut best_spell_value = 0;
            for i in 0..types.len() {
                if values[i] > best_spell_value {
                    if g.unit_can_promote(u, Some(promo(types[i])), None) {
                        best_spell_value = values[i];
                        best_spell = Some(i);
                    }
                }
            }
            best_spell.map(|i| promo(types[i]))
        };
        // Modify spells by their stack value (is it good to have several mages able to cast it?).
        let stack_values = |types: &[&str], stack_mod: &[i32], values: &mut [i32]| {
            for i in 0..types.len() {
                let p = promo(types[i]);
                let mut promonbr = 0;
                for &uu in &plot_units {
                    if g.units[uu].has_promotion[p] {
                        promonbr += 1;
                    }
                }
                values[i] += promonbr * stack_mod[i] * 100;
            }
        };

        // ------------ UNITAI_MAGE
        if ai_type == Some(UnitAIType::UNITAI_MAGE) {
            let s_type: [&str; 20] = [
                "PROMOTION_BODY1",
                "PROMOTION_BODY2",
                "PROMOTION_CHAOS1",
                "PROMOTION_CHAOS2",
                "PROMOTION_DEATH1",
                "PROMOTION_DEATH2",
                "PROMOTION_EARTH1",
                "PROMOTION_ENTROPY1",
                "PROMOTION_ENCHANTMENT1",
                "PROMOTION_ENCHANTMENT2",
                "PROMOTION_MIND1",
                "PROMOTION_MIND2",
                "PROMOTION_NATURE1",
                "PROMOTION_SHADOW1",
                "PROMOTION_SPIRIT1",
                "PROMOTION_SPIRIT2",
                "PROMOTION_COMBAT1",
                "PROMOTION_COMBAT2",
                "PROMOTION_COMBAT3",
                "PROMOTION_COMBAT4",
            ];
            let ix = |name: &str| s_type.iter().position(|&s| s == name).expect("promotion in list");
            // Python quirk: 22 stack modifiers for 20 promotions (misaligned from index 5 on).
            let mut stack_mod: Vec<i32> = vec![
                -100, -100, 0, -100, -100, //
                0, 0, -100, 0, -100, -100, //
                0, -100, 0, -100, -100, //
                -100, -100, //
                0, 0, 0, 0,
            ];
            let mut values = vec![0; s_type.len() + 1];

            // Adding Civ Values
            if is_civ("CIVILIZATION_BALSERAPHS") {
                values[ix("PROMOTION_CHAOS1")] += 500;
                values[ix("PROMOTION_CHAOS2")] += 500;
            }
            if is_civ("CIVILIZATION_LUCHUIRP") {
                values[ix("PROMOTION_ENCHANTMENT1")] += 500;
                stack_mod[ix("PROMOTION_ENCHANTMENT1")] = -2;
            }
            if is_civ("CIVILIZATION_SHEAIM") {
                values[ix("PROMOTION_DEATH1")] += 2000;
                values[ix("PROMOTION_DEATH2")] += 2000;
            }

            stack_values(&s_type, &stack_mod, &mut values);

            // Modify spells by their action type (is it needed to cast them every turn?)
            let mut bcastoffspell = false;
            if has("PROMOTION_DEATH1") {
                bcastoffspell = true;
            } else if has("PROMOTION_CHAOS1") {
                bcastoffspell = true;
            } else if has("PROMOTION_ENTROPY1") {
                bcastoffspell = true;
            } else if has("PROMOTION_MIND2") {
                bcastoffspell = true;
            } else if has("PROMOTION_SHADOW1") {
                bcastoffspell = true;
            }
            if is_civ("CIVILIZATION_LUCHUIRP") {
                values[ix("PROMOTION_ENCHANTMENT1")] += 1000;
            }

            if !bcastoffspell {
                // Python quirk: the bonus is added once per listed promotion (20 times).
                for _ in 0..s_type.len() {
                    values[ix("PROMOTION_DEATH1")] += 1000;
                    values[ix("PROMOTION_CHAOS1")] += 1000;
                    values[ix("PROMOTION_ENTROPY1")] += 1000;
                    values[ix("PROMOTION_MIND2")] += 1000;
                    values[ix("PROMOTION_SHADOW1")] += 1000;
                }
            }

            // Spell usefulness
            for i in 0..s_type.len() {
                match s_type[i] {
                    // Permanent spells
                    "PROMOTION_BODY2" => values[i] += 500,
                    "PROMOTION_EARTH1" => values[i] += 800,
                    "PROMOTION_MIND1" => values[i] += 200,
                    "PROMOTION_SPIRIT2" => {
                        if sorcery {
                            values[i] += 300;
                        }
                    }
                    "PROMOTION_ENCHANTMENT1" | "PROMOTION_ENCHANTMENT2" => {
                        if is_city {
                            values[i] += 250;
                            if let Some(c) = city {
                                if g.city_num_building(c, d.t::<BuildingType>("BUILDING_TRAINING_YARD")) == 1 {
                                    values[i] += 500;
                                }
                            }
                        }
                    }
                    // Powerful debuffs
                    "PROMOTION_DEATH1" => values[i] += 100,
                    "PROMOTION_DEATH2" => values[i] += 400,
                    "PROMOTION_CHAOS1" => values[i] += 150,
                    "PROMOTION_SHADOW1" => values[i] += 130,
                    "PROMOTION_ENTROPY1" => values[i] += 800,
                    "PROMOTION_MIND2" => values[i] += 500,
                    _ => {}
                }
            }

            // Spells that are only needed for Upgrade
            if sorcery {
                values[ix("PROMOTION_BODY1")] = values[ix("PROMOTION_BODY2")] + 10;
                values[ix("PROMOTION_SPIRIT1")] = values[ix("PROMOTION_SPIRIT2")] + 10;
            }

            // Make sure Adepts take combat promos if necessary to be able to upgrade to mage
            if sorcery {
                if experience >= 10 && level < 4 && is_adept {
                    values[ix("PROMOTION_COMBAT1")] = 10;
                    values[ix("PROMOTION_COMBAT2")] = 10;
                    values[ix("PROMOTION_COMBAT3")] = 10;
                    values[ix("PROMOTION_COMBAT4")] = 10;
                }
            }

            return choose(&s_type, &values);
        }

        // ------------ UNITAI_WARWIZARD
        if ai_type == Some(UnitAIType::UNITAI_WARWIZARD) {
            if sorcery {
                let adept = d.t::<UnitCombatType>("UNITCOMBAT_ADEPT");
                let mut countmages = 0;
                for &uu in &plot_units {
                    if g.unit_get_unit_combat_type(uu) == Some(adept) {
                        countmages += 1;
                    }
                }
                if countmages < 3 {
                    return None;
                }
            }

            let s_type: [&str; 31] = [
                "PROMOTION_AIR1",
                "PROMOTION_AIR2",
                "PROMOTION_BODY1",
                "PROMOTION_BODY2",
                "PROMOTION_CHAOS1",
                "PROMOTION_CHAOS2",
                "PROMOTION_DEATH1",
                "PROMOTION_DEATH2",
                "PROMOTION_EARTH1",
                "PROMOTION_EARTH2",
                "PROMOTION_ENTROPY1",
                "PROMOTION_ENCHANTMENT1",
                "PROMOTION_ENCHANTMENT2",
                "PROMOTION_FIRE1",
                "PROMOTION_FIRE2",
                "PROMOTION_LIFE1",
                "PROMOTION_LIFE2",
                "PROMOTION_MIND1",
                "PROMOTION_MIND2",
                "PROMOTION_NATURE1",
                "PROMOTION_NATURE2",
                "PROMOTION_SHADOW1",
                "PROMOTION_SHADOW2",
                "PROMOTION_SPIRIT1",
                "PROMOTION_SUN1",
                "PROMOTION_SUN2",
                "PROMOTION_COMBAT1",
                "PROMOTION_COMBAT2",
                "PROMOTION_COMBAT3",
                "PROMOTION_COMBAT4",
                "PROMOTION_MOBILITY1",
            ];
            let ix = |name: &str| s_type.iter().position(|&s| s == name).expect("promotion in list");
            // Python quirk: 33 stack modifiers for 31 promotions (misaligned from index 18 on).
            let mut stack_mod: Vec<i32> = vec![
                0, -2, -100, -100, -100, //
                -100, 0, 0, 0, -2, //
                -1, -100, -100, 0, 0, //
                -100, -2, 0, -7, 0, //
                -2, -100, -100, -100, -100, -100, //
                0, -2, //
                0, 0, 0, 0, 0,
            ];

            if is_civ("CIVILIZATION_LUCHUIRP") {
                stack_mod[ix("PROMOTION_ENCHANTMENT1")] = 2;
            }

            let mut values = vec![0; s_type.len() + 1];

            // Adding Civ Values
            if is_civ("CIVILIZATION_BALSERAPHS") {
                values[ix("PROMOTION_CHAOS1")] += 500;
                values[ix("PROMOTION_CHAOS2")] += 500;
            }
            if is_civ("CIVILIZATION_HIPPUS") {
                values[ix("PROMOTION_MOBILITY1")] += 200;
            }
            if is_civ("CIVILIZATION_LJOSALFAR") {
                values[ix("PROMOTION_FIRE2")] += 500;
            }
            if is_civ("CIVILIZATION_LUCHUIRP") {
                values[ix("PROMOTION_ENCHANTMENT1")] += 500;
            }
            if is_civ("CIVILIZATION_SVARTALFAR") {
                values[ix("PROMOTION_FIRE2")] += 500;
            }
            if is_civ("CIVILIZATION_SHEAIM") {
                values[ix("PROMOTION_DEATH1")] += 500;
                values[ix("PROMOTION_DEATH2")] += 500;
            }

            stack_values(&s_type, &stack_mod, &mut values);

            // Modify spells by their action type
            let mut bcastoffspell = false;
            if has("PROMOTION_AIR2") {
                bcastoffspell = true;
            } else if has("PROMOTION_CHAOS1") {
                bcastoffspell = true;
            } else if has("PROMOTION_DEATH1") {
                bcastoffspell = true;
            } else if has("PROMOTION_DEATH2") {
                bcastoffspell = true;
            } else if has("PROMOTION_ENTROPY1") {
                bcastoffspell = true;
            } else if has("PROMOTION_FIRE2") {
                bcastoffspell = true;
            } else if has("PROMOTION_MIND2") {
                bcastoffspell = true;
            } else if has("PROMOTION_SHADOW1") {
                bcastoffspell = true;
            }

            if !bcastoffspell {
                // Python quirk: the bonus is added once per listed promotion (31 times).
                for _ in 0..s_type.len() {
                    values[ix("PROMOTION_AIR2")] += 1000;
                    values[ix("PROMOTION_CHAOS1")] += 1000;
                    values[ix("PROMOTION_DEATH1")] += 1000;
                    values[ix("PROMOTION_DEATH2")] += 1000;
                    values[ix("PROMOTION_ENTROPY1")] += 1000;
                    values[ix("PROMOTION_FIRE2")] += 1000;
                    values[ix("PROMOTION_MIND2")] += 1000;
                    values[ix("PROMOTION_SHADOW1")] += 1000;
                }
            }

            if is_civ("CIVILIZATION_LUCHUIRP") {
                values[ix("PROMOTION_ENCHANTMENT1")] += 1000;
            }

            // Spell usefulness: permanent spells
            values[ix("PROMOTION_CHAOS1")] += 250;
            values[ix("PROMOTION_SHADOW1")] += 270;
            values[ix("PROMOTION_SHADOW2")] += 270;
            values[ix("PROMOTION_NATURE1")] += 280;
            values[ix("PROMOTION_SPIRIT1")] += 300;
            values[ix("PROMOTION_ENCHANTMENT1")] += 300;
            values[ix("PROMOTION_ENCHANTMENT2")] += 300;
            values[ix("PROMOTION_NATURE2")] += 300;
            values[ix("PROMOTION_BODY1")] += 500;
            values[ix("PROMOTION_BODY2")] += 500;
            // Powerful debuffs
            values[ix("PROMOTION_EARTH2")] += 100;
            values[ix("PROMOTION_MIND2")] += 400;
            values[ix("PROMOTION_SUN2")] += 400;
            values[ix("PROMOTION_ENTROPY1")] += 800;
            // Direct damage
            values[ix("PROMOTION_LIFE2")] += 0; // definetly needs some check
            values[ix("PROMOTION_FIRE2")] += 200;
            values[ix("PROMOTION_AIR2")] += 500;

            // Spells that are only needed for Upgrade
            if sorcery {
                values[ix("PROMOTION_EARTH1")] = values[ix("PROMOTION_EARTH2")] + 10;
                values[ix("PROMOTION_MIND1")] = values[ix("PROMOTION_MIND2")] + 10;
                values[ix("PROMOTION_LIFE1")] = values[ix("PROMOTION_LIFE2")] + 10;
                values[ix("PROMOTION_FIRE1")] = values[ix("PROMOTION_FIRE2")] + 10;
                values[ix("PROMOTION_AIR1")] = values[ix("PROMOTION_AIR2")] + 10;
            }

            // Make sure Adepts take combat promos if necessary to be able to upgrade to mage
            if sorcery {
                if experience >= 10 && level < 4 && is_adept {
                    values[ix("PROMOTION_COMBAT1")] = 10;
                    values[ix("PROMOTION_COMBAT2")] = 10;
                    values[ix("PROMOTION_COMBAT3")] = 10;
                    values[ix("PROMOTION_COMBAT4")] = 10;
                    values[ix("PROMOTION_MOBILITY1")] = 10;
                }
            }

            return choose(&s_type, &values);
        }

        // ------------ UNITAI_TERRAFORMER
        if ai_type == Some(UnitAIType::UNITAI_TERRAFORMER) || ai_type == Some(UnitAIType::UNITAI_MANA_UPGRADE) {
            let can = |name: &str| g.unit_can_promote(u, Some(promo(name)), None);
            if sorcery {
                if can("PROMOTION_METAMAGIC1") {
                    return Some(promo("PROMOTION_METAMAGIC1"));
                }
                if can("PROMOTION_METAMAGIC2") {
                    return Some(promo("PROMOTION_METAMAGIC2"));
                }
                if experience > 10 && level == 4 && is_adept {
                    return None;
                }
            }
            if g.get_global_counter() > 15 {
                if can("PROMOTION_LIFE1") {
                    return Some(promo("PROMOTION_LIFE1"));
                }
            }
            if can("PROMOTION_SUN1") {
                return Some(promo("PROMOTION_SUN1"));
            }
            if !is_civ("CIVILIZATION_INFERNAL") {
                if can("PROMOTION_WATER1") {
                    return Some(promo("PROMOTION_WATER1"));
                }
            }
            if can("PROMOTION_LIFE1") {
                return Some(promo("PROMOTION_LIFE1"));
            }
            if can("PROMOTION_MOBILITY1") {
                return Some(promo("PROMOTION_MOBILITY1"));
            }
            if can("PROMOTION_BODY1") {
                return Some(promo("PROMOTION_BODY1"));
            }
        }

        None
    }

    /// Python `CvGameUtils.AI_Mage_UPGRADE_MANA(argsList = (pUnit,))`: the `lResult` the DLL
    /// reads (1 = acted, 2 = become a terraformer).
    pub fn py_ai_mage_upgrade_mana(&self, g: &mut Game, u: UnitId) -> i32 {
        let d = g.d;
        let plot = g.unit_plot(u);
        let owner = g.units[u].owner;
        let civ = g.player_get_civilization_type(owner);
        let is_civ = |name: &str| civ == Some(d.t::<CivilizationType>(name));
        let ix = g.units[u].x;
        let iy = g.units[u].y;

        let mut smokeb = true; // Civ likes to put out smoke
        let mut desertb = true; // Civ likes to spring deserts
        let mut snowb = true; // Civ likes to scorch snow to tundra
        let mut tundrab = true; // Civ likes to scorch tundra to plains
        let mut marshb = true; // Civ likes to scorch marsh to grassland
        let grassb = false; // Civ likes to scorch grassland to plains
        let mut hellterrb = true; // Civ likes to remove hell terrain

        if is_civ("CIVILIZATION_INFERNAL") {
            smokeb = false;
        }
        // MALAKIM need SPRING to create floodplains sometimes
        if is_civ("CIVILIZATION_INFERNAL") {
            desertb = false;
        }
        if is_civ("CIVILIZATION_ILLIANS") {
            snowb = false;
        }
        tundrab = false;
        marshb = false;
        if is_civ("CIVILIZATION_SHEAIM") {
            hellterrb = false;
        }

        let mana_class = d.t::<BonusClassType>("BONUSCLASS_MANA");
        let raw_mana = d.t::<BonusType>("BONUS_MANA");

        // The (non-NULL) plots of the search square of radius `isearch` (Python quirk:
        // `range(i - s, i + s)` leaves out the last row and column), y outside, x inside.
        let square = |g: &Game, isearch: i32| -> Vec<(i32, i32, PlotIdx)> {
            let mut v = Vec::new();
            for iiy in (iy - isearch)..(iy + isearch) {
                for iix in (ix - isearch)..(ix + isearch) {
                    if let Some(p2) = g.map.plot_xy(iix, iiy) {
                        v.push((iix, iiy, p2));
                    }
                }
            }
            v
        };

        // Look for non raw mana
        let searchdistance = 15;
        let mut imanatype: Option<BonusType> = None;

        for isearch in 1..(searchdistance + 1) {
            if imanatype.is_some() {
                break;
            }
            for (iix, iiy, p2) in square(g, isearch) {
                if !plot_ok(g, p2, owner) {
                    continue;
                }
                if let Some(bonus) = g.plot_get_bonus_type(p2, None) {
                    if d.bonuses[bonus].bonus_class_type == Some(mana_class) {
                        if !g.plot_is_plot_group_connected_bonus(p2, owner, bonus) {
                            imanatype = g.plot_get_bonus_type(p2, None);
                            if imanatype.is_some() {
                                for build in d.builds.keys() {
                                    if g.unit_can_build(u, p2, build, false) {
                                        push_move_to(g, u, p2);
                                        if g.units.contains(u) {
                                            push_mission(g, u, MissionType::MISSION_BUILD, build.i(), -1, true, None, Some(plot));
                                        }
                                        return 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Look for raw mana
        let searchdistance = 15;
        let imanatype: Option<BonusType> = None;

        for isearch in 1..(searchdistance + 1) {
            if imanatype.is_some() {
                break;
            }
            for (iix, iiy, p2) in square(g, isearch) {
                if !plot_ok(g, p2, owner) {
                    continue;
                }
                if let Some(bonus) = g.plot_get_bonus_type(p2, None) {
                    if bonus == raw_mana && !(iix == ix && iiy == iy) {
                        push_move_to(g, u, p2);
                        return 1;
                    } else if bonus == raw_mana {
                        // Choose Mana to Build
                        let values = self.mana_values(g, u, p2, desertb, snowb, tundrab, marshb, grassb, hellterrb);

                        // Choose the best MANA
                        let mut best_mana: Option<BuildType> = None;
                        let mut best_mana_value = 0;
                        for i in 0..MANA_NAMES.len() {
                            if values[i] > best_mana_value {
                                let build = d.t::<BuildType>(&format!("BUILD_MANA_{}", MANA_NAMES[i]));
                                if g.unit_can_build(u, plot, build, false) {
                                    best_mana_value = values[i];
                                    best_mana = Some(build);
                                }
                            }
                        }

                        if let Some(build) = best_mana {
                            push_mission(g, u, MissionType::MISSION_BUILD, build.i(), -1, false, Some(MissionAIType::MISSIONAI_BUILD), Some(plot));
                            // help out the AI for the moment
                            g.plot_set_route_type(plot, Some(d.t::<RouteType>("ROUTE_ROAD")), true);
                            return 1;
                        }
                    }
                }
            }
        }

        // Look for Mana to Dispel
        let searchdistance = 15;

        if g.units[u].has_promotion[d.t::<PromotionType>("PROMOTION_METAMAGIC2")] {
            let dispel_magic = d.t::<SpellType>("SPELL_DISPEL_MAGIC");
            for isearch in 1..(searchdistance + 1) {
                for (iix, iiy, p2) in square(g, isearch) {
                    if !plot_ok(g, p2, owner) {
                        continue;
                    }
                    if let Some(bonus) = g.plot_get_bonus_type(p2, None) {
                        if d.bonuses[bonus].bonus_class_type == Some(mana_class) {
                            let mut b_dispel = true;

                            let flag = g.players[owner].tower_vic_flag;
                            if flag == 0 {
                                if self.rand(50, "Don't have to Dispel all the Time") != 0 {
                                    b_dispel = false;
                                }
                            }
                            // Keep the last node of each mana the Tower victory flag asks for.
                            for name in tower_mana(flag) {
                                let kb = d.t::<BonusType>(&format!("BONUS_MANA_{}", name));
                                if bonus == kb {
                                    if g.player_get_num_available_bonuses(owner, kb) == 1 {
                                        b_dispel = false;
                                    }
                                }
                            }

                            if b_dispel {
                                if !(iix == ix && iiy == iy) {
                                    push_move_to(g, u, p2);
                                    return 1;
                                }
                                if crate::ai_can_cast(g, u, dispel_magic, false) {
                                    g.tel_cast_reason(u, "ai_mage_upgrade_mana", 0);
                                    g.unit_cast(u, dispel_magic);
                                    return 1;
                                }
                            }
                        }
                    }
                }
            }

            // Dispel more if we seek Tower Victory Condition
            if g.players[owner].tower_vic_flag > 0 {
                let raw_mana_class = d.t::<BonusClassType>("BONUSCLASS_RAWMANA");
                let mut best_count = 0;
                let mut best_city: Option<CityId> = None;
                // PORT ADAPT: `pPlayer.getCity(icity)` for icity < getNumCities() (see the header).
                for c in g.players[owner].cities.clone() {
                    if !g.cities.contains(c) {
                        continue;
                    }
                    let mut count = 0;
                    for i in 1..21 {
                        if let Some(p2) = g.city_index_plot(c, i) {
                            if plot_ok(g, p2, owner) {
                                if let Some(bonus) = g.plot_get_bonus_type(p2, None) {
                                    if d.bonuses[bonus].bonus_class_type == Some(mana_class) {
                                        count += 1;
                                    }
                                    if d.bonuses[bonus].bonus_class_type == Some(raw_mana_class) {
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                    if count > best_count {
                        best_city = Some(c);
                        best_count = count;
                    }
                }
                if let Some(c) = best_city {
                    let cplot = g.city_plot(c);
                    push_move_to(g, u, cplot);
                    return 1;
                }
            }
        }

        // found no mana, return 2 so UNITAI is set to UNITAI_TERRAFORMER
        2
    }

    /// The `lValues` of `AI_Mage_UPGRADE_MANA` for the 16 mana types (`MANA_NAMES` order) when
    /// the unit stands on raw mana at `p2`.
    #[allow(clippy::too_many_arguments)]
    fn mana_values(
        &self,
        g: &Game,
        u: UnitId,
        p2: PlotIdx,
        desertb: bool,
        snowb: bool,
        tundrab: bool,
        marshb: bool,
        grassb: bool,
        hellterrb: bool,
    ) -> Vec<i32> {
        let d = g.d;
        let owner = g.units[u].owner;
        let civ = g.player_get_civilization_type(owner);
        let is_civ = |name: &str| civ == Some(d.t::<CivilizationType>(name));
        let m = mana_index;

        // Set Flags (Python: deathmagicb/holymagicb from the alignment, never read)
        let mut values = vec![0; MANA_NAMES.len() + 1];
        // by default a Civ doesn't like to stack mana
        let stack_mod = vec![-100; MANA_NAMES.len() + 1];

        // Adding Civ Values and their Mana stack values
        // (Python quirk: the stack-value lines are comparisons (`==`), not assignments.)
        if is_civ("CIVILIZATION_BALSERAPHS") {
            values[m("CHAOS")] += 2000;
        }
        if is_civ("CIVILIZATION_LJOSALFAR") {
            values[m("FIRE")] += 2000;
        }
        if is_civ("CIVILIZATION_LUCHUIRP") {
            values[m("ENCHANTMENT")] += 2000;
            values[m("FIRE")] += 2000;
        }
        if is_civ("CIVILIZATION_SVARTALFAR") {
            values[m("FIRE")] += 2000;
        }
        if is_civ("CIVILIZATION_SHEAIM") {
            values[m("DEATH")] += 2000;
            values[m("ENTROPY")] += 3000;
        }

        // MANA TYPE VALUES, SPELL LEVEL 1
        // Permanent
        values[m("NATURE")] += 50;
        values[m("MIND")] += 150;
        values[m("SPIRIT")] += 200;
        values[m("EARTH")] += 200;
        values[m("BODY")] += 500;
        values[m("ENCHANTMENT")] += 250;
        // per turn buffs
        values[m("CHAOS")] += 200;
        values[m("SHADOW")] += 200;
        values[m("ENTROPY")] += 700;
        // Summons
        values[m("DEATH")] += 400;

        // SPELL LEVEL 2
        if g.player_is_has_tech(owner, d.t::<TechType>("TECH_SORCERY")) {
            // Direct Damage
            values[m("AIR")] += 1000;
            values[m("FIRE")] += 450;
            // Permanent
            values[m("CHAOS")] += 0;
            values[m("SPIRIT")] += 100;
            values[m("BODY")] += 100;
            // per turn buffs
            values[m("EARTH")] += 100;
            values[m("SHADOW")] += 100;
            values[m("SUN")] += 400;
            values[m("MIND")] += 400;
            // Summons
            values[m("ENTROPY")] += 100;
            values[m("DEATH")] += 200;
        }

        // need mana for terraforming?
        let desert = d.t::<TerrainType>("TERRAIN_DESERT");
        let snow = d.t::<TerrainType>("TERRAIN_SNOW");
        let tundra = d.t::<TerrainType>("TERRAIN_TUNDRA");
        let marsh = d.t::<TerrainType>("TERRAIN_MARSH");
        let grass = d.t::<TerrainType>("TERRAIN_GRASS");
        let flood_plains = d.t::<FeatureType>("FEATURE_FLOOD_PLAINS");
        let hell = [
            d.t::<TerrainType>("TERRAIN_BROKEN_LANDS"),
            d.t::<TerrainType>("TERRAIN_BURNING_SANDS"),
            d.t::<TerrainType>("TERRAIN_FIELDS_OF_PERDITION"),
            d.t::<TerrainType>("TERRAIN_SHALLOWS"),
        ];
        let global_counter = g.get_global_counter();
        // `PyPlayer(owner).getCityList()`: the owner's cities in order.
        for &c in &g.players[owner].cities {
            if !g.cities.contains(c) {
                continue;
            }
            let (cx, cy) = g.map.xy(g.city_plot(c));
            // Python quirk: `range(c - 2, c + 2)` (4 columns and rows, not 5).
            for iix in (cx - 2)..(cx + 2) {
                for iiy in (cy - 2)..(cy + 2) {
                    let Some(p4) = g.map.plot_xy(iix, iiy) else { continue };
                    if g.plot_is_impassable(p4) {
                        continue;
                    }
                    if g.map.plots[p4].owner != Some(owner) {
                        continue;
                    }
                    let pl4 = &g.map.plots[p4];
                    if desertb {
                        if pl4.terrain_type == Some(desert) && !(pl4.feature_type == Some(flood_plains)) {
                            values[m("WATER")] += 60;
                        }
                    }
                    if snowb && pl4.terrain_type == Some(snow) {
                        values[m("SUN")] += 120;
                    }
                    if tundrab && pl4.terrain_type == Some(tundra) {
                        values[m("SUN")] += 60;
                    }
                    if marshb && pl4.terrain_type == Some(marsh) {
                        values[m("SUN")] += 60;
                    }
                    if grassb && pl4.terrain_type == Some(grass) {
                        values[m("SUN")] += 60;
                    }
                    if hellterrb && global_counter > 20 {
                        // Python quirk: tests the mana plot (pPlot2), not pPlot4.
                        if let Some(t2) = g.map.plots[p2].terrain_type {
                            if hell.contains(&t2) {
                                values[m("LIFE")] += 100;
                            }
                        }
                    }
                }
            }
        }

        // ManaStackvalues
        let bonus = |i: usize| d.t::<BonusType>(&format!("BONUS_MANA_{}", MANA_NAMES[i]));
        for i in 0..MANA_NAMES.len() {
            let number_mana = g.player_get_num_available_bonuses(owner, bonus(i));
            values[i] += stack_mod[i] * 100 * number_mana;
        }

        // Values for Victory Condition
        for name in tower_mana(g.players[owner].tower_vic_flag) {
            let i = m(name);
            if g.player_get_num_available_bonuses(owner, bonus(i)) == 0 {
                values[i] += 30000;
            }
        }

        values
    }

    /// Python `CustomFunctions.warScript(iPlayer)` (called by `onBeginPlayerTurn` for
    /// non-human players). `startWar` is ported in ffh-core (`cf_start_war`).
    pub fn py_war_script(&self, g: &mut Game, p: PlayerId) {
        let d = g.d;
        let mut enemy: Option<PlayerId> = None;
        let tower = d.t::<BuildingClassType>("BUILDINGCLASS_TOWER_OF_MASTERY");
        let altar_divine = d.t::<BuildingType>("BUILDING_ALTAR_OF_THE_LUONNOTAR_DIVINE");
        let altar_exalted = d.t::<BuildingType>("BUILDING_ALTAR_OF_THE_LUONNOTAR_EXALTED");
        let evil = Some(d.t::<AlignmentType>("ALIGNMENT_EVIL"));
        let ashen_veil = Some(d.t::<ReligionType>("RELIGION_THE_ASHEN_VEIL"));
        let civ_is = |g: &Game, pl: PlayerId, name: &str| g.player_get_civilization_type(pl) == Some(d.t::<CivilizationType>(name));
        let rank = |g: &Game, pl: PlayerId| g.player_rank.get(pl).copied().unwrap_or(0);

        for p2 in g.player_ids() {
            if !g.players[p2].alive {
                continue;
            }
            let team = g.players[p].team;
            let team2 = g.players[p2].team;
            if !g.team_is_a_vassal(team) {
                if g.teams[team].at_war[team2] {
                    if self.rand(100, "War Script") < 5 {
                        self.py_dogpile(g, p, p2);
                    }
                }
                if self.py_war_script_allow(g, p, p2) {
                    if g.players[p2].building_class_making[tower] > 0 {
                        if g.team_get_at_war_count(team, true) == 0 {
                            g.cf_start_war(p, p2, WarPlanType::WARPLAN_TOTAL);
                        }
                    }
                    if g.player_get_num_building(p2, altar_divine) > 0 || g.player_get_num_building(p2, altar_exalted) > 0 {
                        if g.players[p].alignment == evil {
                            if g.team_get_at_war_count(team, true) == 0 {
                                g.cf_start_war(p, p2, WarPlanType::WARPLAN_TOTAL);
                            }
                        }
                    }
                    if civ_is(g, p, "CIVILIZATION_MERCURIANS") {
                        if g.player_get_state_religion(p2) == ashen_veil {
                            g.cf_start_war(p, p2, WarPlanType::WARPLAN_TOTAL);
                        }
                    }
                    if g.get_global_counter() > 20 {
                        if civ_is(g, p, "CIVILIZATION_SVARTALFAR") {
                            if civ_is(g, p2, "CIVILIZATION_LJOSALFAR") && rank(g, p) > rank(g, p2) {
                                g.cf_start_war(p, p2, WarPlanType::WARPLAN_TOTAL);
                            }
                        }
                        if civ_is(g, p, "CIVILIZATION_LJOSALFAR") {
                            if civ_is(g, p2, "CIVILIZATION_SVARTALFAR") && rank(g, p) > rank(g, p2) {
                                g.cf_start_war(p, p2, WarPlanType::WARPLAN_TOTAL);
                            }
                        }
                    }
                    if g.get_global_counter() > 40 || civ_is(g, p, "CIVILIZATION_INFERNAL") || civ_is(g, p, "CIVILIZATION_DOVIELLO") {
                        if g.players[p].alignment == evil {
                            if g.team_get_at_war_count(team, true) == 0 && rank(g, p2) > rank(g, p) {
                                if enemy.map_or(true, |e| rank(g, p2) > rank(g, e)) {
                                    enemy = Some(p2);
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(e) = enemy {
            // Python quirk: the enemy was picked with a worse rank than ours, so this never holds.
            if rank(g, p) > rank(g, e) {
                g.cf_start_war(p, e, WarPlanType::WARPLAN_TOTAL);
            }
        }
    }

    /// Python `CustomFunctions.warScriptAllow(iPlayer, iPlayer2)`.
    pub fn py_war_script_allow(&self, g: &Game, p: PlayerId, p2: PlayerId) -> bool {
        let d = g.d;
        let team = g.players[p].team;
        let team2 = g.players[p2].team;
        if p == g.barbarian_player() {
            return false;
        }
        if !g.teams[team].has_met[team2] {
            return false;
        }
        if g.teams[team].ai_at_peace_counter[team2] < 20 {
            return false;
        }
        // (Python: the attitude test is commented out.)
        if g.teams[team].at_war[team2] {
            return false;
        }
        if g.player_get_civilization_type(p) == Some(d.t::<CivilizationType>("CIVILIZATION_INFERNAL")) {
            if g.player_get_state_religion(p2) == Some(d.t::<ReligionType>("RELIGION_THE_ASHEN_VEIL")) {
                return false;
            }
        }
        true
    }

    /// Python `CustomFunctions.dogpile(iPlayer, iVictim)`.
    pub fn py_dogpile(&self, g: &mut Game, p: PlayerId, victim: PlayerId) {
        let d = g.d;
        let balseraphs = Some(d.t::<CivilizationType>("CIVILIZATION_BALSERAPHS"));
        for p2 in g.player_ids() {
            let mut chance = -1;
            if g.players[p2].alive {
                if self.py_dog_pile_allow(g, p, p2) && self.py_war_script_allow(g, p2, victim) {
                    // `pPlayer2.AI_getAttitude(iPlayer)`: p2's attitude (bForced = true), from
                    // p2's controller.
                    chance = g.player_ai_get_attitude(p2, p, true).i() * 5;
                    if chance > 0 {
                        chance = chance - (g.player_ai_get_attitude(p2, victim, true).i() * 5) - 10;
                        if !g.is_option(GameOptionType::GAMEOPTION_AGGRESSIVE_AI) {
                            chance -= 10;
                        }
                        if chance > 0 {
                            chance += py_div(g.get_global_counter(), 3);
                            if g.player_get_civilization_type(p2) == balseraphs {
                                chance = self.rand(50, "Dogpile");
                            }
                            if self.rand(100, "Dogpile") < chance {
                                g.cf_start_war(p2, victim, WarPlanType::WARPLAN_DOGPILE);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Python `CustomFunctions.dogPileAllow(iPlayer, iPlayer2)`.
    pub fn py_dog_pile_allow(&self, g: &Game, p: PlayerId, p2: PlayerId) -> bool {
        let d = g.d;
        let team = g.players[p].team;
        let team2 = g.players[p2].team;
        if p == p2 {
            return false;
        }
        if team == team2 {
            return false;
        }
        if g.players[p2].human {
            return false;
        }
        if g.player_get_civilization_type(p2) == Some(d.t::<CivilizationType>("CIVILIZATION_ELOHIM")) {
            return false;
        }
        if g.team_is_a_vassal(team2) {
            return false;
        }
        true
    }

    /// C++ `int CvUnit::chooseSpell()` (CvUnit.cpp:15917): the spell to cast (`None` =
    /// NO_SPELL).
    pub fn unit_choose_spell(&self, g: &Game, u: UnitId) -> Option<SpellType> {
        let d = g.d;
        let mut best_spell: Option<SpellType> = None;
        let mut best_spell_value = 0;
        let plot = g.unit_plot(u);
        let (px, py) = g.map.xy(plot);
        let team = g.unit_get_team(u);
        let owner = g.units[u].owner;

        for spell in d.spells.keys() {
            let mut value = 0;
            if crate::ai_can_cast(g, u, spell, false) {
                let si = &d.spells[spell];
                let range = si.range;
                if let Some(create_unit) = si.create_unit_type {
                    let move_range = d.units[create_unit].moves + g.unit_get_extra_spell_move(u);
                    let mut enemy = false;
                    for i in -move_range..=move_range {
                        for j in -move_range..=move_range {
                            if let Some(loop_plot) = g.map.plot_xy(px + i, py + j) {
                                if g.plot_is_visible_enemy_unit_for_unit(loop_plot, u) {
                                    enemy = true;
                                }
                            }
                        }
                    }
                    if enemy {
                        let mut temp_value = d.units[create_unit].combat;
                        for dt in d.damages.keys() {
                            temp_value += d.units[create_unit].damage_type_combat[dt];
                        }
                        temp_value *= 100;
                        temp_value *= si.create_unit_num;
                        value += temp_value;
                    }
                }
                if si.damage != 0 {
                    let dmg = si.damage;
                    let dmg_limit = si.damage_limit;
                    for i in -range..=range {
                        for j in -range..=range {
                            if let Some(loop_plot) = g.map.plot_xy(px + i, py + j) {
                                if loop_plot != plot {
                                    for &loop_unit in &g.map.plots[loop_plot].units {
                                        if !g.unit_is_immune_to_spell(loop_unit, u, spell) {
                                            if g.unit_is_enemy(loop_unit, Some(team), None) {
                                                if g.units[loop_unit].damage < dmg_limit {
                                                    value += dmg * 10;
                                                }
                                            }
                                            let loop_team = g.unit_get_team(loop_unit);
                                            if loop_team == team {
                                                value -= dmg * 20;
                                            }
                                            if loop_team != team && !g.unit_is_enemy(loop_unit, Some(team), None) {
                                                value -= 1000;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                for promotion in [si.add_promotion_type1, si.add_promotion_type2, si.add_promotion_type3].into_iter().flatten() {
                    value += self.unit_ai_promotion_value(g, u, promotion);
                }
                for promotion in [si.remove_promotion_type1, si.remove_promotion_type2, si.remove_promotion_type3].into_iter().flatten() {
                    value -= self.unit_ai_promotion_value(g, u, promotion);
                }
                if let Some(convert) = si.convert_unit_type {
                    let area = g.unit_area(u);
                    value += self.player_ai_unit_value(g, owner, convert, UnitAIType::UNITAI_ATTACK, area);
                    value -= self.player_ai_unit_value(g, owner, g.units[u].unit_type, UnitAIType::UNITAI_ATTACK, area);
                }
                if let Some(building) = si.create_building_type {
                    // C++ dereferences `plot()->getPlotCity()` (canCast requires a city).
                    if let Some(c) = plot_city_of(g, plot) {
                        value += self.city_ai_building_value(g, c, building, 0);
                    }
                }
                if si.create_feature_type.is_some() {
                    value += 10;
                }
                if si.create_improvement_type.is_some() {
                    value += 10;
                }
                if si.dispel {
                    value += 25 * (range + 1) * (range + 1);
                }
                if si.push {
                    value += 20 * (range + 1) * (range + 1);
                    if g.plot_is_city(plot, false, None) {
                        value *= 3;
                    }
                }
                if si.change_population != 0 {
                    value += 50 * si.change_population;
                }
                if si.cost != 0 {
                    value -= 4 * si.cost;
                }
                if si.immobile_turns != 0 {
                    value += 20 * si.immobile_turns * (range + 1) * (range + 1);
                }
                if si.sacrifice_caster {
                    let area = g.unit_area(u);
                    value -= g.units[u].level * self.player_ai_unit_value(g, owner, g.units[u].unit_type, UnitAIType::UNITAI_ATTACK, area);
                }
                if si.resistable {
                    value /= 2;
                }
                value += si.ai_weight;
                if value > best_spell_value {
                    best_spell_value = value;
                    best_spell = Some(spell);
                }
            }
        }
        best_spell
    }

    /// C++ `bool CvUnit::AI_canJoinGroup(CvSelectionGroup* pSelectionGroup) const`
    /// (CvUnit.cpp:18499). Unlike `canJoinGroup` it does not test `atPlot()`, so the AI can
    /// plan moves ahead.
    pub fn unit_ai_can_join_group(&self, g: &Game, u: UnitId, grp: GroupId) -> bool {
        let owner = g.units[u].owner;

        // C++ `getOwnerINLINE() == NO_PLAYER` (then the head unit's owner is compared) does not
        // happen in ffh-core: every group has an owner.
        if g.groups[grp].owner != owner {
            return false;
        }

        if !g.groups[grp].units.is_empty() {
            if g.group_get_domain_type(grp) != Some(g.unit_get_domain_type(u)) {
                return false;
            }

            if let Some(head_unit) = g.group_get_head_unit(grp) {
                if g.unit_is_hidden_nationality(head_unit) != g.unit_is_hidden_nationality(u) {
                    return false;
                }
                if g.unit_is_ai_control(head_unit) != g.unit_is_ai_control(u) {
                    return false;
                }
            }
        }

        true
    }

    /// C++ `int CvPlot::AI_sumStrength(PlayerTypes eOwner, PlayerTypes eAttackingPlayer =
    /// NO_PLAYER, DomainTypes eDomainType = NO_DOMAIN, bool bDefensiveBonuses = true, bool
    /// bTestAtWar = false, bool bTestPotentialEnemy = false) const` (CvPlot.cpp:2792).
    /// `owner == None` (NO_PLAYER) counts every owner's units.
    pub fn plot_ai_sum_strength(&self, g: &Game, plot: PlotIdx, owner: Option<PlayerId>, attacking_player: Option<PlayerId>, domain: Option<DomainType>, defensive_bonuses: bool, test_at_war: bool, test_potential_enemy: bool) -> i32 {
        let mut str_sum = 0;
        let attacking_team = attacking_player.map(|ap| g.players[ap].team);

        for &loop_unit in &g.map.plots[plot].units {
            if owner.is_none() || Some(g.units[loop_unit].owner) == owner {
                // C++ `isInvisible(eTeam, false)`: bDebug = false, bCheckCargo = true.
                if attacking_team.map_or(true, |at| !g.unit_is_invisible(loop_unit, at, true)) {
                    if !test_at_war || attacking_team.map_or(true, |at| g.at_war(at, g.unit_get_team(loop_unit))) {
                        if !test_potential_enemy || attacking_team.map_or(true, |at| g.unit_is_potential_enemy(loop_unit, Some(at), Some(plot))) {
                            // we may want to be more sophisticated about domains
                            // somewhere we need to check to see if this is a city, if so, only land units can defend here, etc
                            if domain.map_or(true, |dt| g.unit_get_domain_type(loop_unit) == dt) {
                                let str_plot = if defensive_bonuses { Some(plot) } else { None };
                                str_sum += g.unit_curr_effective_str(loop_unit, str_plot, None, None);
                            }
                        }
                    }
                }
            }
        }

        str_sum
    }

    /// C++ `int CvPlot::AI_neededPermDefenseReserve(int flag)` (CvPlot.cpp:10960): how many
    /// more of a given type the close cities need as defense.
    pub fn plot_ai_needed_perm_defense_reserve(&self, g: &Game, plot: PlotIdx, flag: i32) -> i32 {
        let search_range = 10;
        let mut defvalue = 0;
        let (x, y) = g.map.xy(plot);
        let owner = g.map.plots[plot].owner;
        let area = g.map.plots[plot].area;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                if let Some(loop_plot) = g.map.plot_xy(x + dx, y + dy) {
                    if g.plot_is_city(loop_plot, false, None) {
                        if g.map.plots[loop_plot].owner == owner {
                            // would be better to check if a path exists?
                            if g.map.plots[loop_plot].area == area {
                                if let Some(c) = plot_city_of(g, loop_plot) {
                                    defvalue += self.city_ai_needed_perm_defense(g, c, flag);
                                }
                            }
                        }
                    }
                }
            }
        }

        defvalue.max(0)
    }

    /// C++ `int CvPlot::AI_neededBetterDefender(int type)` (CvPlot.cpp:10992): how many more
    /// defenders of a given type can replace others in close cities.
    pub fn plot_ai_needed_better_defender(&self, g: &Game, plot: PlotIdx, type_: i32) -> i32 {
        let d = g.d;
        let Some(owner) = g.map.plots[plot].owner else {
            return 0;
        };
        let civili = if g.plot_is_city(plot, false, None) {
            match plot_city_of(g, plot) {
                Some(c) => Some(g.city_civilization_type(c)),
                None => g.player_get_civilization_type(owner),
            }
        } else {
            g.player_get_civilization_type(owner)
        };
        let Some(civili) = civili else {
            return 0;
        };
        let ci = &d.civilizations[civili];

        // make Sure we check a valid Unitclass
        match type_ {
            1 => {
                if ci.best_defender1.is_none() {
                    return 0;
                }
            }
            2 => {
                if ci.best_defender2.is_none() {
                    return 0;
                }
            }
            3 => {
                if ci.best_defender3.is_none() {
                    return 0;
                }
            }
            _ => return 0,
        }

        let mut unitsneeded = 0;
        let search_range = 8;
        let (x, y) = g.map.xy(plot);
        let area = g.map.plots[plot].area;

        for dx in -search_range..=search_range {
            for dy in -search_range..=search_range {
                if let Some(loop_plot) = g.map.plot_xy(x + dx, y + dy) {
                    if g.plot_is_city(loop_plot, false, None) && g.map.plots[loop_plot].owner == Some(owner) && g.map.plots[loop_plot].area == area {
                        for &loop_unit in &g.map.plots[loop_plot].units {
                            if g.units[loop_unit].ai_groupflag == GROUPFLAG_PERMDEFENSE && g.units[loop_unit].ai_type == Some(UnitAIType::UNITAI_CITY_DEFENSE) {
                                let class = Some(g.unit_get_unit_class_type(loop_unit));
                                if type_ == 1 {
                                    let mut bvalid = true;
                                    if class == ci.best_defender1 {
                                        bvalid = false;
                                    }
                                    if class == ci.best_defender2 {
                                        bvalid = false;
                                    }
                                    if class == ci.best_defender3 {
                                        bvalid = false;
                                    }
                                    if bvalid {
                                        unitsneeded += 1;
                                    }
                                }
                                if type_ == 2 {
                                    let mut bvalid = true;
                                    if class == ci.best_defender2 {
                                        bvalid = false;
                                    }
                                    if class == ci.best_defender3 {
                                        bvalid = false;
                                    }
                                    if bvalid {
                                        unitsneeded += 1;
                                    }
                                }
                                if type_ == 3 {
                                    let mut bvalid = true;
                                    if class == ci.best_defender3 {
                                        bvalid = false;
                                    }
                                    if bvalid {
                                        unitsneeded += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        unitsneeded
    }

    /// Python `CvEventManager.onUnitBuilt` (CvEventManager.py:1337-1374, Sephi): the UnitAI of
    /// a unit built by a non-human, non-barbarian player (the caller checks the player).
    pub fn py_unit_built_unit_ai(&self, g: &mut Game, u: UnitId) {
        let d = g.d;
        let p = g.units[u].owner;
        let num_cities = g.players[p].cities.len() as i32;
        let unit_type = g.units[u].unit_type;
        let units = g.players[p].units.clone();
        let ai_of = |g: &Game, x: UnitId| g.units[x].ai_type;

        //UNITAI for AdeptUnits
        if d.units[unit_type].unit_class_type == Some(d.t::<UnitClassType>("UNITCLASS_ADEPT")) {
            let mut terraformers_needed = num_cities / 4;
            if g.team_is_has_tech(g.players[p].team, Some(d.t::<TechType>("TECH_SORCERY"))) {
                terraformers_needed += 2;
            }
            let mut number_terraformer = 0;
            for &x in &units {
                if ai_of(g, x) == Some(UnitAIType::UNITAI_TERRAFORMER) {
                    number_terraformer += 1;
                }
                if ai_of(g, x) == Some(UnitAIType::UNITAI_MANA_UPGRADE) {
                    number_terraformer += 1;
                }
            }
            if number_terraformer < terraformers_needed {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_TERRAFORMER);
            } else {
                let plot = g.unit_plot(u);
                if self.plot_ai_needed_perm_defense_reserve(g, plot, 2) > 0 {
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_MAGE);
                } else {
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_WARWIZARD);
                }
            }
        }

        //UNITAI for Terraformers
        if unit_type == d.t::<UnitType>("UNIT_DEVOUT") {
            let number_terraformer =
                units.iter().filter(|&&x| g.units[x].unit_type == unit_type && ai_of(g, x) == Some(UnitAIType::UNITAI_TERRAFORMER)).count() as i32;
            if number_terraformer < (num_cities * g.global_counter) / 100 {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_TERRAFORMER);
            } else if number_terraformer < 3 {
                g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_TERRAFORMER);
            }
        }

        if unit_type == d.t::<UnitType>("UNIT_PRIEST_OF_LEAVES") {
            if g.player_get_state_religion(p) == Some(d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES")) {
                let number_terraformer =
                    units.iter().filter(|&&x| g.units[x].unit_type == unit_type && ai_of(g, x) == Some(UnitAIType::UNITAI_TERRAFORMER)).count() as i32;
                if number_terraformer * 3 < num_cities {
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_TERRAFORMER);
                } else if number_terraformer < 4 {
                    g.unit_ai_set_unit_ai_type(u, UnitAIType::UNITAI_TERRAFORMER);
                }
            }
        }
    }
}
