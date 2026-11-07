//! Batch B9 of the great-person cards (docs/CARDS.md): map and movement rules. Their callbacks
//! (the spells of Powder Charge, False Colours and the Wheeled City), and the predicates the rules
//! they change ask (movement, healing, builds, capture), which stay with those rules:
//!
//! - **Wall of the Kings** (world wonder, `border_stops_enemies`): land units at war with its owner,
//!   and barbarians, that enter the owner's borders end their move there (`plot_movement_cost`).
//! - **Harbor Chain** (building, `blocks_enemy_ships`): ships at war with the city's owner cannot
//!   enter the water plots next to the city (`unit_can_move_into`).
//! - **Siege Lines** (improvement, `enemy_city_adjacent`, `city_heal_tech`): built next to a city
//!   of a civ at war with the builder, in that civ's land too (`plot_can_build`); the holder's
//!   units on it heal as in a city of their own (`unit_heal_rate`).
//! - **Powder Charge** (unit and spell): next to a walled city at war with it, the Charge is used
//!   up and every Sentinel of the city is breached.
//! - **Prize Courts** (national building, `naval_capture_percent`): the owner's ships capture the
//!   ships they defeat, 50% (`unit_combat_won_as`).
//! - **Hall of Judgment** (national building, `conquest_keeps_all`): the owner's conquests keep every
//!   building that can be captured and lose no population (`player_acquire_city`).
//! - **Causeway** (improvement, `land_bridge`): land units move onto and over a Coast plot as over
//!   land; ships cannot enter it (a dam); +50% defence (its `defense_modifier`: a battle on water
//!   is the duel, which counts the plot's defence).
//! - **Canal Locks** (improvement, `canal`): ships can enter the land plot unless it is in the land
//!   of a civ at war with them.
//! - **Mountaineering** (tech `peak_movement`): the holder's land units enter peaks at 3 moves and
//!   get +50% defence and sight 3 there.
//! - **Ferries** (tech `strait_crossing`): the holder's land units enter a one-plot strait as land.
//! - **Wheeled City** (building, spells): Pack Up turns the city into a City Caravan that keeps its
//!   population, buildings and tiers ([`PackedCity`]); Unpack refounds it within 12 plots.
//! - **Forest Paths** (tech `forest_paths`): in the holder's borders its units move through forest
//!   and jungle as through open ground; units at war with it that start their turn there have one
//!   move fewer (`unit_do_turn`, the path finder's turn starts).
//! - **Wayside Shrines** (improvement, `requires_route`, `city_heal_tech`): built on a road; the
//!   holder's units on it heal as in a city of their own.
//! - **False Colours** (unit, spell, promotion `false_colours`): the stack may attack units of a
//!   civ at peace with it without a declaration; the war is declared at the battle, whose defenders
//!   get no defensive strikes and no fortification.

use std::sync::OnceLock;

use crate::ffh::dispatch::*;
use crate::ffh::pycall::PyCall;
use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// `PyResult` functions.
pub fn find_spell(name: &str) -> Option<SpellFn> {
    let f: SpellFn = match name {
        "spellPowderCharge" => spell_powder_charge,
        "spellPackUp" => spell_pack_up,
        "spellUnpack" => spell_unpack,
        _ => return None,
    };
    Some(f)
}

/// `PyRequirement` functions.
pub fn find_req(name: &str) -> Option<ReqFn> {
    let f: ReqFn = match name {
        "reqPowderCharge" => req_powder_charge,
        "reqFalseColours" => req_false_colours,
        "reqPackUp" => req_pack_up,
        "reqUnpack" => req_unpack,
        _ => return None,
    };
    Some(f)
}

/// How far from where it packed up a City Caravan may refound its city (Wheeled City).
pub const UNPACK_RANGE: i32 = 12;
/// What a peak costs a Mountaineering unit to enter, in moves.
pub const PEAK_MOVES: i32 = 3;
/// A Mountaineering unit's defence on a peak, in percent.
pub const PEAK_DEFENSE: i32 = 50;
/// A Mountaineering unit's sight on a peak.
pub const PEAK_SIGHT: i32 = 3;

