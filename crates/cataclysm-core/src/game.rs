//! The `cataclysm-core` pure deterministic simulation (`Game`).
//!
//! No blocking callbacks, no Civ4 turn slices, no HashMap iteration: every human or AI decision
//! arrives via [`Game::apply`] and produces a deterministic stream of [`Event`]s.

use std::collections::{BTreeMap, BTreeSet};

use ffh_battle::{Fate, Side};

use crate::apocalypse::{
    ApocalypseState, GameOutcome, Lifeboat, LifeboatKind, VictoryKind, CARCOSA_PROTECTED_TURNS, CARCOSA_TIMER_TURNS, MAX_GAME_TURNS,
};
use crate::city::{BuildOrder, City, SallyPosture};
use crate::combat::{build_formation, forecast_formation, verdict_of, BattleRecord, Forecast, SideMods};
use crate::command::{CmdErr, Command, Event};
use crate::crusade::Crusade;
use crate::data::{
    Alignment, CaptainAura, DistrictKind, Doctrine, FactionId, GreatSuit, HousePerk, Religion, RitualKind, SecretEffect, SecretTechId, TechId,
    UnitClassId, WonderKind, ARMAGEDDON_CRISES, CHARACTER_POOL, SECRET_TECHS, TECHS, TECH_ARCANA, TECH_ARCHERY, TECH_FIELDCRAFT, TECH_FORTIFICATION,
    TECH_GUILDS, TECH_HORSEMANSHIP, TECH_MASONRY_WARDS, TECH_METALLURGY, TECH_MYSTICISM, TECH_OUTRIDERS, TECH_TRADE_ROADS, TECH_WARHORSE_LORDS,
};
use crate::economy::{find_caravan_path, CaravanRoute, RateBreakdown};
use crate::great_people::{dominant_suit, renown_threshold, GreatPerson, GreatPersonAction, SecretDeck};
use crate::house::{House, HouseMood, HOUSE_MAX_CITIES};
use crate::ids::{BattleId, CaravanId, CityId, CrusadeId, GreatPersonId, HouseId, MemberId, PlayerId, PlotIdx, SquadId};
use crate::rng::Rng;
use crate::squad::{RecoveringHero, Squad, SquadMember, SquadOrder};
use crate::supply::{
    enemy_supply_blocks, trace_supply_line, SupplyLine, BASE_SUPPLY_BUDGET, SUPPLY_BONUS_OUTRIDERS, SUPPLY_BONUS_QUARTERMASTER, SUPPLY_BONUS_TRADE_ROADS,
};
use crate::world::{Improvement, PlotKind, Resource, Terrain, World};

// Rule numbers shared by the rules below and by the text that describes them to the player.

/// Founding a city costs `FOUND_CITY_BASE_GOLD + FOUND_CITY_GOLD_PER_CITY * cities owned`.
pub const FOUND_CITY_BASE_GOLD: i32 = 70;
pub const FOUND_CITY_GOLD_PER_CITY: i32 = 25;
/// A new city must stand at least this many plots from every city.
pub const MIN_CITY_DISTANCE: i32 = 4;
/// Gold to launch a Caravan by command (a city-built Caravan pays its Industry instead).
pub const CARAVAN_GOLD: i32 = 35;
/// Strain added when a city is razed.
pub const RAZE_STRAIN: i32 = 6;
/// Gold paid to a raider to end its raids on your land.
pub const RAID_TRIBUTE_GOLD: i32 = 60;
/// Share of a raided city's Gold + Industry the raiders take (as Gold for themselves).
pub const RAID_TAKE_PCT: i32 = 50;
pub const RAID_TAKE_SALLIED_PCT: i32 = 10;
/// Consecutive raided turns that strip a city's countryside, and for how long.
pub const RAID_STREAK_TO_STRIP: i32 = 5;
pub const STRIP_TURNS: i32 = 10;
/// A "Favoured" sally goes out when the garrison's power is at least this % of the raider's.
pub const SALLY_FAVOURED_PCT: i32 = 95;
/// Lowest culture for the "Last Metropolis" Lifeboat.
pub const LIFEBOAT_CULTURE_MIN: i32 = 120;
/// Turns without a supply line from which a squad loses HP each turn, and how much.
pub const STARVE_ATTRITION_TURNS: i32 = 3;
pub const STARVE_HP_LOSS_PCT: i32 = 10;
/// Per-turn healing (percent of max HP) of a supplied squad by where it stands.
pub const HEAL_OWN_CITY_PCT: i32 = 20;
pub const HEAL_OWN_LAND_PCT: i32 = 12;
pub const HEAL_FORTIFIED_PCT: i32 = 8;
pub const HEAL_FIELD_PCT: i32 = 4;
/// Sentinel damage repaired per turn in a city that is not besieged.
pub const SENTINEL_REPAIR_PER_TURN: i32 = 15;
/// Gold upkeep per squad member per turn.
pub const UPKEEP_PER_MEMBER: i32 = 2;
/// Strain added every this many world turns before the Blowout.
pub const STRAIN_DRIFT_EVERY: i32 = 3;
/// Turns a fallen hero recovers before returning at the capital.
pub const HERO_RECOVERY_TURNS: i32 = 8;
/// Gold taken by ambushing an enemy Caravan.
pub const AMBUSH_GOLD: i32 = 40;
/// Mana a side spends per battle when its squad has a caster.
pub const BATTLE_MANA: i32 = 2;
/// Strain added by a battle in which at least this many units fell.
pub const BLOODY_BATTLE_DEATHS: i32 = 3;
/// Wreckers' Deep Shaft: from this Strain, every this many turns.
pub const DEEP_SHAFT_STRAIN: i32 = 30;
pub const DEEP_SHAFT_EVERY: i32 = 10;
/// Crusade credit for lending a squad (Seventh Garrison, others).
pub const LEND_CREDIT_BANNOR: i32 = 20;
pub const LEND_CREDIT: i32 = 10;
/// Gold-cost discounts (percent paid).
pub const HIPPUS_REINFORCE_PCT: i32 = 75;
pub const WAR_CHESTS_PCT: i32 = 70;
/// A primary doctrine's techs cost this percent; no tech costs less than `MIN_TECH_COST`.
pub const PRIMARY_DOCTRINE_COST_PCT: i32 = 75;
pub const MIN_TECH_COST: i32 = 15;
/// A secondary doctrine opens its techs up to this tier.
pub const SECONDARY_DOCTRINE_MAX_TIER: u8 = 2;

/// Strain change of Vent the Seal cast by the Keepers of the Seal (others: the ritual's own).
pub const SANCTIFY_KEEPERS_STRAIN: i32 = -5;
/// Squad cap: base, Grand Companies bonus, floor.
pub const SQUAD_CAP_BASE: i32 = 3;
pub const SQUAD_CAP_GRAND_COMPANIES: i32 = 2;
pub const SQUAD_CAP_MIN: i32 = 2;
/// Places per squad: base, and with any tier-2 tech or Guilds.
pub const SQUAD_PLACES_BASE: u8 = 4;
pub const SQUAD_PLACES_EXPANDED: u8 = 5;

/// Lair outcomes (one at random).
pub const LAIR_GOLD: i32 = 75;
pub const LAIR_MANA: i32 = 25;
pub const LAIR_LORE: i32 = 35;
pub const LAIR_XP: i32 = 15;
/// Raiders affect cities within this many plots.
pub const RAID_RANGE: i32 = 2;
/// Culture at which a city's border radius grows from 1 to 2.
pub const BORDER_CULTURE_RADIUS_2: i32 = 100;
/// Mana at which a side's battle spells gain power.
pub const MANA_BOOST_MIN: i32 = 15;
/// Strain added by a bloody battle.
pub const BLOODY_BATTLE_STRAIN: i32 = 1;
/// Score points (each player's score is its share of all points, out of 10000).
pub const SCORE_PER_SIZE: i32 = 15;
pub const SCORE_PER_DISTRICT: i32 = 10;
pub const SCORE_PER_TECH: i32 = 20;
pub const SCORE_PER_SECRET: i32 = 30;
pub const SCORE_PER_UNIT: i32 = 8;

/// Renown each source adds to a Great People meter.
pub mod renown {
    pub const AMBUSH_MERCHANT: i32 = 8;
    pub const BATTLE_COMMANDER: i32 = 4;
    pub const BATTLE_COMMANDER_PER_KILL: i32 = 6;
    pub const LAIR_RELICS_SAGE: i32 = 15;
    pub const FOUND_CREED_PROPHET: i32 = 20;
    pub const RITUAL_SAGE: i32 = 10;
    pub const LEND_PROPHET: i32 = 12;
    pub const CARAVAN_MERCHANT: i32 = 2;
    pub const DISTRICT_PER_TURN: i32 = 2;
    pub const WONDER_ENGINEER: i32 = 25;
    pub const CAPTURE_COMMANDER: i32 = 15;
    pub const RAID_MERCHANT: i32 = 4;
}

/// Founding cost for a player that owns `cities` cities.
pub const fn found_city_cost(cities: usize) -> i32 {
    FOUND_CITY_BASE_GOLD + cities as i32 * FOUND_CITY_GOLD_PER_CITY
}

#[derive(Clone, Debug)]
pub struct GameSetup {
    pub seed: u64,
    pub width: i32,
    pub height: i32,
    pub human_faction: FactionId,
    pub ai_count: usize,
}

impl Default for GameSetup {
    fn default() -> Self {
        Self { seed: 42, width: 26, height: 18, human_faction: FactionId::Bannor, ai_count: 5 }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: PlayerId,
    pub faction: FactionId,
    pub is_human: bool,
    pub is_neutral: bool,
    pub alive: bool,
    pub alignment: Alignment,
    pub religion: Option<Religion>,
    pub gold: i32,
    pub lore_progress: i32,
    pub mana: i32,
    pub primary_doctrine: Option<Doctrine>,
    pub secondary_doctrine: Option<Doctrine>,
    pub techs: BTreeSet<TechId>,
    pub current_research: Option<TechId>,
    pub secrets: Vec<SecretTechId>,
    pub renown_meters: [i32; 6],
    pub great_people_born: i32,
    pub vassal_of: Option<PlayerId>,
    pub score: i32,
    pub next_city_name_idx: usize,
    pub next_squad_num: u32,
    pub extra_caravan_slots: i32,
}

impl Player {
    pub fn new(id: PlayerId, faction: FactionId, is_human: bool, is_neutral: bool) -> Self {
        let info = faction.info();
        let mut techs = BTreeSet::new();
        if faction == FactionId::Amurites {
            techs.insert(TECH_ARCANA);
        }
        Self {
            id,
            faction,
            is_human,
            is_neutral,
            alive: true,
            alignment: info.alignment,
            religion: None,
            gold: info.starting_gold,
            lore_progress: 0,
            mana: info.starting_mana,
            primary_doctrine: None,
            secondary_doctrine: None,
            techs,
            current_research: None,
            secrets: Vec::new(),
            renown_meters: [0; 6],
            great_people_born: 0,
            vassal_of: None,
            score: 0,
            next_city_name_idx: 0,
            next_squad_num: 1,
            extra_caravan_slots: 0,
        }
    }

    pub fn name(&self) -> &'static str {
        if self.is_neutral {
            "Wild Clans"
        } else {
            self.faction.info().leader_name
        }
    }

    pub fn has_tech(&self, tech: TechId) -> bool {
        self.techs.contains(&tech)
    }

    pub fn has_secret(&self, secret: SecretTechId) -> bool {
        self.secrets.contains(&secret)
    }

    /// Maximum places per 3x3 squad: starts at 4, expands to 5 upon reaching tier-2 Doctrine (or Guilds).
    pub fn squad_place_cap(&self) -> u8 {
        if self.has_tech(TECH_GUILDS) || self.techs.iter().any(|t| TECHS[t.0 as usize].tier >= 2) {
            SQUAD_PLACES_EXPANDED
        } else {
            SQUAD_PLACES_BASE
        }
    }

    pub fn secret_sum(&self, f: impl Fn(SecretEffect) -> i32) -> i32 {
        self.secrets
            .iter()
            .filter(|&&sid| self.has_tech(SECRET_TECHS[sid.0 as usize].needs))
            .map(|&sid| f(SECRET_TECHS[sid.0 as usize].effect))
            .sum()
    }
}

#[derive(Clone, Debug)]
pub struct LogLine {
    pub turn: i32,
    pub player: Option<PlayerId>,
    pub plot: Option<PlotIdx>,
    pub battle: Option<BattleId>,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub turn: i32,
    pub active_player: PlayerId,
    pub rng: Rng,
    pub world: World,
    pub players: Vec<Player>,
    pub neutral_player: PlayerId,
    /// Symmetric war set storing `(min(a,b), max(a,b))`.
    pub wars: BTreeSet<(PlayerId, PlayerId)>,
    pub founded_religions: BTreeMap<Religion, PlayerId>,
    pub built_wonders: BTreeMap<WonderKind, CityId>,
    pub cities: BTreeMap<CityId, City>,
    pub squads: BTreeMap<SquadId, Squad>,
    pub houses: BTreeMap<HouseId, House>,
    pub caravans: BTreeMap<CaravanId, CaravanRoute>,
    pub court: BTreeMap<GreatPersonId, GreatPerson>,
    pub recovering_heroes: Vec<RecoveringHero>,
    pub crusades: BTreeMap<CrusadeId, Crusade>,
    pub secret_deck: SecretDeck,
    pub apocalypse: ApocalypseState,
    pub battles: Vec<BattleRecord>,
    pub log: Vec<LogLine>,
    /// Next character index into `CHARACTER_POOL` (cycled deterministically).
    pub next_char_idx: usize,
    pub next_city_id: u32,
    pub next_squad_id: u32,
    pub next_member_id: u32,
    pub next_house_id: u32,
    pub next_caravan_id: u32,
    pub next_gp_id: u32,
    pub next_crusade_id: u32,
}

impl Game {
    pub fn new(setup: GameSetup) -> Self {
        let mut rng = Rng::new(setup.seed);
        let ai_count = setup.ai_count.clamp(1, 7);
        let mut factions = vec![setup.human_faction];
        let mut pool: Vec<FactionId> = FactionId::STARTING.into_iter().filter(|&f| f != setup.human_faction).collect();
        rng.shuffle(&mut pool);
        for f in pool.into_iter().take(ai_count) {
            factions.push(f);
        }
        // Neutral / Barbarian slot is always last.
        let num_civs = factions.len();
        let total_players = num_civs + 1;
        let neutral_player = PlayerId(num_civs as u16);

        let (world, starts) = World::generate(setup.width.max(18), setup.height.max(14), total_players, &mut rng);

        let mut players = Vec::with_capacity(total_players);
        for (i, &fac) in factions.iter().enumerate() {
            players.push(Player::new(PlayerId(i as u16), fac, i == 0, false));
        }
        let mut neutral = Player::new(neutral_player, FactionId::Sheaim, false, true);
        neutral.alignment = Alignment::Evil;
        players.push(neutral);

        let secret_deck = SecretDeck::new(&mut rng);
        let mut g = Self {
            turn: 1,
            active_player: PlayerId(0),
            rng,
            world,
            players,
            neutral_player,
            wars: BTreeSet::new(),
            founded_religions: BTreeMap::new(),
            built_wonders: BTreeMap::new(),
            cities: BTreeMap::new(),
            squads: BTreeMap::new(),
            houses: BTreeMap::new(),
            caravans: BTreeMap::new(),
            court: BTreeMap::new(),
            recovering_heroes: Vec::new(),
            crusades: BTreeMap::new(),
            secret_deck,
            apocalypse: ApocalypseState::default(),
            battles: Vec::new(),
            log: Vec::new(),
            next_char_idx: 0,
            next_city_id: 1,
            next_squad_id: 1,
            next_member_id: 1,
            next_house_id: 1,
            next_caravan_id: 1,
            next_gp_id: 1,
            next_crusade_id: 1,
        };

        // Every civilization is perpetually at war with Neutral raiders/guardians.
        for i in 0..num_civs {
            g.set_war(PlayerId(i as u16), neutral_player, true);
        }

        // Found each civilization's capital and spawn its starting 3-unit Vanguard Squad + Scout Squad!
        for i in 0..num_civs {
            let pid = PlayerId(i as u16);
            let start_plot = starts[i];
            let city_id = g.create_city_internal(pid, start_plot, None, true);
            // Guarantee capital starts with Barracks so recruitment & squad cap feel crisp on Turn 1.
            if let Some(c) = g.cities.get_mut(&city_id) {
                c.districts.push(DistrictKind::Barracks);
            }
            // Spawn starting 3-unit Squad right on the capital (Guard + Guard + faction specialty unit!).
            let third = match g.players[i].faction {
                FactionId::Amurites | FactionId::Sheaim => UnitClassId::Adept,
                FactionId::Hippus => UnitClassId::Horseman,
                FactionId::Ljosalfar => UnitClassId::Archer,
                FactionId::Elohim => UnitClassId::Acolyte,
                FactionId::Khazad => UnitClassId::Axeman,
                FactionId::Infernals => UnitClassId::PyreZombie,
                _ => UnitClassId::Archer,
            };
            g.spawn_squad_internal(pid, start_plot, &[UnitClassId::Guard, UnitClassId::Guard, third]);

            // Spawn a fast 2-unit scouting/expansion squad on a neighboring land tile.
            let adj = g
                .world
                .neighbors(start_plot)
                .into_iter()
                .find(|&p| g.world.plot(p).kind.is_passable_land() && g.world.plot(p).city.is_none())
                .unwrap_or(start_plot);
            g.spawn_squad_internal(pid, adj, &[UnitClassId::Guard, UnitClassId::Ranger]);

            // Pick a sensible initial Foundation tech so turn 1 is immediately progressing.
            let start_tech = match g.players[i].faction {
                FactionId::Amurites => TECH_TRADE_ROADS,
                FactionId::Ljosalfar => TECH_ARCHERY,
                FactionId::Hippus => TECH_HORSEMANSHIP,
                FactionId::Elohim => TECH_MYSTICISM,
                FactionId::Khazad => TECH_METALLURGY,
                _ => TECH_FIELDCRAFT,
            };
            g.players[i].current_research = Some(start_tech);
            g.ensure_houses_for(pid);
            g.recompute_vision(pid);
        }

        // Seed a handful of neutral lair guardians far from starting capitals.
        let mut lairs: Vec<PlotIdx> = (0..g.world.len())
            .map(|i| PlotIdx(i as u32))
            .filter(|&p| g.world.plot(p).lair_unexplored && starts.iter().all(|&s| g.world.distance(s, p) >= 4))
            .collect();
        g.rng.shuffle(&mut lairs);
        for p in lairs.into_iter().take(6) {
            g.spawn_squad_internal(neutral_player, p, &[UnitClassId::Guard, UnitClassId::Archer]);
        }

        g.recompute_scores();
        g.push_log(None, None, None, "The Great Empire sealed its rebel furnace-servitors beneath the central sea an age ago. Now the Worldseal groans. Build your Houses, muster your Squads, and prepare for the Blowout.");
        g
    }

