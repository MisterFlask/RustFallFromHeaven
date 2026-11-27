//! Cataclysm game-mode adapter for `ffh-play` and `ffh-web`.
//!
//! Runs `cataclysm-core` + `cataclysm-ai` behind the exact same transport, map arrays, HD-2D / ISO
//! art indices, portrait keys, and `battle_watch.js` v2 battle logs as classic FFH2 — so 100% of
//! the visual art and animation stack is reused with zero Civ4 simulation underneath.

use std::cell::Cell;

use cataclysm_core::apocalypse::{CARCOSA_PROTECTED_TURNS, CARCOSA_TIMER_TURNS, MAX_GAME_TURNS};
use cataclysm_core::supply::supply_combat_pct;
use cataclysm_core::{
    BattleId, BattleRecord, BuildOrder, CityId, Command, CrusadeId, Doctrine, Event, FactionId, Game, GameSetup, GreatPersonAction,
    GreatPersonId, GreatSuit, House, HouseId, HouseMood, LifeboatKind, MemberId, PlayerId, PlotIdx, Religion, Resource, RitualKind, SallyPosture,
    SquadId, TechId, UnitClassId, VictoryKind, WonderKind, ARMAGEDDON_CRISES, DISTRICTS, FACTIONS, SECRET_TECHS, TECHS, UNIT_CLASSES,
};
use ffh_battle::{ActionKind, Fate, GroundRule, Note, RowActions, Side, ROWS};
use ffh_data::{BonusType, CivilizationType, FeatureType, GameData, Idx, ImprovementType, LeaderHeadType, RouteType, TerrainType, UnitType};

use crate::json::Json;
use crate::transport::ProtoErr;
use crate::view::map::{MapRows, FOGGED, NUM_LAYERS, UNREVEALED, VISIBLE};

/// Cataclysm-specific query names accepted by `Host` / `CataclysmSession` (verified by `tests/web.rs`).
pub const CATACLYSM_QUERY_NAMES: &[&str] = &["cataclysm_setup", "cataclysm_preview"];

/// Cataclysm-specific command names accepted by `CataclysmSession` (verified by `tests/web.rs`).
pub const CATACLYSM_COMMAND_NAMES: &[&str] = &[
    "cat_move",
    "cat_attack",
    "cat_set_cell",
    "cat_auto_arrange",
    "cat_reinforce",
    "cat_upgrade",
    "cat_raid",
    "cat_fortify",
    "cat_road",
    "cat_fort",
    "cat_explore",
    "cat_found_city",
    "cat_disband",
    "cat_city_build",
    "cat_sally",
    "cat_raze",
    "cat_caravan",
    "cat_research",
    "cat_doctrine",
    "cat_religion",
    "cat_ritual",
    "cat_gp",
    "cat_war",
    "cat_peace",
    "cat_crusade",
    "cat_lend",
    "cat_payoff",
];

/// Setup metadata served to the Cataclysm New Game screen.
pub fn setup_json(d: &GameData) -> Json {
    let factions: Vec<Json> = FACTIONS
        .iter()
        .map(|f| {
            crate::jobj! {
                "key" => f.key,
                "name" => f.name,
                "leader_name" => f.leader_name,
                "civ" => d.ty::<CivilizationType>(f.civ_art).map(Idx::idx),
                "leader" => d.ty::<LeaderHeadType>(f.leader_art).map(Idx::idx),
                "leader_art" => f.leader_art,
                "alignment" => f.alignment.name(),
                "can_mortal_win" => f.can_mortal_win,
                "can_demonic_win" => f.can_demonic_win,
                "lore_blurb" => f.lore_blurb,
                "rule_summary" => f.rule_summary,
                "victory_lines" => victory_lines(f.id),
            }
        })
        .collect();
    let sizes = vec![
        crate::jobj! { "key" => "small", "name" => "Small (22x16)", "w" => 22, "h" => 16 },
        crate::jobj! { "key" => "standard", "name" => "Standard (26x18)", "w" => 26, "h" => 18 },
        crate::jobj! { "key" => "large", "name" => "Large (32x22)", "w" => 32, "h" => 22 },
    ];
    crate::jobj! {
        "mode" => "cataclysm",
        "max_turns" => MAX_GAME_TURNS,
        "factions" => Json::Arr(factions),
        "sizes" => Json::Arr(sizes),
    }
}

pub struct CataclysmSession {
    pub g: Game,
    pub d: &'static GameData,
    pub human: PlayerId,
    pub reveal: Cell<bool>,
    pub view_version: u64,
    pub game_tag: String,
    pub last_report_lines: Vec<String>,
    pub last_report_battles: Vec<u32>,
}

impl CataclysmSession {
    pub fn new(setup_json: &Json) -> Result<Self, ProtoErr> {
        let d = ffh_data::global();
        let seed = setup_json.get("seed").and_then(Json::as_i64).unwrap_or(42) as u64;
        let ai_count = setup_json.get("opponents").and_then(Json::as_i64).unwrap_or(5).clamp(1, 7) as usize;
        let (width, height) = match setup_json.get("size").and_then(Json::as_str).unwrap_or("standard") {
            "small" => (22, 16),
            "large" => (32, 22),
            _ => (26, 18),
        };
        let fac_key = setup_json.get("faction").and_then(Json::as_str).unwrap_or("bannor");
        let human_faction = FACTIONS.iter().find(|f| f.key == fac_key).map_or(FactionId::Bannor, |f| f.id);

        let g = Game::new(GameSetup { seed, width, height, human_faction, ai_count });
        let mut sess = Self {
            g,
            d,
            human: PlayerId(0),
            reveal: Cell::new(false),
            view_version: 1,
            game_tag: format!("cataclysm-{fac_key}-{seed}"),
            last_report_lines: Vec::new(),
            last_report_battles: Vec::new(),
        };
        let first = sess.report_line_default();
        sess.last_report_lines.push(first);
        Ok(sess)
    }

    pub fn set_reveal(&self, reveal: bool) {
        self.reveal.set(reveal);
    }

    fn is_revealed(&self, plot: PlotIdx) -> bool {
        self.reveal.get() || self.g.world.is_revealed(self.human, plot)
    }

    fn is_visible(&self, plot: PlotIdx) -> bool {
        self.reveal.get() || self.g.world.is_visible(self.human, plot)
    }

    pub fn status(&self) -> Json {
        // A human with no city and no squad left is out of the game: the client is told so, though
        // the rules' own outcome waits for the other civilizations.
        let eliminated = !self.g.players[self.human.idx()].alive;
        let phase = if self.g.is_over() || eliminated { "over" } else { "human" };
        let over = match &self.g.apocalypse.outcome {
            Some(o) => Some(self.over_json(o)),
            None if eliminated => Some(self.eliminated_json()),
            None => None,
        };
        crate::jobj! {
            "mode" => "cataclysm",
            "phase" => phase,
            "turn" => self.g.turn,
            "year" => self.g.turn,
            "human" => self.human.0 as i32,
            "active_players" => vec![self.g.active_player.0 as i32],
            "view_version" => self.view_version as i64,
            "decision" => Json::Null,
            "over" => over,
            "reveal" => self.reveal.get(),
            "ai_auto_play" => 0,
            "observe" => false,
            "build" => crate::BUILD_COMMIT,
            "game" => crate::jobj! { "dir" => self.game_tag.as_str(), "journal" => self.game_tag.as_str(), "autosave" => Json::Null },
        }
    }

    pub fn map_rows(&self) -> MapRows {
        let w = self.g.world.w;
        let h = self.g.world.h;
        let d = self.d;
        let road_idx = d.ty::<RouteType>("ROUTE_ROAD").map_or(-1, |r| r.idx() as i32);
        let mut rows = Vec::with_capacity(self.g.world.len());
        for i in 0..self.g.world.len() {
            let p = PlotIdx(i as u32);
            if !self.is_revealed(p) {
                let mut r = [-1; NUM_LAYERS];
                r[0] = UNREVEALED;
                rows.push(r);
                continue;
            }
            let pl = self.g.world.plot(p);
            let vis = if self.is_visible(p) { VISIBLE } else { FOGGED };
            let terrain = d.ty::<TerrainType>(pl.terrain.art_key()).map_or(-1, |t| t.idx() as i32);
            let feature = pl.feature.art_key().and_then(|k| d.ty::<FeatureType>(k)).map_or(-1, |f| f.idx() as i32);
            let bonus = pl.resource.art_key().and_then(|k| d.ty::<BonusType>(k)).map_or(-1, |b| b.idx() as i32);
            let imp = pl.improvement.art_key().and_then(|k| d.ty::<ImprovementType>(k)).map_or(-1, |m| m.idx() as i32);
            let route = if pl.road { road_idx } else { -1 };
            let owner = pl.owner.map_or(-1, |o| o.0 as i32);
            let city = pl.city.map_or(-1, |c| c.0 as i32);
            let packed_yield = if vis == VISIBLE {
                let (f, ind, g) = pl.yields();
                f.clamp(0, 255) | (ind.clamp(0, 255) << 8) | (g.clamp(0, 255) << 16)
            } else {
                -1
            };
            rows.push([vis, pl.kind.art_plot_type(), terrain, feature, bonus, imp, route, owner, pl.river_bits, city, packed_yield, -1, -1, 0]);
        }
        MapRows { w, h, wrap_x: self.g.world.wrap_x, wrap_y: false, reveal: self.reveal.get(), rows }
    }

    pub fn map(&self) -> Json {
        self.map_rows().to_json()
    }

    fn unit_art_idx(&self, class: UnitClassId) -> i32 {
        self.d.ty::<UnitType>(class.info().art_unit).map_or(0, |u| u.idx() as i32)
    }

    fn civ_art_idx(&self, p: PlayerId) -> Json {
        let pl = &self.g.players[p.idx()];
        let key = if pl.is_neutral { "CIVILIZATION_BARBARIAN" } else { pl.faction.info().civ_art };
        self.d.ty::<CivilizationType>(key).map(|c| Json::from(c.idx() as i32)).unwrap_or(Json::Null)
    }

    fn leader_art_idx(&self, p: PlayerId) -> Json {
        let pl = &self.g.players[p.idx()];
        let key = if pl.is_neutral { "LEADER_BARBARIAN" } else { pl.faction.info().leader_art };
        self.d.ty::<LeaderHeadType>(key).map(|l| Json::from(l.idx() as i32)).unwrap_or(Json::Null)
    }