/// The data the batch's rules read in hot paths, looked up once (the game data is one global).
pub struct MapRules {
    pub peak_techs: Vec<TechType>,
    pub strait_techs: Vec<TechType>,
    /// Each Forest Paths tech and the features it opens.
    pub forest_techs: Vec<(TechType, Vec<FeatureType>)>,
    /// Per improvement: Causeway's land bridge, Canal Locks' canal.
    pub land_bridge: Vec<bool>,
    pub canal: Vec<bool>,
    pub land_bridges: Vec<ImprovementType>,
    pub border_stop_classes: Vec<BuildingClassType>,
    pub ship_block_buildings: Vec<BuildingType>,
    pub conquest_keep_classes: Vec<BuildingClassType>,
    pub naval_capture: Vec<(BuildingClassType, i32)>,
    pub false_colours: Vec<PromotionType>,
    pub peak_terrain: Option<TerrainType>,
}

static RULES: OnceLock<(usize, MapRules)> = OnceLock::new();

/// The batch's data (built on first use; see [`MapRules`]).
pub fn rules(d: &'static GameData) -> &'static MapRules {
    let (ptr, r) = RULES.get_or_init(|| (d as *const GameData as usize, MapRules::build(d)));
    assert_eq!(*ptr, d as *const GameData as usize, "map rules were built for different GameData");
    r
}

impl MapRules {
    fn build(d: &GameData) -> MapRules {
        let techs = |f: &dyn Fn(&TechInfo) -> bool| d.techs.keys().filter(|&t| f(&d.techs[t])).collect::<Vec<_>>();
        let classes = |f: &dyn Fn(&BuildingInfo) -> bool| {
            let mut v: Vec<BuildingClassType> = d.buildings.iter().filter(|b| f(b)).filter_map(|b| b.building_class_type).collect();
            v.sort();
            v.dedup();
            v
        };
        MapRules {
            peak_techs: techs(&|t| t.peak_movement),
            strait_techs: techs(&|t| t.strait_crossing),
            forest_techs: d.techs.keys().filter(|&t| !d.techs[t].forest_paths.is_empty()).map(|t| (t, d.techs[t].forest_paths.clone())).collect(),
            land_bridge: d.improvements.iter().map(|i| i.land_bridge).collect(),
            canal: d.improvements.iter().map(|i| i.canal).collect(),
            land_bridges: d.improvements.keys().filter(|&i| d.improvements[i].land_bridge).collect(),
            border_stop_classes: classes(&|b| b.border_stops_enemies),
            ship_block_buildings: d.buildings.keys().filter(|&b| d.buildings[b].blocks_enemy_ships).collect(),
            conquest_keep_classes: classes(&|b| b.conquest_keeps_all),
            naval_capture: d.buildings.iter().filter(|b| b.naval_capture_percent > 0).filter_map(|b| b.building_class_type.map(|c| (c, b.naval_capture_percent))).collect(),
            false_colours: d.promotions.keys().filter(|&p| d.promotions[p].false_colours).collect(),
            peak_terrain: d.ty::<TerrainType>("TERRAIN_PEAK"),
        }
    }
}

/// A real building a City Caravan carries: its count, original owner and the turn it was first
/// built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedBuilding {
    pub building: BuildingType,
    pub count: i32,
    pub original_owner: Option<PlayerId>,
    pub original_time: i32,
}

/// What a City Caravan carries (Wheeled City): the city it was, to refound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedCity {
    pub name: String,
    /// Where it packed up: it refounds within [`UNPACK_RANGE`] of it.
    pub origin: PlotIdx,
    pub population: i32,
    /// Each real building.
    pub buildings: Vec<PackedBuilding>,
    /// The city's building tiers (docs/SUITABILITY.md).
    pub tiers: IdxVec<BuildingClassType, BuildingTierType>,
}

impl Game {
    // ---------------------------------------------------------------------------------------
    // The holder's capabilities (card techs).

    /// Mountaineering: `t`'s land units can enter peaks.
    pub fn team_peak_movement(&self, t: TeamId) -> bool {
        rules(self.d).peak_techs.iter().any(|&tech| self.teams[t].has_tech[tech])
    }