    pub fn is_over(&self) -> bool {
        self.apocalypse.outcome.is_some()
    }

    pub fn at_war(&self, a: PlayerId, b: PlayerId) -> bool {
        if a == b {
            return false;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        self.wars.contains(&pair)
    }

    pub fn set_war(&mut self, a: PlayerId, b: PlayerId, war: bool) {
        if a == b {
            return;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        if war {
            self.wars.insert(pair);
        } else {
            self.wars.remove(&pair);
        }
    }

    fn alloc_character(&mut self) -> &'static crate::data::PortraitCharacter {
        let ch = &CHARACTER_POOL[self.next_char_idx % CHARACTER_POOL.len()];
        self.next_char_idx += 1;
        ch
    }

    pub fn push_log(&mut self, player: Option<PlayerId>, plot: Option<PlotIdx>, battle: Option<BattleId>, text: impl Into<String>) {
        self.log.push(LogLine { turn: self.turn, player, plot, battle, text: text.into() });
        if self.log.len() > 400 {
            self.log.drain(0..100);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Queries & Economy Formulas (with transparent calc breakdowns!)
    // -----------------------------------------------------------------------------------------

    pub fn player_cities(&self, p: PlayerId) -> Vec<CityId> {
        self.cities.values().filter(|c| c.owner == p).map(|c| c.id).collect()
    }

    pub fn player_squads(&self, p: PlayerId) -> Vec<SquadId> {
        self.squads.values().filter(|s| s.owner == p).map(|s| s.id).collect()
    }

    pub fn squads_at(&self, plot: PlotIdx) -> Vec<SquadId> {
        self.squads.values().filter(|s| s.plot == plot).map(|s| s.id).collect()
    }

    pub fn house_loyalty(&self, house_id: HouseId) -> (i32, HouseMood, Vec<String>) {
        let Some(h) = self.houses.get(&house_id) else {
            return (0, HouseMood::Content, Vec::new());
        };
        let pl = &self.players[h.owner.idx()];
        let at_war_sym = h.sympathy.is_some_and(|s| self.at_war(h.owner, s));
        let temples = h.cities.iter().filter(|id| self.cities.get(id).is_some_and(|c| c.has_district(DistrictKind::Temple))).count() as i32;
        let mut emp_bonus = pl.secret_sum(|e| match e {
            SecretEffect::HouseLoyaltyBonus(n) => n,
            _ => 0,
        });
        if pl.has_tech(crate::data::TECH_PRIESTHOOD) {
            emp_bonus += 1;
        }
        if pl.has_tech(crate::data::TECH_CROWN_CHARTERS) {
            emp_bonus += 2;
        }
        if self.cities.values().any(|c| c.owner == h.owner && c.has_wonder(WonderKind::AltarOfLuonnotar)) {
            emp_bonus += 2;
        }
        let total_houses = self.houses.values().filter(|x| x.owner == h.owner).count();
        let (score, lines) = h.compute_loyalty(pl.alignment, pl.religion, at_war_sym, temples, emp_bonus, total_houses);
        (score, House::mood_of(score), lines)
    }

    pub fn active_house_perks(&self, p: PlayerId) -> Vec<HousePerk> {
        let mut out = Vec::new();
        for h in self.houses.values().filter(|h| h.owner == p) {
            let (loy, _, _) = self.house_loyalty(h.id);
            if House::perk_active(loy) {
                out.push(h.perk);
            }
        }
        out
    }

    pub fn squad_cap(&self, p: PlayerId) -> usize {
        let pl = &self.players[p.idx()];
        let mut cap: i32 = SQUAD_CAP_BASE;
        for c in self.cities.values().filter(|c| c.owner == p) {
            if !c.is_outpost {
                cap += 1;
            }
            if c.has_district(DistrictKind::Barracks) {
                cap += 1;
            }
        }
        if pl.has_tech(crate::data::TECH_GRAND_COMPANIES) {
            cap += SQUAD_CAP_GRAND_COMPANIES;
        }
        cap += pl.secret_sum(|e| match e {
            SecretEffect::SquadCapBonus(n) => n,
            _ => 0,
        });
        for perk in self.active_house_perks(p) {
            if perk == HousePerk::BannerLevy {
                cap += 1;
            }
        }
        cap.max(SQUAD_CAP_MIN) as usize
    }

    pub fn caravan_cap(&self, p: PlayerId) -> usize {
        let pl = &self.players[p.idx()];
        let mut cap = 1 + pl.extra_caravan_slots;
        for c in self.cities.values().filter(|c| c.owner == p) {
            if c.has_district(DistrictKind::Market) {
                cap += 1;
            }
        }
        if pl.has_tech(TECH_GUILDS) {
            cap += 2;
        }
        cap += pl.secret_sum(|e| match e {
            SecretEffect::ExtraCaravanSlots(n) => n,
            _ => 0,
        });
        for perk in self.active_house_perks(p) {
            if perk == HousePerk::CaravanPatronage {
                cap += 1;
            }
        }
        cap.max(1) as usize
    }

    pub fn supply_budget(&self, p: PlayerId, aura: CaptainAura) -> i32 {
        let pl = &self.players[p.idx()];
        let mut b = BASE_SUPPLY_BUDGET;
        if pl.has_tech(TECH_TRADE_ROADS) {
            b += SUPPLY_BONUS_TRADE_ROADS;
        }
        if pl.has_tech(TECH_OUTRIDERS) {
            b += SUPPLY_BONUS_OUTRIDERS;
        }
        if aura == CaptainAura::Quartermaster {
            b += SUPPLY_BONUS_QUARTERMASTER;
        }
        b += pl.secret_sum(|e| match e {
            SecretEffect::SupplyRangeBonus(n) => n,
            _ => 0,
        });
        b
    }

    pub fn is_megacity(&self, c: &City) -> bool {
        self.players[c.owner.idx()].faction == FactionId::Kuriotates && !c.is_outpost
    }

    pub fn khazad_full_vault(&self, p: PlayerId) -> bool {
        let pl = &self.players[p.idx()];
        if pl.faction != FactionId::Khazad {
            return false;
        }
        let cities = self.player_cities(p).len().max(1) as i32;
        pl.gold >= cities * 200
    }

    /// Computes a city's per-turn Food, Industry, Gold, Lore, Culture, and Mana with transparent formula lines.
    pub fn city_yields(&self, city_id: CityId) -> (i32, RateBreakdown, i32, i32, i32, i32) {
        let Some(c) = self.cities.get(&city_id) else {
            return (0, RateBreakdown::default(), 0, 0, 0, 0);
        };
        let pl = &self.players[c.owner.idx()];
        let (cx, cy) = (self.world.plot(c.plot).x, self.world.plot(c.plot).y);
        let mut food = 2;
        let mut tile_ind = 2;
        let mut tile_gold = 2;
        let mut mana = 0;

        for dy in -2..=2 {
            for dx in -2..=2 {
                if let Some(p) = self.world.idx(cx + dx, cy + dy) {
                    let pt = self.world.plot(p);
                    if pt.owner == Some(c.owner) && self.world.distance(p, c.plot) <= 2 {
                        let (f, i, g) = pt.yields();
                        // Scale countryside contribution smoothly by city Size without per-tile micro!
                        let weight = if dx == 0 && dy == 0 { 100 } else if dx.abs().max(dy.abs()) == 1 { 45 } else { 20 };
                        food += f * weight / 100;
                        tile_ind += i * weight / 100;
                        tile_gold += g * weight / 100;
                        if pt.resource.is_mana() && pt.improvement == Improvement::ManaNode {
                            mana += 2;
                        }
                    }
                }
            }
        }
        food += c.size / 2;
        let mut ind_calc = RateBreakdown::default();
        ind_calc.add(tile_ind + c.size * 2, format!("Base & Size {}", c.size));
        if c.has_district(DistrictKind::Forge) {
            ind_calc.add(4, "Forge District");
        }
        let sec_ind = pl.secret_sum(|e| match e {
            SecretEffect::IndustryPerCity(n) => n,
            _ => 0,
        });
        ind_calc.add(sec_ind, "Secret Techs");

        let mut gold = tile_gold + c.size * 2;
        let mut lore = 2 + c.size;
        let mut culture = 2 + if c.capital { 3 } else { 0 };

        if c.has_district(DistrictKind::Market) {
            gold += 4;
        }
        if c.has_district(DistrictKind::Academy) {
            lore += 5;
            mana += if pl.faction == FactionId::Amurites { 3 } else { 1 };
        }
        if c.has_district(DistrictKind::Temple) {
            culture += 4;
        }
        if pl.religion == Some(Religion::Runes) {
            gold += 2;
        }
        if pl.religion == Some(Religion::AshenVeil) {
            mana += 2;
        }
        if c.has_wonder(WonderKind::TowerOfMastery) {
            mana += 8;
            lore += 8;
        }
        if c.has_wonder(WonderKind::AltarOfLuonnotar) {
            culture += 8;
        }
        culture += pl.secret_sum(|e| match e {
            SecretEffect::CulturePerCity(n) => n,
            _ => 0,
        });

        // House perk & loyalty scaling!
        if let Some(hid) = c.house {
            let (loy, mood, _) = self.house_loyalty(hid);
            if House::perk_active(loy) {
                if let Some(h) = self.houses.get(&hid) {
                    match h.perk {
                        HousePerk::TaxCharter => gold += 3,
                        HousePerk::MasonGuild => ind_calc.add(2, "House Mason Guild"),
                        HousePerk::LoreConclave => lore += 3,
                        HousePerk::FestivalCourts => culture += 4,
                        _ => {}
                    }
                }
            }
            let pct = mood.output_pct();
            if pct != 100 {
                let delta_i = ind_calc.total * (pct - 100) / 100;
                ind_calc.add(delta_i, format!("House {} ({}%)", mood.name(), pct));
                gold = gold * pct / 100;
                lore = lore * pct / 100;
            }
        }

        if self.is_megacity(c) {
            let bonus = ind_calc.total / 2;
            ind_calc.add(bonus, "Three Hearths Megacity (+50%)");
            gold = gold * 150 / 100;
            lore = lore * 150 / 100;
            culture = culture * 150 / 100;
        } else if c.is_outpost {
            let pen = -(ind_calc.total / 2);
            ind_calc.add(pen, "Outpost (-50%)");
            gold /= 2;
            lore /= 2;
        }

        if self.khazad_full_vault(c.owner) {
            let bonus = ind_calc.total * 35 / 100;
            ind_calc.add(bonus, "Vault & Crucible Full Vault (+35%)");
        }
        if pl.has_tech(crate::data::TECH_COLOSSUS_FORGE) {
            let bonus = ind_calc.total * 25 / 100;
            ind_calc.add(bonus, "Colossus Forge (+25%)");
        }

        // Stripped countryside from continuous raiding cuts output in half!
        if c.stripped_turns > 0 {
            let pen = -(ind_calc.total / 2);
            ind_calc.add(pen, format!("Stripped countryside ({}t left)", c.stripped_turns));
            gold /= 2;
        }
        ind_calc.total = ind_calc.total.max(2);

        (food.max(1), ind_calc, gold.max(1), lore.max(1), culture.max(1), mana)
    }

    pub fn gold_breakdown(&self, p: PlayerId) -> RateBreakdown {
        let mut r = RateBreakdown::default();
        let pl = &self.players[p.idx()];
        let mut city_gold = 0;
        for c in self.cities.values().filter(|c| c.owner == p) {
            city_gold += self.city_yields(c.id).2;
        }
        r.add(city_gold, "Cities & Houses");

        let mut caravan_gold = 0;
        let extra_pct = if pl.has_tech(crate::data::TECH_WAR_CHESTS) { 25 } else { 0 };
        for cv in self.caravans.values().filter(|cv| cv.owner == p) {
            let (Some(o), Some(d)) = (self.cities.get(&cv.origin), self.cities.get(&cv.destination)) else { continue };
            let foreign = o.owner != d.owner;
            let mut ep = extra_pct;
            if o.has_district(DistrictKind::Market) {
                ep += 25;
            }
            if foreign && pl.has_tech(crate::data::TECH_CROWN_CHARTERS) {
                ep += 50;
            }
            caravan_gold += cv.gold_per_turn(foreign, o.size, d.size, ep);
        }
        r.add(caravan_gold, "Caravan Routes");

        let sec_gold = pl.secret_sum(|e| match e {
            SecretEffect::GoldPerTurn(n) => n,
            _ => 0,
        });
        r.add(sec_gold, "Secret Techs");

        // Squad upkeep: 3 Gold per squad member (free if lent to a Crusade by Bannor!).
        let mut upkeep = 0;
        for s in self.squads.values().filter(|s| s.owner == p) {
            if pl.faction == FactionId::Bannor && s.lent_by.is_some() {
                continue;
            }
            upkeep += s.members.len() as i32 * UPKEEP_PER_MEMBER;
        }
        r.add(-upkeep, "Squad Upkeep");
        r
    }

    pub fn lore_breakdown(&self, p: PlayerId) -> RateBreakdown {
        let mut r = RateBreakdown::default();
        let pl = &self.players[p.idx()];
        r.add(4, "Base Lore");
        let mut city_lore = 0;
        for c in self.cities.values().filter(|c| c.owner == p) {
            city_lore += self.city_yields(c.id).3;
        }
        r.add(city_lore, "Cities & Academies");
        let sec = pl.secret_sum(|e| match e {
            SecretEffect::LorePerTurn(n) => n,
            _ => 0,
        });
        r.add(sec, "Secret Techs");
        r
    }

    pub fn mana_breakdown(&self, p: PlayerId) -> RateBreakdown {
        let mut r = RateBreakdown::default();
        let pl = &self.players[p.idx()];
        r.add(1, "Natural Seepage");
        let mut city_mana = 0;
        for c in self.cities.values().filter(|c| c.owner == p) {
            city_mana += self.city_yields(c.id).5;
        }
        r.add(city_mana, "Cities (Mana Vents, Academies, Creed, Wonders)");
        if pl.has_tech(crate::data::TECH_GRAND_CONJURATION) {
            r.add(3, "Grand Conjuration");
        }
        let sec = pl.secret_sum(|e| match e {
            SecretEffect::ManaPerTurn(n) => n,
            _ => 0,
        });
        r.add(sec, "Secret Techs");
        for perk in self.active_house_perks(p) {
            if perk == HousePerk::ManaConduits {
                r.add(2, "House Mana Conduits");
            }
        }
        r
    }

    pub fn tech_effective_cost(&self, p: PlayerId, tech: TechId) -> i32 {
        let info = &TECHS[tech.0 as usize];
        let pl = &self.players[p.idx()];
        let mut cost = info.cost;
        if let Some(doc) = info.doctrine {
            if pl.primary_doctrine == Some(doc) {
                cost = cost * PRIMARY_DOCTRINE_COST_PCT / 100;
            }
        }
        cost.max(MIN_TECH_COST)
    }

    pub fn can_research(&self, p: PlayerId, tech: TechId) -> Result<(), String> {
        let pl = &self.players[p.idx()];
        if pl.has_tech(tech) {
            return Err("Already researched.".into());
        }
        let info = &TECHS[tech.0 as usize];
        if let Some(req) = info.prereq {
            if !pl.has_tech(req) {
                return Err(format!("Requires {}.", TECHS[req.0 as usize].name));
            }
        }
        if let Some(doc) = info.doctrine {
            if pl.primary_doctrine == Some(doc) {
                return Ok(());
            }
            if pl.secondary_doctrine == Some(doc) && info.tier <= SECONDARY_DOCTRINE_MAX_TIER {
                return Ok(());
            }
            if info.tier <= SECONDARY_DOCTRINE_MAX_TIER {
                return Err(format!("Requires {} as Primary or Secondary Doctrine.", doc.name()));
            }
            return Err(format!("Requires {} as Primary Doctrine.", doc.name()));
        }
        Ok(())
    }

    /// Why a tech's effective cost differs from its listed cost (`None` when it does not).
    pub fn tech_cost_note(&self, p: PlayerId, tech: TechId) -> Option<String> {
        let info = &TECHS[tech.0 as usize];
        let cost = self.tech_effective_cost(p, tech);
        if cost == info.cost {
            return None;
        }
        let pl = &self.players[p.idx()];
        if info.doctrine.is_some() && info.doctrine == pl.primary_doctrine {
            Some(format!("{}% of {} as a Primary Doctrine tech.", PRIMARY_DOCTRINE_COST_PCT, info.cost))
        } else {
            Some(format!("Minimum cost {}.", MIN_TECH_COST))
        }
    }

    /// Checks an AdoptDoctrine command.
    pub fn check_doctrine(&self, p: PlayerId, doctrine: Doctrine, secondary: bool) -> Result<(), String> {
        let pl = &self.players[p.idx()];
        if !secondary {
            if pl.primary_doctrine.is_some() {
                return Err("A Primary Doctrine is already adopted for this game.".into());
            }
        } else {
            if pl.primary_doctrine.is_none() {
                return Err("Adopt a Primary Doctrine first.".into());
            }
            if pl.primary_doctrine == Some(doctrine) {
                return Err("Already your Primary Doctrine.".into());
            }
            if pl.secondary_doctrine.is_some() {
                return Err("A Secondary Doctrine is already adopted for this game.".into());
            }
        }
        Ok(())
    }

    pub fn side_mods(&self, p: PlayerId) -> SideMods {
        let pl = &self.players[p.idx()];
        let mut m = SideMods { supply_pct: 100, has_mana_boost: pl.mana >= MANA_BOOST_MIN, ..SideMods::default() };
        match pl.faction {
            FactionId::Bannor => m.front_row_atk_pct += 15,
            FactionId::Elohim => m.sentinel_hp_pct += 25,
            FactionId::Amurites => m.spell_pct += 20,
            FactionId::Khazad => {
                m.bombard_pct += 25;
                if self.khazad_full_vault(p) {
                    m.armor_bonus += 4;
                }
            }
            FactionId::Ljosalfar => m.forest_ranged_immune = true,
            FactionId::Infernals => m.hell_ground_bonus = true,
            _ => {}
        }
        if pl.religion == Some(Religion::Order) {
            m.front_row_atk_pct += 15;
        }
        if pl.religion == Some(Religion::Runes) {
            m.sentinel_hp_pct += 20;
        }
        if pl.religion == Some(Religion::Overlords) {
            m.spell_pct += 15;
        }
        if pl.has_tech(crate::data::TECH_BRONZE_ARMS) {
            m.front_row_atk_pct += 10;
        }
        if pl.has_tech(crate::data::TECH_PHALANX_WALL) {
            m.front_row_def_pct += 15;
        }
        if pl.has_tech(crate::data::TECH_MITHRIL_LEGIONS) {
            m.atk_pct += 20;
        }
        if pl.has_tech(crate::data::TECH_STIRRUPS) {
            m.charge_bonus_pct += 25;
        }
        if pl.has_tech(TECH_WARHORSE_LORDS) {
            m.pursuit = true;
        }
        if pl.has_tech(crate::data::TECH_GRAND_CONJURATION) {
            m.spell_pct += 25;
        }
        if pl.has_tech(crate::data::TECH_DIVINE_ESSENCE) {
            m.heal_revive_pct = 50;
        }
        if pl.has_tech(crate::data::TECH_APEX_PREDATORS) {
            m.ranged_pierce = true;
            m.crit_bonus += 15;
        }
        if self.cities.values().any(|c| c.owner == p && c.has_wonder(WonderKind::TowerOfMastery)) {
            m.spell_pct += 30;
        }
        if self.cities.values().any(|c| c.owner == p && c.has_wonder(WonderKind::AltarOfLuonnotar)) {
            m.heal_pct += 30;
            m.sentinel_hp_pct += 30;
        }
        for perk in self.active_house_perks(p) {
            if perk == HousePerk::BorderWardens {
                m.sentinel_hp_pct += 25;
            }
        }
        m.atk_pct += pl.secret_sum(|e| match e {
            SecretEffect::AllAtkPct(n) => n,
            _ => 0,
        });
        m.def_pct += pl.secret_sum(|e| match e {
            SecretEffect::AllDefPct(n) => n,
            _ => 0,
        });
        m.spell_pct += pl.secret_sum(|e| match e {
            SecretEffect::SpellPowerPct(n) => n,
            _ => 0,
        });
        m.heal_pct += pl.secret_sum(|e| match e {
            SecretEffect::HealBonusPct(n) => n,
            _ => 0,
        });
        m.armor_bonus += pl.secret_sum(|e| match e {
            SecretEffect::ArmorBonus(n) => n,
            _ => 0,
        });
        m.sentinel_hp_pct += pl.secret_sum(|e| match e {
            SecretEffect::SentinelHpPct(n) => n,
            _ => 0,
        });
        m
    }

    pub fn squad_supply_line(&self, squad_id: SquadId) -> Option<SupplyLine> {
        let s = self.squads.get(&squad_id)?;
        if self.players[s.owner.idx()].is_neutral || s.order == SquadOrder::Raid {
            return Some(SupplyLine { path: vec![s.plot], cost: 0, source: s.plot });
        }
        // Within own borders is always supplied (unless standing in a besieged city without another path).
        if self.world.plot(s.plot).owner == Some(s.owner) {
            let besieged_here = self.world.plot(s.plot).city.and_then(|cid| self.cities.get(&cid)).is_some_and(|c| c.besieged);
            if !besieged_here {
                return Some(SupplyLine { path: vec![s.plot], cost: 0, source: s.plot });
            }
        }
        let own_sources: BTreeSet<PlotIdx> = self
            .cities
            .values()
            .filter(|c| c.owner == s.owner && !c.besieged)
            .map(|c| c.plot)
            .collect();
        if own_sources.is_empty() {
            return None;
        }
        let enemy_squad_plots: Vec<PlotIdx> = self.squads.values().filter(|o| self.at_war(s.owner, o.owner)).map(|o| o.plot).collect();
        let (enemy_plots, fort_zoc) = enemy_supply_blocks(&self.world, &enemy_squad_plots);
        let budget = self.supply_budget(s.owner, s.captain_aura);
        let forest_supply = self.players[s.owner.idx()].faction == FactionId::Ljosalfar;
        trace_supply_line(&self.world, s.owner, s.plot, budget, forest_supply, &own_sources, &enemy_plots, &fort_zoc)
    }

    pub fn can_recruit_class(&self, p: PlayerId, class: UnitClassId) -> Result<(), String> {
        if class == UnitClassId::Sentinel {
            return Err("Sentinels are city walls, not trained units.".into());
        }
        let pl = &self.players[p.idx()];
        let info = class.info();
        if let Some(req) = info.tech {
            if !pl.has_tech(req) {
                return Err(format!("Requires {}.", TECHS[req.0 as usize].name));
            }
        }
        if class == UnitClassId::Paladin && pl.alignment == Alignment::Evil {
            return Err("Requires Good or Neutral alignment.".into());
        }
        if class == UnitClassId::Eidolon && pl.alignment != Alignment::Evil {
            return Err("Requires Evil alignment.".into());
        }
        if matches!(class, UnitClassId::PyreZombie | UnitClassId::Balor)
            && !matches!(pl.faction, FactionId::Sheaim | FactionId::Infernals)
            && pl.religion != Some(Religion::AshenVeil)
        {
            return Err(format!(
                "Requires the {} or {} faction, or the {} creed.",
                FactionId::Sheaim.info().name,
                FactionId::Infernals.info().name,
                Religion::AshenVeil.name()
            ));
        }
        Ok(())
    }

    /// The movement a squad gets at the start of its owner's turn, standing where it stands now.
    pub fn squad_turn_start_moves(&self, sid: SquadId) -> i32 {
        let Some(s) = self.squads.get(&sid) else { return 0 };
        let pl = &self.players[s.owner.idx()];
        let plot = self.world.plot(s.plot);
        let on_open = !plot.feature.is_woods() && plot.kind == PlotKind::Land;
        let mut move_bonus = 0;
        if pl.faction == FactionId::Hippus && on_open {
            move_bonus += 1;
        }
        if pl.has_tech(TECH_WARHORSE_LORDS) && s.members.iter().any(|m| matches!(m.class, UnitClassId::Horseman | UnitClassId::Knight)) {
            move_bonus += 1;
        }
        s.max_moves(move_bonus)
    }

    fn own_squad(&self, p: PlayerId, sid: SquadId) -> Result<&Squad, CmdErr> {
        let s = self.squads.get(&sid).ok_or_else(|| CmdErr::invalid("Unknown squad."))?;
        if s.owner != p {
            return Err(CmdErr::cannot("Not your squad."));
        }
        Ok(s)
    }

    /// Checks a Raid order (activating it).
    pub fn check_raid(&self, p: PlayerId, sid: SquadId) -> Result<(), CmdErr> {
        let s = self.own_squad(p, sid)?;
        if !self.players[p.idx()].has_tech(TECH_FIELDCRAFT) {
            return Err(CmdErr::cannot(format!("Requires {}.", TECHS[TECH_FIELDCRAFT.0 as usize].name)));
        }
        let plot_owner = self.world.plot(s.plot).owner;
        let hostile_land = plot_owner.is_some_and(|o| o != p && self.at_war(p, o));
        if !hostile_land || self.world.plot(s.plot).city.is_some() {
            return Err(CmdErr::cannot("Stand on the land of a civilization you are at war with, outside its cities."));
        }
        Ok(())
    }

    /// Checks a squad Road (`is_road`) or Fort order.
    pub fn check_field_work(&self, p: PlayerId, sid: SquadId, is_road: bool) -> Result<(), CmdErr> {
        if !self.players[p.idx()].has_tech(TECH_FIELDCRAFT) {
            return Err(CmdErr::cannot(format!("Requires {}.", TECHS[TECH_FIELDCRAFT.0 as usize].name)));
        }
        let s = self.own_squad(p, sid)?;
        if s.moves_left <= 0 {
            return Err(CmdErr::cannot("No movement left this turn."));
        }
        let pl = self.world.plot(s.plot);
        if is_road {
            if pl.road {
                return Err(CmdErr::cannot("This plot already has a road."));
            }
        } else if pl.city.is_some() {
            return Err(CmdErr::cannot("A Fort cannot be built in a city."));
        } else if pl.improvement == Improvement::Fort {
            return Err(CmdErr::cannot("This plot already has a Fort."));
        }
        Ok(())
    }

    /// Checks whether a city may be founded on `plot` (the site only).
    pub fn check_city_site(&self, plot: PlotIdx) -> Result<(), String> {
        let pl = self.world.plot(plot);
        if !pl.kind.is_passable_land() || pl.city.is_some() {
            return Err("A city cannot be founded on this plot.".into());
        }
        if self.cities.values().any(|c| self.world.distance(c.plot, plot) < MIN_CITY_DISTANCE) {
            return Err(format!("Too close to a city (minimum distance {}).", MIN_CITY_DISTANCE));
        }
        Ok(())
    }

    /// Checks a Found City order.
    pub fn check_found_city(&self, p: PlayerId, sid: SquadId) -> Result<(), CmdErr> {
        let s = self.own_squad(p, sid)?;
        if s.moves_left <= 0 {
            return Err(CmdErr::cannot("No movement left this turn."));
        }
        self.check_city_site(s.plot).map_err(CmdErr::cannot)?;
        let cost = found_city_cost(self.player_cities(p).len());
        let gold = self.players[p.idx()].gold;
        if gold < cost {
            return Err(CmdErr::cannot(format!("Need {cost} Gold (have {gold}).")));
        }
        Ok(())
    }

    /// Checks a Raze City order.
    pub fn check_raze(&self, p: PlayerId, city: CityId) -> Result<(), CmdErr> {
        let c = self.cities.get(&city).ok_or_else(|| CmdErr::invalid("Unknown city."))?;
        if c.owner != p {
            return Err(CmdErr::cannot("You must hold the city to raze it."));
        }
        if c.original_owner == p && !self.players[p.idx()].faction.info().can_raze_own {
            return Err(CmdErr::cannot(format!(
                "Only the {} and {} may raze a city they founded.",
                FactionId::Sheaim.info().name,
                FactionId::Infernals.info().name
            )));
        }
        if !self.apocalypse.can_raze_city(city) {
            return Err(CmdErr::cannot(format!("Caldera Keep cannot be razed for {} more turns.", self.apocalypse.carcosa_protected_turns)));
        }
        Ok(())
    }

    /// Checks the empire-wide part of a Launch Caravan order (tech, free slot, Gold).
    pub fn check_caravan_launch(&self, p: PlayerId) -> Result<(), CmdErr> {
        if !self.players[p.idx()].has_tech(TECH_TRADE_ROADS) {
            return Err(CmdErr::cannot(format!("Requires {}.", TECHS[TECH_TRADE_ROADS.0 as usize].name)));
        }
        let active = self.caravans.values().filter(|c| c.owner == p).count();
        let cap = self.caravan_cap(p);
        if active >= cap {
            return Err(CmdErr::cannot(format!("All Caravan slots are in use ({active}/{cap}).")));
        }
        Ok(())
    }

    /// Checks a Launch Caravan order between two cities; returns the route.
    pub fn check_caravan(&self, p: PlayerId, origin: CityId, dest: CityId) -> Result<Vec<PlotIdx>, CmdErr> {
        self.check_caravan_launch(p)?;
        let o = self.cities.get(&origin).ok_or_else(|| CmdErr::invalid("Unknown origin city."))?;
        let d = self.cities.get(&dest).ok_or_else(|| CmdErr::invalid("Unknown destination city."))?;
        if o.owner != p {
            return Err(CmdErr::cannot("A Caravan must start from one of your cities."));
        }
        if origin == dest {
            return Err(CmdErr::cannot("A Caravan needs a different destination city."));
        }
        if self.at_war(p, d.owner) {
            return Err(CmdErr::cannot("No trade with a civilization you are at war with."));
        }
        let gold = self.players[p.idx()].gold;
        if gold < CARAVAN_GOLD {
            return Err(CmdErr::cannot(format!("Need {CARAVAN_GOLD} Gold (have {gold}).")));
        }
        find_caravan_path(&self.world, o.plot, d.plot).ok_or_else(|| CmdErr::cannot("No land or coast route between these cities."))
    }

    /// Checks a Found or Adopt Creed order.
    pub fn check_religion(&self, p: PlayerId, religion: Religion) -> Result<(), CmdErr> {
        let pl = &self.players[p.idx()];
        if !pl.has_tech(TECH_MYSTICISM) {
            return Err(CmdErr::cannot(format!("Requires {}.", TECHS[TECH_MYSTICISM.0 as usize].name)));
        }
        if self.founded_religions.contains_key(&religion) {
            return Ok(());
        }
        let cost = religion.lore_cost();
        if pl.lore_progress < cost && pl.gold < cost {
            return Err(CmdErr::cannot(format!("Need {cost} Lore or {cost} Gold (have {} Lore, {} Gold).", pl.lore_progress, pl.gold)));
        }
        Ok(())
    }

    /// Checks a Cast Ritual order.
    pub fn check_ritual(&self, p: PlayerId, ritual: RitualKind) -> Result<(), CmdErr> {
        let cost = ritual.mana_cost();
        let mana = self.players[p.idx()].mana;
        if mana < cost {
            return Err(CmdErr::cannot(format!("Need {cost} Mana (have {mana}).")));
        }
        Ok(())
    }

    /// The plot a ritual that acts at "your capital" acts at (the capital, else any city).
    pub fn ritual_plot(&self, p: PlayerId) -> Option<PlotIdx> {
        self.cities.values().find(|c| c.owner == p && c.capital).or_else(|| self.cities.values().find(|c| c.owner == p)).map(|c| c.plot)
    }

    /// The city an Engineer or Bard Tempo Burst applies to.
    pub fn burst_city(&self, p: PlayerId) -> Option<CityId> {
        crate::great_people::burst_city(&self.player_cities(p))
    }

    /// Checks a Call Crusade order.
    pub fn check_call_crusade(&self, p: PlayerId, target: PlayerId) -> Result<(), CmdErr> {
        let pl = &self.players[p.idx()];
        if !pl.has_tech(crate::data::TECH_FANATICISM) && pl.religion != Some(Religion::Order) {
            return Err(CmdErr::cannot(format!(
                "Requires {} or the {} creed.",
                TECHS[crate::data::TECH_FANATICISM.0 as usize].name,
                Religion::Order.name()
            )));
        }
        if p == target || target == self.neutral_player || !self.players.get(target.idx()).is_some_and(|t| t.alive) {
            return Err(CmdErr::invalid("Invalid Crusade target."));
        }
        if self.crusades.values().any(|c| c.active && c.target == target) {
            return Err(CmdErr::cannot("A Crusade against this civilization is already active."));
        }
        Ok(())
    }

    /// Checks a Lend Squad order.
    pub fn check_lend(&self, p: PlayerId, crusade: CrusadeId, sid: SquadId) -> Result<(), CmdErr> {
        let cr = self.crusades.get(&crusade).ok_or_else(|| CmdErr::invalid("Unknown Crusade."))?;
        if !cr.active {
            return Err(CmdErr::cannot("This Crusade has ended."));
        }
        if p == cr.target {
            return Err(CmdErr::cannot("This Crusade is against you."));
        }
        let s = self.own_squad(p, sid)?;
        if s.lent_by.is_some() {
            return Err(CmdErr::cannot("This squad is already lent to a Crusade."));
        }
        Ok(())
    }

    /// Checks a Commission Hero order (the Great Person and the squad).
    pub fn check_hero(&self, p: PlayerId, gp: GreatPersonId, sid: SquadId) -> Result<(), CmdErr> {
        let g = self.court.get(&gp).ok_or_else(|| CmdErr::invalid("This Great Person is not in your Court."))?;
        if g.owner != p {
            return Err(CmdErr::cannot("Not your Great Person."));
        }
        let cap = self.players[p.idx()].squad_place_cap();
        let s = self.own_squad(p, sid)?;
        if s.places_used() + 1 > cap {
            return Err(CmdErr::cannot(format!("The squad has no free place ({}/{}).", s.places_used(), cap)));
        }
        if s.find_open_cell(g.suit.hero_class()).is_none() {
            return Err(CmdErr::cannot("The squad's 3x3 grid has no free cell."));
        }
        Ok(())
    }

    /// Checks a Commission Lord order (the Great Person and the House).
    pub fn check_lord(&self, p: PlayerId, gp: GreatPersonId, house: HouseId) -> Result<(), CmdErr> {
        let g = self.court.get(&gp).ok_or_else(|| CmdErr::invalid("This Great Person is not in your Court."))?;
        if g.owner != p {
            return Err(CmdErr::cannot("Not your Great Person."));
        }
        let h = self.houses.get(&house).ok_or_else(|| CmdErr::invalid("Unknown House."))?;
        if h.owner != p {
            return Err(CmdErr::cannot("Not your House."));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Pure Command Application
    // -----------------------------------------------------------------------------------------

    pub fn apply(&mut self, player: PlayerId, cmd: Command) -> Result<Vec<Event>, CmdErr> {
        if self.is_over() {
            return Err(CmdErr::cannot("The game is over."));
        }
        if !self.players.get(player.idx()).is_some_and(|p| p.alive) {
            return Err(CmdErr::cannot("This player is eliminated."));
        }
        let mut events = Vec::new();
        match cmd {
            Command::MoveSquad { squad, to } => self.cmd_move_squad(player, squad, to, &mut events)?,
            Command::AttackPlot { squad, to } => self.cmd_attack_plot(player, squad, to, &mut events)?,
            Command::SetSquadCell { squad, member, row, lane } => {
                self.own_squad(player, squad)?;
                self.squads.get_mut(&squad).unwrap().move_member(member, row, lane).map_err(CmdErr::cannot)?;
            }
            Command::AutoArrangeSquad { squad } => {
                self.own_squad(player, squad)?;
                self.squads.get_mut(&squad).unwrap().auto_arrange();
            }
            Command::ReinforceSquad { squad, class } => self.cmd_reinforce_squad(player, squad, class, &mut events)?,
            Command::UpgradeMember { squad, member } => self.cmd_upgrade_member(player, squad, member, &mut events)?,
            Command::SetSquadRaid { squad, active } => self.cmd_set_raid(player, squad, active)?,
            Command::SetSquadFortify { squad } => {
                self.own_squad(player, squad)?;
                let s = self.squads.get_mut(&squad).unwrap();
                s.order = SquadOrder::Fortify;
                s.moves_left = 0;
            }
            Command::SquadBuildRoad { squad } => self.cmd_field_work(player, squad, true)?,
            Command::SquadBuildFort { squad } => self.cmd_field_work(player, squad, false)?,
            Command::SquadExploreLair { squad } => self.cmd_explore_lair(player, squad, &mut events)?,
            Command::SquadFoundCity { squad, name } => self.cmd_found_city(player, squad, name, &mut events)?,
            Command::DisbandSquad { squad } => {
                self.own_squad(player, squad)?;
                self.squads.remove(&squad);
                self.recompute_vision(player);
            }
            Command::CityBuild { city, order } => self.cmd_city_build(player, city, order)?,
            Command::CitySallyPosture { city, posture } => {
                let c = self.cities.get_mut(&city).ok_or_else(|| CmdErr::invalid("Unknown city."))?;
                if c.owner != player {
                    return Err(CmdErr::cannot("Not your city."));
                }
                c.sally = posture;
            }
            Command::CityWaypoint { city, plot } => {
                let c = self.cities.get_mut(&city).ok_or_else(|| CmdErr::invalid("Unknown city."))?;
                if c.owner != player {
                    return Err(CmdErr::cannot("Not your city."));
                }
                c.waypoint = plot;
            }
            Command::RazeCity { city } => self.cmd_raze_city(player, city, &mut events)?,
            Command::LaunchCaravan { origin, destination, resource } => self.cmd_launch_caravan(player, origin, destination, resource, &mut events)?,
            Command::ChooseResearch { tech } => {
                self.can_research(player, tech).map_err(CmdErr::cannot)?;
                self.players[player.idx()].current_research = Some(tech);
            }
            Command::AdoptDoctrine { doctrine, secondary } => {
                self.check_doctrine(player, doctrine, secondary).map_err(CmdErr::cannot)?;
                let pl = &mut self.players[player.idx()];
                if !secondary {
                    pl.primary_doctrine = Some(doctrine);
                    self.push_log(Some(player), None, None, format!("Adopted Primary Doctrine: {}.", doctrine.name()));
                } else {
                    pl.secondary_doctrine = Some(doctrine);
                    self.push_log(
                        Some(player),
                        None,
                        None,
                        format!("Adopted Secondary Doctrine: {} (tiers 1-{}).", doctrine.name(), SECONDARY_DOCTRINE_MAX_TIER),
                    );
                }
            }
            Command::FoundOrAdoptReligion { religion } => self.cmd_religion(player, religion, &mut events)?,
            Command::CastRitual { ritual } => self.cmd_cast_ritual(player, ritual, &mut events)?,
            Command::UseGreatPerson { gp, action } => self.cmd_use_great_person(player, gp, action, &mut events)?,
            Command::DeclareWar { target } => self.cmd_declare_war(player, target)?,
            Command::MakePeace { target } => {
                if target == self.neutral_player {
                    return Err(CmdErr::cannot("No peace can be made with the Wild Clans."));
                }
                if !self.at_war(player, target) {
                    return Err(CmdErr::cannot("Not at war with this civilization."));
                }
                self.set_war(player, target, false);
                let tn = self.players[target.idx()].name();
                self.push_log(Some(player), None, None, format!("Signed peace with {}.", tn));
            }
            Command::BreakVassalage => {
                if !self.apocalypse.carcosa_risen {
                    return Err(CmdErr::cannot("A vassal may break free only once Worldseal Strain reaches 100."));
                }
                let Some(master) = self.players[player.idx()].vassal_of.take() else {
                    return Err(CmdErr::cannot("Not a vassal."));
                };
                self.set_war(player, master, true);
                let mn = self.players[master.idx()].name();
                self.push_log(Some(player), None, None, format!("Broke the vassal oath and declared war on {}.", mn));
            }
            Command::CallCrusade { target } => self.cmd_call_crusade(player, target, &mut events)?,
            Command::LendSquadToCrusade { crusade, squad } => self.cmd_lend_squad(player, crusade, squad)?,
            Command::PayOffRaiders { raider } => self.cmd_pay_off_raiders(player, raider)?,
            Command::EndTurn => {
                self.finish_player_turn(player, &mut events);
            }
        }
        self.check_victory(&mut events);
        Ok(events)
    }

    fn cmd_move_squad(&mut self, player: PlayerId, squad_id: SquadId, to: PlotIdx, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        let s = self.own_squad(player, squad_id)?.clone();
        if s.moves_left <= 0 {
            return Err(CmdErr::cannot("No movement left this turn."));
        }
        if self.world.distance(s.plot, to) != 1 {
            return Err(CmdErr::invalid("The destination must be adjacent."));
        }
        let pl = self.world.plot(to);
        if !pl.kind.is_passable_land() {
            return Err(CmdErr::cannot("Impassable terrain."));
        }
        // Check if an enemy squad or enemy city occupies `to` -> redirect to AttackPlot!
        let enemy_squad = self.squads.values().any(|o| o.plot == to && self.at_war(player, o.owner));
        let enemy_city = pl.city.and_then(|cid| self.cities.get(&cid)).is_some_and(|c| self.at_war(player, c.owner));
        if enemy_squad || enemy_city {
            return self.cmd_attack_plot(player, squad_id, to, events);
        }
        // Cannot enter a neutral/peaceful rival's city without war.
        if let Some(cid) = pl.city {
            if let Some(c) = self.cities.get(&cid) {
                if c.owner != player {
                    return Err(CmdErr::cannot("Declare war first to enter this city."));
                }
            }
        }
        // Check if an enemy Caravan is passing through `to` -> ambush for Gold!
        let ambushed: Vec<CaravanId> = self
            .caravans
            .values()
            .filter(|cv| cv.current_plot() == Some(to) && self.at_war(player, cv.owner))
            .map(|cv| cv.id)
            .collect();
        for cid in ambushed {
            if let Some(cv) = self.caravans.get_mut(&cid) {
                cv.reform_turns = 8;
                cv.growth_pct /= 2;
                self.players[player.idx()].gold += AMBUSH_GOLD;
                self.players[player.idx()].renown_meters[GreatSuit::Merchant.id()] += renown::AMBUSH_MERCHANT;
                self.push_log(Some(player), Some(to), None, format!("Ambushed an enemy Caravan: +{AMBUSH_GOLD} Gold."));
            }
        }

        let forest_free = self.players[player.idx()].faction == FactionId::Ljosalfar;
        let cost = self.world.plot(to).move_cost(forest_free);
        let from = s.plot;
        if let Some(sq) = self.squads.get_mut(&squad_id) {
            sq.plot = to;
            sq.moves_left = (sq.moves_left - cost).max(0);
            sq.order = SquadOrder::None;
        }
        self.recompute_vision(player);
        self.update_city_sieges();
        events.push(Event::SquadMoved { squad: squad_id, from, to });
        Ok(())
    }

    pub fn preview_attack(&self, player: PlayerId, squad_id: SquadId, to: PlotIdx) -> Result<(crate::combat::BuiltFormation, Forecast), CmdErr> {
        let att = self.own_squad(player, squad_id)?;
        if self.world.distance(att.plot, to) != 1 {
            return Err(CmdErr::invalid("The target plot must be adjacent."));
        }
        let city = self.world.plot(to).city.and_then(|cid| self.cities.get(&cid));
        let def_squad_id = self.pick_defending_squad(player, to);
        if def_squad_id.is_none() && !city.is_some_and(|c| c.owner != player && c.standing_sentinels() > 0) {
            return Err(CmdErr::cannot("No enemy defenders on that plot."));
        }
        let def_owner = def_squad_id
            .and_then(|id| self.squads.get(&id).map(|s| s.owner))
            .or_else(|| city.map(|c| c.owner))
            .ok_or_else(|| CmdErr::cannot("No defender on that plot."))?;
        let def_squad = def_squad_id.and_then(|id| self.squads.get(&id));
        let built = build_formation(
            &self.world,
            att.plot,
            to,
            att,
            def_squad,
            city,
            &self.side_mods(player),
            &self.side_mods(def_owner),
            self.apocalypse.counter,
        );
        let seed = (self.turn as u64).wrapping_mul(7919) ^ (squad_id.0 as u64) ^ ((to.0 as u64) << 16);
        let fc = forecast_formation(&built, 16, seed);
        Ok((built, fc))
    }

    fn pick_defending_squad(&self, attacker: PlayerId, plot: PlotIdx) -> Option<SquadId> {
        // Fair defense rotation: pick among enemy squads that fought fewest times this turn, then strongest.
        let mut cands: Vec<&Squad> = self.squads.values().filter(|s| s.plot == plot && s.owner != attacker && self.at_war(attacker, s.owner)).collect();
        if cands.is_empty() {
            // Also allow previewing before declaring war.
            cands = self.squads.values().filter(|s| s.plot == plot && s.owner != attacker).collect();
        }
        if cands.is_empty() {
            return None;
        }
        let min_fought = cands.iter().map(|s| s.fought_this_turn).min().unwrap_or(0);
        cands
            .into_iter()
            .filter(|s| s.fought_this_turn == min_fought)
            .max_by(|a, b| a.power_rating().cmp(&b.power_rating()).then_with(|| b.id.cmp(&a.id)))
            .map(|s| s.id)
    }

    fn cmd_attack_plot(&mut self, player: PlayerId, squad_id: SquadId, to: PlotIdx, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        let att = self.own_squad(player, squad_id)?.clone();
        if att.moves_left <= 0 {
            return Err(CmdErr::cannot("No movement left this turn."));
        }
        if self.world.distance(att.plot, to) != 1 {
            return Err(CmdErr::invalid("The target must be adjacent."));
        }
        let city_id = self.world.plot(to).city;
        let def_squad_id = self.pick_defending_squad(player, to);
        let def_owner = def_squad_id
            .and_then(|id| self.squads.get(&id).map(|s| s.owner))
            .or_else(|| city_id.and_then(|cid| self.cities.get(&cid).map(|c| c.owner)))
            .ok_or_else(|| CmdErr::cannot("No enemy on the target plot."))?;
        if def_owner == player {
            return Err(CmdErr::cannot("You cannot attack your own forces."));
        }
        if !self.at_war(player, def_owner) {
            self.set_war(player, def_owner, true);
            let dn = self.players[def_owner.idx()].name();
            self.push_log(Some(player), Some(to), None, format!("Declared war on {}.", dn));
        }

        // Empty undefended city with 0 standing sentinels -> walk right in and capture!
        let standing_sentinels = city_id.and_then(|cid| self.cities.get(&cid)).map_or(0, City::standing_sentinels);
        if def_squad_id.is_none() && standing_sentinels == 0 {
            if let Some(cid) = city_id {
                self.capture_city(cid, player, events);
                if let Some(sq) = self.squads.get_mut(&squad_id) {
                    sq.plot = to;
                    sq.moves_left = 0;
                }
                self.recompute_vision(player);
                return Ok(());
            }
            return Err(CmdErr::cannot("Nothing to attack on that plot."));
        }

        // Spend 2 Mana automatically if side has casters for empowered battle spells!
        if att.members.iter().any(|m| m.class.info().magic > 0) && self.players[player.idx()].mana >= BATTLE_MANA {
            self.players[player.idx()].mana -= BATTLE_MANA;
        }
        if let Some(dsid) = def_squad_id {
            if self.squads.get(&dsid).is_some_and(|ds| ds.members.iter().any(|m| m.class.info().magic > 0))
                && self.players[def_owner.idx()].mana >= BATTLE_MANA
            {
                self.players[def_owner.idx()].mana -= BATTLE_MANA;
            }
        }

        let battle_id = self.resolve_engagement(squad_id, def_squad_id, city_id, to, false, events);
        if let Some(rec) = self.battles.iter().find(|b| b.id == battle_id) {
            events.push(Event::BattleFought {
                battle: battle_id,
                attacker: player,
                defender: def_owner,
                plot: to,
                plot_taken: rec.plot_taken,
            });
        }
        self.recompute_vision(player);
        self.recompute_vision(def_owner);
        self.update_city_sieges();
        Ok(())
    }

    fn resolve_engagement(
        &mut self,
        att_squad_id: SquadId,
        def_squad_id: Option<SquadId>,
        city_id: Option<CityId>,
        to: PlotIdx,
        is_sally: bool,
        events: &mut Vec<Event>,
    ) -> BattleId {
        let att = self.squads[&att_squad_id].clone();
        let def_sq = def_squad_id.and_then(|id| self.squads.get(&id).cloned());
        let city = if is_sally { None } else { city_id.and_then(|cid| self.cities.get(&cid).cloned()) };
        let def_owner = def_sq.as_ref().map(|s| s.owner).or_else(|| city.as_ref().map(|c| c.owner)).unwrap_or(self.neutral_player);

        let att_mods = self.side_mods(att.owner);
        let def_mods = self.side_mods(def_owner);
        let mut built = build_formation(
            &self.world,
            att.plot,
            to,
            &att,
            def_sq.as_ref(),
            city.as_ref(),
            &att_mods,
            &def_mods,
            self.apocalypse.counter,
        );
        let outcome = ffh_battle::resolve(&built.formation, &mut self.rng);
        let (loser, decided_by, verdict_calc, hp_lost) = verdict_of(&built.snaps, &outcome);

        for (i, snap) in built.snaps.iter_mut().enumerate() {
            snap.hp_end = outcome.fighters[i].hp;
            snap.fate = outcome.fighters[i].fate;
        }

        // Apply fighter HP, deaths, Wounded Hero recovery, Infernal kill-healing, and XP!
        let mut att_kills = 0i32;
        let mut def_kills = 0i32;
        for snap in &built.snaps {
            if matches!(snap.fate, Fate::Fell { .. }) && snap.sentinel_idx.is_none() {
                if snap.side == Side::Defender {
                    att_kills += 1;
                } else {
                    def_kills += 1;
                }
            }
        }

        // Update Sentinels
        if let Some(cid) = city_id {
            if !is_sally {
                if let Some(c) = self.cities.get_mut(&cid) {
                    for snap in &built.snaps {
                        if let Some(si) = snap.sentinel_idx {
                            if si < c.sentinel_damage.len() {
                                let max_c = (snap.max_hp * 100).max(1);
                                let dmg = if snap.hp_end <= 0 { 100 } else { (100 - snap.hp_end * 100 / max_c).clamp(0, 99) };
                                c.sentinel_damage[si] = dmg;
                            }
                        }
                    }
                }
            }
        }

        // Helper to apply results to a squad
        let apply_squad = |game: &mut Game, sid: SquadId, side: Side, kills: i32, snaps: &[crate::combat::FighterSnap]| {
            let mut fallen_heroes = Vec::new();
            let owner = game.squads[&sid].owner;
            let infernal = game.players[owner.idx()].faction == FactionId::Infernals;
            if let Some(sq) = game.squads.get_mut(&sid) {
                sq.fought_this_turn += 1;
                let mut survivors = Vec::new();
                for mut m in std::mem::take(&mut sq.members) {
                    if let Some(snap) = snaps.iter().find(|sn| sn.side == side && sn.member_id == Some(m.id)) {
                        if matches!(snap.fate, Fate::Fell { .. }) || snap.hp_end <= 0 {
                            if m.is_hero() {
                                m.wounds += 1;
                                m.hp_centi = (m.max_hp() * 25).max(100);
                                fallen_heroes.push(m);
                            }
                            continue;
                        }
                        m.hp_centi = snap.hp_end;
                        m.grant_xp(3 + kills * 4);
                        if infernal && kills > 0 {
                            m.heal_pct(15 * kills.min(3));
                        }
                    }
                    survivors.push(m);
                }
                sq.members = survivors;
            }
            for h in fallen_heroes {
                let hname = h.display_name().to_string();
                game.recovering_heroes.push(RecoveringHero { owner, member: h, turns_left: HERO_RECOVERY_TURNS });
                game.push_log(Some(owner), None, None, format!("Hero {} fell in battle: +1 Wound, returns at the capital in {} turns.", hname, HERO_RECOVERY_TURNS));
            }
            if game.squads.get(&sid).is_some_and(|s| s.members.is_empty()) {
                game.squads.remove(&sid);
            }
        };

        apply_squad(self, att_squad_id, Side::Attacker, att_kills, &built.snaps);
        if let Some(dsid) = def_squad_id {
            apply_squad(self, dsid, Side::Defender, def_kills, &built.snaps);
        }

        // Commander Renown & Crusade credit from kills!
        self.players[att.owner.idx()].renown_meters[GreatSuit::Commander.id()] += renown::BATTLE_COMMANDER + att_kills * renown::BATTLE_COMMANDER_PER_KILL;
        self.players[def_owner.idx()].renown_meters[GreatSuit::Commander.id()] += renown::BATTLE_COMMANDER + def_kills * renown::BATTLE_COMMANDER_PER_KILL;
        let bloody = att_kills + def_kills >= BLOODY_BATTLE_DEATHS;
        if let Some(lender) = att.lent_by {
            for cr in self.crusades.values_mut().filter(|c| c.active && c.target == def_owner) {
                cr.add_credit(lender, 5 + att_kills * 10);
            }
        }

        // Push back losing defenders outside cities!
        let mut plot_taken = false;
        let mut city_captured = false;
        if let Some(sq) = self.squads.get_mut(&att_squad_id) {
            if !is_sally {
                sq.moves_left = 0;
            }
        }
        if loser == Some(Side::Defender) && !is_sally && city_id.is_none() {
            // Push surviving enemy squads off `to` to an adjacent friendly/empty land tile.
            let push_ids: Vec<SquadId> = self.squads.values().filter(|s| s.plot == to && s.owner == def_owner).map(|s| s.id).collect();
            let retreat = self
                .world
                .neighbors(to)
                .into_iter()
                .find(|&p| self.world.plot(p).kind.is_passable_land() && !self.squads.values().any(|s| s.plot == p && self.at_war(def_owner, s.owner)));
            for pid in push_ids {
                if let Some(rp) = retreat {
                    if let Some(sq) = self.squads.get_mut(&pid) {
                        sq.plot = rp;
                        sq.order = SquadOrder::None;
                    }
                }
            }
        }

        let defenders_left = self.squads.values().filter(|s| s.plot == to && s.owner == def_owner).count()
            + city_id.and_then(|cid| self.cities.get(&cid)).map_or(0, City::standing_sentinels);

        if defenders_left == 0 && !is_sally && self.squads.contains_key(&att_squad_id) {
            plot_taken = true;
            if let Some(cid) = city_id {
                self.capture_city(cid, att.owner, events);
                city_captured = true;
            }
            if let Some(sq) = self.squads.get_mut(&att_squad_id) {
                sq.plot = to;
            }
        }

        let battle_id = BattleId(self.battles.len() as u32 + 1);
        let place_name = city_id.and_then(|cid| self.cities.get(&cid).map(|c| c.name.clone()));
        let rec = BattleRecord {
            id: battle_id,
            turn: self.turn,
            from: att.plot,
            to,
            place_name: place_name.clone(),
            is_city: city_id.is_some() && !is_sally,
            attacker: att.owner,
            defender: def_owner,
            attacker_label: att.name.clone(),
            defender_label: def_sq.as_ref().map(|s| s.name.clone()).or(place_name).unwrap_or_else(|| "Garrison".into()),
            attacker_squad: att_squad_id,
            defender_squad: def_squad_id,
            ground: built.formation.ground,
            ground_rules: built.formation.ground.rules(&built.formation.params),
            fighters: built.snaps,
            events: outcome.events,
            rounds: outcome.rounds as i32,
            loser,
            decided_by,
            verdict_calc,
            hp_lost,
            plot_taken,
            city_captured,
            defenders_left,
        };
        let summary = format!(
            "Battle #{}: {} vs {} ({}, {}).",
            battle_id.0,
            rec.attacker_label,
            rec.defender_label,
            if rec.plot_taken { "Plot Taken" } else if rec.loser == Some(Side::Defender) { "Defender Repulsed" } else { "Defender Held" },
            rec.decided_by
        );
        self.battles.push(rec);
        self.push_log(Some(att.owner), Some(to), Some(battle_id), summary.clone());
        if def_owner != self.neutral_player {
            self.push_log(Some(def_owner), Some(to), Some(battle_id), summary);
        }
        // A bloody battle strains the Worldseal. Through `shift_ac`, once the battle is settled, so
        // that a crisis whose threshold this crosses takes effect (the Blowout included): a bare
        // `apocalypse.shift` marked the crisis as triggered and skipped what it does.
        if bloody {
            self.shift_ac(BLOODY_BATTLE_STRAIN, "bloody battle", events);
        }
        battle_id
    }

    fn cmd_reinforce_squad(&mut self, player: PlayerId, squad_id: SquadId, class: UnitClassId, _events: &mut Vec<Event>) -> Result<(), CmdErr> {
        self.can_recruit_class(player, class).map_err(CmdErr::cannot)?;
        let cap = self.players[player.idx()].squad_place_cap();
        let s = self.own_squad(player, squad_id)?;
        // Must be inside own borders (supplied home territory) to reinforce.
        if self.world.plot(s.plot).owner != Some(player) {
            return Err(CmdErr::cannot("The squad must stand inside your borders to reinforce."));
        }
        let info = class.info();
        let need_places = if info.large { 2 } else { 1 };
        if s.places_used() + need_places > cap {
            return Err(CmdErr::cannot(format!("The squad has no free place ({}/{cap}).", s.places_used())));
        }
        let Some((row, lane)) = s.find_open_cell(class) else {
            return Err(CmdErr::cannot("The squad's 3x3 grid has no free cell for this unit."));
        };
        let mut gold_cost = info.cost_gold;
        if self.players[player.idx()].faction == FactionId::Hippus {
            gold_cost = gold_cost * HIPPUS_REINFORCE_PCT / 100;
        }
        if self.players[player.idx()].has_tech(crate::data::TECH_WAR_CHESTS) {
            gold_cost = gold_cost * WAR_CHESTS_PCT / 100;
        }
        if self.players[player.idx()].gold < gold_cost {
            return Err(CmdErr::cannot(format!("Need {gold_cost} Gold (have {}).", self.players[player.idx()].gold)));
        }
        self.players[player.idx()].gold -= gold_cost;
        let mid = MemberId(self.next_member_id);
        self.next_member_id += 1;
        self.squads.get_mut(&squad_id).unwrap().members.push(SquadMember::new(mid, class, row, lane));
        Ok(())
    }

    fn cmd_upgrade_member(&mut self, player: PlayerId, squad_id: SquadId, member_id: MemberId, _events: &mut Vec<Event>) -> Result<(), CmdErr> {
        let s = self.own_squad(player, squad_id)?;
        if self.world.plot(s.plot).owner != Some(player) {
            return Err(CmdErr::cannot("The squad must stand inside your borders to upgrade."));
        }
        let m = s.members.iter().find(|m| m.id == member_id).ok_or_else(|| CmdErr::invalid("This unit is not in the squad."))?;
        let next = m.class.info().upgrades_to.ok_or_else(|| CmdErr::cannot("This unit has no further upgrade."))?;
        self.can_recruit_class(player, next).map_err(CmdErr::cannot)?;
        let mut cost = (next.info().cost_gold - m.class.info().cost_gold / 2).max(25);
        if self.players[player.idx()].has_tech(crate::data::TECH_WAR_CHESTS) {
            cost = cost * WAR_CHESTS_PCT / 100;
        }
        if self.players[player.idx()].gold < cost {
            return Err(CmdErr::cannot(format!("Upgrading costs {cost} Gold.")));
        }
        self.players[player.idx()].gold -= cost;
        if let Some(sq) = self.squads.get_mut(&squad_id) {
            if let Some(mm) = sq.members.iter_mut().find(|x| x.id == member_id) {
                let ratio = mm.hp_pct().max(25);
                mm.class = next;
                mm.hp_centi = mm.max_hp() * ratio;
            }
        }
        Ok(())
    }

    fn cmd_set_raid(&mut self, player: PlayerId, squad_id: SquadId, active: bool) -> Result<(), CmdErr> {
        self.own_squad(player, squad_id)?;
        if !active {
            self.squads.get_mut(&squad_id).unwrap().order = SquadOrder::None;
            return Ok(());
        }
        self.check_raid(player, squad_id)?;
        let sq = self.squads.get_mut(&squad_id).unwrap();
        sq.order = SquadOrder::Raid;
        sq.moves_left = 0;
        Ok(())
    }

    fn cmd_field_work(&mut self, player: PlayerId, squad_id: SquadId, is_road: bool) -> Result<(), CmdErr> {
        self.check_field_work(player, squad_id, is_road)?;
        let plot = self.squads[&squad_id].plot;
        let pl = self.world.plot_mut(plot);
        if is_road {
            pl.road = true;
            self.world.rev += 1;
        } else {
            pl.improvement = Improvement::Fort;
            self.world.rev += 1;
        }
        self.squads.get_mut(&squad_id).unwrap().moves_left = 0;
        Ok(())
    }

    fn cmd_explore_lair(&mut self, player: PlayerId, squad_id: SquadId, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        let s = self.own_squad(player, squad_id)?.clone();
        if !self.world.plot(s.plot).lair_unexplored {
            return Err(CmdErr::cannot("No unexplored Imperial Ruin on this plot."));
        }
        self.world.plot_mut(s.plot).lair_unexplored = false;
        self.world.plot_mut(s.plot).improvement = Improvement::None;
        self.world.rev += 1;
        let roll = self.rng.range(4);
        let summary = match roll {
            0 => {
                self.players[player.idx()].gold += LAIR_GOLD;
                format!("Imperial Ruin explored: +{LAIR_GOLD} Gold.")
            }
            1 => {
                self.players[player.idx()].mana += LAIR_MANA;
                self.players[player.idx()].renown_meters[GreatSuit::Sage.id()] += renown::LAIR_RELICS_SAGE;
                format!("Imperial Ruin explored: +{LAIR_MANA} Mana, +{} Sage Renown.", renown::LAIR_RELICS_SAGE)
            }
            2 => {
                self.players[player.idx()].lore_progress += LAIR_LORE;
                format!("Imperial Ruin explored: +{LAIR_LORE} Lore.")
            }
            _ => {
                if let Some(sq) = self.squads.get_mut(&squad_id) {
                    for m in &mut sq.members {
                        m.grant_xp(LAIR_XP);
                    }
                }
                format!("Imperial Ruin explored: +{LAIR_XP} XP to every unit in the squad.")
            }
        };
        if let Some(sq) = self.squads.get_mut(&squad_id) {
            sq.moves_left = 0;
        }
        self.push_log(Some(player), Some(s.plot), None, summary.clone());
        events.push(Event::LairExplored { player, plot: s.plot, summary });
        Ok(())
    }

    fn cmd_found_city(&mut self, player: PlayerId, squad_id: SquadId, name: Option<String>, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        self.check_found_city(player, squad_id)?;
        let s = self.squads[&squad_id].clone();
        let cost = found_city_cost(self.player_cities(player).len());
        self.players[player.idx()].gold -= cost;
        let cid = self.create_city_internal(player, s.plot, name, false);
        let cname = self.cities[&cid].name.clone();
        if let Some(sq) = self.squads.get_mut(&squad_id) {
            sq.moves_left = 0;
        }
        self.ensure_houses_for(player);
        self.recompute_vision(player);
        events.push(Event::CityFounded { city: cid, owner: player, plot: s.plot, name: cname.clone() });
        self.push_log(Some(player), Some(s.plot), None, format!("Founded {}.", cname));
        Ok(())
    }

    fn cmd_city_build(&mut self, player: PlayerId, city_id: CityId, order: BuildOrder) -> Result<(), CmdErr> {
        let c = self.cities.get(&city_id).ok_or_else(|| CmdErr::invalid("Unknown city."))?;
        if c.owner != player {
            return Err(CmdErr::cannot("Not your city."));
        }
        match &order {
            BuildOrder::District(d) => {
                if c.has_district(*d) {
                    return Err(CmdErr::cannot("This District is already built here."));
                }
                if c.districts.len() >= c.max_district_slots(self.is_megacity(c)) {
                    return Err(CmdErr::cannot("All District slots in this city are full."));
                }
                if let Some(req) = d.info().tech {
                    if !self.players[player.idx()].has_tech(req) {
                        return Err(CmdErr::cannot(format!("Requires {}.", TECHS[req.0 as usize].name)));
                    }
                }
            }
            BuildOrder::Squad(classes) => {
                if classes.is_empty() {
                    return Err(CmdErr::invalid("A squad needs at least 1 unit."));
                }
                let places: u8 = classes.iter().map(|cl| if cl.info().large { 2 } else { 1 }).sum();
                let cap = self.players[player.idx()].squad_place_cap();
                if places > cap {
                    return Err(CmdErr::cannot(format!("The squad uses {places} places; the limit is {cap}.")));
                }
                let (n, scap) = (self.player_squads(player).len(), self.squad_cap(player));
                if n >= scap {
                    return Err(CmdErr::cannot(format!("Squad cap reached ({n}/{scap}).")));
                }
                for &cl in classes {
                    self.can_recruit_class(player, cl).map_err(CmdErr::cannot)?;
                }
            }
            BuildOrder::Reinforce(cl) => {
                self.can_recruit_class(player, *cl).map_err(CmdErr::cannot)?;
            }
            BuildOrder::Caravan => {
                if !self.players[player.idx()].has_tech(TECH_TRADE_ROADS) {
                    return Err(CmdErr::cannot(format!("Requires {}.", TECHS[TECH_TRADE_ROADS.0 as usize].name)));
                }
            }
            BuildOrder::Wonder(w) => {
                if self.built_wonders.contains_key(w) {
                    return Err(CmdErr::cannot("This Wonder is already built in the world."));
                }
                if !self.players[player.idx()].has_tech(w.tech()) {
                    return Err(CmdErr::cannot(format!("Requires {}.", TECHS[w.tech().0 as usize].name)));
                }
            }
        }
        if let Some(city) = self.cities.get_mut(&city_id) {
            city.building = Some(order);
            city.build_progress = 0;
        }
        Ok(())
    }

    fn cmd_raze_city(&mut self, player: PlayerId, city_id: CityId, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        self.check_raze(player, city_id)?;
        let c = self.cities[&city_id].clone();
        self.cities.remove(&city_id);
        self.world.plot_mut(c.plot).city = None;
        self.world.plot_mut(c.plot).terrain = Terrain::BurningSands;
        self.world.rev += 1;
        for lb in &mut self.apocalypse.lifeboats {
            if lb.city_id == city_id {
                lb.razed = true;
            }
        }
        if let Some(hid) = c.house {
            if let Some(h) = self.houses.get_mut(&hid) {
                h.cities.retain(|&id| id != city_id);
            }
            self.houses.retain(|_, h| !h.cities.is_empty());
        }
        self.caravans.retain(|_, cv| cv.origin != city_id && cv.destination != city_id);
        let strain_before = self.apocalypse.counter;
        self.shift_ac(RAZE_STRAIN, &format!("{} razed", c.name), events);
        events.push(Event::CityRazed { plot: c.plot, name: c.name.clone(), by: player });
        let moved = self.apocalypse.counter - strain_before;
        let text = if moved != 0 { format!("Razed {} ({:+} Strain).", c.name, moved) } else { format!("Razed {}.", c.name) };
        self.push_log(Some(player), Some(c.plot), None, text);
        Ok(())
    }

    fn cmd_launch_caravan(&mut self, player: PlayerId, origin: CityId, dest: CityId, resource: Resource, _events: &mut Vec<Event>) -> Result<(), CmdErr> {
        let path = self.check_caravan(player, origin, dest)?;
        let o_plot = self.cities[&origin].plot;
        let dname = self.cities[&dest].name.clone();
        self.players[player.idx()].gold -= CARAVAN_GOLD;
        let id = CaravanId(self.next_caravan_id);
        self.next_caravan_id += 1;
        self.caravans.insert(
            id,
            CaravanRoute { id, owner: player, origin, destination: dest, resource, path, pos_idx: 0, outbound: true, reform_turns: 0, growth_pct: 0 },
        );
        self.push_log(Some(player), Some(o_plot), None, format!("Established Caravan route to {}.", dname));
        Ok(())
    }

    fn cmd_religion(&mut self, player: PlayerId, religion: Religion, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        self.check_religion(player, religion)?;
        if let Some(&_founder) = self.founded_religions.get(&religion) {
            // Adopt an already-founded faith!
            self.players[player.idx()].religion = Some(religion);
            self.players[player.idx()].alignment = religion.alignment();
            self.push_log(Some(player), None, None, format!("Adopted {} ({} alignment).", religion.name(), religion.alignment().name()));
            return Ok(());
        }
        // Found it for Lore!
        let cost = religion.lore_cost();
        if self.players[player.idx()].lore_progress >= cost {
            self.players[player.idx()].lore_progress -= cost;
        } else {
            self.players[player.idx()].gold -= cost;
        }
        self.founded_religions.insert(religion, player);
        self.players[player.idx()].religion = Some(religion);
        self.players[player.idx()].alignment = religion.alignment();
        self.players[player.idx()].renown_meters[GreatSuit::Prophet.id()] += renown::FOUND_CREED_PROPHET;
        self.shift_ac(religion.founding_ac_delta(), &format!("{} founded", religion.name()), events);
        events.push(Event::ReligionFounded { player, religion });
        self.push_log(Some(player), None, None, format!("Founded {}.", religion.name()));
        Ok(())
    }

    fn cmd_cast_ritual(&mut self, player: PlayerId, ritual: RitualKind, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        self.check_ritual(player, ritual)?;
        let cost = ritual.mana_cost();
        self.players[player.idx()].mana -= cost;
        self.players[player.idx()].renown_meters[GreatSuit::Sage.id()] += renown::RITUAL_SAGE;

        let cap_plot = self.ritual_plot(player);

        match ritual {
            RitualKind::Sanctify => {
                let delta = if self.players[player.idx()].faction == FactionId::Elohim { SANCTIFY_KEEPERS_STRAIN } else { ritual.ac_delta() };
                self.shift_ac(delta, ritual.name(), events);
                if let Some(cp) = cap_plot {
                    self.world.cleanse_hell(cp, 4);
                }
            }
            RitualKind::HasteMarch => {
                for s in self.squads.values_mut().filter(|s| s.owner == player) {
                    s.moves_left = s.max_moves(1);
                }
                self.shift_ac(ritual.ac_delta(), ritual.name(), events);
            }
            RitualKind::ArcaneSurge => {
                self.players[player.idx()].lore_progress += 65;
                self.shift_ac(ritual.ac_delta(), ritual.name(), events);
            }
            RitualKind::SummonHost => {
                let Some(cp) = cap_plot else {
                    return Err(CmdErr::cannot("Furnace Levy needs a city."));
                };
                self.spawn_squad_internal(player, cp, &[UnitClassId::PyreZombie, UnitClassId::PyreZombie, UnitClassId::Adept]);
                self.shift_ac(ritual.ac_delta(), ritual.name(), events);
            }
            RitualKind::BlightStrike => {
                let enemies: Vec<PlayerId> = self.players.iter().filter(|p| p.alive && !p.is_neutral && self.at_war(player, p.id)).map(|p| p.id).collect();
                self.world.spread_hell(6, &enemies, &mut self.rng);
                self.shift_ac(ritual.ac_delta(), ritual.name(), events);
            }
            RitualKind::WardWalls => {
                for c in self.cities.values_mut().filter(|c| c.owner == player) {
                    for d in &mut c.sentinel_damage {
                        *d = 0;
                    }
                }
                for s in self.squads.values_mut().filter(|s| s.owner == player) {
                    if self.world.plot(s.plot).city.is_some() {
                        for m in &mut s.members {
                            m.heal_pct(30);
                        }
                    }
                }
            }
        }
        self.push_log(Some(player), cap_plot, None, format!("Cast Strategic Ritual: {}.", ritual.name()));
        Ok(())
    }

    fn cmd_use_great_person(&mut self, player: PlayerId, gp_id: GreatPersonId, action: GreatPersonAction, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        let gp = self.court.get(&gp_id).ok_or_else(|| CmdErr::invalid("This Great Person is not in your Court."))?.clone();
        if gp.owner != player {
            return Err(CmdErr::cannot("Not your Great Person."));
        }
        match action {
            GreatPersonAction::DiscoverSecret => {
                let sid = gp.secret_tech.ok_or_else(|| CmdErr::cannot("This Great Person holds no Secret Tech card."))?;
                self.court.remove(&gp_id);
                self.players[player.idx()].secrets.push(sid);
                let info = &SECRET_TECHS[sid.0 as usize];
                if let SecretEffect::AcSurgeOnce(delta) = info.effect {
                    self.players[player.idx()].mana += crate::great_people::SECRET_SURGE_MANA;
                    self.shift_ac(delta, info.name, events);
                } else if info.id == SecretTechId(2) {
                    self.shift_ac(4, info.name, events);
                }
                self.push_log(Some(player), None, None, format!("{} discovered the Secret Tech {}.", gp.name, info.name));
            }
            GreatPersonAction::TempoBurst => {
                self.court.remove(&gp_id);
                if let Some(sid) = gp.secret_tech {
                    self.secret_deck.return_card(sid);
                }
                use crate::great_people as gpc;
                match gp.suit {
                    GreatSuit::Sage => {
                        self.players[player.idx()].lore_progress += gpc::BURST_SAGE_LORE;
                        self.players[player.idx()].mana += gpc::BURST_SAGE_MANA;
                    }
                    GreatSuit::Prophet => {
                        self.shift_ac(gpc::BURST_PROPHET_STRAIN, &format!("{}'s Benediction", gp.name), events);
                        for h in self.houses.values_mut().filter(|h| h.owner == player) {
                            h.bonus_loyalty += gpc::BURST_PROPHET_LOYALTY;
                        }
                    }
                    GreatSuit::Engineer => {
                        if let Some(cid) = self.burst_city(player) {
                            if let Some(c) = self.cities.get_mut(&cid) {
                                c.build_progress += gpc::BURST_ENGINEER_INDUSTRY;
                            }
                            self.try_complete_city_build(cid, events);
                        }
                    }
                    GreatSuit::Merchant => {
                        self.players[player.idx()].gold += gpc::BURST_MERCHANT_GOLD;
                        self.players[player.idx()].extra_caravan_slots += gpc::BURST_MERCHANT_CARAVAN_SLOTS;
                    }
                    GreatSuit::Bard => {
                        if let Some(cid) = self.burst_city(player) {
                            if let Some(c) = self.cities.get_mut(&cid) {
                                c.culture += gpc::BURST_BARD_CULTURE;
                            }
                            self.expand_city_borders(cid);
                        }
                    }
                    GreatSuit::Commander => {
                        for s in self.squads.values_mut().filter(|s| s.owner == player) {
                            for m in &mut s.members {
                                m.heal_pct(gpc::BURST_COMMANDER_HEAL_PCT);
                                m.grant_xp(gpc::BURST_COMMANDER_XP);
                            }
                        }
                    }
                }
                self.push_log(Some(player), None, None, format!("{} performed a {} Tempo Burst; the Secret Tech card returned to the world deck.", gp.name, gp.suit.name()));
            }
            GreatPersonAction::CommissionHero(squad_id) => {
                self.check_hero(player, gp_id, squad_id)?;
                let hero_class = gp.suit.hero_class();
                let (row, lane) = self.squads[&squad_id].find_open_cell(hero_class).unwrap();
                self.court.remove(&gp_id);
                if let Some(sid) = gp.secret_tech {
                    self.secret_deck.return_card(sid);
                }
                let mid = MemberId(self.next_member_id);
                self.next_member_id += 1;
                let mut hero = SquadMember::new_hero(mid, hero_class, gp.name.clone(), gp.portrait, gp.suit);
                hero.row = row;
                hero.lane = lane;
                let (sq_plot, sq_name) = {
                    let sq = self.squads.get_mut(&squad_id).unwrap();
                    sq.captain_name = gp.name.clone();
                    sq.captain_portrait = gp.portrait;
                    sq.members.push(hero);
                    (sq.plot, sq.name.clone())
                };
                self.push_log(Some(player), Some(sq_plot), None, format!("Commissioned {} as Hero Captain of {}.", gp.name, sq_name));
            }
            GreatPersonAction::CommissionLord(house_id) => {
                self.check_lord(player, gp_id, house_id)?;
                let h = self.houses.get_mut(&house_id).unwrap();
                self.court.remove(&gp_id);
                if let Some(sid) = gp.secret_tech {
                    self.secret_deck.return_card(sid);
                }
                h.lord_name = gp.name.clone();
                h.lord_portrait = gp.portrait;
                h.alignment = self.players[player.idx()].alignment;
                h.religion = self.players[player.idx()].religion;
                h.sympathy = None;
                h.bonus_loyalty += crate::great_people::LORD_LOYALTY;
                let hname = h.name.clone();
                self.push_log(
                    Some(player),
                    None,
                    None,
                    format!("Appointed {} as Lord of {} (+{} Loyalty).", gp.name, hname, crate::great_people::LORD_LOYALTY),
                );
            }
        }
        Ok(())
    }

    fn cmd_declare_war(&mut self, player: PlayerId, target: PlayerId) -> Result<(), CmdErr> {
        if player == target || target.idx() >= self.players.len() || !self.players[target.idx()].alive {
            return Err(CmdErr::invalid("Invalid war target."));
        }
        if self.at_war(player, target) {
            return Err(CmdErr::cannot("Already at war."));
        }
        self.set_war(player, target, true);
        // Cancel caravans between warring nations.
        self.caravans.retain(|_, cv| {
            let Some(d) = self.cities.get(&cv.destination) else { return false };
            !((cv.owner == player && d.owner == target) || (cv.owner == target && d.owner == player))
        });
        let tn = self.players[target.idx()].name();
        self.push_log(Some(player), None, None, format!("Declared war on {}.", tn));
        Ok(())
    }

    fn cmd_call_crusade(&mut self, player: PlayerId, target: PlayerId, events: &mut Vec<Event>) -> Result<(), CmdErr> {
        self.check_call_crusade(player, target)?;
        let cid = CrusadeId(self.next_crusade_id);
        self.next_crusade_id += 1;
        self.crusades.insert(cid, Crusade::new(cid, player, target));
        self.set_war(player, target, true);
        let tn = self.players[target.idx()].name();
        self.push_log(Some(player), None, None, format!("Called a Crusade against {}.", tn));
        events.push(Event::CrusadeCalled { crusade: cid, caller: player, target });
        Ok(())
    }

    fn cmd_lend_squad(&mut self, player: PlayerId, crusade_id: CrusadeId, squad_id: SquadId) -> Result<(), CmdErr> {
        self.check_lend(player, crusade_id, squad_id)?;
        let target = self.crusades[&crusade_id].target;
        self.squads.get_mut(&squad_id).unwrap().lent_by = Some(player);
        self.set_war(player, target, true);
        let credit = if self.players[player.idx()].faction == FactionId::Bannor { LEND_CREDIT_BANNOR } else { LEND_CREDIT };
        if let Some(cr) = self.crusades.get_mut(&crusade_id) {
            cr.lent_squads.push((player, squad_id));
            cr.add_credit(player, credit);
        }
        self.players[player.idx()].renown_meters[GreatSuit::Prophet.id()] += renown::LEND_PROPHET;
        Ok(())
    }

    fn cmd_pay_off_raiders(&mut self, player: PlayerId, raider: PlayerId) -> Result<(), CmdErr> {
        if raider == player || raider.idx() >= self.players.len() {
            return Err(CmdErr::invalid("Unknown raider."));
        }
        let tribute = RAID_TRIBUTE_GOLD;
        if self.players[player.idx()].gold < tribute {
            return Err(CmdErr::cannot(format!("Need {tribute} Gold (have {}).", self.players[player.idx()].gold)));
        }
        self.players[player.idx()].gold -= tribute;
        self.players[raider.idx()].gold += tribute;
        for s in self.squads.values_mut().filter(|s| s.owner == raider && s.order == SquadOrder::Raid) {
            if self.world.plot(s.plot).owner == Some(player) {
                s.order = SquadOrder::None;
            }
        }
        self.push_log(Some(player), None, None, format!("Paid {tribute} Gold tribute to end enemy raids."));
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Turn Resolution
    // -----------------------------------------------------------------------------------------

    fn finish_player_turn(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        // 1. Resolve Raids & Sallies on this player's cities.
        self.resolve_raids_on_player(player, events);

        // 2. Step this player's Caravans.
        let my_caravans: Vec<CaravanId> = self.caravans.values().filter(|c| c.owner == player).map(|c| c.id).collect();
        for cid in my_caravans {
            if let Some(cv) = self.caravans.get_mut(&cid) {
                cv.step();
                if cv.reform_turns == 0 {
                    self.players[player.idx()].renown_meters[GreatSuit::Merchant.id()] += renown::CARAVAN_MERCHANT;
                }
            }
        }

        // 3. Advance to next alive player; when wrapping back to Player 0, increment world turn!
        let n = self.players.len();
        let mut next = (player.idx() + 1) % n;
        let mut wrapped = next <= player.idx();
        while !self.players[next].alive && next != player.idx() {
            next = (next + 1) % n;
            if next == 0 {
                wrapped = true;
            }
        }
        if wrapped {
            self.turn += 1;
            self.process_world_turn(events);
            // The world turn marks the eliminated: the player chosen above may be one of them now,
            // and a turn handed to a player who cannot act would never end.
            let mut tried = 0;
            while !self.players[next].alive && tried < n {
                next = (next + 1) % n;
                tried += 1;
            }
        }
        self.active_player = PlayerId(next as u16);
        self.start_player_turn(self.active_player, events);
        events.push(Event::TurnStarted { turn: self.turn, active_player: self.active_player });
    }

    fn start_player_turn(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        if !self.players[player.idx()].alive {
            return;
        }
        // Economy income (4 decoupled currencies: Gold, Industry, Lore, Mana).
        let gold_net = self.gold_breakdown(player).total;
        let lore_net = self.lore_breakdown(player).total;
        let mana_net = self.mana_breakdown(player).total;
        self.players[player.idx()].gold = (self.players[player.idx()].gold + gold_net).max(0);
        self.players[player.idx()].lore_progress += lore_net;
        self.players[player.idx()].mana = (self.players[player.idx()].mana + mana_net).min(500);

        // Complete research if ready.
        if let Some(tech) = self.players[player.idx()].current_research {
            let cost = self.tech_effective_cost(player, tech);
            if self.players[player.idx()].lore_progress >= cost {
                self.players[player.idx()].lore_progress -= cost;
                self.players[player.idx()].techs.insert(tech);
                self.players[player.idx()].current_research = None;
                let info = &TECHS[tech.0 as usize];
                self.shift_ac(info.ac_delta, &format!("{} researched", info.name), events);
                events.push(Event::TechCompleted { player, tech });
                self.push_log(Some(player), None, None, format!("Completed research: {}.", info.name));
            }
        }

        // Cities: growth, Work Crews, Sentinel repair, Industry build progress, Renown meters!
        let city_ids = self.player_cities(player);
        let has_masonry = self.players[player.idx()].has_tech(TECH_MASONRY_WARDS);
        for cid in city_ids {
            let (food, ind_calc, _gold, _lore, culture, _mana) = self.city_yields(cid);
            // District Renown generation
            if let Some(c) = self.cities.get(&cid) {
                if c.has_district(DistrictKind::Academy) {
                    self.players[player.idx()].renown_meters[GreatSuit::Sage.id()] += renown::DISTRICT_PER_TURN;
                }
                if c.has_district(DistrictKind::Temple) {
                    self.players[player.idx()].renown_meters[GreatSuit::Prophet.id()] += renown::DISTRICT_PER_TURN;
                    self.players[player.idx()].renown_meters[GreatSuit::Bard.id()] += renown::DISTRICT_PER_TURN;
                }
                if c.has_district(DistrictKind::Forge) {
                    self.players[player.idx()].renown_meters[GreatSuit::Engineer.id()] += renown::DISTRICT_PER_TURN;
                }
                if c.has_district(DistrictKind::Market) {
                    self.players[player.idx()].renown_meters[GreatSuit::Merchant.id()] += renown::DISTRICT_PER_TURN;
                }
            }
            if let Some(c) = self.cities.get_mut(&cid) {
                c.sync_sentinels(has_masonry);
                if c.stripped_turns > 0 {
                    c.stripped_turns -= 1;
                }
                // Sentinel repair when NOT besieged.
                if !c.besieged {
                    for d in &mut c.sentinel_damage {
                        if *d > 0 {
                            *d = (*d - SENTINEL_REPAIR_PER_TURN).max(0);
                        }
                    }
                }
                // Growth (Size 1..10)
                if c.size < 10 {
                    c.food_stored += food;
                    let need = c.food_needed();
                    if c.food_stored >= need {
                        c.food_stored -= need;
                        c.size += 1;
                    }
                }
                c.culture += culture;
                c.build_progress += ind_calc.total;
            }
            self.expand_city_borders(cid);
            self.step_city_work_crew(cid);
            self.try_complete_city_build(cid, events);
        }

        // Squads: Supply Lines, Starvation, Healing, Movement reset!
        let squad_ids = self.player_squads(player);
        let leaves = self.players[player.idx()].religion == Some(Religion::Leaves);
        for sid in squad_ids {
            let supplied = self.squad_supply_line(sid).is_some();
            let plot = self.squads[&sid].plot;
            let in_own_city = self.world.plot(plot).city.and_then(|cid| self.cities.get(&cid)).is_some_and(|c| c.owner == player && !c.besieged);
            let in_own_land = self.world.plot(plot).owner == Some(player);
            let in_forest = self.world.plot(plot).feature.is_woods();
            let turn_moves = self.squad_turn_start_moves(sid);
            if let Some(s) = self.squads.get_mut(&sid) {
                s.fought_this_turn = 0;
                s.moves_left = turn_moves;
                if supplied {
                    s.unsupplied_turns = 0;
                    let heal = if in_own_city {
                        HEAL_OWN_CITY_PCT
                    } else if in_own_land || (leaves && in_forest) {
                        HEAL_OWN_LAND_PCT
                    } else if s.order == SquadOrder::Fortify {
                        HEAL_FORTIFIED_PCT
                    } else {
                        HEAL_FIELD_PCT
                    };
                    for m in &mut s.members {
                        m.heal_pct(heal);
                    }
                } else {
                    s.unsupplied_turns += 1;
                    if s.unsupplied_turns >= STARVE_ATTRITION_TURNS {
                        for m in &mut s.members {
                            let loss = m.max_hp() * STARVE_HP_LOSS_PCT;
                            m.hp_centi = (m.hp_centi - loss).max(100);
                        }
                    }
                }
            }
        }

        // Recovering Wounded Heroes return to capital!
        let cap_plot = self.cities.values().find(|c| c.owner == player && c.capital).or_else(|| self.cities.values().find(|c| c.owner == player)).map(|c| c.plot);
        let mut ready_heroes = Vec::new();
        for rh in &mut self.recovering_heroes {
            if rh.owner == player {
                rh.turns_left -= 1;
                if rh.turns_left <= 0 && cap_plot.is_some() {
                    ready_heroes.push(rh.member.clone());
                }
            }
        }
        self.recovering_heroes.retain(|rh| !(rh.owner == player && rh.turns_left <= 0 && cap_plot.is_some()));
        if let Some(cp) = cap_plot {
            for h in ready_heroes {
                let hname = h.display_name().to_string();
                let ch = self.alloc_character();
                let sid = SquadId(self.next_squad_id);
                self.next_squad_id += 1;
                let num = self.players[player.idx()].next_squad_num;
                self.players[player.idx()].next_squad_num += 1;
                let portrait = h.hero_portrait.unwrap_or(ch.portrait_key);
                let mut sq = Squad {
                    id: sid,
                    owner: player,
                    name: format!("{}'s Return (#{})", hname, num),
                    captain_name: hname.clone(),
                    captain_portrait: portrait,
                    captain_aura: ch.aura,
                    plot: cp,
                    members: vec![h],
                    moves_left: 2,
                    order: SquadOrder::None,
                    unsupplied_turns: 0,
                    fought_this_turn: 0,
                    lent_by: None,
                };
                sq.auto_arrange();
                self.squads.insert(sid, sq);
                self.push_log(Some(player), Some(cp), None, format!("Hero {} recovered and returned to the capital.", hname));
            }
        }

        // Great People Court birth check!
        if !self.players[player.idx()].is_neutral {
            let born = self.players[player.idx()].great_people_born;
            let need = renown_threshold(born);
            let total_renown: i32 = self.players[player.idx()].renown_meters.iter().sum();
            if total_renown >= need {
                let suit = dominant_suit(&self.players[player.idx()].renown_meters);
                self.players[player.idx()].renown_meters[suit.id()] = 0;
                self.players[player.idx()].great_people_born += 1;
                let ch = self.alloc_character();
                let secret = self.secret_deck.draw_for_suit(suit);
                let gpid = GreatPersonId(self.next_gp_id);
                self.next_gp_id += 1;
                let gp = GreatPerson {
                    id: gpid,
                    owner: player,
                    suit,
                    name: ch.name.to_string(),
                    portrait: ch.portrait_key,
                    secret_tech: secret,
                };
                self.court.insert(gpid, gp);
                events.push(Event::GreatPersonBorn { player, gp: gpid, name: ch.name.to_string() });
                self.push_log(Some(player), None, None, format!("Great {} {} arrived in your Court.", suit.name(), ch.name));
            }
        }

        // Wreckers Deep Shaft bonus as Strain rises!
        if self.players[player.idx()].faction == FactionId::Sheaim && self.apocalypse.counter >= DEEP_SHAFT_STRAIN && self.turn % DEEP_SHAFT_EVERY == 0 {
            if let Some(cp) = cap_plot {
                if self.player_squads(player).len() < self.squad_cap(player) {
                    self.spawn_squad_internal(player, cp, &[UnitClassId::PyreZombie, UnitClassId::PyreZombie]);
                    self.push_log(Some(player), Some(cp), None, "Deep Shaft: a squad of 2 Stokers joined at the capital.");
                }
            }
        }

        self.ensure_houses_for(player);
        self.recompute_vision(player);
    }

    fn process_world_turn(&mut self, events: &mut Vec<Event>) {
        // 1. Coupled Worldseal Strain drift (+1 every 3 turns, counteracted by Valve-Keeper's Ledger).
        if !self.apocalypse.carcosa_risen && self.turn % STRAIN_DRIFT_EVERY == 0 {
            let drain: i32 = self
                .players
                .iter()
                .filter(|p| p.alive && !p.is_neutral)
                .map(|p| {
                    p.secret_sum(|e| match e {
                        SecretEffect::AcDrainPerTurn(n) => n,
                        _ => 0,
                    })
                })
                .sum();
            let net = (1 - drain).max(0);
            if net > 0 {
                self.shift_ac(net, "caldera pressure", events);
            }
        }

        // 2. Active Crusades tick down & dissolve.
        let cr_ids: Vec<CrusadeId> = self.crusades.values().filter(|c| c.active).map(|c| c.id).collect();
        for cid in cr_ids {
            let mut end = false;
            if let Some(cr) = self.crusades.get_mut(&cid) {
                cr.turns_left -= 1;
                if cr.turns_left <= 0 || !self.players[cr.target.idx()].alive {
                    cr.active = false;
                    end = true;
                }
            }
            if end {
                let lent = self.crusades[&cid].lent_squads.clone();
                for (_, sid) in lent {
                    if let Some(sq) = self.squads.get_mut(&sid) {
                        sq.lent_by = None;
                    }
                }
                self.push_log(None, None, None, format!("Crusade #{} dissolved; lent squads return to normal service.", cid.0));
            }
        }

        // 3. Carcosa countdown timer after AC 100!
        if self.apocalypse.carcosa_risen {
            if self.apocalypse.carcosa_protected_turns > 0 {
                self.apocalypse.carcosa_protected_turns -= 1;
            }
            if let Some(cd) = &mut self.apocalypse.countdown {
                *cd -= 1;
            }
        }

        // 4. Mark eliminated civilizations.
        for p in 0..self.players.len() {
            if self.players[p].is_neutral || !self.players[p].alive {
                continue;
            }
            let pid = PlayerId(p as u16);
            if self.player_cities(pid).is_empty() && self.player_squads(pid).is_empty() {
                self.players[p].alive = false;
                let name = self.players[p].name();
                self.push_log(None, None, None, format!("{} has been eliminated.", name));
            }
        }

        self.recompute_scores();
    }

    fn shift_ac(&mut self, delta: i32, reason: &str, events: &mut Vec<Event>) {
        let fired = self.apocalypse.shift(delta, reason);
        for idx in fired {
            let crisis = &ARMAGEDDON_CRISES[idx];
            events.push(Event::CrisisTriggered {
                threshold: crisis.threshold,
                name: crisis.name.into(),
                summary: crisis.summary.into(),
            });
            self.push_log(None, None, None, format!("Worldseal Strain {}: {}. {}", crisis.threshold, crisis.name, crisis.summary));
            self.apply_crisis(crisis.threshold, events);
        }
    }

    fn apply_crisis(&mut self, threshold: i32, events: &mut Vec<Event>) {
        let evil_owners: Vec<PlayerId> = self.players.iter().filter(|p| p.alive && p.alignment == Alignment::Evil).map(|p| p.id).collect();
        match threshold {
            10 => {
                // Spawn 2 wild raiding squads on empty wild land.
                for _ in 0..2 {
                    if let Some(p) = self.find_wild_land_plot() {
                        self.spawn_squad_internal(self.neutral_player, p, &[UnitClassId::Guard, UnitClassId::Archer]);
                    }
                }
            }
            20 => {
                self.world.spread_hell(10, &evil_owners, &mut self.rng);
            }
            30 => {
                for h in self.houses.values_mut() {
                    h.bonus_loyalty -= 1;
                }
            }
            40 => {
                self.world.spread_hell(14, &evil_owners, &mut self.rng);
                for p in &mut self.players {
                    if p.alive && !p.is_neutral {
                        p.mana += 15;
                    }
                }
            }
            50 => {
                // King Cinder's Host rises automatically if not already in the game!
                if !self.apocalypse.hyborem_spawned && !self.players.iter().any(|p| p.faction == FactionId::Infernals && p.alive) {
                    self.spawn_hyborem(events);
                }
            }
            60 => {
                for c in self.cities.values_mut() {
                    if !c.has_district(DistrictKind::Temple) && !c.has_wonder(WonderKind::AltarOfLuonnotar) && c.size > 1 {
                        c.size -= 1;
                    }
                }
            }
            70 => {
                for _ in 0..2 {
                    if let Some(p) = self.find_wild_land_plot() {
                        self.spawn_squad_internal(self.neutral_player, p, &[UnitClassId::Knight, UnitClassId::Balor]);
                    }
                }
            }
            80 => {
                self.world.spread_hell(20, &evil_owners, &mut self.rng);
            }
            90 => {
                self.world.spread_hell(24, &evil_owners, &mut self.rng);
            }
            100 => {
                self.trigger_carcosa(events);
            }
            _ => {}
        }
    }

    fn spawn_hyborem(&mut self, events: &mut Vec<Event>) {
        let Some(plot) = self.find_wild_land_plot() else { return };
        self.apocalypse.hyborem_spawned = true;
        let new_id = PlayerId(self.players.len() as u16);
        let mut pl = Player::new(new_id, FactionId::Infernals, false, false);
        pl.techs.insert(TECH_FIELDCRAFT);
        pl.techs.insert(TECH_ARCANA);
        pl.techs.insert(TECH_METALLURGY);
        pl.techs.insert(TECH_FORTIFICATION);
        pl.primary_doctrine = Some(Doctrine::Steel);
        pl.gold = 220;
        pl.mana = 60;
        self.players.push(pl);
        self.world.vis.push(vec![crate::world::Visibility::Unrevealed; self.world.len()]);
        for pt in &mut self.world.plots {
            pt.culture.push(0);
        }
        // King Cinder's Host enters at war with Good empires & Neutral.
        for i in 0..self.players.len() - 1 {
            let oid = PlayerId(i as u16);
            if self.players[i].alignment == Alignment::Good || self.players[i].is_neutral {
                self.set_war(new_id, oid, true);
            }
        }
        let cid = self.create_city_internal(new_id, plot, Some("Great Furnace".into()), true);
        if let Some(c) = self.cities.get_mut(&cid) {
            c.size = 4;
            c.districts.push(DistrictKind::Barracks);
            c.districts.push(DistrictKind::Bulwark);
            c.sync_sentinels(false);
        }
        self.spawn_squad_internal(new_id, plot, &[UnitClassId::Balor, UnitClassId::PyreZombie, UnitClassId::PyreZombie]);
        self.spawn_squad_internal(new_id, plot, &[UnitClassId::Champion, UnitClassId::Mage, UnitClassId::PyreZombie]);
        self.world.spread_hell(10, &[new_id], &mut self.rng);
        self.recompute_vision(new_id);
        events.push(Event::HyboremArrived { player: new_id, city: cid });
        self.push_log(None, Some(plot), None, "King Cinder's Host entered the game with the city Great Furnace, at war with every Good civilization and the Wild Clans.");
    }

    fn trigger_carcosa(&mut self, events: &mut Vec<Event>) {
        if self.apocalypse.carcosa_risen {
            return;
        }
        self.apocalypse.carcosa_risen = true;
        self.apocalypse.countdown = Some(CARCOSA_TIMER_TURNS);
        self.apocalypse.carcosa_protected_turns = CARCOSA_PROTECTED_TURNS;

        let cp = self.world.raise_carcosa();
        let cid = self.create_city_internal(self.neutral_player, cp, Some("Caldera Keep".into()), false);
        if let Some(c) = self.cities.get_mut(&cid) {
            c.is_carcosa = true;
            c.size = 6;
            c.districts.push(DistrictKind::Bulwark);
            c.sync_sentinels(true);
        }
        // Strong neutral guardian squad defends Caldera Keep until someone conquers it!
        self.spawn_squad_internal(
            self.neutral_player,
            cp,
            &[UnitClassId::Phalanx, UnitClassId::Phalanx, UnitClassId::Archmage, UnitClassId::Acolyte],
        );
        self.apocalypse.carcosa_city = Some(cid);

        // Reveal Caldera Keep and its bridges to every living player!
        for p in 0..self.players.len() {
            if self.players[p].alive {
                self.world.reveal_radius(PlayerId(p as u16), cp, 3);
            }
        }

        // Register all scarce Lifeboats!
        let mut lifeboats = vec![Lifeboat { kind: LifeboatKind::Carcosa, city_id: cid, razed: false }];
        if let Some(&tc) = self.built_wonders.get(&WonderKind::TowerOfMastery) {
            if self.cities.contains_key(&tc) {
                lifeboats.push(Lifeboat { kind: LifeboatKind::TowerOfMastery, city_id: tc, razed: false });
            }
        }
        if let Some(&ac) = self.built_wonders.get(&WonderKind::AltarOfLuonnotar) {
            if self.cities.contains_key(&ac) {
                lifeboats.push(Lifeboat { kind: LifeboatKind::AltarOfLuonnotar, city_id: ac, razed: false });
            }
        }
        if let Some(leg) = self
            .cities
            .values()
            .filter(|c| !c.is_carcosa && c.owner != self.neutral_player && c.culture >= LIFEBOAT_CULTURE_MIN)
            .max_by_key(|c| (c.culture, std::cmp::Reverse(c.id)))
        {
            if !lifeboats.iter().any(|l| l.city_id == leg.id) {
                lifeboats.push(Lifeboat { kind: LifeboatKind::MostLegendary, city_id: leg.id, razed: false });
            }
        }
        self.apocalypse.lifeboats = lifeboats;
        events.push(Event::CarcosaRisen { city: cid, plot: cp });
        self.push_log(
            None,
            Some(cp),
            None,
            format!(
                "Caldera Keep has risen. {} Lifeboats stand; the civilizations holding one when the {}-turn countdown ends win.",
                self.apocalypse.lifeboats.len(),
                CARCOSA_TIMER_TURNS
            ),
        );
    }

    fn check_victory(&mut self, events: &mut Vec<Event>) {
        if self.apocalypse.outcome.is_some() {
            return;
        }
        // 1. After Strain 100: Furnace victory if every Lifeboat is razed!
        if self.apocalypse.carcosa_risen && self.apocalypse.standing_lifeboats() == 0 {
            let demonic_winners: Vec<PlayerId> = self
                .players
                .iter()
                .filter(|p| p.alive && !p.is_neutral && ApocalypseState::demonic_victory_eligible(p.faction))
                .map(|p| p.id)
                .collect();
            let outcome = if demonic_winners.is_empty() {
                GameOutcome {
                    kind: VictoryKind::MutualExtinction,
                    winners: Vec::new(),
                    summary: "Every Lifeboat was razed, leaving no survivors.".into(),
                }
            } else {
                GameOutcome {
                    kind: VictoryKind::DemonicRuin,
                    winners: demonic_winners,
                    summary: "Every Lifeboat was razed. Furnace victory.".into(),
                }
            };
            events.push(Event::GameEnded { summary: outcome.summary.clone() });
            self.apocalypse.outcome = Some(outcome);
            return;
        }

        // 2. After Strain 100: Countdown expires -> holders of standing Lifeboats win!
        if self.apocalypse.carcosa_risen && self.apocalypse.countdown.is_some_and(|t| t <= 0) {
            let mut winners = Vec::new();
            for lb in &self.apocalypse.lifeboats {
                if lb.razed {
                    continue;
                }
                if let Some(c) = self.cities.get(&lb.city_id) {
                    let owner = c.owner;
                    if !self.players[owner.idx()].is_neutral
                        && ApocalypseState::mortal_victory_eligible(self.players[owner.idx()].faction)
                        && !winners.contains(&owner)
                    {
                        winners.push(owner);
                    }
                }
            }
            let summary = if winners.is_empty() {
                "The timer expired with no surface empire holding a Lifeboat. The surface burns.".into()
            } else {
                let names: Vec<&str> = winners.iter().map(|p| self.players[p.idx()].name()).collect();
                format!("Lifeboat survival. Winners: {}.", names.join(", "))
            };
            let kind = if winners.is_empty() { VictoryKind::MutualExtinction } else { VictoryKind::MortalLifeboat };
            events.push(Event::GameEnded { summary: summary.clone() });
            self.apocalypse.outcome = Some(GameOutcome { kind, winners, summary });
            return;
        }

        // 3. Before Strain 100: Conquest (only 1 non-neutral civ alive).
        let alive_civs: Vec<PlayerId> = self.players.iter().filter(|p| p.alive && !p.is_neutral).map(|p| p.id).collect();
        if alive_civs.len() == 1 && !self.apocalypse.carcosa_risen {
            let w = alive_civs[0];
            let summary = format!("Conquest victory: {}.", self.players[w.idx()].name());
            events.push(Event::GameEnded { summary: summary.clone() });
            self.apocalypse.outcome = Some(GameOutcome { kind: VictoryKind::Conquest, winners: vec![w], summary });
            return;
        }

        // 4. Turn 160 without Blowout -> Keepers of the Seal win if alive, else highest Score!
        if self.turn >= MAX_GAME_TURNS && !self.apocalypse.carcosa_risen {
            if let Some(elohim) = self.players.iter().find(|p| p.alive && p.faction == FactionId::Elohim) {
                let summary = format!("Turn {} reached before the Blowout. {} win.", MAX_GAME_TURNS, FactionId::Elohim.info().name);
                events.push(Event::GameEnded { summary: summary.clone() });
                self.apocalypse.outcome = Some(GameOutcome { kind: VictoryKind::ElohimVigil, winners: vec![elohim.id], summary });
            } else if let Some(best) = self.players.iter().filter(|p| p.alive && !p.is_neutral).max_by_key(|p| p.score) {
                let summary = format!("Turn {} reached before the Blowout. {} wins with the highest score.", MAX_GAME_TURNS, best.name());
                events.push(Event::GameEnded { summary: summary.clone() });
                self.apocalypse.outcome = Some(GameOutcome { kind: VictoryKind::ScoreAtTurnLimit, winners: vec![best.id], summary });
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Internal Helpers (Cities, Work Crews, Houses, Raids/Sallies, Vision, Score)
    // -----------------------------------------------------------------------------------------

    fn create_city_internal(&mut self, owner: PlayerId, plot: PlotIdx, custom_name: Option<String>, capital: bool) -> CityId {
        let cid = CityId(self.next_city_id);
        self.next_city_id += 1;
        let pl = &mut self.players[owner.idx()];
        let name = custom_name.unwrap_or_else(|| {
            let names = pl.faction.info().city_names;
            let idx = pl.next_city_name_idx;
            pl.next_city_name_idx += 1;
            if idx < names.len() {
                names[idx].to_string()
            } else {
                format!("{} Hold {}", pl.faction.info().name, idx + 1)
            }
        });
        let is_outpost = if let Some(max_super) = pl.faction.info().max_super_cities {
            self.cities.values().filter(|c| c.owner == owner && !c.is_outpost).count() >= max_super
        } else {
            false
        };
        let mut city = City::new(cid, owner, name, plot, capital, is_outpost);
        city.sync_sentinels(self.players[owner.idx()].has_tech(TECH_MASONRY_WARDS));
        self.world.plot_mut(plot).city = Some(cid);
        self.world.plot_mut(plot).road = true;
        self.world.plot_mut(plot).owner = Some(owner);
        self.world.rev += 1;
        self.cities.insert(cid, city);
        self.expand_city_borders(cid);
        cid
    }

    pub fn spawn_squad_internal(&mut self, owner: PlayerId, plot: PlotIdx, classes: &[UnitClassId]) -> SquadId {
        let sid = SquadId(self.next_squad_id);
        self.next_squad_id += 1;
        let ch = self.alloc_character();
        let num = self.players[owner.idx()].next_squad_num;
        self.players[owner.idx()].next_squad_num += 1;
        let mut members = Vec::new();
        for &cl in classes {
            let mid = MemberId(self.next_member_id);
            self.next_member_id += 1;
            members.push(SquadMember::new(mid, cl, cl.info().row_order[0] as u8, 1));
        }
        let mut sq = Squad {
            id: sid,
            owner,
            name: format!("{}'s {} Squad", ch.name.split_whitespace().next().unwrap_or("War"), ordinal(num)),
            captain_name: ch.name.to_string(),
            captain_portrait: ch.portrait_key,
            captain_aura: ch.aura,
            plot,
            members,
            moves_left: 2,
            order: SquadOrder::None,
            unsupplied_turns: 0,
            fought_this_turn: 0,
            lent_by: None,
        };
        sq.auto_arrange();
        sq.moves_left = sq.max_moves(0);
        self.squads.insert(sid, sq);
        sid
    }

    fn expand_city_borders(&mut self, city_id: CityId) {
        let Some(c) = self.cities.get(&city_id).cloned() else { return };
        let radius: i32 = if c.culture >= BORDER_CULTURE_RADIUS_2 { 2 } else { 1 };
        let (cx, cy) = (self.world.plot(c.plot).x, self.world.plot(c.plot).y);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() + dy.abs() > radius + 1 {
                    continue;
                }
                if let Some(p) = self.world.idx(cx + dx, cy + dy) {
                    let pt = self.world.plot_mut(p);
                    if pt.culture.len() > c.owner.idx() {
                        pt.culture[c.owner.idx()] += (c.culture / 10).max(2);
                    }
                    if pt.city.is_none() {
                        // Highest culture claims border tile.
                        if let Some((best_p, &best_v)) = pt.culture.iter().enumerate().max_by_key(|&(_, v)| v) {
                            if best_v > 0 && pt.owner != Some(PlayerId(best_p as u16)) {
                                pt.owner = Some(PlayerId(best_p as u16));
                                self.world.rev += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    fn step_city_work_crew(&mut self, city_id: CityId) {
        let Some(c) = self.cities.get(&city_id) else { return };
        if self.players[c.owner.idx()].is_neutral || c.besieged {
            return;
        }
        let mut task = c.crew_task.clone();
        if task.is_none() {
            task = c.pick_crew_task(&self.world);
        }
        if let Some(mut t) = task {
            t.progress += 1;
            if t.progress >= t.needed {
                let pl = self.world.plot_mut(t.plot);
                if t.build_road {
                    pl.road = true;
                } else {
                    pl.improvement = t.improvement;
                }
                self.world.rev += 1;
                self.cities.get_mut(&city_id).unwrap().crew_task = None;
            } else {
                self.cities.get_mut(&city_id).unwrap().crew_task = Some(t);
            }
        }
    }

    fn try_complete_city_build(&mut self, city_id: CityId, events: &mut Vec<Event>) {
        let Some(c) = self.cities.get(&city_id).cloned() else { return };
        let Some(order) = c.building.clone() else { return };
        if c.build_progress < order.cost() {
            return;
        }
        if let Some(cm) = self.cities.get_mut(&city_id) {
            cm.building = None;
            cm.build_progress = 0;
        }
        match order {
            BuildOrder::District(d) => {
                let has_masonry = self.players[c.owner.idx()].has_tech(TECH_MASONRY_WARDS);
                if let Some(cm) = self.cities.get_mut(&city_id) {
                    if !cm.has_district(d) {
                        cm.districts.push(d);
                        cm.sync_sentinels(has_masonry);
                    }
                }
                self.world.rev += 1;
                self.push_log(Some(c.owner), Some(c.plot), None, format!("{}: completed {} District.", c.name, d.info().name));
            }
            BuildOrder::Squad(classes) => {
                let sid = self.spawn_squad_internal(c.owner, c.plot, &classes);
                // If Forge is present, give fresh recruits +5 XP!
                if c.has_district(DistrictKind::Forge) {
                    if let Some(sq) = self.squads.get_mut(&sid) {
                        for m in &mut sq.members {
                            m.grant_xp(5);
                        }
                    }
                }
                self.push_log(Some(c.owner), Some(c.plot), None, format!("{}: raised a new squad.", c.name));
            }
            BuildOrder::Reinforce(cl) => {
                // Reinforce the first squad on the city plot with room, else start a new 1-unit squad.
                let cap = self.players[c.owner.idx()].squad_place_cap();
                let target_sq = self
                    .squads
                    .values()
                    .find(|s| s.owner == c.owner && s.plot == c.plot && s.places_used() + if cl.info().large { 2 } else { 1 } <= cap)
                    .map(|s| s.id);
                if let Some(sid) = target_sq {
                    if let Some((row, lane)) = self.squads[&sid].find_open_cell(cl) {
                        let mid = MemberId(self.next_member_id);
                        self.next_member_id += 1;
                        self.squads.get_mut(&sid).unwrap().members.push(SquadMember::new(mid, cl, row, lane));
                    }
                } else {
                    self.spawn_squad_internal(c.owner, c.plot, &[cl]);
                }
                self.push_log(Some(c.owner), Some(c.plot), None, format!("{}: trained {}.", c.name, cl.info().name));
            }
            BuildOrder::Caravan => {
                // Auto-route to the richest partner city!
                let partner = self
                    .cities
                    .values()
                    .filter(|o| o.id != city_id && !self.at_war(c.owner, o.owner))
                    .max_by_key(|o| (o.owner != c.owner, o.size, self.world.distance(c.plot, o.plot)))
                    .map(|o| o.id);
                if let Some(dest) = partner {
                    if let Some(path) = find_caravan_path(&self.world, c.plot, self.cities[&dest].plot) {
                        let id = CaravanId(self.next_caravan_id);
                        self.next_caravan_id += 1;
                        self.caravans.insert(
                            id,
                            CaravanRoute {
                                id,
                                owner: c.owner,
                                origin: city_id,
                                destination: dest,
                                resource: Resource::None,
                                path,
                                pos_idx: 0,
                                outbound: true,
                                reform_turns: 0,
                                growth_pct: 0,
                            },
                        );
                    }
                }
            }
            BuildOrder::Wonder(w) => {
                if !self.built_wonders.contains_key(&w) {
                    self.built_wonders.insert(w, city_id);
                    if let Some(cm) = self.cities.get_mut(&city_id) {
                        cm.wonders.push(w);
                    }
                    self.players[c.owner.idx()].renown_meters[GreatSuit::Engineer.id()] += renown::WONDER_ENGINEER;
                    let strain_before = self.apocalypse.counter;
                    self.shift_ac(w.ac_delta(), w.name(), events);
                    let moved = self.apocalypse.counter - strain_before;
                    if self.apocalypse.carcosa_risen {
                        let kind = match w {
                            WonderKind::TowerOfMastery => LifeboatKind::TowerOfMastery,
                            WonderKind::AltarOfLuonnotar => LifeboatKind::AltarOfLuonnotar,
                        };
                        self.apocalypse.lifeboats.push(Lifeboat { kind, city_id, razed: false });
                    }
                    let text = if moved != 0 {
                        format!("{} completed the {} ({:+} Strain).", c.name, w.name(), moved)
                    } else {
                        format!("{} completed the {}.", c.name, w.name())
                    };
                    self.push_log(Some(c.owner), Some(c.plot), None, text);
                }
            }
        }
    }

    fn capture_city(&mut self, city_id: CityId, new_owner: PlayerId, events: &mut Vec<Event>) {
        let Some(c) = self.cities.get(&city_id).cloned() else { return };
        let old_owner = c.owner;
        if let Some(old_h) = c.house {
            if let Some(h) = self.houses.get_mut(&old_h) {
                h.cities.retain(|&id| id != city_id);
            }
            self.houses.retain(|_, h| !h.cities.is_empty());
        }
        if let Some(cm) = self.cities.get_mut(&city_id) {
            cm.owner = new_owner;
            cm.capital = false;
            cm.house = None;
            cm.building = None;
            cm.build_progress = 0;
            cm.crew_task = None;
            for d in &mut cm.sentinel_damage {
                *d = 100;
            }
        }
        self.world.plot_mut(c.plot).owner = Some(new_owner);
        self.world.rev += 1;
        // Reassign capital if old_owner lost theirs.
        if c.capital {
            if let Some(next_cap) = self.cities.values_mut().find(|x| x.owner == old_owner) {
                next_cap.capital = true;
            }
        }
        if !self.cities.values().any(|x| x.owner == new_owner && x.capital) {
            if let Some(cm) = self.cities.get_mut(&city_id) {
                cm.capital = true;
            }
        }
        self.ensure_houses_for(new_owner);
        self.players[new_owner.idx()].renown_meters[GreatSuit::Commander.id()] += renown::CAPTURE_COMMANDER;
        events.push(Event::CityCaptured { city: city_id, new_owner, old_owner, name: c.name.clone() });
        self.push_log(Some(new_owner), Some(c.plot), None, format!("Captured {}.", c.name));
        if old_owner != self.neutral_player {
            self.push_log(Some(old_owner), Some(c.plot), None, format!("Lost {} to {}.", c.name, self.players[new_owner.idx()].name()));
        }
    }

    fn resolve_raids_on_player(&mut self, defender: PlayerId, events: &mut Vec<Event>) {
        let def_cities = self.player_cities(defender);
        for cid in def_cities {
            let Some(c) = self.cities.get(&cid).cloned() else { continue };
            let raiders: Vec<SquadId> = self
                .squads
                .values()
                .filter(|s| {
                    s.order == SquadOrder::Raid
                        && self.at_war(defender, s.owner)
                        && self.world.plot(s.plot).owner == Some(defender)
                        && self.world.distance(s.plot, c.plot) <= RAID_RANGE
                })
                .map(|s| s.id)
                .collect();
            if raiders.is_empty() {
                if let Some(cm) = self.cities.get_mut(&cid) {
                    cm.raid_streak = 0;
                }
                continue;
            }
            // Attempt sally from garrisoned squad if posture permits!
            let mut sallied = false;
            if c.sally != SallyPosture::Never {
                if let Some(guard_id) = self.squads.values().find(|s| s.owner == defender && s.plot == c.plot).map(|s| s.id) {
                    let r_id = raiders[0];
                    let should_sally = if c.sally == SallyPosture::Always {
                        true
                    } else {
                        self.squads[&guard_id].power_rating() * 100 >= self.squads[&r_id].power_rating() * SALLY_FAVOURED_PCT
                    };
                    if should_sally {
                        sallied = true;
                        let r_plot = self.squads[&r_id].plot;
                        self.resolve_engagement(guard_id, Some(r_id), Some(cid), r_plot, true, events);
                    }
                }
            }
            // Surviving raiders siphon 50% of city Gold+Industry (10% if contested by a sally)!
            let surviving: Vec<SquadId> = raiders.into_iter().filter(|id| self.squads.contains_key(id)).collect();
            if !surviving.is_empty() {
                let (_, ind_calc, gold, _, _, _) = self.city_yields(cid);
                let pct = if sallied { RAID_TAKE_SALLIED_PCT } else { RAID_TAKE_PCT };
                let base_take = ((gold + ind_calc.total) * pct / 100).max(3);
                let per_raider = (base_take / surviving.len() as i32).max(2);
                for rid in &surviving {
                    let ro = self.squads[rid].owner;
                    let mut bonus = 100;
                    if self.players[ro.idx()].faction == FactionId::Hippus {
                        bonus += 50;
                    }
                    bonus += self.players[ro.idx()].secret_sum(|e| match e {
                        SecretEffect::RaidBonusPct(n) => n,
                        _ => 0,
                    });
                    let plunder = per_raider * bonus / 100;
                    self.players[ro.idx()].gold += plunder;
                    self.players[ro.idx()].renown_meters[GreatSuit::Merchant.id()] += renown::RAID_MERCHANT;
                }
                if let Some(cm) = self.cities.get_mut(&cid) {
                    cm.raid_streak += 1;
                    if cm.raid_streak >= RAID_STREAK_TO_STRIP {
                        cm.stripped_turns = STRIP_TURNS;
                        cm.raid_streak = 0;
                        self.push_log(Some(defender), Some(c.plot), None, format!("Raiders stripped the countryside of {} for {} turns.", c.name, STRIP_TURNS));
                    }
                }
            }
        }
    }

    fn ensure_houses_for(&mut self, player: PlayerId) {
        if self.players[player.idx()].is_neutral {
            return;
        }
        let unhoused: Vec<CityId> = self.cities.values().filter(|c| c.owner == player && c.house.is_none() && !c.is_outpost).map(|c| c.id).collect();
        for cid in unhoused {
            let origin = self.cities[&cid].original_owner;
            // Try joining an existing House of the same origin with room.
            let existing = self
                .houses
                .values()
                .find(|h| h.owner == player && h.origin == origin && h.cities.len() < HOUSE_MAX_CITIES)
                .map(|h| h.id);
            if let Some(hid) = existing {
                self.houses.get_mut(&hid).unwrap().cities.push(cid);
                self.cities.get_mut(&cid).unwrap().house = Some(hid);
            } else {
                let ch = self.alloc_character();
                let hid = HouseId(self.next_house_id);
                self.next_house_id += 1;
                let perk = HousePerk::ALL[self.rng.range_usize(HousePerk::ALL.len())];
                let sympathy = if origin != player && !self.players[origin.idx()].is_neutral { Some(origin) } else { None };
                let surname = ch.name.split_whitespace().last().unwrap_or(ch.name);
                let house = House {
                    id: hid,
                    owner: player,
                    origin,
                    name: format!("House {}", surname),
                    lord_name: ch.name.to_string(),
                    lord_portrait: ch.portrait_key,
                    alignment: if origin == player { self.players[player.idx()].alignment } else { ch.alignment },
                    religion: self.players[player.idx()].religion,
                    sympathy,
                    perk,
                    cities: vec![cid],
                    bonus_loyalty: 0,
                };
                self.houses.insert(hid, house);
                self.cities.get_mut(&cid).unwrap().house = Some(hid);
            }
        }
    }

    pub fn update_city_sieges(&mut self) {
        let city_ids: Vec<CityId> = self.cities.keys().copied().collect();
        for cid in city_ids {
            let (owner, plot) = (self.cities[&cid].owner, self.cities[&cid].plot);
            let adj_enemy = self.squads.values().any(|s| self.at_war(owner, s.owner) && self.world.distance(s.plot, plot) == 1);
            if let Some(c) = self.cities.get_mut(&cid) {
                c.besieged = adj_enemy;
            }
        }
    }

    pub fn recompute_vision(&mut self, player: PlayerId) {
        if player.idx() >= self.world.vis.len() {
            return;
        }
        self.world.clear_active_vision(player);
        let sight_bonus = if self.players[player.idx()].has_tech(crate::data::TECH_TRACKING) { 1 } else { 0 };
        let city_plots: Vec<PlotIdx> = self.cities.values().filter(|c| c.owner == player).map(|c| c.plot).collect();
        for p in city_plots {
            self.world.reveal_radius(player, p, 3);
        }
        let squad_sights: Vec<(PlotIdx, i32)> = self.squads.values().filter(|s| s.owner == player).map(|s| (s.plot, s.sight_range() + sight_bonus)).collect();
        for (p, r) in squad_sights {
            self.world.reveal_radius(player, p, r);
        }
        let car_plots: Vec<PlotIdx> = self.caravans.values().filter(|c| c.owner == player).filter_map(CaravanRoute::current_plot).collect();
        for p in car_plots {
            self.world.reveal_radius(player, p, 1);
        }
    }

    pub fn recompute_scores(&mut self) {
        let mut raw = vec![0i32; self.players.len()];
        let mut total = 0i32;
        for p in 0..self.players.len() {
            if self.players[p].is_neutral || !self.players[p].alive {
                continue;
            }
            let pid = PlayerId(p as u16);
            let pop: i32 = self.cities.values().filter(|c| c.owner == pid).map(|c| c.size * SCORE_PER_SIZE + c.districts.len() as i32 * SCORE_PER_DISTRICT).sum();
            let tech = self.players[p].techs.len() as i32 * SCORE_PER_TECH + self.players[p].secrets.len() as i32 * SCORE_PER_SECRET;
            let army: i32 = self.squads.values().filter(|s| s.owner == pid).map(|s| s.members.len() as i32 * SCORE_PER_UNIT).sum();
            let s = (pop + tech + army).max(1);
            raw[p] = s;
            total += s;
        }
        total = total.max(1);
        for p in 0..self.players.len() {
            self.players[p].score = raw[p] * 10000 / total;
        }
    }

    fn find_wild_land_plot(&mut self) -> Option<PlotIdx> {
        let mut cands: Vec<PlotIdx> = (0..self.world.len())
            .map(|i| PlotIdx(i as u32))
            .filter(|&p| {
                let pl = self.world.plot(p);
                pl.kind.is_passable_land() && pl.owner.is_none() && pl.city.is_none() && self.squads_at(p).is_empty()
            })
            .collect();
        if cands.is_empty() {
            cands = (0..self.world.len())
                .map(|i| PlotIdx(i as u32))
                .filter(|&p| self.world.plot(p).kind.is_passable_land() && self.world.plot(p).city.is_none())
                .collect();
        }
        if cands.is_empty() {
            None
        } else {
            Some(cands[self.rng.range_usize(cands.len())])
        }
    }

    /// Strict simulation invariant checker used by unit and AI soak tests.
    pub fn check_invariants(&self) -> Result<(), String> {
        if self.apocalypse.counter < 0 || self.apocalypse.counter > 100 {
            return Err(format!("Armageddon counter out of bounds: {}", self.apocalypse.counter));
        }
        // A crisis marked as triggered has taken effect: the Blowout raised Caldera Keep and started
        // the countdown (every change of Strain goes through `shift_ac`).
        if self.apocalypse.triggered_crises.contains(&crate::apocalypse::MAX_STRAIN) != self.apocalypse.carcosa_risen {
            return Err(format!(
                "the Blowout crisis is {} but Caldera Keep has {}",
                if self.apocalypse.carcosa_risen { "not marked" } else { "marked as triggered" },
                if self.apocalypse.carcosa_risen { "risen" } else { "not risen" }
            ));
        }
        for (cid, c) in &self.cities {
            if c.id != *cid {
                return Err(format!("city id mismatch: {cid:?}"));
            }
            if self.world.plot(c.plot).city != Some(*cid) {
                return Err(format!("city {cid:?} missing on plot {:?}", c.plot));
            }
            if c.size < 1 || c.size > 10 {
                return Err(format!("city {cid:?} invalid size {}", c.size));
            }
        }
        for (sid, s) in &self.squads {
            if s.members.is_empty() {
                return Err(format!("empty squad {sid:?}"));
            }
            if !self.world.plot(s.plot).kind.is_passable_land() {
                return Err(format!("squad {sid:?} on impassable plot {:?}", s.plot));
            }
            for m in &s.members {
                if m.row >= 3 || m.lane >= 3 || (m.class.info().large && m.lane >= 2) {
                    return Err(format!("squad {sid:?} member {:?} out of grid bounds ({}, {})", m.id, m.row, m.lane));
                }
                if m.hp_centi <= 0 {
                    return Err(format!("squad {sid:?} member {:?} dead but still in squad", m.id));
                }
            }
        }
        for (hid, h) in &self.houses {
            if h.cities.is_empty() {
                return Err(format!("empty house {hid:?}"));
            }
            for &cid in &h.cities {
                if self.cities.get(&cid).is_none_or(|c| c.owner != h.owner) {
                    return Err(format!("house {hid:?} references invalid city {cid:?}"));
                }
            }
        }
        Ok(())
    }
}

fn ordinal(n: u32) -> String {
    let suf = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}