    /// Emits the full `view` payload: 100% compatible with HD-2D / ISO / Minimap / `battle_watch.js`
    /// plus `mode: "cataclysm"` and the rich native `cataclysm` tree!
    pub fn view(&self) -> Json {
        let g = &self.g;
        let h = self.human;
        let me_pl = &g.players[h.idx()];
        let gold_b = g.gold_breakdown(h);
        let lore_b = g.lore_breakdown(h);
        let mana_b = g.mana_breakdown(h);

        // 1. Standard `players` list (so HD-2D/ISO assign every faction's real town family & team colors!)
        let mut std_players = Vec::new();
        for pl in &g.players {
            if pl.id == h {
                continue;
            }
            std_players.push(crate::jobj! {
                "id" => pl.id.0 as i32,
                "name" => pl.name(),
                "alignment" => Json::Null,
                "state_religion" => Json::Null,
                "leader" => self.leader_art_idx(pl.id),
                "civ" => self.civ_art_idx(pl.id),
                "team" => pl.id.0 as i32,
                "barbarian" => pl.is_neutral,
                "alive" => pl.alive,
                "war" => g.at_war(h, pl.id),
                "attitude" => Json::Null,
                "open_borders" => !g.at_war(h, pl.id),
                "defensive_pact" => false,
                "vassal_of_us" => pl.vassal_of == Some(h),
                "our_master" => me_pl.vassal_of == Some(pl.id),
                "score" => pl.score,
                "contact" => true,
            });
        }

        // 2. Standard `cities` list (so HD-2D Batch 5 renders full pixel-art towns, walls, banners!)
        let mut std_cities = Vec::new();
        let mut cat_cities = Vec::new();
        for c in g.cities.values() {
            if !self.is_revealed(c.plot) && c.owner != h {
                continue;
            }
            let pl = g.world.plot(c.plot);
            let visible = self.is_visible(c.plot) || c.owner == h;
            let sentinels_j: Vec<Json> = c
                .sentinel_damage
                .iter()
                .map(|&dmg| crate::jobj! { "hp" => (100 - dmg).max(0), "breached" => dmg >= 100 })
                .collect();
            let lifeboat = g.apocalypse.is_lifeboat(c.id).map(|k| k.name());
            std_cities.push(crate::jobj! {
                "id" => c.id.0 as i32,
                "owner" => c.owner.0 as i32,
                "x" => pl.x,
                "y" => pl.y,
                "name" => c.name.as_str(),
                "capital" => c.capital,
                "visible" => visible,
                "size" => c.size,
                "defense" => c.defense_pct(),
                "walls" => !c.sentinel_damage.is_empty(),
                "besieged" => c.besieged,
                "sentinels" => c.sentinel_damage.iter().map(|&dmg| (100 - dmg).max(0)).collect::<Vec<i32>>(),
                "idle" => c.owner == h && c.building.is_none(),
            });

            let districts_j = Json::Arr(c.districts.iter().map(|d| crate::jobj! { "key" => d.info().key, "name" => d.info().name }).collect());
            let wonders_j = Json::Arr(c.wonders.iter().map(|w| crate::jobj! { "key" => w.key(), "name" => w.name() }).collect());
            if c.owner != h {
                // A rival's city: only what a rival may see.
                cat_cities.push(crate::jobj! {
                    "id" => c.id.0 as i32,
                    "owner" => c.owner.0 as i32,
                    "foreign" => true,
                    "name" => c.name.as_str(),
                    "x" => pl.x,
                    "y" => pl.y,
                    "capital" => c.capital,
                    "is_carcosa" => c.is_carcosa,
                    "size" => c.size,
                    "defense_pct" => c.defense_pct(),
                    "besieged" => c.besieged,
                    "lifeboat" => lifeboat,
                    "districts" => districts_j,
                    "wonders" => wonders_j,
                    "sentinels" => Json::Arr(sentinels_j),
                });
                continue;
            }
            let (_food, ind_calc, gold_r, lore_r, cult_r, mana_r) = g.city_yields(c.id);
            let building = c.building.as_ref().map(|b| {
                let cost = b.cost().max(1);
                let rem = (cost - c.build_progress).max(1);
                let turns = (rem + ind_calc.total.max(1) - 1) / ind_calc.total.max(1);
                crate::jobj! {
                    "label" => b.label(),
                    "progress" => c.build_progress,
                    "cost" => cost,
                    "turns" => turns,
                }
            });
            let crew = c.crew_task.as_ref().map(|t| {
                let label = if t.build_road { "Road".into() } else { t.improvement.name().to_string() };
                crate::jobj! { "label" => label, "progress" => t.progress, "needed" => t.needed }
            });
            cat_cities.push(crate::jobj! {
                "id" => c.id.0 as i32,
                "owner" => c.owner.0 as i32,
                "foreign" => false,
                "name" => c.name.as_str(),
                "x" => pl.x,
                "y" => pl.y,
                "plot" => c.plot.0 as i32,
                "capital" => c.capital,
                "is_carcosa" => c.is_carcosa,
                "is_outpost" => c.is_outpost,
                "size" => c.size,
                "food_stored" => c.food_stored,
                "food_needed" => c.food_needed(),
                "max_size" => cataclysm_core::city::MAX_CITY_SIZE,
                "culture" => c.culture,
                "defense_pct" => c.defense_pct(),
                "besieged" => c.besieged,
                "raid_streak" => c.raid_streak,
                "stripped_turns" => c.stripped_turns,
                "sally" => match c.sally { SallyPosture::Never => "never", SallyPosture::Favoured => "favoured", SallyPosture::Always => "always" },
                "lifeboat" => lifeboat,
                "max_district_slots" => c.max_district_slots(g.is_megacity(c)) as i32,
                "districts" => districts_j,
                "wonders" => wonders_j,
                "sentinels" => Json::Arr(sentinels_j),
                "industry_rate" => ind_calc.total,
                "industry_calc" => Json::Arr(ind_calc.lines.into_iter().map(Json::from).collect()),
                "gold_rate" => gold_r,
                "lore_rate" => lore_r,
                "culture_rate" => cult_r,
                "mana_rate" => mana_r,
                "building" => building,
                "crew_task" => crew,
                "crew" => self.crew_json(c),
                "notes" => Json::Arr(self.city_notes(c)),
                "raze" => self.raze_json(c),
                "caravan_destinations" => Json::Arr(self.caravan_destinations(c)),
                "district_slot_note" => self.district_slot_note(c),
                "grow_turns" => self.grow_turns(c),
                "house_id" => c.house.map(|x| x.0 as i32),
            });
        }

        // 3. Standard `units` (every SquadMember emits its real sprite & `stand: {rank, cell, large}`
        // so HD-2D renders full 3x3 Ogre Battle squad formations on the map!) + `cataclysm.squads`!
        let mut std_units = Vec::new();
        let mut std_groups = Vec::new();
        let mut std_armies = Vec::new();
        let mut cat_squads = Vec::new();
        let mut plot_squad_rank: std::collections::BTreeMap<PlotIdx, i32> = std::collections::BTreeMap::new();

        let place_cap = me_pl.squad_place_cap();
        let found_cost = cataclysm_core::game::found_city_cost(g.player_cities(h).len());

        for s in g.squads.values() {
            if s.owner != h && !self.is_visible(s.plot) {
                continue;
            }
            let pl = g.world.plot(s.plot);
            let own = s.owner == h;
            let max_mv = g.squad_turn_start_moves(s.id).max(s.moves_left);
            let rank = *plot_squad_rank.entry(s.plot).and_modify(|r| *r += 1).or_insert(0);
            let mut member_ids = Vec::new();
            let mut cat_members = Vec::new();
            for m in &s.members {
                let art_ty = self.unit_art_idx(m.class);
                let cell = (m.row as i32) * 3 + (m.lane as i32);
                let info = m.class.info();
                member_ids.push(m.id.0 as i32);
                std_units.push(crate::jobj! {
                    "id" => m.id.0 as i32,
                    "owner" => s.owner.0 as i32,
                    "type" => art_ty,
                    "x" => pl.x,
                    "y" => pl.y,
                    "hp" => m.hp_pct(),
                    "max_hp" => 100,
                    "level" => m.level,
                    "group" => s.id.0 as i32,
                    "moves" => s.moves_left * 60,
                    "max_moves" => max_mv * 60,
                    "engages" => true,
                    "cataclysm_squad" => s.id.0 as i32,
                    "stand" => crate::jobj! { "rank" => rank, "cell" => cell, "large" => info.large },
                });
                let upgrade_to = info.upgrades_to.filter(|&nxt| own && g.can_recruit_class(s.owner, nxt).is_ok());
                let upgrade_cost = upgrade_to.map(|nxt| {
                    let mut c = (nxt.info().cost_gold - info.cost_gold / 2).max(25);
                    if g.players[s.owner.idx()].has_tech(cataclysm_core::data::TECH_WAR_CHESTS) {
                        c = c * cataclysm_core::game::WAR_CHESTS_PCT / 100;
                    }
                    c
                });
                cat_members.push(crate::jobj! {
                    "id" => m.id.0 as i32,
                    "class" => info.name,
                    "class_key" => info.key,
                    "name" => m.display_name(),
                    "art_unit_type" => art_ty,
                    "row" => m.row as i32,
                    "lane" => m.lane as i32,
                    "large" => info.large,
                    "hp_pct" => m.hp_pct(),
                    "max_hp" => m.max_hp(),
                    "level" => m.level,
                    "xp" => m.xp,
                    "is_hero" => m.is_hero(),
                    "wounds" => m.wounds,
                    "atk" => info.atk,
                    "def" => info.def,
                    "magic" => info.magic,
                    "mdef" => info.mdef,
                    "speed" => info.speed,
                    "actions" => actions_json(&info.rows),
                    "upgrades_to" => upgrade_to.map(|n| n.info().name),
                    "upgrade_cost" => upgrade_cost,
                });
            }

            let supply_line = g.squad_supply_line(s.id);
            let line_coords: Vec<Json> = supply_line
                .as_ref()
                .map(|sl| {
                    sl.path
                        .iter()
                        .map(|&p| {
                            let pt = g.world.plot(p);
                            crate::jarr![pt.x, pt.y]
                        })
                        .collect()
                })
                .unwrap_or_default();

            if s.owner == h {
                std_groups.push(crate::jobj! {
                    "id" => s.id.0 as i32,
                    "head" => member_ids.first().copied(),
                    "units" => member_ids,
                    "ready" => s.moves_left > 0,
                    "selectable" => s.moves_left > 0,
                    "busy" => false,
                });
                if line_coords.len() >= 2 {
                    std_armies.push(crate::jobj! {
                        "id" => s.id.0 as i32,
                        "name" => s.name.as_str(),
                        "x" => pl.x,
                        "y" => pl.y,
                        "supply" => crate::jobj! {
                            "supplied" => supply_line.is_some(),
                            "unsupplied_turns" => s.unsupplied_turns,
                            "line" => Json::Arr(line_coords.clone()),
                        },
                    });
                }
            }

            let order = match s.order {
                cataclysm_core::SquadOrder::None => "none",
                cataclysm_core::SquadOrder::Fortify => "fortify",
                cataclysm_core::SquadOrder::Raid => "raid",
            };
            if !own {
                // A rival's squad: only what a rival may see.
                cat_squads.push(crate::jobj! {
                    "id" => s.id.0 as i32,
                    "owner" => s.owner.0 as i32,
                    "foreign" => true,
                    "name" => s.name.as_str(),
                    "captain_name" => s.captain_name.as_str(),
                    "captain_portrait" => s.captain_portrait,
                    "captain_aura" => s.captain_aura.name(),
                    "x" => pl.x,
                    "y" => pl.y,
                    "plot" => s.plot.0 as i32,
                    "order" => order,
                    "power" => s.power_rating(),
                    "places_used" => s.places_used() as i32,
                    "members" => Json::Arr(cat_members),
                });
                continue;
            }

            let site = g.check_city_site(s.plot);
            let found_check = g.check_found_city(h, s.id);
            let road = g.check_field_work(h, s.id, true);
            let fort = g.check_field_work(h, s.id, false);
            let supply = self.supply_json(s, supply_line.as_ref());

            cat_squads.push(crate::jobj! {
                "id" => s.id.0 as i32,
                "owner" => s.owner.0 as i32,
                "name" => s.name.as_str(),
                "captain_name" => s.captain_name.as_str(),
                "captain_portrait" => s.captain_portrait,
                "captain_aura" => s.captain_aura.name(),
                "x" => pl.x,
                "y" => pl.y,
                "plot" => s.plot.0 as i32,
                "foreign" => false,
                "moves_left" => s.moves_left,
                "max_moves" => max_mv,
                "order" => order,
                "supplied" => supply_line.is_some(),
                "unsupplied_turns" => s.unsupplied_turns,
                "supply" => supply,
                "can_road" => road.is_ok(),
                "road_why" => road.err().map(|e| e.detail),
                "can_fort" => fort.is_ok(),
                "fort_why" => fort.err().map(|e| e.detail),
                "can_disband" => true,
                "power" => s.power_rating(),
                "places_used" => s.places_used() as i32,
                "place_cap" => place_cap as i32,
                "in_home_borders" => pl.owner == Some(s.owner),
                "can_found_city" => site.is_ok(),
                "found_why" => found_check.err().map(|e| e.detail),
                "found_cost" => found_cost,
                "has_lair" => pl.lair_unexplored,
                "on_enemy_countryside" => pl.owner.is_some_and(|o| o != s.owner && g.at_war(s.owner, o)) && pl.city.is_none(),
                "lent_by" => s.lent_by.map(|p| p.0 as i32),
                "supply_line" => Json::Arr(line_coords),
                "members" => Json::Arr(cat_members),
            });
        }

        // Emit on-map Caravan figures (reusing UNIT_CARAVAN art!)
        let caravan_art = self.d.ty::<UnitType>("UNIT_CARAVAN").map_or(0, |u| u.idx() as i32);
        let mut cat_caravans = Vec::new();
        for cv in g.caravans.values() {
            let (Some(o), Some(dest)) = (g.cities.get(&cv.origin), g.cities.get(&cv.destination)) else { continue };
            let foreign = o.owner != dest.owner;
            let pay = cv.gold_per_turn(foreign, o.size, dest.size, 0);
            if let Some(p) = cv.current_plot() {
                if cv.owner == h || self.is_visible(p) {
                    let pt = g.world.plot(p);
                    std_units.push(crate::jobj! {
                        "id" => 900_000 + cv.id.0 as i32,
                        "owner" => cv.owner.0 as i32,
                        "type" => caravan_art,
                        "x" => pt.x,
                        "y" => pt.y,
                        "hp" => 100,
                        "max_hp" => 100,
                        "level" => 1,
                        "moves" => 0,
                        "max_moves" => 120,
                    });
                }
            }
            if cv.owner == h || dest.owner == h {
                let pt = cv.current_plot().map(|p| g.world.plot(p));
                cat_caravans.push(crate::jobj! {
                    "id" => cv.id.0 as i32,
                    "owner" => cv.owner.0 as i32,
                    "origin_name" => o.name.as_str(),
                    "dest_name" => dest.name.as_str(),
                    "gold_rate" => pay,
                    "x" => pt.map(|q| q.x),
                    "y" => pt.map(|q| q.y),
                    "reform_turns" => cv.reform_turns,
                    "growth_pct" => cv.growth_pct,
                });
            }
        }

        // 4. Techs, Doctrines, Religions, Rituals
        let techs_j: Vec<Json> = TECHS
            .iter()
            .map(|t| {
                let known = me_pl.has_tech(t.id);
                let check = g.can_research(h, t.id);
                let cost = g.tech_effective_cost(h, t.id);
                // Only the first civilization to research a tech moves Strain (none after the Blowout).
                let discovered = g.discovered_techs.contains(&t.id);
                let strain = if discovered || g.apocalypse.carcosa_risen { 0 } else { t.ac_delta };
                let strain_text = if t.ac_delta == 0 {
                    None
                } else if discovered {
                    Some("Already researched in the world: 0 Strain.".to_string())
                } else {
                    Some(format!("{:+} Strain for the first civilization to research it.", t.ac_delta))
                };
                crate::jobj! {
                    "id" => t.id.0 as i32,
                    "key" => t.key,
                    "name" => t.name,
                    "doctrine" => t.doctrine.map(Doctrine::key),
                    "doctrine_name" => t.doctrine.map(Doctrine::name),
                    "tier" => t.tier as i32,
                    "cost" => cost,
                    "cost_base" => t.cost,
                    "cost_note" => g.tech_cost_note(h, t.id),
                    "prereq" => t.prereq.map(|p| TECHS[p.0 as usize].name),
                    "ac_delta" => strain,
                    "strain_text" => strain_text,
                    "known" => known,
                    "can_research" => check.is_ok(),
                    "why" => check.err(),
                    "unlocks" => t.unlocks_summary,
                }
            })
            .collect();

        let doctrines_j: Vec<Json> = Doctrine::ALL
            .iter()
            .map(|&doc| {
                crate::jobj! {
                    "key" => doc.key(),
                    "name" => doc.name(),
                    "summary" => doc.summary(),
                    "primary" => me_pl.primary_doctrine == Some(doc),
                    "secondary" => me_pl.secondary_doctrine == Some(doc),
                }
            })
            .collect();

        let current_res = me_pl.current_research.map(|tid| {
            let cost = g.tech_effective_cost(h, tid);
            let rem = (cost - me_pl.lore_progress).max(1);
            let turns = (rem + lore_b.total.max(1) - 1) / lore_b.total.max(1);
            crate::jobj! {
                "id" => tid.0 as i32,
                "name" => TECHS[tid.0 as usize].name,
                "progress" => me_pl.lore_progress,
                "cost" => cost,
                "turns" => turns,
            }
        });

        let religions_j: Vec<Json> = Religion::ALL
            .iter()
            .map(|&rel| {
                let founder = g.founded_religions.get(&rel).map(|p| g.players[p.idx()].name());
                let adopted = me_pl.religion == Some(rel);
                let founded = g.founded_religions.contains_key(&rel);
                let action = if adopted {
                    "adopted"
                } else if founded {
                    "adopt"
                } else {
                    "found"
                };
                let check = g.check_religion(h, rel);
                let why = if adopted { Some("Already your Creed.".to_string()) } else { check.err().map(|e| e.detail) };
                let cost = rel.lore_cost();
                let cost_text = match action {
                    "found" => Some(format!("{cost} Lore from research progress if you have it, else {cost} Gold")),
                    "adopt" => Some("Free".to_string()),
                    _ => None,
                };
                let mut effect = format!("Your Creed becomes {}; your alignment becomes {}.", rel.name(), rel.alignment().name());
                if action == "found" {
                    let moved = if g.apocalypse.carcosa_risen {
                        0
                    } else {
                        (g.apocalypse.counter + rel.founding_ac_delta()).clamp(0, 100) - g.apocalypse.counter
                    };
                    if moved != 0 {
                        effect.push_str(&format!(" {:+} Strain.", moved));
                    }
                }
                crate::jobj! {
                    "id" => rel.id() as i32,
                    "name" => rel.name(),
                    "alignment" => rel.alignment().name(),
                    "founded_by" => founder,
                    "adopted" => adopted,
                    "lore_cost" => cost,
                    "ac_delta" => rel.founding_ac_delta(),
                    "summary" => rel.summary(),
                    "action" => action,
                    "can_act" => why.is_none(),
                    "why" => why,
                    "cost_text" => cost_text,
                    "effect_text" => effect,
                }
            })
            .collect();

        let rituals_j: Vec<Json> = RitualKind::ALL
            .iter()
            .map(|&rk| {
                let mut why = g.check_ritual(h, rk).err().map(|e| e.detail);
                if why.is_none() && rk == RitualKind::SummonHost && g.ritual_plot(h).is_none() {
                    why = Some("Needs a city.".to_string());
                }
                // The Strain change the ritual makes now (Keepers' Vent the Seal is larger; none after the Blowout).
                let delta = if g.apocalypse.carcosa_risen {
                    0
                } else if rk == RitualKind::Sanctify && me_pl.faction == FactionId::Elohim {
                    cataclysm_core::game::SANCTIFY_KEEPERS_STRAIN
                } else {
                    rk.ac_delta()
                };
                let summary = if rk.recast_turns() > 0 {
                    format!("{} Can be cast again {} turns after each cast.", rk.summary(), rk.recast_turns())
                } else {
                    rk.summary().to_string()
                };
                crate::jobj! {
                    "key" => rk.key(),
                    "name" => rk.name(),
                    "mana_cost" => rk.mana_cost(),
                    "ac_delta" => delta,
                    "can_cast" => why.is_none(),
                    "why" => why,
                    "summary" => summary,
                    "recast_turns" => rk.recast_turns(),
                    "ready_in" => g.ritual_turns_left(h, rk),
                }
            })
            .collect();

        // 5. Houses
        let houses_j: Vec<Json> = g
            .houses
            .values()
            .filter(|ho| ho.owner == h)
            .map(|ho| {
                let (loy, mood, calc) = g.house_loyalty(ho.id);
                let city_names: Vec<Json> = ho
                    .cities
                    .iter()
                    .filter_map(|cid| g.cities.get(cid).map(|c| Json::from(c.name.as_str())))
                    .collect();
                crate::jobj! {
                    "id" => ho.id.0 as i32,
                    "name" => ho.name.as_str(),
                    "lord_name" => ho.lord_name.as_str(),
                    "lord_portrait" => ho.lord_portrait,
                    "alignment" => ho.alignment.name(),
                    "religion" => ho.religion.map(Religion::name),
                    "sympathy_name" => ho.sympathy.map(|p| g.players[p.idx()].name()),
                    "perk" => ho.perk.name(),
                    "loyalty" => loy,
                    "mood" => mood.name(),
                    "perk_active" => cataclysm_core::House::perk_active(loy),
                    "output_pct" => mood.output_pct(),
                    "loyalty_calc" => Json::Arr(calc.into_iter().map(Json::from).collect()),
                    "cities" => Json::Arr(city_names),
                }
            })
            .collect();

        // 6. Great People Court & Renown
        let court_j: Vec<Json> = g
            .court
            .values()
            .filter(|gp| gp.owner == h)
            .map(|gp| {
                let secret = gp.secret_tech.map(|sid| {
                    let info = &SECRET_TECHS[sid.0 as usize];
                    crate::jobj! {
                        "id" => sid.0 as i32,
                        "name" => info.name,
                        "needs_name" => TECHS[info.needs.0 as usize].name,
                        "needs_known" => me_pl.has_tech(info.needs),
                        "summary" => info.summary,
                        "expert_line" => info.expert_line,
                    }
                });
                let burst_note = match gp.suit {
                    GreatSuit::Engineer => g.burst_city(h).map(|cid| {
                        let c = &g.cities[&cid];
                        match &c.building {
                            Some(b) => format!("Applies to {}: {}, {} of {} Industry.", c.name, b.label(), c.build_progress, b.cost()),
                            None => format!("Applies to {}, which has no build order; the Industry is lost when one is chosen.", c.name),
                        }
                    }),
                    GreatSuit::Bard => g.burst_city(h).map(|cid| format!("Applies to {}.", g.cities[&cid].name)),
                    GreatSuit::Prophet if g.apocalypse.carcosa_risen => Some("Strain no longer changes after the Blowout.".to_string()),
                    _ => None,
                };
                let hero_squads: Vec<Json> = g
                    .squads
                    .values()
                    .filter(|s| s.owner == h && g.check_hero(h, gp.id, s.id).is_ok())
                    .map(|s| crate::jobj! { "id" => s.id.0 as i32, "name" => s.name.as_str() })
                    .collect();
                let lord_houses: Vec<Json> = g
                    .houses
                    .values()
                    .filter(|ho| ho.owner == h && g.check_lord(h, gp.id, ho.id).is_ok())
                    .map(|ho| crate::jobj! { "id" => ho.id.0 as i32, "name" => ho.name.as_str(), "lord_name" => ho.lord_name.as_str() })
                    .collect();
                let hero_class = gp.suit.hero_class().info().name;
                let hero_effect = format!(
                    "{} joins the squad as a level {} hero {} (+{} max HP, +{}% battle stats) and becomes its captain: the squad takes {}'s name and portrait as captain; its aura is unchanged. A fallen hero returns at the capital after {} turns with 1 more Wound. The Secret Tech card returns to the world deck.",
                    gp.name,
                    cataclysm_core::squad::HERO_LEVEL,
                    hero_class,
                    cataclysm_core::squad::HERO_HP_BONUS,
                    cataclysm_core::squad::HERO_STAT_PCT,
                    gp.name,
                    cataclysm_core::game::HERO_RECOVERY_TURNS
                );
                let lord_effect = format!(
                    "{} replaces the House's lord. The House takes your alignment ({}) and Creed ({}), drops its sympathy for a rival, and gains +{} lasting Loyalty; its perk is unchanged. The Secret Tech card returns to the world deck.",
                    gp.name,
                    me_pl.alignment.name(),
                    me_pl.religion.map_or("none", Religion::name),
                    cataclysm_core::great_people::LORD_LOYALTY
                );
                crate::jobj! {
                    "id" => gp.id.0 as i32,
                    "suit" => gp.suit.name(),
                    "name" => gp.name.as_str(),
                    "portrait" => gp.portrait,
                    "burst_summary" => gp.suit.burst_summary(),
                    "burst_note" => burst_note,
                    "hero_class_name" => hero_class,
                    "hero_squads" => Json::Arr(hero_squads),
                    "hero_effect" => hero_effect,
                    "lord_houses" => Json::Arr(lord_houses),
                    "lord_effect" => lord_effect,
                    "secret_tech" => secret,
                }
            })
            .collect();

        let renown_meters_j: Vec<Json> = GreatSuit::ALL
            .iter()
            .map(|&s| crate::jobj! { "suit" => s.name(), "value" => me_pl.renown_meters[s.id()] })
            .collect();
        let renown_total: i32 = me_pl.renown_meters.iter().sum();
        let renown_need = cataclysm_core::great_people::renown_threshold(me_pl.great_people_born);

        let secrets_known_j: Vec<Json> = me_pl
            .secrets
            .iter()
            .map(|&sid| {
                let info = &SECRET_TECHS[sid.0 as usize];
                crate::jobj! {
                    "id" => sid.0 as i32,
                    "name" => info.name,
                    "needs_name" => TECHS[info.needs.0 as usize].name,
                    "active" => me_pl.has_tech(info.needs),
                    "summary" => info.summary,
                    "expert_line" => info.expert_line,
                }
            })
            .collect();

        // 7. Apocalypse & Lifeboats & Crusades
        let crises_j: Vec<Json> = ARMAGEDDON_CRISES
            .iter()
            .map(|c| {
                crate::jobj! {
                    "threshold" => c.threshold,
                    "name" => c.name,
                    "summary" => c.summary,
                    "fired" => g.apocalypse.triggered_crises.contains(&c.threshold),
                }
            })
            .collect();
        let lifeboats_j: Vec<Json> = g
            .apocalypse
            .lifeboats
            .iter()
            .map(|lb| {
                let city = g.cities.get(&lb.city_id);
                let pt = city.map(|c| g.world.plot(c.plot));
                crate::jobj! {
                    "kind" => lb.kind.name(),
                    "city_id" => lb.city_id.0 as i32,
                    "city_name" => city.map_or("Razed", |c| c.name.as_str()),
                    "owner" => city.map(|c| c.owner.0 as i32),
                    "owner_name" => city.map(|c| g.players[c.owner.idx()].name()),
                    "x" => pt.map(|p| p.x),
                    "y" => pt.map(|p| p.y),
                    "razed" => lb.razed,
                }
            })
            .collect();
        let crusades_j: Vec<Json> = g
            .crusades
            .values()
            .filter(|cr| cr.active)
            .map(|cr| {
                let lendable = g.squads.values().any(|s| s.owner == h && g.check_lend(h, cr.id, s.id).is_ok());
                let lend_why = if lendable {
                    None
                } else if cr.target == h {
                    Some("This Crusade is against you.".to_string())
                } else if let Err(e) = g.check_war(h, cr.target) {
                    Some(e.detail)
                } else {
                    Some("You have no squad that is not already lent.".to_string())
                };
                crate::jobj! {
                    "id" => cr.id.0 as i32,
                    "caller_name" => g.players[cr.caller.idx()].name(),
                    "target_id" => cr.target.0 as i32,
                    "target_name" => g.players[cr.target.idx()].name(),
                    "turns_left" => cr.turns_left,
                    "lent_count" => cr.lent_squads.len() as i32,
                    "against_me" => cr.target == h,
                    "can_lend" => lendable,
                    "lend_why" => lend_why,
                    "lend_declares_war" => cr.target != h && !g.at_war(h, cr.target),
                }
            })
            .collect();

        // 8. Recruitable unit classes, districts, wonders & rival summaries
        let classes_j: Vec<Json> = UnitClassId::RECRUITABLE
            .iter()
            .map(|&cl| {
                let info = cl.info();
                let ok = g.can_recruit_class(h, cl);
                let mut gold_cost = info.cost_gold;
                if me_pl.faction == FactionId::Hippus {
                    gold_cost = gold_cost * cataclysm_core::game::HIPPUS_REINFORCE_PCT / 100;
                }
                if me_pl.has_tech(cataclysm_core::data::TECH_WAR_CHESTS) {
                    gold_cost = gold_cost * cataclysm_core::game::WAR_CHESTS_PCT / 100;
                }
                crate::jobj! {
                    "key" => info.key,
                    "name" => info.name,
                    "tier" => info.tier as i32,
                    "cost_gold" => gold_cost,
                    "cost_industry" => info.cost_industry,
                    "large" => info.large,
                    "unlocked" => ok.is_ok(),
                    "why" => ok.err(),
                    "row_pref" => match info.row_order[0] { 0 => "Front", 1 => "Middle", _ => "Rear" },
                    "hp" => info.hp,
                    "atk" => info.atk,
                    "def" => info.def,
                    "magic" => info.magic,
                }
            })
            .collect();

        let districts_j: Vec<Json> = DISTRICTS
            .iter()
            .map(|di| {
                let unlocked = di.tech.is_none_or(|t| me_pl.has_tech(t));
                let why = di.tech.filter(|&t| !me_pl.has_tech(t)).map(|t| format!("Requires {}", TECHS[t.0 as usize].name));
                crate::jobj! {
                    "key" => di.key,
                    "name" => di.name,
                    "cost" => di.cost,
                    "unlocked" => unlocked,
                    "why" => why,
                    "summary" => di.summary,
                }
            })
            .collect();

        let wonders_j: Vec<Json> = WonderKind::ALL
            .iter()
            .map(|&wk| {
                let built = g.built_wonders.contains_key(&wk);
                let unlocked = !built && me_pl.has_tech(wk.tech());
                let why = if built {
                    Some("Already built".into())
                } else if !me_pl.has_tech(wk.tech()) {
                    Some(format!("Requires {}", TECHS[wk.tech().0 as usize].name))
                } else {
                    None
                };
                crate::jobj! {
                    "key" => wk.key(),
                    "name" => wk.name(),
                    "cost" => wk.cost(),
                    "unlocked" => unlocked,
                    "built" => built,
                    "why" => why,
                    "summary" => wk.summary(),
                }
            })
            .collect();

        let cat_players: Vec<Json> = g
            .players
            .iter()
            .map(|pl| {
                let offer_to_us = g.peace_offer_from(pl.id, h);
                let offer_from_us = g.peace_offer_from(h, pl.id);
                let truce = g.truce_turns_left(h, pl.id);
                let peace = g.check_peace(h, pl.id);
                let war_why = g.check_war(h, pl.id).err().map(|e| e.detail);
                let crusade_why = g.check_call_crusade(h, pl.id).err().map(|e| e.detail);
                let mut peace_lines = Vec::new();
                if let Some(t) = offer_to_us {
                    peace_lines.push(format!("Offers peace to you: {t} turns left."));
                }
                if let Some(t) = offer_from_us {
                    peace_lines.push(format!("Your peace offer: {t} turns left."));
                }
                if truce > 0 {
                    peace_lines.push(format!("Truce: {truce} turns left."));
                }
                crate::jobj! {
                    "id" => pl.id.0 as i32,
                    "name" => pl.name(),
                    "peace_offer_to_us" => offer_to_us,
                    "peace_offer_from_us" => offer_from_us,
                    "truce_turns" => truce,
                    "peace_lines" => peace_lines,
                    "peace_label" => if peace == Ok(true) { "Accept peace" } else { "Offer peace" },
                    "can_peace" => peace.is_ok(),
                    "peace_why" => peace.err().map(|e| e.detail),
                    "can_call_crusade" => crusade_why.is_none(),
                    "call_crusade_why" => crusade_why,
                    "can_declare_war" => war_why.is_none(),
                    "war_why" => war_why,
                    "faction_name" => if pl.is_neutral { "Wild Clans" } else { pl.faction.info().name },
                    "leader_art" => if pl.is_neutral { "LEADER_BARBARIAN" } else { pl.faction.info().leader_art },
                    "alignment" => pl.alignment.name(),
                    "religion" => pl.religion.map(Religion::name),
                    "alive" => pl.alive,
                    "is_neutral" => pl.is_neutral,
                    "war" => g.at_war(h, pl.id),
                    "score" => pl.score,
                    "cities" => g.player_cities(pl.id).len() as i32,
                    "squads" => g.player_squads(pl.id).len() as i32,
                    "can_mortal_win" => pl.faction.info().can_mortal_win,
                    "can_demonic_win" => pl.faction.info().can_demonic_win,
                    "rule_summary" => pl.faction.info().rule_summary,
                }
            })
            .collect();

        // Recent battles for `battle_watch.js` map markers & playback!
        let recent_battles: Vec<Json> = g
            .battles
            .iter()
            .rev()
            .take(8)
            .filter(|b| b.turn + 1 >= g.turn && (b.attacker == h || b.defender == h || self.reveal.get()))
            .map(|b| self.battle_summary_json(b))
            .collect();

        let me_j = crate::jobj! {
            "player" => h.0 as i32,
            "team" => h.0 as i32,
            "name" => me_pl.name(),
            "civ" => self.civ_art_idx(h),
            "leader" => self.leader_art_idx(h),
            "gold" => me_pl.gold,
            "gold_rate" => gold_b.total,
            "research" => Json::Null,
            "research_queue" => Json::arr(),
            "research_rate" => lore_b.total,
            "commerce_percent" => vec![0, 0, 0, 0],
            "flexible" => vec![false, false, false, false],
            "anarchy" => 0,
            "golden_age" => 0,
            "civics" => Json::arr(),
            "score" => me_pl.score,
            "civs_alive" => g.players.iter().filter(|p| p.alive && !p.is_neutral).count() as i32,
            "armageddon" => g.apocalypse.counter,
            "mana" => Json::arr(),
            "stasis" => crate::jobj! { "research" => false, "production" => false, "spells" => false },
            "alive" => me_pl.alive,
            "world_actions" => Json::arr(),
            "options" => Json::arr(),
            "ai_auto_play" => 0,
        };

        let caravan_check = g.check_caravan_launch(h).err().map(|e| e.detail).or_else(|| {
            (me_pl.gold < cataclysm_core::game::CARAVAN_GOLD)
                .then(|| format!("Need {} Gold (have {}).", cataclysm_core::game::CARAVAN_GOLD, me_pl.gold))
        });
        let crusade_why = g.check_call_crusade(h, h).err().filter(|e| e.code == "cannot").map(|e| e.detail).or_else(|| {
            let any_target = g.players.iter().any(|p| p.id != h && !p.is_neutral && p.alive && g.check_call_crusade(h, p.id).is_ok());
            (!any_target).then(|| "No civilization can be the target of a Crusade now.".to_string())
        });
        let cataclysm_payload = crate::jobj! {
            "turn" => g.turn,
            "max_turns" => MAX_GAME_TURNS,
            "turns_left" => self.turns_left(),
            "victory" => self.victory_json(),
            "squad_build" => crate::jobj! { "min_members" => 1, "max_members" => place_cap as i32 },
            "caravan" => crate::jobj! {
                "cost" => cataclysm_core::game::CARAVAN_GOLD,
                "can_launch" => caravan_check.is_none(),
                "why" => caravan_check,
            },
            "can_call_crusade" => crusade_why.is_none(),
            "call_crusade_why" => crusade_why,
            "crusade_effect" => crusade_effect(),
            "war_effect" => "Ends every Caravan between you and the target. At war, squads may attack, raid and ambush Caravans, and each side's squads block the other's supply lines. Attacking, calling a Crusade or lending a squad also starts a war, without ending Caravans.",
            "peace_effect" => peace_effect(),
            "doctrine_rule" => doctrine_rule(),
            "order_help" => order_help_json(found_cost),
            "sally_help" => sally_help_json(),
            "court_rules" => court_rules_json(),
            "rules_help" => rules_help_json(),
            "act_label" => g.apocalypse.act_label(g.turn),
            "human_faction" => me_pl.faction.info().name,
            "human_leader" => me_pl.name(),
            "human_leader_art" => me_pl.faction.info().leader_art,
            "human_lore_blurb" => me_pl.faction.info().lore_blurb,
            "human_rule_summary" => me_pl.faction.info().rule_summary,
            "human_alignment" => me_pl.alignment.name(),
            "human_religion" => me_pl.religion.map(Religion::name),
            "squad_cap" => g.squad_cap(h) as i32,
            "squad_count" => g.player_squads(h).len() as i32,
            "caravan_cap" => g.caravan_cap(h) as i32,
            "caravan_count" => g.caravans.values().filter(|c| c.owner == h).count() as i32,
            "currencies" => crate::jobj! {
                "gold" => me_pl.gold,
                "gold_rate" => gold_b.total,
                "gold_calc" => Json::Arr(gold_b.lines.into_iter().map(Json::from).collect()),
                "lore" => me_pl.lore_progress,
                "lore_rate" => lore_b.total,
                "lore_calc" => Json::Arr(lore_b.lines.into_iter().map(Json::from).collect()),
                "mana" => me_pl.mana,
                "mana_rate" => mana_b.total,
                "mana_calc" => Json::Arr(mana_b.lines.into_iter().map(Json::from).collect()),
            },
            "research" => crate::jobj! {
                "current" => current_res,
                "idle_lore_text" => (!g.can_research_any(h)).then(|| format!(
                    "No tech can be researched: at your turn start Lore converts to Sage Renown, {} Lore per 1 Renown.",
                    cataclysm_core::game::LORE_PER_SAGE_RENOWN
                )),
                "primary_doctrine" => me_pl.primary_doctrine.map(Doctrine::key),
                "primary_doctrine_name" => me_pl.primary_doctrine.map(Doctrine::name),
                "secondary_doctrine" => me_pl.secondary_doctrine.map(Doctrine::key),
                "secondary_doctrine_name" => me_pl.secondary_doctrine.map(Doctrine::name),
                "techs" => Json::Arr(techs_j),
                "doctrines" => Json::Arr(doctrines_j),
            },
            "religions" => Json::Arr(religions_j),
            "creed_rule" => creed_rule(),
            "rituals" => Json::Arr(rituals_j),
            "squads" => Json::Arr(cat_squads),
            "cities" => Json::Arr(cat_cities),
            "houses" => Json::Arr(houses_j),
            "court" => Json::Arr(court_j),
            "renown" => crate::jobj! {
                "meters" => Json::Arr(renown_meters_j),
                "total" => renown_total,
                "next_threshold" => renown_need,
                "sources" => renown_sources(),
            },
            "secrets_known" => Json::Arr(secrets_known_j),
            "apocalypse" => crate::jobj! {
                "counter" => g.apocalypse.counter,
                "max" => cataclysm_core::apocalypse::MAX_STRAIN,
                "act_label" => g.apocalypse.act_label(g.turn),
                "hyborem_spawned" => g.apocalypse.hyborem_spawned,
                "carcosa_risen" => g.apocalypse.carcosa_risen,
                "countdown" => g.apocalypse.countdown,
                "carcosa_protected_turns" => g.apocalypse.carcosa_protected_turns,
                "recent_changes" => Json::Arr(g.apocalypse.recent_changes.iter().cloned().map(Json::from).collect()),
                "crises" => Json::Arr(crises_j),
                "lifeboats" => Json::Arr(lifeboats_j),
                "rules" => apocalypse_rules(),
                "lifeboat_rules" => lifeboat_rules(),
                "outcome" => g.apocalypse.outcome.as_ref().map(|o| crate::jobj! {
                    "kind" => o.kind.name(),
                    "summary" => o.summary.as_str(),
                }),
            },
            "caravans" => Json::Arr(cat_caravans),
            "crusades" => Json::Arr(crusades_j),
            "players" => Json::Arr(cat_players),
            "recruitable_classes" => Json::Arr(classes_j),
            "available_districts" => Json::Arr(districts_j),
            "available_wonders" => Json::Arr(wonders_j),
            "recovering_heroes" => Json::Arr(
                g.recovering_heroes
                    .iter()
                    .filter(|rh| rh.owner == h)
                    .map(|rh| crate::jobj! { "name" => rh.member.display_name(), "turns_left" => rh.turns_left, "wounds" => rh.member.wounds })
                    .collect()
            ),
        };

        crate::jobj! {
            "mode" => "cataclysm",
            "view_version" => self.view_version as i64,
            "turn" => g.turn,
            "year" => g.turn,
            "reveal" => self.reveal.get(),
            "me" => me_j,
            "players" => Json::Arr(std_players),
            "cities" => Json::Arr(std_cities),
            "units" => Json::Arr(std_units),
            "groups" => Json::Arr(std_groups),
            "armies" => Json::Arr(std_armies),
            "supply_lines" => Json::arr(),
            "seas" => Json::arr(),
            "sea_lanes" => Json::arr(),
            "outpost_lanes" => Json::arr(),
            "trade" => crate::jobj! { "routes" => Json::arr(), "incoming" => Json::arr(), "limit" => g.caravan_cap(h) as i32 },
            "area_effects" => Json::arr(),
            "delves" => Json::arr(),
            "battles" => Json::Arr(recent_battles),
            "rituals" => Json::arr(),
            "houses" => Json::arr(),
            "group_cycle" => Json::arr(),
            "has_auto_unit" => false,
            "pending" => crate::jobj! {
                "events" => Json::arr(),
                "votes" => Json::arr(),
                "elections" => Json::arr(),
                "crusades" => Json::arr(),
                "specialties" => Json::arr(),
                "research_needed" => false,
                "idle_cities" => Json::arr(),
                "ready_groups" => Json::arr(),
            },
            "log_next" => g.log.len() as i64,
            "moments_next" => 0,
            "cataclysm" => cataclysm_payload,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Battle Log v2 Adapter (feeds existing `battle_watch.js` + `battle_report.js` directly!)
    // -----------------------------------------------------------------------------------------

    fn plot_snap_json(&self, plot: PlotIdx) -> Json {
        let pl = self.g.world.plot(plot);
        let city = pl.city.and_then(|cid| self.g.cities.get(&cid)).map(|c| {
            crate::jobj! {
                "id" => c.id.0 as i32,
                "name" => c.name.as_str(),
                "owner" => c.owner.0 as i32,
                "size" => c.size,
                "defense_pct" => c.defense_pct(),
            }
        });
        crate::jobj! {
            "x" => pl.x,
            "y" => pl.y,
            "terrain" => self.d.ty::<TerrainType>(pl.terrain.art_key()).map(|t| t.idx() as i32),
            "feature" => pl.feature.art_key().and_then(|k| self.d.ty::<FeatureType>(k)).map(|f| f.idx() as i32),
            "hills" => pl.kind == cataclysm_core::PlotKind::Hills,
            "river_crossing" => pl.river_bits != 0,
            "city" => city,
        }
    }

    fn battle_summary_json(&self, b: &BattleRecord) -> Json {
        let pt = self.g.world.plot(b.to);
        let pf = self.g.world.plot(b.from);
        let viewer_side = if b.attacker == self.human {
            Some(0)
        } else if b.defender == self.human {
            Some(1)
        } else {
            None
        };
        let side_snap = |side: Side, player: PlayerId, label: &str| {
            let committed = b.fighters.iter().filter(|f| f.side == side).count() as i32;
            let lost = b.fighters.iter().filter(|f| f.side == side && matches!(f.fate, Fate::Fell { .. })).count() as i32;
            crate::jobj! {
                "player" => player.0 as i32,
                "civ" => self.civ_art_idx(player),
                "label" => label,
                "slots" => 5,
                "lanes" => 3,
                "used" => committed,
                "committed" => committed,
                "lost" => lost,
                "lost_types" => Json::arr(),
                "escaped" => 0,
                "sat_out" => 0,
                "standing" => (committed - lost).max(0),
            }
        };
        crate::jobj! {
            "id" => b.id.0 as i32,
            "turn" => b.turn,
            "x" => pt.x,
            "y" => pt.y,
            "from" => crate::jobj! { "x" => pf.x, "y" => pf.y },
            "place" => b.place_name.as_deref(),
            "city" => b.is_city,
            "viewer_side" => viewer_side,
            "plot_taken" => b.plot_taken,
            "city_captured" => b.city_captured,
            "rounds" => b.rounds,
            "defenders_left" => b.defenders_left as i32,
            "has_log" => true,
            "sides" => Json::Arr(vec![
                side_snap(Side::Attacker, b.attacker, &b.attacker_label),
                side_snap(Side::Defender, b.defender, &b.defender_label),
            ]),
        }
    }

    pub fn battle_log_json(&self, report_id: u32) -> Result<Json, ProtoErr> {
        let b = self
            .g
            .battles
            .iter()
            .find(|x| x.id.0 == report_id)
            .ok_or_else(|| ProtoErr::new("gone", format!("No battle report {report_id}.")))?;
        let viewer_side = if b.attacker == self.human {
            Some(0)
        } else if b.defender == self.human {
            Some(1)
        } else {
            None
        };
        let fighters_j: Vec<Json> = b
            .fighters
            .iter()
            .map(|f| {
                crate::jobj! {
                    "b" => f.index as i32,
                    "unit" => f.member_id.map(|m| m.0 as i32),
                    "sentinel" => f.sentinel_idx.is_some(),
                    "illusion" => false,
                    "summoner" => Json::Null,
                    "on_kill" => false,
                    "unit_type" => self.unit_art_idx(f.class),
                    "name" => f.name.as_str(),
                    "owner" => f.owner.0 as i32,
                    "side" => f.side.index() as i32,
                    "level" => f.level,
                    "hero" => f.is_hero,
                    "promotions" => Json::arr(),
                    "large" => f.large,
                    "lane" => f.lane as i32,
                    "row" => f.row as i32,
                    "max_hp" => f.max_hp,
                    "hp_start" => f.hp_start,
                    "a_x100" => f.a_x100,
                    "dv_x100" => f.dv_x100,
                    "ma_x100" => f.ma_x100,
                    "md_x100" => f.md_x100,
                    "speed" => f.speed,
                    "actions" => actions_json(&f.actions),
                    "bonus_action" => false,
                    "spell" => Json::Null,
                    "accuracy" => f.accuracy,
                    "evasion" => f.evasion,
                    "crit" => f.crit,
                    "anti_flank" => f.anti_flank,
                    "withdraw_pct" => 0,
                    "combat_limit" => 100,
                    "splash" => false,
                    "first_strikes" => 0,
                }
            })
            .collect();

        let events_j: Vec<Json> = b
            .events
            .iter()
            .map(|ev| {
                let hits: Vec<Json> = ev
                    .hits
                    .iter()
                    .map(|ht| {
                        crate::jobj! {
                            "target" => ht.target as i32,
                            "dmg" => ht.dmg,
                            "expected" => ht.expected,
                            "r" => ht.r,
                            "crit" => ht.crit,
                            "miss" => ht.miss,
                            "killed" => ht.killed,
                            "withdrew" => ht.withdrew,
                            "splash" => ht.splash,
                        }
                    })
                    .collect();
                let heal = ev.heal.map(|hl| crate::jobj! { "target" => hl.target as i32, "amount" => hl.amount, "r" => hl.r });
                let knocked = ev.knocked.map(|kn| crate::jobj! { "unit" => kn.unit as i32, "from" => kn.from as i32, "to" => kn.to as i32 });
                let note = ev.note.map(|n| match n {
                    Note::NoTarget => "NoTarget",
                    Note::NoOneHurt => "NoOneHurt",
                    Note::Held => "Held",
                    Note::Skipped => "Skipped",
                });
                crate::jobj! {
                    "round" => ev.round as i32,
                    "pass" => ev.pass as i32,
                    "actor" => ev.actor as i32,
                    "kind" => kind_name(ev.kind),
                    "spell" => Json::Null,
                    "target" => ev.target.map(|t| t as i32),
                    "hits" => Json::Arr(hits),
                    "heal" => heal,
                    "knocked" => knocked,
                    "note" => note,
                    "entered" => Json::arr(),
                }
            })
            .collect();

        let side_tally = |side: Side| {
            let committed = b.fighters.iter().filter(|f| f.side == side).count() as i32;
            let fallen = b.fighters.iter().filter(|f| f.side == side && matches!(f.fate, Fate::Fell { .. })).count() as i32;
            crate::jobj! {
                "committed" => committed,
                "sat_out" => 0,
                "fallen" => fallen,
                "escaped" => 0,
                "standing" => (committed - fallen).max(0),
            }
        };

        let rules_j: Vec<Json> = b.ground_rules.iter().filter_map(ground_rule_json).collect();
        let slot_j = || crate::jobj! { "base" => 5, "changes" => Json::arr(), "total" => 5, "river_only" => false };
        Ok(crate::jobj! {
            "v" => 2,
            "id" => b.id.0 as i32,
            "turn" => b.turn,
            "viewer_side" => viewer_side,
            "plot" => self.plot_snap_json(b.to),
            "from" => self.plot_snap_json(b.from),
            "ground" => crate::jobj! {
                "city" => b.ground.city,
                "forest" => b.ground.forest,
                "hills" => b.ground.hills,
                "open" => b.ground.open,
                "marsh" => b.ground.marsh,
                "crossing" => b.ground.crossing,
                "flanking" => b.ground.flanking(),
                "rules" => Json::Arr(rules_j),
            },
            "params" => crate::jobj! { "bolt_pct" => 60, "splash_pct" => 50, "first_strike_speed" => 2, "extra_action_tier" => 4, "extra_actions" => 1 },
            "slots" => Json::Arr(vec![slot_j(), slot_j()]),
            "lanes" => vec![3, 3],
            "sides" => Json::Arr(vec![
                crate::jobj! { "player" => b.attacker.0 as i32, "shown_player" => b.attacker.0 as i32, "civ" => self.civ_art_idx(b.attacker), "label" => b.attacker_label.as_str(), "lanes" => 3, "ordered" => true },
                crate::jobj! { "player" => b.defender.0 as i32, "shown_player" => b.defender.0 as i32, "civ" => self.civ_art_idx(b.defender), "label" => b.defender_label.as_str(), "lanes" => 3, "ordered" => true },
            ]),
            "fighters" => Json::Arr(fighters_j),
            "events" => Json::Arr(events_j),
            "outcome" => crate::jobj! {
                "plot_taken" => b.plot_taken,
                "city_captured" => if b.city_captured { self.g.world.plot(b.to).city.map(|c| c.0 as i32) } else { None },
                "rounds" => b.rounds,
                "defenders_left" => b.defenders_left as i32,
                "pushed" => Json::arr(),
                "hp_lost" => vec![b.hp_lost[0], b.hp_lost[1]],
                "loser" => b.loser.map(|s| s.index() as i32),
                "decided_by" => b.decided_by,
                "calc" => Json::Arr(b.verdict_calc.iter().cloned().map(Json::from).collect()),
                "sides" => Json::Arr(vec![side_tally(Side::Attacker), side_tally(Side::Defender)]),
            },
            "after" => Json::arr(),
            "summary" => self.battle_summary_json(b),
        })
    }

    // -----------------------------------------------------------------------------------------
    // Protocol Queries & Commands
    // -----------------------------------------------------------------------------------------

    pub fn query(&self, j: &Json) -> Result<Json, ProtoErr> {
        let q = j.req_str("q")?;
        match q {
            "status" => Ok(self.status()),
            "view" => Ok(self.view()),
            "map" => Ok(self.map()),
            "data" => Ok(crate::view::data::data(self.d).clone()),
            "setup" => Ok(crate::setup::setup_lists(self.d)),
            "cataclysm_setup" => Ok(setup_json(self.d)),
            "log" => {
                let since = j.get("since").and_then(Json::as_i64).unwrap_or(0).max(0) as usize;
                let entries: Vec<Json> = self
                    .g
                    .log
                    .iter()
                    .enumerate()
                    .skip(since)
                    .filter(|(_, e)| e.player.is_none() || e.player == Some(self.human) || self.reveal.get())
                    .map(|(i, e)| {
                        let at = e.plot.map(|p| {
                            let pt = self.g.world.plot(p);
                            crate::jarr![pt.x, pt.y]
                        });
                        let mut o = crate::jobj! {
                            "i" => i as i64,
                            "t" => e.turn,
                            "text" => e.text.as_str(),
                            "player" => e.player.map(|p| p.0 as i32),
                        };
                        if let Some(a) = at {
                            o.set("at", a);
                        }
                        if let Some(bid) = e.battle {
                            o.set("kind", "engagement");
                            o.set("report", bid.0 as i32);
                        }
                        o
                    })
                    .collect();
                Ok(crate::jobj! { "entries" => Json::Arr(entries), "next" => self.g.log.len() as i64 })
            }
            "report" => {
                let engs: Vec<Json> = self
                    .last_report_battles
                    .iter()
                    .filter_map(|&id| self.g.battles.iter().find(|b| b.id.0 == id))
                    .map(|b| self.battle_summary_json(b))
                    .collect();
                Ok(crate::jobj! {
                    "turn" => self.g.turn,
                    "from_turn" => self.g.turn,
                    "lines" => Json::Arr(self.last_report_lines.iter().cloned().map(Json::from).collect()),
                    "messages" => Json::arr(),
                    "engagements" => Json::Arr(engs),
                })
            }
            "moments" => Ok(crate::jobj! { "entries" => Json::arr(), "next" => 0 }),
            "battle_log" => {
                let rep = j.req::<u32>("report")?;
                self.battle_log_json(rep)
            }
            "battle_reports" => {
                let reports: Vec<Json> = self
                    .g
                    .battles
                    .iter()
                    .rev()
                    .filter(|b| b.attacker == self.human || b.defender == self.human || self.reveal.get())
                    .take(25)
                    .map(|b| self.battle_summary_json(b))
                    .collect();
                Ok(crate::jobj! { "reports" => Json::Arr(reports) })
            }
            "cataclysm_preview" => {
                let sid = SquadId(j.req::<u32>("squad")?);
                let x = j.req::<i32>("x")?;
                let y = j.req::<i32>("y")?;
                let to = self.g.world.idx(x, y).ok_or_else(|| ProtoErr::invalid("The plot is outside the map."))?;
                let (built, fc) = self.g.preview_attack(self.human, sid, to).map_err(|e| ProtoErr::new(e.code, e.detail))?;
                Ok(crate::jobj! {
                    "runs" => fc.runs,
                    "plot_falls_pct" => fc.plot_falls_permille / 10,
                    "line_wiped_pct" => fc.line_wiped_permille / 10,
                    "attackers_wiped_pct" => fc.attackers_wiped_permille / 10,
                    "expected_att_lost" => format!("{:.1}", fc.expected_lost[0] as f64 / 100.0),
                    "expected_def_lost" => format!("{:.1}", fc.expected_lost[1] as f64 / 100.0),
                    "ground_city" => built.formation.ground.city,
                    "ground_hills" => built.formation.ground.hills,
                    "ground_forest" => built.formation.ground.forest,
                    "flanking" => built.formation.ground.flanking(),
                })
            }
            "plot" => {
                let x = j.req::<i32>("x")?;
                let y = j.req::<i32>("y")?;
                let Some(p) = self.g.world.idx(x, y) else {
                    return Err(ProtoErr::invalid("The plot is outside the map."));
                };
                Ok(self.plot_json(x, y, p))
            }
            "group" => Ok(crate::jobj! { "units" => Json::arr(), "actions" => Json::arr() }),
            _ => Ok(crate::jobj! {}),
        }
    }

    pub fn execute_json(&mut self, j: &Json) -> Result<Json, ProtoErr> {
        let c = j.req_str("c")?;
        let cmd = self.parse_command(c, j)?;
        let is_end_turn = matches!(cmd, Command::EndTurn);

        let mut all_events = self.g.apply(self.human, cmd).map_err(|e| ProtoErr::new(e.code, e.detail))?;
        let mut last_battle: Option<BattleId> = None;
        for ev in &all_events {
            if let Event::BattleFought { battle, .. } = ev {
                last_battle = Some(*battle);
            }
        }

        // If the human ended their turn, immediately run every AI & Neutral turn synchronously (takes ~1ms!)
        if is_end_turn {
            self.last_report_lines.clear();
            self.last_report_battles.clear();
            let mut guard = 0;
            while !self.g.is_over() && self.g.active_player != self.human && guard < 32 {
                let ap = self.g.active_player;
                let ai_ev = cataclysm_ai::play_turn(&mut self.g, ap);
                for ev in &ai_ev {
                    match ev {
                        Event::BattleFought { battle, attacker, defender, .. } if *attacker == self.human || *defender == self.human => {
                            self.last_report_battles.push(battle.0);
                        }
                        Event::CrisisTriggered { threshold, name, summary } => {
                            self.last_report_lines.push(format!("Worldseal Strain {threshold}: {name}. {summary}"));
                        }
                        Event::CarcosaRisen { .. } => {
                            self.last_report_lines.push(format!(
                                "Caldera Keep has risen. The {CARCOSA_TIMER_TURNS}-turn countdown has started; civilizations holding a Lifeboat when it ends win."
                            ));
                        }
                        Event::HyboremArrived { city, .. } => {
                            let cname = self.g.cities.get(city).map_or("Great Furnace", |c| c.name.as_str()).to_string();
                            self.last_report_lines.push(format!(
                                "King Cinder's Host entered the game at {cname}, at war with every Good civilization and the Wild Clans."
                            ));
                        }
                        Event::GreatPersonBorn { player, name, .. } if *player == self.human => {
                            self.last_report_lines.push(format!("Great Person {name} joined your Court."));
                        }
                        Event::TechCompleted { player, tech } if *player == self.human => {
                            self.last_report_lines.push(format!("Research complete: {}.", TECHS[tech.0 as usize].name));
                        }
                        Event::GameEnded { summary } => {
                            self.last_report_lines.push(summary.clone());
                        }
                        _ => {}
                    }
                }
                all_events.extend(ai_ev);
                guard += 1;
            }
            if self.last_report_lines.is_empty() {
                let line = self.report_line_default();
                self.last_report_lines.push(line);
            }
        }

        self.view_version += 1;
        let mut r = crate::jobj! { "applied" => true, "accepted" => true };
        if let Some(bid) = last_battle {
            r.set("report", bid.0 as i32);
            if let Some(rec) = self.g.battles.iter().find(|b| b.id == bid) {
                r.set("battle", self.battle_summary_json(rec));
            }
        }
        Ok(r)
    }

    fn parse_command(&self, c: &str, j: &Json) -> Result<Command, ProtoErr> {
        let plot_xy = |j: &Json| -> Result<PlotIdx, ProtoErr> {
            let x = j.req::<i32>("x")?;
            let y = j.req::<i32>("y")?;
            self.g.world.idx(x, y).ok_or_else(|| ProtoErr::invalid("The plot is outside the map."))
        };
        let parse_class = |key: &str| -> Result<UnitClassId, ProtoErr> {
            UNIT_CLASSES
                .iter()
                .find(|u| u.key == key)
                .map(|u| u.id)
                .ok_or_else(|| ProtoErr::invalid(format!("Unknown unit class: {key}.")))
        };
        Ok(match c {
            "end_turn" => Command::EndTurn,
            "cat_move" => Command::MoveSquad { squad: SquadId(j.req::<u32>("squad")?), to: plot_xy(j)? },
            "cat_attack" => Command::AttackPlot { squad: SquadId(j.req::<u32>("squad")?), to: plot_xy(j)? },
            "cat_set_cell" => Command::SetSquadCell {
                squad: SquadId(j.req::<u32>("squad")?),
                member: MemberId(j.req::<u32>("member")?),
                row: j.req::<u32>("row")? as u8,
                lane: j.req::<u32>("lane")? as u8,
            },
            "cat_auto_arrange" => Command::AutoArrangeSquad { squad: SquadId(j.req::<u32>("squad")?) },
            "cat_reinforce" => Command::ReinforceSquad {
                squad: SquadId(j.req::<u32>("squad")?),
                class: parse_class(j.req_str("class")?)?,
            },
            "cat_upgrade" => Command::UpgradeMember {
                squad: SquadId(j.req::<u32>("squad")?),
                member: MemberId(j.req::<u32>("member")?),
            },
            "cat_raid" => Command::SetSquadRaid { squad: SquadId(j.req::<u32>("squad")?), active: j.get_or("active", true)? },
            "cat_fortify" => Command::SetSquadFortify { squad: SquadId(j.req::<u32>("squad")?) },
            "cat_road" => Command::SquadBuildRoad { squad: SquadId(j.req::<u32>("squad")?) },
            "cat_fort" => Command::SquadBuildFort { squad: SquadId(j.req::<u32>("squad")?) },
            "cat_explore" => Command::SquadExploreLair { squad: SquadId(j.req::<u32>("squad")?) },
            "cat_found_city" => Command::SquadFoundCity {
                squad: SquadId(j.req::<u32>("squad")?),
                name: j.opt_str("name")?.map(str::to_string),
            },
            "cat_disband" => Command::DisbandSquad { squad: SquadId(j.req::<u32>("squad")?) },
            "cat_city_build" => {
                let city = CityId(j.req::<u32>("city")?);
                let kind = j.req_str("kind")?;
                let order = match kind {
                    "district" => {
                        let key = j.req_str("key")?;
                        let d = DISTRICTS.iter().find(|d| d.key == key).ok_or_else(|| ProtoErr::invalid("Unknown District."))?;
                        BuildOrder::District(d.kind)
                    }
                    "wonder" => {
                        let key = j.req_str("key")?;
                        let w = WonderKind::ALL.into_iter().find(|w| w.key() == key).ok_or_else(|| ProtoErr::invalid("Unknown Wonder."))?;
                        BuildOrder::Wonder(w)
                    }
                    "squad" => {
                        let arr = j.get("classes").and_then(Json::as_arr).ok_or_else(|| ProtoErr::invalid("A squad order needs a list of unit classes."))?;
                        let mut cls = Vec::new();
                        for item in arr {
                            cls.push(parse_class(item.as_str().ok_or_else(|| ProtoErr::invalid("A unit class must be a string."))?)?);
                        }
                        BuildOrder::Squad(cls)
                    }
                    "reinforce" => BuildOrder::Reinforce(parse_class(j.req_str("key")?)?),
                    "caravan" => BuildOrder::Caravan,
                    other => return Err(ProtoErr::invalid(format!("Unknown build kind: {other}."))),
                };
                Command::CityBuild { city, order }
            }
            "cat_sally" => {
                let city = CityId(j.req::<u32>("city")?);
                let posture = match j.req_str("posture")? {
                    "never" => SallyPosture::Never,
                    "always" => SallyPosture::Always,
                    _ => SallyPosture::Favoured,
                };
                Command::CitySallyPosture { city, posture }
            }
            "cat_raze" => Command::RazeCity { city: CityId(j.req::<u32>("city")?) },
            "cat_caravan" => Command::LaunchCaravan {
                origin: CityId(j.req::<u32>("origin")?),
                destination: CityId(j.req::<u32>("destination")?),
                resource: Resource::None,
            },
            "cat_research" => {
                let tid = j.req::<u32>("tech")? as u8;
                if tid as usize >= TECHS.len() {
                    return Err(ProtoErr::invalid("Unknown tech."));
                }
                Command::ChooseResearch { tech: TechId(tid) }
            }
            "cat_doctrine" => {
                let key = j.req_str("doctrine")?;
                let doc = Doctrine::ALL.into_iter().find(|d| d.key() == key).ok_or_else(|| ProtoErr::invalid("Unknown Doctrine."))?;
                Command::AdoptDoctrine { doctrine: doc, secondary: j.get_or("secondary", false)? }
            }
            "cat_religion" => {
                let id = j.req::<usize>("religion")?;
                let rel = *Religion::ALL.get(id).ok_or_else(|| ProtoErr::invalid("Unknown Creed."))?;
                Command::FoundOrAdoptReligion { religion: rel }
            }
            "cat_ritual" => {
                let key = j.req_str("ritual")?;
                let rk = RitualKind::ALL.into_iter().find(|r| r.key() == key).ok_or_else(|| ProtoErr::invalid("Unknown Ritual."))?;
                Command::CastRitual { ritual: rk }
            }
            "cat_gp" => {
                let gp = GreatPersonId(j.req::<u32>("gp")?);
                let action = match j.req_str("action")? {
                    "secret" => GreatPersonAction::DiscoverSecret,
                    "burst" => GreatPersonAction::TempoBurst,
                    "hero" => GreatPersonAction::CommissionHero(SquadId(j.req::<u32>("squad")?)),
                    "lord" => GreatPersonAction::CommissionLord(HouseId(j.req::<u32>("house")?)),
                    other => return Err(ProtoErr::invalid(format!("Unknown Great Person action: {other}."))),
                };
                Command::UseGreatPerson { gp, action }
            }
            "cat_war" => Command::DeclareWar { target: PlayerId(j.req::<u32>("target")? as u16) },
            "cat_peace" => Command::MakePeace { target: PlayerId(j.req::<u32>("target")? as u16) },
            "cat_crusade" => Command::CallCrusade { target: PlayerId(j.req::<u32>("target")? as u16) },
            "cat_lend" => Command::LendSquadToCrusade {
                crusade: CrusadeId(j.req::<u32>("crusade")?),
                squad: SquadId(j.req::<u32>("squad")?),
            },
            "cat_payoff" => Command::PayOffRaiders { raider: PlayerId(j.req::<u32>("raider")? as u16) },
            other => return Err(ProtoErr::invalid(format!("Unknown Cataclysm command: {other}."))),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Rules text: every sentence below states what `cataclysm-core` does, with its numbers taken from
// the rules' constants and functions.
// ---------------------------------------------------------------------------------------------

fn tech_name(t: TechId) -> &'static str {
    TECHS[t.0 as usize].name
}

fn faction_name(f: FactionId) -> &'static str {
    f.info().name
}

/// Every way the game can end that applies to a player of faction `f`.
pub fn victory_lines(f: FactionId) -> Vec<String> {
    let info = f.info();
    let mut out = Vec::new();
    if info.can_mortal_win {
        out.push(format!(
            "Lifeboat survival: hold a standing Lifeboat when the {}-turn countdown that starts at Strain 100 ends. Every civilization holding one wins.",
            CARCOSA_TIMER_TURNS
        ));
    }
    if info.can_demonic_win {
        out.push("Furnace victory: after Strain 100, every Lifeboat is razed while you are alive. Every living civilization that can win this way wins.".into());
    }
    out.push("Conquest: before Strain 100, be the last civilization alive. A civilization with no city and no squad is eliminated.".into());
    if f == FactionId::Elohim {
        out.push(format!("Turn limit: if turn {} comes before Strain 100, you win.", MAX_GAME_TURNS));
    } else {
        out.push(format!(
            "Turn limit: if turn {} comes before Strain 100, the {} win if any is alive; otherwise the civilization with the highest score wins.",
            MAX_GAME_TURNS,
            faction_name(FactionId::Elohim)
        ));
    }
    out
}

fn score_line() -> String {
    use cataclysm_core::game::{SCORE_PER_DISTRICT, SCORE_PER_SECRET, SCORE_PER_SIZE, SCORE_PER_TECH, SCORE_PER_UNIT};
    format!(
        "Score is your share of all civilizations' points: {} per city Size, {} per District, {} per tech, {} per Secret Tech, {} per squad unit.",
        SCORE_PER_SIZE, SCORE_PER_DISTRICT, SCORE_PER_TECH, SCORE_PER_SECRET, SCORE_PER_UNIT
    )
}

fn supply_pct_line() -> String {
    use cataclysm_core::game::{STARVE_ATTRITION_TURNS, STARVE_HP_LOSS_PCT};
    format!(
        "Out of supply, a squad's attack and magic in battle are {}% for turns 1-{} and {}% from turn {}; from turn {} every unit also loses {}% of its max HP at each of your turn starts (not below 1 HP).",
        supply_combat_pct(1),
        STARVE_ATTRITION_TURNS - 1,
        supply_combat_pct(STARVE_ATTRITION_TURNS),
        STARVE_ATTRITION_TURNS,
        STARVE_ATTRITION_TURNS,
        STARVE_HP_LOSS_PCT
    )
}

fn heal_line() -> String {
    use cataclysm_core::game::{HEAL_FIELD_PCT, HEAL_FORTIFIED_PCT, HEAL_OWN_CITY_PCT, HEAL_OWN_LAND_PCT};
    format!(
        "A supplied squad heals at your turn start: {}% of max HP in your own city that is not besieged, {}% in your land, {}% fortified elsewhere, {}% elsewhere. An unsupplied squad does not heal.",
        HEAL_OWN_CITY_PCT, HEAL_OWN_LAND_PCT, HEAL_FORTIFIED_PCT, HEAL_FIELD_PCT
    )
}

fn rules_help_json() -> Json {
    use cataclysm_core::city::{CREW_IMPROVE_TURNS, CREW_RADIUS, CREW_ROAD_TURNS, GROWTH_FOOD_BASE, GROWTH_FOOD_PER_SIZE, MAX_CITY_SIZE};
    use cataclysm_core::game::{
        renown, BORDER_CULTURE_RADIUS_2, SENTINEL_REPAIR_PER_TURN, UPKEEP_PER_MEMBER,
    };
    use cataclysm_core::supply::{BASE_SUPPLY_BUDGET, SUPPLY_COST_OWN, SUPPLY_COST_ROAD, SUPPLY_COST_WILD};
    let supply = vec![
        "Inside your own land a squad is always supplied, except in a besieged city.".to_string(),
        format!(
            "Elsewhere a supply line is traced to the nearest of your cities that is not besieged. Each plot costs {} on a road, {} in your land, {} elsewhere; the line may cost up to {} (+{} with {}, +{} with {}, +{} with a Quartermaster captain, plus Secret Techs).",
            SUPPLY_COST_ROAD,
            SUPPLY_COST_OWN,
            SUPPLY_COST_WILD,
            BASE_SUPPLY_BUDGET,
            cataclysm_core::supply::SUPPLY_BONUS_TRADE_ROADS,
            tech_name(cataclysm_core::data::TECH_TRADE_ROADS),
            cataclysm_core::supply::SUPPLY_BONUS_OUTRIDERS,
            tech_name(cataclysm_core::data::TECH_OUTRIDERS),
            cataclysm_core::supply::SUPPLY_BONUS_QUARTERMASTER
        ),
        "A plot with a squad of a civilization you are at war with blocks the line, and so does every plot next to such a squad standing on a Fort.".into(),
        format!("{} squads trace supply through forests at the cost of their own land.", faction_name(FactionId::Ljosalfar)),
        "A squad on a Raid order is always supplied.".into(),
        "At each of your turn starts a squad without a line adds 1 turn out of supply; a squad with a line resets to 0.".into(),
        supply_pct_line(),
    ];
    let squads = vec![
        "A squad is a 3x3 grid: front, middle and rear rows of 3 cells. The row a unit stands in decides the action it takes in battle.".to_string(),
        format!(
            "Each unit uses 1 place and a Large unit 2 places (2 cells side by side). A squad holds {} places, or {} once you know any tier-2 tech or {}.",
            cataclysm_core::game::SQUAD_PLACES_BASE,
            cataclysm_core::game::SQUAD_PLACES_EXPANDED,
            tech_name(cataclysm_core::data::TECH_GUILDS)
        ),
        format!(
            "Squad cap: {}, +1 per city that is not an Outpost, +1 per Barracks, +{} with {}, plus Secret Techs and the Banner Levy House perk (minimum {}).",
            cataclysm_core::game::SQUAD_CAP_BASE,
            cataclysm_core::game::SQUAD_CAP_GRAND_COMPANIES,
            tech_name(cataclysm_core::data::TECH_GRAND_COMPANIES),
            cataclysm_core::game::SQUAD_CAP_MIN
        ),
        format!(
            "Upkeep: {} Gold per unit per turn; the {} pay none for squads lent to a Crusade.",
            UPKEEP_PER_MEMBER,
            faction_name(FactionId::Bannor)
        ),
        "A squad moves as far as its slowest unit allows, +1 with a Swift March captain. Each captain gives the squad one aura.".into(),
        heal_line(),
        "Reinforcing a squad with a unit, or upgrading a unit, costs Gold and needs the squad inside your borders.".into(),
        format!(
            "A hero that falls returns at the capital after {} turns with 1 more Wound (-{}% battle stats per Wound).",
            cataclysm_core::game::HERO_RECOVERY_TURNS,
            cataclysm_core::squad::WOUND_PENALTY_PCT
        ),
    ];
    let p = ffh_battle::Params::default();
    let red = cataclysm_core::combat::default_params(cataclysm_core::combat::RED_SKY_STRAIN);
    let battle = vec![
        format!(
            "A battle lasts {} rounds. In each round every unit acts in order of speed, highest first, with the action of the row it stands in.",
            p.rounds
        ),
        "The loser is the side with no unit standing; else the side with more units fallen; else the side that lost more HP; otherwise it is even.".into(),
        "Outside a city a losing defender is pushed to a next plot; when no defender remains the attacker takes the plot. A city falls when no squad and no Sentinel stands in it.".into(),
        "Attacking ends the attacking squad's movement. Attacking a civilization you are at peace with declares war on it, except during a truce.".into(),
        "In a city, defending units add half the city's defense % to their defense. Standing Sentinels hold the front row, scaled by the city's defense %, and keep their damage between battles.".into(),
        format!(
            "Attacker shots into a city deal {}%; ranged attacks into a forest deal {}%; on hills the attacker deals {}% and the defender's ranged attacks {}%.",
            p.city_attacker_shot_pct, p.forest_ranged_pct, p.hills_attacker_pct, p.hills_defender_ranged_pct
        ),
        "Flanking happens only outside cities, forests and marsh.".into(),
        format!(
            "From Strain {} every battle deals {:+}% damage. A squad with a caster spends {} Mana per battle when it has it; with {} or more Mana its spells are stronger.",
            cataclysm_core::combat::RED_SKY_STRAIN,
            red.base_damage * 100 / p.base_damage.max(1) - 100,
            cataclysm_core::game::BATTLE_MANA,
            cataclysm_core::game::MANA_BOOST_MIN
        ),
    ];
    // District slots by Size, from the rule itself.
    // `extra_only`: list only the Sizes where a capital's count goes beyond an ordinary city's.
    let slot_table = |capital: bool, extra_only: bool| -> String {
        let mut base = cataclysm_core::City::new(CityId(0), PlayerId(0), String::new(), PlotIdx(0), false, false);
        let mut c = cataclysm_core::City::new(CityId(0), PlayerId(0), String::new(), PlotIdx(0), capital, false);
        let mut parts = Vec::new();
        let mut last = 0usize;
        for size in 1..=MAX_CITY_SIZE {
            c.size = size;
            base.size = size;
            let n = c.max_district_slots(false);
            if n != last {
                if !extra_only || n > base.max_district_slots(false) {
                    parts.push(format!("{n} at Size {size}"));
                }
                last = n;
            }
        }
        parts.join(", ")
    };
    let cities = vec![
        format!(
            "A city grows 1 Size when its stored Food reaches {} + {} x Size (up to Size {}).",
            GROWTH_FOOD_BASE, GROWTH_FOOD_PER_SIZE, MAX_CITY_SIZE
        ),
        format!(
            "District slots: {}; a capital or Megacity also gets {}; an Outpost has 1. Each District can be built once per city.",
            slot_table(false, false),
            slot_table(true, true)
        ),
        format!(
            "A city builds one thing at a time from its Industry: a District, a squad (the sum of its units' Industry, at least {}), one unit, a Caravan ({} Industry; it picks its own destination and needs no Caravan slot or Gold), or a Wonder ({} Industry, one of each in the world).",
            BuildOrder::Squad(Vec::new()).cost(),
            BuildOrder::Caravan.cost(),
            WonderKind::TowerOfMastery.cost()
        ),
        "Industry adds up every turn; choosing a build resets its progress to 0.".into(),
        format!(
            "Each city's Work Crew improves one plot of your land within {} plots at a time: the most valuable improvement first (Mana Vent, Fishing Boats, Mine or Pasture, Farm, Lumbermill, Cottage), else a road on a next plot. An improvement takes {} turns, a road {}. It stops while the city is besieged.",
            CREW_RADIUS, CREW_IMPROVE_TURNS, CREW_ROAD_TURNS
        ),
        format!(
            "Borders reach 1 plot around a city, 2 from {} culture; a plot belongs to the civilization with the most culture on it.",
            BORDER_CULTURE_RADIUS_2
        ),
        format!(
            "A capital has 1 Sentinel and a Bulwark 2 (3 with {}). Sentinels repair {} damage per turn while the city is not besieged.",
            tech_name(cataclysm_core::data::TECH_MASONRY_WARDS),
            SENTINEL_REPAIR_PER_TURN
        ),
        format!(
            "Capturing a city clears its build and Work Crew, breaches its Sentinels, removes it from its House and gives the captor {} Commander Renown.",
            renown::CAPTURE_COMMANDER
        ),
    ];
    // Moods: thresholds found from the rule itself.
    let mood_lines = {
        let mut parts = Vec::new();
        let mut cur: Option<(HouseMood, i32, i32)> = None;
        for loy in -10..=10 {
            let m = House::mood_of(loy);
            match cur {
                Some((cm, lo, _)) if cm == m => cur = Some((cm, lo, loy)),
                _ => {
                    if let Some(x) = cur {
                        parts.push(x);
                    }
                    cur = Some((m, loy, loy));
                }
            }
        }
        if let Some(x) = cur {
            parts.push(x);
        }
        parts
            .iter()
            .enumerate()
            .map(|(i, (m, lo, hi))| {
                let range = if i == 0 {
                    format!("{hi} or less")
                } else if i == parts.len() - 1 {
                    format!("{lo} or more")
                } else if lo == hi {
                    format!("{lo}")
                } else {
                    format!("{lo} to {hi}")
                };
                format!("{} ({}): {}%", m.name(), range, m.output_pct())
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let houses = vec![
        format!(
            "Every city that is not an Outpost belongs to a House. Cities of the same origin join one House, up to {} cities; then a new House forms with a random perk.",
            cataclysm_core::house::HOUSE_MAX_CITIES
        ),
        format!(
            "Loyalty is 1, plus empire bonuses ({} +1, {} +2, the Great Cistern +2, Secret Techs) and lasting modifiers.",
            tech_name(cataclysm_core::data::TECH_PRIESTHOOD),
            tech_name(cataclysm_core::data::TECH_CROWN_CHARTERS)
        ),
        "+1 if the House shares your alignment, -2 if one is Good and the other Evil; +1 if it shares your Creed, -1 if both have a Creed and they differ.".into(),
        "-3 while you are at war with the civilization the House sympathizes with; +1 per Temple in the House (at most +2); -1 per House you own beyond 2 (at most -3).".into(),
        format!("Moods by Loyalty, with the Industry, Gold and Lore of the House's cities: {}.", mood_lines),
        "The House perk works unless the House is Furious.".into(),
        "Appointing a Great Person as Lord adds lasting Loyalty and aligns the House to you.".into(),
    ];
    crate::jobj! {
        "supply" => supply,
        "squads" => squads,
        "battle" => battle,
        "cities" => cities,
        "houses" => houses,
    }
}

fn apocalypse_rules() -> Vec<String> {
    use cataclysm_core::game::{BLOODY_BATTLES_PER_STRAIN, BLOODY_BATTLE_DEATHS, BLOODY_BATTLE_STRAIN, RAZE_STRAIN, STRAIN_DRIFT_EVERY};
    vec![
        format!(
            "Strain rises by 1 every {} turns until the Blowout; Valve-Keeper's Ledger, held by any civilization that knows its tech, stops this.",
            STRAIN_DRIFT_EVERY
        ),
        format!(
            "A tech changes Strain by the amount it shows only for the first civilization to research it. Founding a Creed, Vent the Seal, Crack the Crust, Wonders, some Secret Techs and razing a city ({:+}) change Strain by the amount each shows.",
            RAZE_STRAIN
        ),
        format!(
            "Every {} battles in the world in which {} or more units fall add {:+} Strain.",
            BLOODY_BATTLES_PER_STRAIN, BLOODY_BATTLE_DEATHS, BLOODY_BATTLE_STRAIN
        ),
        format!(
            "Strain falls through Vent the Seal, a Prophet's Tempo Burst ({:+}), and the techs and Creeds with a negative Strain change.",
            cataclysm_core::great_people::BURST_PROPHET_STRAIN
        ),
        "Each multiple of 10 triggers its crisis once.".into(),
        "At Strain 50 King Cinder's Host enters the game, if it is not already in it.".into(),
        format!(
            "At Strain 100 (the Blowout) Caldera Keep rises, the Lifeboats are fixed, a {}-turn countdown starts, and Strain no longer changes.",
            CARCOSA_TIMER_TURNS
        ),
        format!("Caldera Keep cannot be razed during its first {} turns.", CARCOSA_PROTECTED_TURNS),
    ]
}

fn lifeboat_rules() -> Vec<String> {
    use cataclysm_core::game::LIFEBOAT_CULTURE_MIN;
    vec![
        "A Lifeboat is a city: whoever holds it when the countdown ends wins, if that civilization can win by Lifeboat survival.".to_string(),
        format!(
            "Kinds: {} (always), the {} and the {} (the cities holding those Wonders), and the {} (the city of any civilization with the most culture, at least {}, if not already a Lifeboat).",
            LifeboatKind::Carcosa.name(),
            LifeboatKind::TowerOfMastery.name(),
            LifeboatKind::AltarOfLuonnotar.name(),
            LifeboatKind::MostLegendary.name(),
            LIFEBOAT_CULTURE_MIN
        ),
        "The Lifeboats are fixed at Strain 100; a Wonder completed later becomes a Lifeboat when it is completed.".into(),
        "Caldera Keep starts held by the Wild Clans with a guard squad.".into(),
        "Razing a Lifeboat city destroys that Lifeboat. If every Lifeboat is razed, the Furnace civilizations alive win; if none is alive, no one wins.".into(),
        "If the countdown ends and no Lifeboat is held by a civilization that can win by it, no one wins. The Wild Clans and King Cinder's Host win nothing by holding one.".into(),
    ]
}

fn order_help_json(found_cost: i32) -> Json {
    use cataclysm_core::game::{
        FOUND_CITY_BASE_GOLD, FOUND_CITY_GOLD_PER_CITY, HEAL_FIELD_PCT, HEAL_FORTIFIED_PCT, LAIR_GOLD, LAIR_LORE, LAIR_MANA, LAIR_XP, MIN_CITY_DISTANCE,
        RAID_RANGE, RAID_STREAK_TO_STRIP, RAID_TAKE_PCT, RAID_TAKE_SALLIED_PCT, STRIP_TURNS,
    };
    let fc = tech_name(cataclysm_core::data::TECH_FIELDCRAFT);
    crate::jobj! {
        "fortify" => format!(
            "Ends the squad's movement this turn. Supplied outside your land, it heals {}% per turn instead of {}%. It has no effect in battle; moving ends it.",
            HEAL_FORTIFIED_PCT, HEAL_FIELD_PCT
        ),
        "raid" => format!(
            "Requires {fc}; the squad must stand on the land of a civilization you are at war with, outside a city. Ends its movement; moving ends the order. At the end of that civilization's turn, the raiders within {RAID_RANGE} plots of each of its cities take {RAID_TAKE_PCT}% of the city's Gold + Industry as Gold ({RAID_TAKE_SALLIED_PCT}% if its garrison sallies); the city's owner loses nothing. {RAID_STREAK_TO_STRIP} raided turns in a row strip the countryside (Industry and Gold halved) for {STRIP_TURNS} turns. A raiding squad is always supplied."
        ),
        "road" => format!(
            "Requires {fc} and movement left; uses the rest of the squad's movement. Builds a road on its plot: entering it costs 1 move and supply lines cross it at cost {}. No Gold.",
            cataclysm_core::supply::SUPPLY_COST_ROAD
        ),
        "fort" => format!(
            "Requires {fc} and movement left; uses the rest of the squad's movement. Builds a Fort on its plot (not in a city), replacing any improvement. While a squad stands on it, the enemy's supply lines cannot pass the plots next to it. No Gold."
        ),
        "found_city" => format!(
            "Founds a city on the squad's plot, at least {MIN_CITY_DISTANCE} plots from every city; uses the squad's movement and keeps the squad. Costs {FOUND_CITY_BASE_GOLD} Gold + {FOUND_CITY_GOLD_PER_CITY} per city you own (now {found_cost})."
        ),
        "disband" => "Removes the squad and all its units. No refund.",
        "auto_arrange" => "Places every unit in its preferred row, Large units first, centre lane first. Free; uses no movement.",
        "explore" => format!(
            "Explores the Imperial Ruin on the squad's plot and ends its movement. One at random: +{LAIR_GOLD} Gold; +{LAIR_MANA} Mana and +{} Sage Renown; +{LAIR_LORE} Lore; +{LAIR_XP} XP for every unit in the squad.",
            cataclysm_core::game::renown::LAIR_RELICS_SAGE
        ),
    }
}

fn sally_help_json() -> Json {
    use cataclysm_core::game::{RAID_RANGE, RAID_TAKE_PCT, RAID_TAKE_SALLIED_PCT, SALLY_FAVOURED_PCT};
    let cut = format!("A sally, won or lost, cuts what every raider takes from this city that turn from {RAID_TAKE_PCT}% to {RAID_TAKE_SALLIED_PCT}%.");
    crate::jobj! {
        "never" => format!("The garrison never fights raiders within {RAID_RANGE} plots; they take {RAID_TAKE_PCT}% of the city's Gold + Industry each turn."),
        "favoured" => format!(
            "At the end of your turn, the first squad in the city fights the first raider within {RAID_RANGE} plots if its power is at least {SALLY_FAVOURED_PCT}% of the raider's. {cut}"
        ),
        "always" => format!("At the end of your turn, the first squad in the city always fights the first raider within {RAID_RANGE} plots. {cut}"),
    }
}

fn court_rules_json() -> Json {
    use cataclysm_core::great_people::{LORD_LOYALTY, RENOWN_FIRST, RENOWN_STEP};
    use cataclysm_core::squad::{HERO_HP_BONUS, HERO_LEVEL, HERO_STAT_PCT};
    let suits: Vec<&str> = GreatSuit::ALL.iter().map(|s| s.name()).collect();
    crate::jobj! {
        "arrival" => format!(
            "At your turn start, when your Renown across all six meters reaches the threshold ({RENOWN_FIRST} for the first Great Person, +{RENOWN_STEP} for each later one), a Great Person of your highest meter's suit joins the Court and that meter resets to 0 (ties: the first in the order {}). It draws a Secret Tech card of its suit from the world deck, if one is left.",
            suits.join(", ")
        ),
        "secret" => "You keep the Secret Tech card; its effect works while you know the tech it needs. The Great Person leaves the Court.",
        "burst" => "The Secret Tech card returns to the world deck and the Great Person leaves the Court.",
        "hero" => format!(
            "Joins one of your squads as a level {HERO_LEVEL} hero unit of the suit's class (+{HERO_HP_BONUS} max HP, +{HERO_STAT_PCT}% battle stats) and becomes its captain. Needs a free place and cell. The Secret Tech card returns to the world deck."
        ),
        "lord" => format!(
            "Replaces the lord of one of your Houses, which takes your alignment and Creed, drops its sympathy for a rival and gains +{LORD_LOYALTY} lasting Loyalty. The Secret Tech card returns to the world deck."
        ),
    }
}

fn renown_sources() -> String {
    use cataclysm_core::game::renown as r;
    format!(
        "Renown: Commander +{} per battle fought (attacker and defender) and +{} per enemy unit killed, +{} per city captured; Sage +{} per Ritual, +{} from some Imperial Ruins, +{} per Academy per turn, +1 per {} Lore when no tech can be researched; Prophet +{} for founding a Creed, +{} for lending a squad to a Crusade, +{} per Temple per turn; Bard +{} per Temple per turn; Engineer +{} per Forge per turn, +{} per Wonder; Merchant +{} per Market per turn, +{} per moving Caravan per turn, +{} per raiding squad per turn, +{} per Caravan ambushed.",
        r::BATTLE_COMMANDER,
        r::BATTLE_COMMANDER_PER_KILL,
        r::CAPTURE_COMMANDER,
        r::RITUAL_SAGE,
        r::LAIR_RELICS_SAGE,
        r::DISTRICT_PER_TURN,
        cataclysm_core::game::LORE_PER_SAGE_RENOWN,
        r::FOUND_CREED_PROPHET,
        r::LEND_PROPHET,
        r::DISTRICT_PER_TURN,
        r::DISTRICT_PER_TURN,
        r::DISTRICT_PER_TURN,
        r::WONDER_ENGINEER,
        r::DISTRICT_PER_TURN,
        r::CARAVAN_MERCHANT,
        r::RAID_MERCHANT,
        r::AMBUSH_MERCHANT
    )
}

fn creed_rule() -> String {
    format!(
        "Founding or adopting a Creed requires {}. Founding one gives +{} Prophet Renown; any other civilization may then adopt it for free. A House with a Creed has +1 Loyalty while it shares yours and -1 while it differs.",
        tech_name(cataclysm_core::data::TECH_MYSTICISM),
        cataclysm_core::game::renown::FOUND_CREED_PROPHET
    )
}

fn doctrine_rule() -> String {
    use cataclysm_core::game::{PRIMARY_DOCTRINE_COST_PCT, SECONDARY_DOCTRINE_MAX_TIER};
    format!(
        "Each player adopts one Primary and one Secondary Doctrine, once each; neither can be changed this game. The Primary opens all tiers of its techs at {}% cost; the Secondary, adopted after the Primary, opens tiers 1-{} at full cost. {}",
        PRIMARY_DOCTRINE_COST_PCT,
        SECONDARY_DOCTRINE_MAX_TIER,
        lore_overflow_rule()
    )
}

fn lore_overflow_rule() -> String {
    format!(
        "When no tech can be researched now (Doctrines not yet adopted open none), Lore converts to Sage Renown at your turn start: {} Lore per 1 Renown, the remainder kept.",
        cataclysm_core::game::LORE_PER_SAGE_RENOWN
    )
}

fn peace_effect() -> String {
    use cataclysm_core::game::{PEACE_OFFER_TURNS, TRUCE_TURNS};
    format!(
        "Peace needs both sides: an offer stands for {} turns, and peace is signed when the other side offers peace while it stands. After peace, neither side may declare war, attack the other, call a Crusade against it or lend a squad to one for {} turns. No peace can be made with the Wild Clans.",
        PEACE_OFFER_TURNS, TRUCE_TURNS
    )
}

fn crusade_effect() -> String {
    use cataclysm_core::game::{renown, LEND_CREDIT, LEND_CREDIT_BANNOR};
    format!(
        "Declares war on the target (Caravans are not ended) and opens a Crusade for {} turns. Any other civilization may lend squads to it: the lender goes to war with the target and gains {} Prophet Renown; Crusade credit ({} per squad, {} for the {}) is recorded but has no effect. The {} pay no upkeep for lent squads.",
        cataclysm_core::crusade::CRUSADE_TURNS,
        renown::LEND_PROPHET,
        LEND_CREDIT,
        LEND_CREDIT_BANNOR,
        faction_name(FactionId::Bannor),
        faction_name(FactionId::Bannor)
    )
}

fn plural(n: i32, one: &str, many: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {many}")
    }
}

impl CataclysmSession {
    /// The squad's supply state from its counter, its line and the rules' thresholds.
    fn supply_json(&self, s: &cataclysm_core::Squad, line: Option<&cataclysm_core::SupplyLine>) -> Json {
        use cataclysm_core::game::{STARVE_ATTRITION_TURNS, STARVE_HP_LOSS_PCT};
        let n = s.unsupplied_turns;
        let pct = supply_combat_pct(n);
        let pen = |p: i32| format!("{:+}%", p - 100);
        let (key, label, detail) = match (line, n) {
            (Some(l), 0) => {
                let budget = self.g.supply_budget(s.owner, s.captain_aura);
                let d = if l.cost == 0 {
                    "Supplied. Full strength in battle.".to_string()
                } else {
                    format!("Supplied: the line costs {} of {}. Full strength in battle.", l.cost, budget)
                };
                ("supplied", "Supplied".to_string(), d)
            }
            (Some(_), _) => (
                "recovering",
                format!("Recovering ({})", pen(pct)),
                format!(
                    "A supply line is open again. Attack and magic stay at {}% until your next turn start, when the squad returns to full strength and heals.",
                    pct
                ),
            ),
            (None, 0) => (
                "cut",
                "Cut".to_string(),
                format!(
                    "No supply line. No penalty yet; at your next turn start without a line the squad becomes Hungry: attack and magic {}%, no healing.",
                    supply_combat_pct(1)
                ),
            ),
            (None, _) if n < STARVE_ATTRITION_TURNS => {
                let next = n + 1;
                let mut d = format!(
                    "No supply line for {}: attack and magic {}%, no healing. At your next turn start without a line: {}%",
                    plural(n, "turn", "turns"),
                    pct,
                    supply_combat_pct(next)
                );
                if next >= STARVE_ATTRITION_TURNS {
                    d.push_str(&format!(", and every unit loses {}% of its max HP each turn.", STARVE_HP_LOSS_PCT));
                } else {
                    d.push('.');
                }
                ("hungry", format!("Hungry ({})", pen(pct)), d)
            }
            (None, _) => (
                "starving",
                format!("Starving ({})", pen(pct)),
                format!(
                    "No supply line for {}: attack and magic {}%; every unit loses {}% of its max HP at each of your turn starts (not below 1 HP).",
                    plural(n, "turn", "turns"),
                    pct,
                    STARVE_HP_LOSS_PCT
                ),
            ),
        };
        crate::jobj! { "key" => key, "label" => label, "detail" => detail }
    }

    fn crew_json(&self, c: &cataclysm_core::City) -> Json {
        let g = &self.g;
        let label = |t: &cataclysm_core::WorkCrewTask| if t.build_road { "Road".to_string() } else { t.improvement.name().to_string() };
        let at = |t: &cataclysm_core::WorkCrewTask| {
            let p = g.world.plot(t.plot);
            format!("({}, {})", p.x, p.y)
        };
        let (state, text) = if g.players[c.owner.idx()].is_neutral || c.besieged {
            ("halted", "Work Crew: halted while the city is besieged.".to_string())
        } else if let Some(t) = &c.crew_task {
            ("working", format!("Work Crew: {} at {}, {} of {} turns.", label(t), at(t), t.progress, t.needed))
        } else if let Some(t) = c.pick_crew_task(&g.world) {
            ("starting", format!("Work Crew: starts a {} at {} next turn ({} turns).", label(&t), at(&t), t.needed))
        } else {
            ("done", format!("Work Crew: nothing left to improve within {} plots.", cataclysm_core::city::CREW_RADIUS))
        };
        crate::jobj! { "state" => state, "text" => text }
    }

    fn city_notes(&self, c: &cataclysm_core::City) -> Vec<Json> {
        use cataclysm_core::game::{
            HEAL_OWN_CITY_PCT, HEAL_OWN_LAND_PCT, RAID_RANGE, RAID_STREAK_TO_STRIP, RAID_TAKE_PCT, SENTINEL_REPAIR_PER_TURN, STRIP_TURNS,
        };
        let mut out = Vec::new();
        let warn = |t: String| crate::jobj! { "tone" => "warn", "text" => t };
        if c.besieged {
            out.push(warn(format!(
                "Besieged (an enemy squad stood next to the city at the last move or battle). Sentinels are not repaired (otherwise {} per turn), the Work Crew stops, new Sentinels start breached, and the city supplies no squad. Squads here are supplied only by a line to another of your cities that is not besieged, and then heal {}% instead of {}%.",
                SENTINEL_REPAIR_PER_TURN, HEAL_OWN_LAND_PCT, HEAL_OWN_CITY_PCT
            )));
        }
        if c.stripped_turns > 0 {
            out.push(warn(format!(
                "Stripped countryside: Industry and Gold halved for the next {}.",
                plural(c.stripped_turns, "turn", "turns")
            )));
        }
        if c.raid_streak > 0 {
            out.push(warn(format!(
                "Raided {} in a row: raiders within {} plots take {}% of this city's Gold + Industry as Gold each turn (you lose nothing). At {} in a row the countryside is stripped for {} turns.",
                plural(c.raid_streak, "turn", "turns"),
                RAID_RANGE,
                RAID_TAKE_PCT,
                RAID_STREAK_TO_STRIP,
                STRIP_TURNS
            )));
        }
        out
    }

    fn raze_json(&self, c: &cataclysm_core::City) -> Json {
        use cataclysm_core::game::RAZE_STRAIN;
        let g = &self.g;
        let check = g.check_raze(self.human, c.id);
        let mut effect = format!("Removes {} for good; its plot becomes {}.", c.name, cataclysm_core::Terrain::BurningSands.name());
        if !g.apocalypse.carcosa_risen {
            let moved = (g.apocalypse.counter + RAZE_STRAIN).clamp(0, 100) - g.apocalypse.counter;
            if moved != 0 {
                effect.push_str(&format!(" {:+} Strain.", moved));
            }
        }
        if let Some(kind) = g.apocalypse.is_lifeboat(c.id) {
            effect.push_str(&format!(" Destroys the {} Lifeboat.", kind.name()));
        }
        if g.caravans.values().any(|cv| cv.origin == c.id || cv.destination == c.id) {
            effect.push_str(" Ends the Caravans to and from it.");
        }
        crate::jobj! {
            "allowed" => check.is_ok(),
            "why" => check.err().map(|e| e.detail),
            "effect" => effect,
        }
    }

    fn caravan_destinations(&self, c: &cataclysm_core::City) -> Vec<Json> {
        let g = &self.g;
        g.cities
            .values()
            .filter(|d| d.id != c.id && (d.owner == self.human || self.is_revealed(d.plot)))
            .filter(|d| !g.at_war(self.human, d.owner))
            .filter(|d| cataclysm_core::economy::find_caravan_path(&g.world, c.plot, d.plot).is_some())
            .map(|d| crate::jobj! { "id" => d.id.0 as i32, "name" => d.name.as_str(), "owner_name" => g.players[d.owner.idx()].name() })
            .collect()
    }

    fn district_slot_note(&self, c: &cataclysm_core::City) -> Option<String> {
        let mega = self.g.is_megacity(c);
        let now = c.max_district_slots(mega);
        if c.is_outpost {
            return Some("An Outpost has 1 District slot.".into());
        }
        let mut probe = c.clone();
        for size in c.size + 1..=cataclysm_core::city::MAX_CITY_SIZE {
            probe.size = size;
            if probe.max_district_slots(mega) > now {
                return Some(format!("Next slot at Size {size}."));
            }
        }
        Some(format!("No more District slots from growth (maximum {now})."))
    }

    fn grow_turns(&self, c: &cataclysm_core::City) -> Option<i32> {
        if c.size >= cataclysm_core::city::MAX_CITY_SIZE {
            return None;
        }
        let food = self.g.city_yields(c.id).0.max(1);
        let rem = (c.food_needed() - c.food_stored).max(0);
        Some((rem + food - 1) / food)
    }

    fn victory_json(&self) -> Json {
        let g = &self.g;
        let me = &g.players[self.human.idx()];
        let status = g.apocalypse.countdown.map(|cd| {
            let standing: Vec<_> = g.apocalypse.lifeboats.iter().filter(|l| !l.razed).collect();
            let held = standing.iter().filter(|l| g.cities.get(&l.city_id).is_some_and(|c| c.owner == self.human)).count();
            format!("Countdown: {}. You hold {} of {} standing Lifeboats.", plural(cd.max(0), "turn", "turns"), held, standing.len())
        });
        let mut lines = victory_lines(me.faction);
        lines.push(score_line());
        crate::jobj! { "lines" => lines, "status" => status }
    }

    fn turns_left(&self) -> i32 {
        match self.g.apocalypse.countdown {
            Some(cd) => cd.max(0),
            None => (MAX_GAME_TURNS - self.g.turn).max(0),
        }
    }

    fn over_json(&self, o: &cataclysm_core::GameOutcome) -> Json {
        let g = &self.g;
        let h = self.human;
        let you_won = o.winners.contains(&h);
        let first = o.winners.first().map(|p| g.players[p.idx()].name()).unwrap_or("No one");
        let your_result = match o.kind {
            VictoryKind::MortalLifeboat if you_won => "You held a Lifeboat when the countdown ended.".to_string(),
            VictoryKind::MortalLifeboat => "You held no Lifeboat that counts when the countdown ended.".to_string(),
            VictoryKind::DemonicRuin if you_won => "Every Lifeboat was razed and you can win by the Furnace victory.".to_string(),
            VictoryKind::DemonicRuin => "Every Lifeboat was razed; only Furnace civilizations win.".to_string(),
            VictoryKind::ElohimVigil if you_won => format!("Turn {} came before Strain 100 and you are the {}.", MAX_GAME_TURNS, faction_name(FactionId::Elohim)),
            VictoryKind::ElohimVigil => format!("Turn {} came before Strain 100; the {} win.", MAX_GAME_TURNS, faction_name(FactionId::Elohim)),
            VictoryKind::Conquest if you_won => "You are the last civilization alive.".to_string(),
            VictoryKind::Conquest => format!("{} is the last civilization alive.", first),
            VictoryKind::ScoreAtTurnLimit if you_won => format!("Turn {} came before Strain 100 and you have the highest score.", MAX_GAME_TURNS),
            VictoryKind::ScoreAtTurnLimit => format!("Turn {} came before Strain 100; {} has the highest score.", MAX_GAME_TURNS, first),
            VictoryKind::MutualExtinction => "No civilization won.".to_string(),
        };
        let your_result = if !you_won && !g.players[h.idx()].alive { format!("You were eliminated. {your_result}") } else { your_result };
        let (lifeboats, scores) = self.ending_tables();
        crate::jobj! {
            "winner_team" => o.winners.first().map(|p| p.0 as i32),
            "victory" => Json::Null,
            "cataclysm_kind" => o.kind.name(),
            "summary" => o.summary.as_str(),
            "winners" => Json::Arr(o.winners.iter().map(|p| Json::from(g.players[p.idx()].name())).collect()),
            "you_won" => you_won,
            "winner_factions" => Json::Arr(o.winners.iter().map(|p| Json::from(faction_name(g.players[p.idx()].faction))).collect()),
            "your_result" => your_result,
            "lifeboats" => lifeboats,
            "scores" => scores,
        }
    }

    /// The `over` object for a human eliminated before the game has an outcome.
    fn eliminated_json(&self) -> Json {
        let (lifeboats, scores) = self.ending_tables();
        crate::jobj! {
            "winner_team" => Json::Null,
            "victory" => Json::Null,
            "cataclysm_kind" => "Eliminated",
            "summary" => format!("{} was eliminated.", self.g.players[self.human.idx()].name()),
            "winners" => Json::arr(),
            "you_won" => false,
            "winner_factions" => Json::arr(),
            "your_result" => "You were eliminated: a civilization with no city and no squad left is out of the game.",
            "lifeboats" => lifeboats,
            "scores" => scores,
        }
    }

    /// The Lifeboat and score tables of an ending.
    fn ending_tables(&self) -> (Json, Json) {
        let g = &self.g;
        let h = self.human;
        let lifeboats: Vec<Json> = g
            .apocalypse
            .lifeboats
            .iter()
            .map(|lb| {
                let city = g.cities.get(&lb.city_id);
                crate::jobj! {
                    "kind" => lb.kind.name(),
                    "city_name" => city.map(|c| c.name.as_str()),
                    "owner_name" => city.map(|c| g.players[c.owner.idx()].name()),
                    "razed" => lb.razed,
                }
            })
            .collect();
        let mut scores: Vec<&cataclysm_core::Player> = g.players.iter().filter(|p| !p.is_neutral).collect();
        scores.sort_by(|a, b| b.score.cmp(&a.score).then(a.id.cmp(&b.id)));
        let scores: Vec<Json> = scores
            .iter()
            .map(|p| {
                crate::jobj! {
                    "name" => faction_name(p.faction),
                    "leader" => p.name(),
                    "score" => p.score,
                    "alive" => p.alive,
                    "you" => p.id == h,
                }
            })
            .collect();
        (Json::Arr(lifeboats), Json::Arr(scores))
    }

    fn report_line_default(&self) -> String {
        format!("Turn {} ({}). Worldseal Strain {}/100.", self.g.turn, self.g.apocalypse.act_label(self.g.turn), self.g.apocalypse.counter)
    }

    fn plot_json(&self, x: i32, y: i32, p: PlotIdx) -> Json {
        let revealed = self.is_revealed(p);
        let mut o = crate::jobj! { "x" => x, "y" => y, "revealed" => revealed, "visible" => self.is_visible(p) };
        if !revealed {
            return o;
        }
        let g = &self.g;
        let pl = g.world.plot(p);
        let (f, i, gld) = pl.yields();
        let mut features = Vec::new();
        if pl.feature != cataclysm_core::Feature::None {
            features.push(pl.feature.name().to_string());
        }
        if pl.kind == cataclysm_core::PlotKind::Hills {
            features.push("Hills".to_string());
        }
        if pl.kind == cataclysm_core::PlotKind::Peak {
            features.push("Peak".to_string());
        }
        if pl.river_bits != 0 {
            features.push("River".to_string());
        }
        let forest_free = g.players[self.human.idx()].faction == FactionId::Ljosalfar;
        let mc = pl.move_cost(forest_free);
        let mut notes = Vec::new();
        if pl.terrain.is_hell() {
            notes.push(format!(
                "Scorched ground: units of {} fight here with +{}% attack and defense.",
                faction_name(FactionId::Infernals),
                cataclysm_core::combat::HELL_GROUND_PCT
            ));
        }
        if pl.improvement == cataclysm_core::Improvement::Fort {
            notes.push("Fort: while a squad stands here, the enemy's supply lines cannot pass the plots next to it.".into());
        }
        if pl.lair_unexplored {
            notes.push("Unexplored Imperial Ruin: a squad standing here may explore it.".into());
        }
        o.set("terrain_name", pl.terrain.name());
        o.set("features", features);
        o.set("improvement", if pl.improvement == cataclysm_core::Improvement::None { None } else { Some(pl.improvement.name()) });
        o.set("road", pl.road);
        o.set("resource", if pl.resource == Resource::None { None } else { Some(pl.resource.name()) });
        o.set("owner_name", pl.owner.map(|ow| g.players[ow.idx()].name()));
        o.set("yields", crate::jobj! { "food" => f, "industry" => i, "gold" => gld });
        o.set("move_cost", if pl.kind.is_passable_land() { Some(mc) } else { None });
        o.set("lair", pl.lair_unexplored);
        o.set("notes", notes);
        o
    }
}

fn kind_name(k: ActionKind) -> &'static str {
    match k {
        ActionKind::None => "None",
        ActionKind::Strike => "Strike",
        ActionKind::Charge => "Charge",
        ActionKind::Stab => "Stab",
        ActionKind::Shoot => "Shoot",
        ActionKind::Spell => "Spell",
        ActionKind::Bombard => "Bombard",
        ActionKind::Heal => "Heal",
    }
}

fn actions_json(rows: &[RowActions; ROWS]) -> Json {
    Json::Arr(rows.iter().map(|r| crate::jobj! { "kind" => kind_name(r.kind), "n" => r.count }).collect())
}

fn ground_rule_json(r: &GroundRule) -> Option<Json> {
    let reach = r.reach();
    let j = |kind: &str, pct: i32| {
        Some(crate::jobj! {
            "kind" => kind,
            "pct" => pct,
            "side" => reach.side.map(Side::index),
            "kinds" => reach.kinds.map(|ks| Json::Arr(ks.iter().map(|&k| Json::from(kind_name(k))).collect())),
            "first_round" => reach.first_round,
            "units" => match reach.units {
                ffh_battle::RuleUnits::All => "all",
                ffh_battle::RuleUnits::Large => "large",
                ffh_battle::RuleUnits::NotRiver => "not_river",
            },
        })
    };
    match *r {
        GroundRule::CityShots { pct } => j("CityShots", pct),
        GroundRule::ForestRanged { pct } => j("ForestRanged", pct),
        GroundRule::HillsAttacker { pct } => j("HillsAttacker", pct),
        GroundRule::HillsDefenderRanged { pct } => j("HillsDefenderRanged", pct),
        GroundRule::Charge { pct } if pct != 100 => j("ChargeFirstRound", pct),
        GroundRule::ForestStab { pct } => j("ForestStab", pct),
        GroundRule::LargeDealt { pct } => j("LargeDealt", pct),
        GroundRule::Wading { pct } => j("Wading", pct),
        GroundRule::Trench { pct } => j("Trench", pct),
        GroundRule::Uphill { pct } if pct != 100 => j("Uphill", pct),
        GroundRule::Charge { .. } | GroundRule::Uphill { .. } | GroundRule::NoFlanking => None,
    }
}