    /// Ferries: `t`'s land units can enter one-plot straits as land.
    pub fn team_strait_crossing(&self, t: TeamId) -> bool {
        rules(self.d).strait_techs.iter().any(|&tech| self.teams[t].has_tech[tech])
    }

    /// Forest Paths: `t`'s units move through feature `f` in its borders as through open ground.
    pub fn team_forest_path(&self, t: TeamId, f: FeatureType) -> bool {
        rules(self.d).forest_techs.iter().any(|(tech, fs)| self.teams[t].has_tech[*tech] && fs.contains(&f))
    }

    // ---------------------------------------------------------------------------------------
    // The plots.

    /// Causeway: plot `p` has a land bridge (land units walk on it, ships cannot enter it).
    #[inline]
    pub fn plot_is_land_bridge(&self, p: PlotIdx) -> bool {
        self.map.plots[p].improvement_type.is_some_and(|i| rules(self.d).land_bridge[i.idx()])
    }

    /// Canal Locks: plot `p` has a canal (ships can enter it).
    #[inline]
    pub fn plot_is_canal(&self, p: PlotIdx) -> bool {
        self.map.plots[p].improvement_type.is_some_and(|i| rules(self.d).canal[i.idx()])
    }

    /// Ferries: `p` is a one-plot strait, a water plot with land on two opposite sides.
    pub fn plot_is_strait(&self, p: PlotIdx) -> bool {
        if !self.plot_is_water(p) {
            return false;
        }
        let (x, y) = self.map.xy(p);
        let land = |dx: i32, dy: i32| self.map.plot_xy(x + dx, y + dy).is_some_and(|q| !self.plot_is_water(q));
        (land(0, 1) && land(0, -1)) || (land(1, 0) && land(-1, 0)) || (land(1, 1) && land(-1, -1)) || (land(1, -1) && land(-1, 1))
    }

    /// Whether land unit `u` walks on water plot `p` as on land: a Causeway, or a strait for a
    /// Ferries holder (not as cargo: a unit aboard a ship stays aboard).
    pub fn unit_walks_on_water(&self, u: UnitId, p: PlotIdx) -> bool {
        self.plot_is_land_bridge(p)
            || (self.plot_is_water(p) && self.team_strait_crossing(self.unit_get_team(u)) && self.plot_is_strait(p) && !self.unit_is_cargo(u))
    }

    /// Whether plot `p` is land for land unit `u`: not water, or water it walks on.
    pub fn unit_is_on_land_at(&self, u: UnitId, p: PlotIdx) -> bool {
        !self.plot_is_water(p) || self.unit_walks_on_water(u, p)
    }

    /// Whether any Causeway stands in the world (the AI path finder then lets land groups reach
    /// other land areas).
    pub fn map_has_land_bridges(&self) -> bool {
        let r = rules(self.d);
        !r.land_bridges.is_empty() && self.map.areas.iter().any(|(_, a)| r.land_bridges.iter().any(|&i| a.num_improvements[i] > 0))
    }

    /// Canal Locks: whether ship `u` may enter the canal on `p` (not in the land of a civ at war
    /// with it).
    pub fn unit_can_use_canal(&self, u: UnitId, p: PlotIdx) -> bool {
        self.plot_is_canal(p) && !self.plot_get_team(p).is_some_and(|t| self.unit_is_enemy(u, Some(t), Some(p)))
    }

    /// Canal Locks: whether a canal on `p` would touch water or another canal.
    pub fn plot_canal_connected(&self, p: PlotIdx) -> bool {
        let (x, y) = self.map.xy(p);
        (0..DirectionType::NUM).any(|i| {
            self.plot_direction(x, y, DirectionType::from_usize(i)).is_some_and(|q| self.plot_is_water(q) || self.plot_is_canal(q))
        })
    }

    /// Siege Lines: whether `p` is next to a city of a civ `team` is at war with.
    pub fn plot_next_to_enemy_city(&self, p: PlotIdx, team: TeamId) -> bool {
        let (x, y) = self.map.xy(p);
        (0..DirectionType::NUM).any(|i| {
            self.plot_direction(x, y, DirectionType::from_usize(i)).is_some_and(|q| {
                self.map.plots[q].plot_city.is_some_and(|c| self.cities.contains(c) && self.teams[team].at_war[self.city_team(c)])
            })
        })
    }

