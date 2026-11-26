//! Cataclysm game-mode adapter for `ffh-play` and `ffh-web`.
//!
//! Runs `cataclysm-core` + `cataclysm-ai` behind the exact same transport, map arrays, HD-2D / ISO
//! art indices, portrait keys, and `battle_watch.js` v2 battle logs as classic FFH2 — so 100% of
//! the visual art and animation stack is reused with zero Civ4 simulation underneath.

use std::cell::Cell;

use cataclysm_core::{
    BattleId, BattleRecord, BuildOrder, CityId, Command, CrusadeId, Doctrine, Event, FactionId, Game, GameSetup, GreatPersonAction,
    GreatPersonId, GreatSuit, HouseId, MemberId, PlayerId, PlotIdx, Religion, Resource, RitualKind, SallyPosture, SquadId, TechId, UnitClassId,
    WonderKind, ARMAGEDDON_CRISES, DISTRICTS, FACTIONS, SECRET_TECHS, TECHS, UNIT_CLASSES,
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
                "preferred_doctrine" => f.preferred_doctrine.name(),
                "can_mortal_win" => f.can_mortal_win,
                "can_demonic_win" => f.can_demonic_win,
                "lore_blurb" => f.lore_blurb,
                "rule_summary" => f.rule_summary,
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
        Ok(Self {
            g,
            d,
            human: PlayerId(0),
            reveal: Cell::new(false),
            view_version: 1,
            game_tag: format!("cataclysm-{fac_key}-{seed}"),
            last_report_lines: vec!["Act I begins. Move your Vanguard Squad, pick a District, or explore nearby lairs.".into()],
            last_report_battles: Vec::new(),
        })
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
        let phase = if self.g.is_over() { "over" } else { "human" };
        let over = self.g.apocalypse.outcome.as_ref().map(|o| {
            crate::jobj! {
                "winner_team" => o.winners.first().map(|p| p.0 as i32),
                "victory" => Json::Null,
                "cataclysm_kind" => o.kind.name(),
                "summary" => o.summary.as_str(),
                "winners" => Json::Arr(o.winners.iter().map(|p| Json::from(self.g.players[p.idx()].name())).collect()),
            }
        });
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
                "sentinels" => Json::Arr(sentinels_j.clone()),
                "idle" => c.owner == h && c.building.is_none(),
            });

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
                "culture" => c.culture,
                "defense_pct" => c.defense_pct(),
                "besieged" => c.besieged,
                "raid_streak" => c.raid_streak,
                "stripped_turns" => c.stripped_turns,
                "sally" => match c.sally { SallyPosture::Never => "never", SallyPosture::Favoured => "favoured", SallyPosture::Always => "always" },
                "lifeboat" => lifeboat,
                "max_district_slots" => c.max_district_slots(g.is_megacity(c)) as i32,
                "districts" => Json::Arr(c.districts.iter().map(|d| crate::jobj! { "key" => d.info().key, "name" => d.info().name }).collect()),
                "wonders" => Json::Arr(c.wonders.iter().map(|w| crate::jobj! { "key" => w.key(), "name" => w.name() }).collect()),
                "sentinels" => Json::Arr(sentinels_j),
                "industry_rate" => ind_calc.total,
                "industry_calc" => Json::Arr(ind_calc.lines.into_iter().map(Json::from).collect()),
                "gold_rate" => gold_r,
                "lore_rate" => lore_r,
                "culture_rate" => cult_r,
                "mana_rate" => mana_r,
                "building" => building,
                "crew_task" => crew,
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
        let found_cost = 70 + g.player_cities(h).len() as i32 * 25;

        for s in g.squads.values() {
            if s.owner != h && !self.is_visible(s.plot) {
                continue;
            }
            let pl = g.world.plot(s.plot);
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
                    "max_moves" => s.max_moves(0) * 60,
                    "engages" => true,
                    "cataclysm_squad" => s.id.0 as i32,
                    "stand" => crate::jobj! { "rank" => rank, "cell" => cell, "large" => info.large },
                });
                let upgrade_to = info.upgrades_to.filter(|&nxt| g.can_recruit_class(s.owner, nxt).is_ok());
                let upgrade_cost = upgrade_to.map(|nxt| {
                    let mut c = (nxt.info().cost_gold - info.cost_gold / 2).max(25);
                    if g.players[s.owner.idx()].has_tech(cataclysm_core::data::TECH_WAR_CHESTS) {
                        c = c * 70 / 100;
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

            let can_found = s.owner == h
                && s.moves_left > 0
                && pl.kind.is_passable_land()
                && pl.city.is_none()
                && g.cities.values().all(|c| g.world.distance(c.plot, s.plot) >= 4);

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
                "moves_left" => s.moves_left,
                "max_moves" => s.max_moves(0),
                "order" => match s.order {
                    cataclysm_core::SquadOrder::None => "none",
                    cataclysm_core::SquadOrder::Fortify => "fortify",
                    cataclysm_core::SquadOrder::Raid => "raid",
                },
                "supplied" => supply_line.is_some(),
                "unsupplied_turns" => s.unsupplied_turns,
                "power" => s.power_rating(),
                "places_used" => s.places_used() as i32,
                "place_cap" => place_cap as i32,
                "in_home_borders" => pl.owner == Some(s.owner),
                "can_found_city" => can_found,
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
                crate::jobj! {
                    "id" => t.id.0 as i32,
                    "key" => t.key,
                    "name" => t.name,
                    "doctrine" => t.doctrine.map(Doctrine::key),
                    "doctrine_name" => t.doctrine.map(Doctrine::name),
                    "tier" => t.tier as i32,
                    "cost" => cost,
                    "prereq" => t.prereq.map(|p| TECHS[p.0 as usize].name),
                    "ac_delta" => t.ac_delta,
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
                crate::jobj! {
                    "id" => rel.id() as i32,
                    "name" => rel.name(),
                    "alignment" => rel.alignment().name(),
                    "founded_by" => founder,
                    "adopted" => me_pl.religion == Some(rel),
                    "lore_cost" => rel.lore_cost(),
                    "ac_delta" => rel.founding_ac_delta(),
                    "summary" => rel.summary(),
                }
            })
            .collect();

        let rituals_j: Vec<Json> = RitualKind::ALL
            .iter()
            .map(|&rk| {
                crate::jobj! {
                    "key" => rk.key(),
                    "name" => rk.name(),
                    "mana_cost" => rk.mana_cost(),
                    "ac_delta" => rk.ac_delta(),
                    "can_cast" => me_pl.mana >= rk.mana_cost(),
                    "summary" => rk.summary(),
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
                crate::jobj! {
                    "id" => gp.id.0 as i32,
                    "suit" => gp.suit.name(),
                    "name" => gp.name.as_str(),
                    "portrait" => gp.portrait,
                    "burst_summary" => gp.suit.burst_summary(),
                    "hero_class_name" => gp.suit.hero_class().info().name,
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
                crate::jobj! {
                    "id" => cr.id.0 as i32,
                    "caller_name" => g.players[cr.caller.idx()].name(),
                    "target_id" => cr.target.0 as i32,
                    "target_name" => g.players[cr.target.idx()].name(),
                    "turns_left" => cr.turns_left,
                    "lent_count" => cr.lent_squads.len() as i32,
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
                    gold_cost = gold_cost * 75 / 100;
                }
                if me_pl.has_tech(cataclysm_core::data::TECH_WAR_CHESTS) {
                    gold_cost = gold_cost * 70 / 100;
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
                crate::jobj! {
                    "id" => pl.id.0 as i32,
                    "name" => pl.name(),
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

        let cataclysm_payload = crate::jobj! {
            "turn" => g.turn,
            "max_turns" => cataclysm_core::apocalypse::MAX_GAME_TURNS,
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
                "primary_doctrine" => me_pl.primary_doctrine.map(Doctrine::key),
                "primary_doctrine_name" => me_pl.primary_doctrine.map(Doctrine::name),
                "secondary_doctrine" => me_pl.secondary_doctrine.map(Doctrine::key),
                "secondary_doctrine_name" => me_pl.secondary_doctrine.map(Doctrine::name),
                "techs" => Json::Arr(techs_j),
                "doctrines" => Json::Arr(doctrines_j),
            },
            "religions" => Json::Arr(religions_j),
            "rituals" => Json::Arr(rituals_j),
            "squads" => Json::Arr(cat_squads),
            "cities" => Json::Arr(cat_cities),
            "houses" => Json::Arr(houses_j),
            "court" => Json::Arr(court_j),
            "renown" => crate::jobj! {
                "meters" => Json::Arr(renown_meters_j),
                "total" => renown_total,
                "next_threshold" => renown_need,
            },
            "secrets_known" => Json::Arr(secrets_known_j),
            "apocalypse" => crate::jobj! {
                "counter" => g.apocalypse.counter,
                "act_label" => g.apocalypse.act_label(g.turn),
                "hyborem_spawned" => g.apocalypse.hyborem_spawned,
                "carcosa_risen" => g.apocalypse.carcosa_risen,
                "countdown" => g.apocalypse.countdown,
                "carcosa_protected_turns" => g.apocalypse.carcosa_protected_turns,
                "recent_changes" => Json::Arr(g.apocalypse.recent_changes.iter().cloned().map(Json::from).collect()),
                "crises" => Json::Arr(crises_j),
                "lifeboats" => Json::Arr(lifeboats_j),
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
            .ok_or_else(|| ProtoErr::new("gone", format!("no battle report {report_id}")))?;
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
                let to = self.g.world.idx(x, y).ok_or_else(|| ProtoErr::invalid("out of bounds"))?;
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
                    return Err(ProtoErr::invalid("plot out of bounds"));
                };
                let pl = self.g.world.plot(p);
                let (f, i, gld) = pl.yields();
                Ok(crate::jobj! {
                    "x" => x, "y" => y,
                    "revealed" => self.is_revealed(p),
                    "visible" => self.is_visible(p),
                    "terrain" => pl.terrain.art_key(),
                    "resource" => if pl.resource == Resource::None { None } else { Some(pl.resource.name()) },
                    "improvement" => if pl.improvement == cataclysm_core::Improvement::None { None } else { Some(pl.improvement.name()) },
                    "road" => pl.road,
                    "yields" => vec![f, i, gld],
                })
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
                            self.last_report_lines.push(format!("WORLDSEAL STRAIN {threshold}: {name} — {summary}"));
                        }
                        Event::CarcosaRisen { .. } => {
                            self.last_report_lines.push("CALDERA KEEP HAS RISEN! Race for the Lifeboats!".into());
                        }
                        Event::HyboremArrived { .. } => {
                            self.last_report_lines.push("THE DEEP BREACH! King Cinder's Host marches out from Great Furnace!".into());
                        }
                        Event::GreatPersonBorn { player, name, .. } if *player == self.human => {
                            self.last_report_lines.push(format!("Great Person {name} arrived in your Court!"));
                        }
                        Event::TechCompleted { player, tech } if *player == self.human => {
                            self.last_report_lines.push(format!("Completed tech: {}!", TECHS[tech.0 as usize].name));
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
                self.last_report_lines.push(format!(
                    "Turn {} ({}) · Worldseal Strain {}/100.",
                    self.g.turn,
                    self.g.apocalypse.act_label(self.g.turn),
                    self.g.apocalypse.counter
                ));
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
            self.g.world.idx(x, y).ok_or_else(|| ProtoErr::invalid("plot out of bounds"))
        };
        let parse_class = |key: &str| -> Result<UnitClassId, ProtoErr> {
            UNIT_CLASSES
                .iter()
                .find(|u| u.key == key)
                .map(|u| u.id)
                .ok_or_else(|| ProtoErr::invalid(format!("unknown unit class {key:?}")))
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
                        let d = DISTRICTS.iter().find(|d| d.key == key).ok_or_else(|| ProtoErr::invalid("unknown district"))?;
                        BuildOrder::District(d.kind)
                    }
                    "wonder" => {
                        let key = j.req_str("key")?;
                        let w = WonderKind::ALL.into_iter().find(|w| w.key() == key).ok_or_else(|| ProtoErr::invalid("unknown wonder"))?;
                        BuildOrder::Wonder(w)
                    }
                    "squad" => {
                        let arr = j.get("classes").and_then(Json::as_arr).ok_or_else(|| ProtoErr::invalid("missing classes"))?;
                        let mut cls = Vec::new();
                        for item in arr {
                            cls.push(parse_class(item.as_str().ok_or_else(|| ProtoErr::invalid("class must be string"))?)?);
                        }
                        BuildOrder::Squad(cls)
                    }
                    "reinforce" => BuildOrder::Reinforce(parse_class(j.req_str("key")?)?),
                    "caravan" => BuildOrder::Caravan,
                    other => return Err(ProtoErr::invalid(format!("unknown build kind {other:?}"))),
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
                    return Err(ProtoErr::invalid("invalid tech id"));
                }
                Command::ChooseResearch { tech: TechId(tid) }
            }
            "cat_doctrine" => {
                let key = j.req_str("doctrine")?;
                let doc = Doctrine::ALL.into_iter().find(|d| d.key() == key).ok_or_else(|| ProtoErr::invalid("unknown doctrine"))?;
                Command::AdoptDoctrine { doctrine: doc, secondary: j.get_or("secondary", false)? }
            }
            "cat_religion" => {
                let id = j.req::<usize>("religion")?;
                let rel = *Religion::ALL.get(id).ok_or_else(|| ProtoErr::invalid("invalid religion"))?;
                Command::FoundOrAdoptReligion { religion: rel }
            }
            "cat_ritual" => {
                let key = j.req_str("ritual")?;
                let rk = RitualKind::ALL.into_iter().find(|r| r.key() == key).ok_or_else(|| ProtoErr::invalid("unknown ritual"))?;
                Command::CastRitual { ritual: rk }
            }
            "cat_gp" => {
                let gp = GreatPersonId(j.req::<u32>("gp")?);
                let action = match j.req_str("action")? {
                    "secret" => GreatPersonAction::DiscoverSecret,
                    "burst" => GreatPersonAction::TempoBurst,
                    "hero" => GreatPersonAction::CommissionHero(SquadId(j.req::<u32>("squad")?)),
                    "lord" => GreatPersonAction::CommissionLord(HouseId(j.req::<u32>("house")?)),
                    other => return Err(ProtoErr::invalid(format!("unknown gp action {other:?}"))),
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
            other => return Err(ProtoErr::invalid(format!("unknown Cataclysm command {other:?}"))),
        })
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