    /// Siege Lines, Wayside Shrines: whether `u`'s team heals on `p` as in a city of its own (the
    /// plot's improvement names a tech it knows).
    pub fn unit_heals_as_in_city_at(&self, u: UnitId, p: PlotIdx) -> bool {
        self.map.plots[p]
            .improvement_type
            .and_then(|i| self.d.improvements[i].city_heal_tech)
            .is_some_and(|t| self.team_is_has_tech(self.unit_get_team(u), Some(t)))
    }

    // ---------------------------------------------------------------------------------------
    // The buildings' owners.

    /// Wall of the Kings: whether `p` holds a building that stops enemies at its borders.
    pub fn player_border_stops_enemies(&self, p: PlayerId) -> bool {
        rules(self.d).border_stop_classes.iter().any(|&c| self.players[p].building_class_count[c] > 0)
    }

    /// Wall of the Kings: whether land unit `u` moving from `from` to `to` enters the borders of a
    /// Wall holder it is an enemy of, and so ends its move there.
    pub fn unit_stopped_at_border(&self, u: UnitId, from: PlotIdx, to: PlotIdx) -> bool {
        let Some(owner) = self.map.plots[to].owner else { return false };
        if self.map.plots[from].owner == Some(owner) || rules(self.d).border_stop_classes.is_empty() {
            return false;
        }
        self.player_border_stops_enemies(owner)
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && self.unit_is_enemy(u, Some(self.players[owner].team), Some(to))
    }

    /// Harbor Chain: whether ship `u` is kept out of water plot `p` by a city next to it that it
    /// is an enemy of.
    pub fn unit_harbor_blocked(&self, u: UnitId, p: PlotIdx) -> bool {
        let r = rules(self.d);
        if r.ship_block_buildings.is_empty() {
            return false;
        }
        let (x, y) = self.map.xy(p);
        (0..DirectionType::NUM).any(|i| {
            let Some(q) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { return false };
            let Some(c) = self.map.plots[q].plot_city.filter(|&c| self.cities.contains(c)) else { return false };
            r.ship_block_buildings.iter().any(|&b| self.city_num_active_building(c, b) > 0) && self.unit_is_enemy(u, Some(self.city_team(c)), Some(p))
        })
    }

    /// Hall of Judgment: whether `p`'s conquests keep every building and their population.
    pub fn player_conquest_keeps_all(&self, p: PlayerId) -> bool {
        rules(self.d).conquest_keep_classes.iter().any(|&c| self.players[p].building_class_count[c] > 0)
    }

    /// Prize Courts: `p`'s ships' chance to capture a ship they defeat, in percent (the most any
    /// of its buildings gives).
    pub fn player_naval_capture_percent(&self, p: PlayerId) -> i32 {
        rules(self.d).naval_capture.iter().filter(|(c, _)| self.players[p].building_class_count[*c] > 0).map(|&(_, pct)| pct).max().unwrap_or(0)
    }

    // ---------------------------------------------------------------------------------------
    // Units.

    /// Mountaineering: whether land unit `u` can climb onto peak `p`.
    pub fn unit_can_climb(&self, u: UnitId, p: PlotIdx) -> bool {
        self.plot_is_peak(p)
            && !self.map.plots[p].feature_type.is_some_and(|f| self.d.features[f].impassable)
            && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND
            && self.team_peak_movement(self.unit_get_team(u))
    }

    /// Mountaineering: whether `u` stands on a peak with the card's defence and sight.
    pub fn unit_on_climbed_peak(&self, u: UnitId, p: PlotIdx) -> bool {
        self.plot_is_peak(p) && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND && self.team_peak_movement(self.unit_get_team(u))
    }

    /// Mountaineering: the team's land units on peaks lose (`increment` false) or regain their
    /// sight, around a change of the card (their sight range depends on it).
    pub fn team_change_peak_sight(&mut self, t: TeamId, increment: bool) {
        let mut on_peaks = Vec::new();
        for p in self.player_ids() {
            if self.players[p].team != t {
                continue;
            }
            for &u in &self.players[p].units {
                if self.units.contains(u) && self.units[u].x >= 0 {
                    if let Some(plot) = self.unit_plot_opt(u) {
                        if self.plot_is_peak(plot) && self.unit_get_domain_type(u) == DomainType::DOMAIN_LAND {
                            on_peaks.push((u, plot));
                        }
                    }
                }
            }
        }
        for (u, plot) in on_peaks {
            let range = self.unit_visibility_range(u);
            self.plot_change_adjacent_sight(plot, t, range, increment, Some(u), true);
        }
    }

    /// Forest Paths: whether `u` stands in a forest or jungle of a Forest Paths holder, in its
    /// borders, and is its enemy.
    pub fn unit_slowed_by_forest(&self, u: UnitId, p: PlotIdx) -> bool {
        let plot = &self.map.plots[p];
        let (Some(f), Some(owner)) = (plot.feature_type, plot.owner) else { return false };
        if rules(self.d).forest_techs.is_empty() {
            return false;
        }
        let t = self.players[owner].team;
        self.team_forest_path(t, f) && self.unit_is_enemy(u, Some(t), Some(p))
    }

    /// Forest Paths: the moves `u` starts a turn with on plot `p` (one fewer in an enemy's
    /// Forest Paths, never below one).
    pub fn unit_turn_moves_at(&self, u: UnitId, p: PlotIdx) -> i32 {
        let max = self.unit_max_moves(u);
        let one = self.d.defines.move_denominator;
        if max > one && self.unit_slowed_by_forest(u, p) {
            max - one
        } else {
            max
        }
    }

    /// False Colours: whether `u` is under false colours.
    pub fn unit_has_false_colours(&self, u: UnitId) -> bool {
        rules(self.d).false_colours.iter().any(|&p| self.units[u].has_promotion[p])
    }

    /// False Colours: the team `u`'s false colours let it attack on land plot `plot`: one at peace
    /// with its own (not its vassal or master) whose units there it sees (the first in the plot's
    /// order).
    pub fn unit_false_colours_team(&self, u: UnitId, plot: PlotIdx) -> Option<TeamId> {
        if !self.unit_has_false_colours(u) || self.plot_is_water(plot) || self.is_option(GameOptionType::GAMEOPTION_ALWAYS_PEACE) {
            return None;
        }
        let team = self.unit_get_team(u);
        if self.team_is_crusade_host(team) {
            return None;
        }
        self.map.plots[plot].units.iter().copied().find_map(|d| {
            let dt = self.unit_get_team(d);
            (dt != team
                && !self.teams[team].at_war[dt]
                && !self.team_is_barbarian(dt)
                && !self.teams[team].vassal[dt]
                && !self.teams[dt].vassal[team]
                && !self.team_is_crusade_host(dt)
                && !self.unit_is_invisible(d, team, false))
            .then_some(dt)
        })
    }

    /// False Colours: the team the group `g`'s false colours let it attack on `dest`, if any of
    /// its owner's units on its plot is under them.
    pub fn group_false_colours_team(&self, g: GroupId, dest: PlotIdx) -> Option<TeamId> {
        let owner = self.groups[g].owner;
        let from = self.group_plot(g)?;
        self.map.plots[from].units.iter().filter(|&&u| self.units[u].owner == owner).find_map(|&u| self.unit_false_colours_team(u, dest))
    }

    /// False Colours, at the attack (`group_attack_engage`): the war on the target team is declared
    /// now, the defenders lose their fortification, and the stack's colours are spent (one
    /// battle). Returns whether the battle is a surprise (the war began with it).
    pub(crate) fn group_false_colours_strike(&mut self, g: GroupId, dest: PlotIdx) -> bool {
        let Some(target) = self.group_false_colours_team(g, dest) else { return false };
        let Some(from) = self.group_plot(g) else { return false };
        let owner = self.groups[g].owner;
        let team = self.players[owner].team;
        self.team_declare_war(team, target, true, None);
        if !self.teams[team].at_war[target] {
            return false;
        }
        for d in self.map.plots[dest].units.clone() {
            if self.units.contains(d) && self.unit_is_enemy(d, Some(team), Some(dest)) {
                self.unit_set_fortify_turns(d, 0);
            }
        }
        let promotions = rules(self.d).false_colours.clone();
        for u in self.map.plots[from].units.clone() {
            if self.units.contains(u) && self.units[u].owner == owner {
                for &p in &promotions {
                    if self.units[u].has_promotion[p] {
                        self.unit_set_has_promotion(u, p, false);
                    }
                }
            }
        }
        let turn = self.game_turn();
        self.log.msg(turn, Some(owner), || format!("False Colours: a surprise attack, war on team {}", target));
        true
    }

    // ---------------------------------------------------------------------------------------
    // Powder Charge.

    /// Powder Charge: the walled city next to `u` that its charge would breach: a city of a team
    /// at war with it with Sentinels standing (the most, then the first in direction order).
    pub fn unit_powder_charge_city(&self, u: UnitId) -> Option<CityId> {
        let team = self.unit_get_team(u);
        let (x, y) = self.map.xy(self.unit_plot_opt(u)?);
        let mut best: Option<(CityId, usize)> = None;
        for i in 0..DirectionType::NUM {
            let Some(q) = self.plot_direction(x, y, DirectionType::from_usize(i)) else { continue };
            let Some(c) = self.map.plots[q].plot_city.filter(|&c| self.cities.contains(c)) else { continue };
            if !self.teams[team].at_war[self.city_team(c)] {
                continue;
            }
            let n = self.city_sentinels_standing(c).len();
            if n > 0 && best.map_or(true, |(_, m)| n > m) {
                best = Some((c, n));
            }
        }
        best.map(|(c, _)| c)
    }

    // ---------------------------------------------------------------------------------------
    // Wheeled City.

    /// Wheeled City: whether `c` may pack up (not the capital, not a holy city).
    pub fn city_can_pack_up(&self, c: CityId) -> bool {
        !self.city_is_capital(c) && !self.d.religions.keys().any(|r| self.city_is_holy_city(c, r)) && self.cities[c].population > 0
    }

    /// Wheeled City, Pack Up: city `c` becomes a unit of type `caravan` on its plot carrying its
    /// population, buildings, tiers and name; its culture is lost. Returns the caravan.
    pub fn city_pack_up(&mut self, c: CityId, caravan: UnitType) -> Option<UnitId> {
        if !self.city_can_pack_up(c) {
            return None;
        }
        let d = self.d;
        let city = &self.cities[c];
        let owner = city.owner;
        let (x, y) = (city.x, city.y);
        let plot = self.city_plot(c);
        let buildings: Vec<PackedBuilding> = d
            .buildings
            .keys()
            .filter(|&b| city.num_real_building[b] > 0)
            .map(|b| PackedBuilding {
                building: b,
                count: city.num_real_building[b],
                original_owner: city.building_original_owner[b],
                original_time: city.building_original_time[b],
            })
            .collect();
        let packed = PackedCity { name: city.name.clone(), origin: plot, population: city.population, buildings, tiers: city.building_tiers.clone() };
        self.city_kill(c, true);
        // The city moved away: no ruins.
        if self.map.plots[plot].improvement_type.is_some() && opt_to_i32(self.map.plots[plot].improvement_type) == d.defines.ruins_improvement {
            self.plot_set_improvement_type(plot, None);
        }
        let u = self.player_init_unit(owner, caravan, x, y, None, None);
        self.units[u].packed_city = Some(packed);
        let turn = self.game_turn();
        let name = self.units[u].packed_city.as_ref().map(|pc| pc.name.clone()).unwrap_or_default();
        self.log.msg_at(turn, Some(owner), (x, y), || format!("{} packs up", d.text.name(&name)));
        Some(u)
    }

    /// Wheeled City: whether caravan `u` can refound its city where it stands (a valid site within
    /// [`UNPACK_RANGE`] of where it packed up).
    pub fn unit_can_unpack(&self, u: UnitId) -> bool {
        let Some(pc) = self.units[u].packed_city.as_ref() else { return false };
        let Some(plot) = self.unit_plot_opt(u) else { return false };
        let (x, y) = self.map.xy(plot);
        let (ox, oy) = self.map.xy(pc.origin);
        self.plot_distance(x, y, ox, oy) <= UNPACK_RANGE && self.player_can_found(self.units[u].owner, x, y, false)
    }

    /// Wheeled City, Unpack: caravan `u` refounds its city where it stands, with its population,
    /// buildings (where they can stand) and tiers. Returns the city.
    pub fn unit_unpack(&mut self, u: UnitId) -> Option<CityId> {
        if !self.unit_can_unpack(u) {
            return None;
        }
        let pc = self.units[u].packed_city.take()?;
        let owner = self.units[u].owner;
        let (x, y) = (self.units[u].x, self.units[u].y);
        let c = self.player_init_city_as(owner, x, y, true, true, Some(pc.tiers.clone()));
        self.city_set_name(c, &pc.name, false);
        self.city_set_population(c, pc.population.max(1));
        for pb in &pc.buildings {
            let b = pb.building;
            let class = self.d.buildings[b].building_class_type;
            if !self.city_is_valid_building_location(c, b) || class.is_some_and(|k| self.player_is_production_maxed_building_class(owner, k, true)) {
                continue;
            }
            let value = (self.cities[c].num_real_building[b] + pb.count).min(self.d.defines.city_max_num_buildings);
            self.city_set_num_real_building_timed(c, b, value, false, pb.original_owner, pb.original_time);
        }
        let turn = self.game_turn();
        let name = pc.name.clone();
        let d = self.d;
        self.log.msg_at(turn, Some(owner), (x, y), || format!("{} is refounded", d.text.name(&name)));
        Some(c)
    }
}

// -------------------------------------------------------------------------------------------
// The callbacks.

/// `reqPowderCharge(caster)`: next to a walled city at war with it.
fn req_powder_charge(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.unit_powder_charge_city(caster).is_some()
}

/// `spellPowderCharge(caster)`: every Sentinel of that city is breached (the spell uses the
/// Charge up); no battle.
fn spell_powder_charge(g: &mut Game, caster: UnitId, _call: &PyCall) {
    let Some(c) = g.unit_powder_charge_city(caster) else { return };
    g.city_breach_sentinels(c);
    let (turn, owner, at) = (g.game_turn(), g.units[caster].owner, (g.cities[c].x, g.cities[c].y));
    let name = g.cities[c].name.clone();
    let d = g.d;
    g.log.msg_at(turn, Some(owner), at, || format!("Powder Charge: the walls of {} are breached", d.text.name(&name)));
}

/// `reqFalseColours(caster)`: a unit of its owner on its plot that can attack and is not under
/// false colours yet.
fn req_false_colours(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    let Some(plot) = g.unit_plot_opt(caster) else { return false };
    let owner = g.units[caster].owner;
    g.map.plots[plot].units.iter().any(|&u| u != caster && g.units[u].owner == owner && g.unit_can_attack(u) && !g.unit_has_false_colours(u))
}

/// `reqPackUp(caster)`: in a city of its owner that may pack up (the spell's `building_prereq` is
/// the Wheeled City).
fn req_pack_up(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    let Some(plot) = g.unit_plot_opt(caster) else { return false };
    g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c)).is_some_and(|c| g.cities[c].owner == g.units[caster].owner && g.city_can_pack_up(c))
}

/// `spellPackUp(caster, 'UNIT_X')`: the city becomes a caravan of that type.
fn spell_pack_up(g: &mut Game, caster: UnitId, call: &PyCall) {
    let Some(plot) = g.unit_plot_opt(caster) else { return };
    let Some(c) = g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c)) else { return };
    let Some(caravan) = call.str_arg(1).and_then(|n| g.d.ty::<UnitType>(n)) else { return };
    g.city_pack_up(c, caravan);
}

/// `reqUnpack(caster)`: a caravan on a valid site within range of where it packed up.
fn req_unpack(g: &Game, caster: UnitId, _call: &PyCall) -> bool {
    g.unit_can_unpack(caster)
}

/// `spellUnpack(caster)`: the city is refounded (the spell uses the caravan up).
fn spell_unpack(g: &mut Game, caster: UnitId, _call: &PyCall) {
    g.unit_unpack(caster);
}
