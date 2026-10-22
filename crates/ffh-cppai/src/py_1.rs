//! Port of the FFH2 0.41 Python AI hooks `CvGameUtils.AI_chooseTech` (CvGameUtils.py:594-1239)
//! and `CvGameUtils.AI_chooseProduction` (CvGameUtils.py:1240-2956).
//!
//! Python semantics kept: `gc.getInfoTypeForString` names are resolved with `d.t` (all of them
//! exist in the data except `BUILDINGCLASS_MACHINISTS_SHOP` and `UNITCLASS_EXPLORER` in the
//! production list, whose values are never set, so they can never be chosen); Python 2 integer
//! `/` is `py_div`; `AI_chooseProduction`'s values become Python floats once `*= 1.2` touches
//! them, so the value list is kept as `f64` (exact for the integers it otherwise holds).
#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]
use crate::prelude::*;
use ffh_core::ffh::game_utils::py_div;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// `sProd` / `iProdType` of `AI_chooseProduction` (0 = building, 1 = building class, 2 = unit
/// class, 3 = project), in the Python's order (the best item is the first strict maximum).
const PROD: &[(&str, u8)] = &[
    ("PROJECT_BLOOD_OF_THE_PHOENIX", 3),
    ("PROJECT_GENESIS", 3),
    ("PROJECT_RITES_OF_OGHMA", 3),
    ("PROJECT_NATURES_REVOLT", 3),
    ("PROJECT_ELEGY_OF_THE_SHEAIM", 3),
    ("PROJECT_HALLOWING_OF_THE_ELOHIM", 3),
    ("PROJECT_BIRTHRIGHT_REGAINED", 3),
    ("PROJECT_PURGE_THE_UNFAITHFUL", 3),
    ("PROJECT_SAMHAIN", 3),
    ("PROJECT_THE_WHITE_HAND", 3),
    ("PROJECT_THE_DEEPENING", 3),
    ("PROJECT_STIR_FROM_SLUMBER", 3),
    ("PROJECT_THE_DRAW", 3),
    ("PROJECT_ASCENSION", 3),
    ("PROJECT_PACT_OF_THE_NILHORN", 3),
    ("UNITCLASS_SETTLER", 2),
    ("UNITCLASS_WORKER", 2),
    ("UNITCLASS_WARRIOR", 2),
    ("UNITCLASS_WORKBOAT", 2),
    ("UNITCLASS_SCOUT", 2),
    ("UNITCLASS_ADEPT", 2),
    ("UNITCLASS_AXEMAN", 2),
    ("UNITCLASS_HUNTER", 2),
    ("UNITCLASS_ARCHER", 2),
    ("UNITCLASS_HORSEMAN", 2),
    ("UNITCLASS_HORSE_ARCHER", 2),
    ("UNITCLASS_FREAK", 2),
    ("UNITCLASS_CATAPULT", 2),
    ("UNITCLASS_CANNON", 2),
    ("UNITCLASS_MONK", 2),
    ("UNITCLASS_GRIGORI_MEDIC", 2),
    ("UNITCLASS_PRIEST_OF_THE_VEIL", 2),
    ("UNITCLASS_PRIEST_OF_THE_ORDER", 2),
    ("UNITCLASS_PRIEST_OF_KILMORPH", 2),
    ("UNITCLASS_PRIEST_OF_LEAVES", 2),
    ("UNITCLASS_PRIEST_OF_THE_EMPYREAN", 2),
    ("UNITCLASS_PRIEST_OF_THE_OVERLORDS", 2),
    ("UNITCLASS_DISCIPLE_THE_ASHEN_VEIL", 2),
    ("UNITCLASS_DISCIPLE_THE_ORDER", 2),
    ("UNITCLASS_DISCIPLE_RUNES_OF_KILMORPH", 2),
    ("UNITCLASS_DISCIPLE_FELLOWSHIP_OF_LEAVES", 2),
    ("UNITCLASS_DISCIPLE_EMPYREAN", 2),
    ("UNITCLASS_DISCIPLE_OCTOPUS_OVERLORDS", 2),
    ("UNITCLASS_CHAMPION", 2),
    ("UNITCLASS_CHARIOT", 2),
    ("UNITCLASS_RANGER", 2),
    ("UNITCLASS_LONGBOWMAN", 2),
    ("UNITCLASS_ASSASSIN", 2),
    ("UNITCLASS_GOVANNON", 2),
    ("UNITCLASS_LOKI", 2),
    ("UNITCLASS_DONAL", 2),
    ("UNITCLASS_RANTINE", 2),
    ("UNITCLASS_LOSHA", 2),
    ("UNITCLASS_GALLEY", 2),
    ("UNITCLASS_CARAVEL", 2),
    ("UNITCLASS_EXPLORER", 2),
    ("UNITCLASS_QUEEN_OF_THE_LINE", 2),
    ("UNITCLASS_TRIREME", 2),
    ("UNITCLASS_FRIGATE", 2),
    ("UNITCLASS_MAN_O_WAR", 2),
    ("BUILDINGCLASS_ALCHEMY_LAB", 1),
    ("BUILDINGCLASS_AQUEDUCT", 1),
    ("BUILDINGCLASS_TRAINING_YARD", 1),
    ("BUILDINGCLASS_BASILICA", 1),
    ("BUILDINGCLASS_BREWERY", 1),
    ("BUILDINGCLASS_CARNIVAL", 1),
    ("BUILDINGCLASS_COURTHOUSE", 1),
    ("BUILDINGCLASS_DUNGEON", 1),
    ("BUILDINGCLASS_ELDER_COUNCIL", 1),
    ("BUILDINGCLASS_ARCHERY_RANGE", 1),
    ("BUILDINGCLASS_FORGE", 1),
    ("BUILDINGCLASS_GAMBLING_HOUSE", 1),
    ("BUILDINGCLASS_GRANARY", 1),
    ("BUILDINGCLASS_GROVE", 1),
    ("BUILDINGCLASS_HARBOR", 1),
    ("BUILDINGCLASS_HERBALIST", 1),
    ("BUILDINGCLASS_HUNTING_LODGE", 1),
    ("BUILDINGCLASS_INFIRMARY", 1),
    ("BUILDINGCLASS_INN", 1),
    ("BUILDINGCLASS_LIBRARY", 1),
    ("BUILDINGCLASS_LIGHTHOUSE", 1),
    ("BUILDINGCLASS_MACHINISTS_SHOP", 1),
    ("BUILDINGCLASS_MAGE_GUILD", 1),
    ("BUILDINGCLASS_MARKET", 1),
    ("BUILDINGCLASS_MONEYCHANGER", 1),
    ("BUILDINGCLASS_MONUMENT", 1),
    ("BUILDINGCLASS_PAGAN_TEMPLE", 1),
    ("BUILDINGCLASS_PALISADE", 1),
    ("BUILDINGCLASS_PUBLIC_BATHS", 1),
    ("BUILDINGCLASS_SIEGE_WORKSHOP", 1),
    ("BUILDINGCLASS_SMOKEHOUSE", 1),
    ("BUILDINGCLASS_STABLE", 1),
    ("BUILDINGCLASS_TAX_OFFICE", 1),
    ("BUILDINGCLASS_TAVERN", 1),
    ("BUILDINGCLASS_TEMPLE_OF_KILMORPH", 1),
    ("BUILDINGCLASS_TEMPLE_OF_LEAVES", 1),
    ("BUILDINGCLASS_TEMPLE_OF_THE_EMPYREAN", 1),
    ("BUILDINGCLASS_TEMPLE_OF_THE_OVERLORDS", 1),
    ("BUILDINGCLASS_TEMPLE_OF_THE_VEIL", 1),
    ("BUILDINGCLASS_TEMPLE_OF_THE_ORDER", 1),
    ("BUILDINGCLASS_THEATRE", 1),
    ("BUILDINGCLASS_WALLS", 1),
    ("BUILDINGCLASS_TOWER_OF_ALTERATION", 1),
    ("BUILDINGCLASS_TOWER_OF_DIVINATION", 1),
    ("BUILDINGCLASS_TOWER_OF_NECROMANCY", 1),
    ("BUILDINGCLASS_TOWER_OF_THE_ELEMENTS", 1),
    ("BUILDINGCLASS_TOWER_OF_MASTERY", 1),
    ("BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_FINAL", 1),
    ("BUILDINGCLASS_GUILD_OF_THE_NINE", 1),
    ("BUILDINGCLASS_HERON_THRONE", 1),
    ("BUILDINGCLASS_CITY_OF_A_THOUSAND_SLUMS", 1),
    ("BUILDINGCLASS_THEATRE_OF_DREAMS", 1),
    ("BUILDINGCLASS_PILLAR_OF_CHAINS", 1),
    ("BUILDINGCLASS_GUILD_OF_HAMMERS", 1),
    ("BUILDINGCLASS_GRAND_MENAGERIE", 1),
    ("BUILDINGCLASS_MERCURIAN_GATE", 1),
    ("BUILDINGCLASS_CELESTIAL_COMPASS", 1),
    ("BUILDINGCLASS_RIDE_OF_THE_NINE_KINGS", 1),
    ("BUILDINGCLASS_MOKKAS_CAULDRON", 1),
    ("BUILDINGCLASS_SHRINE_OF_SIRONA", 1),
    ("BUILDINGCLASS_EYES_AND_EARS_NETWORK", 1),
    ("BUILDINGCLASS_SHRINE_OF_THE_CHAMPION", 1),
    ("BUILDINGCLASS_AQUAE_SUCELLUS", 1),
    ("BUILDINGCLASS_HALL_OF_KINGS", 1),
    ("BUILDINGCLASS_GREAT_LIBRARY", 1),
    ("BUILDINGCLASS_GREAT_LIGHTHOUSE", 1),
    ("BUILDINGCLASS_PROPHECY_OF_RAGNAROK", 1),
    ("BUILDINGCLASS_TEMPLE_OF_TEMPORENCE", 1),
    ("BUILDINGCLASS_HEROIC_EPIC", 1),
    ("BUILDINGCLASS_NATIONAL_EPIC", 1),
    ("BUILDINGCLASS_TOWER_OF_COMPLACENCY", 1),
    ("BUILDINGCLASS_TOWER_OF_EYES", 1),
    ("BUILDINGCLASS_SYLIVENS_PERFECT_LYRE", 1),
    ("BUILDINGCLASS_CROWN_OF_AKHARIEN", 1),
    ("BUILDINGCLASS_FORBIDDEN_PALACE", 1),
    ("BUILDINGCLASS_CATACOMB_LIBRALUS", 1),
    ("BUILDINGCLASS_FORM_OF_THE_TITAN", 1),
    ("BUILDINGCLASS_MINES_OF_GALDUR", 1),
    ("BUILDINGCLASS_BAZAAR_OF_MAMMON", 1),
    ("BUILDING_ADULARIA_CHAMBER", 0),
    ("BUILDING_ADVENTURERS_GUILD", 0),
    ("BUILDING_BLASTING_WORKSHOP", 0),
    ("BUILDING_BREEDING_PIT", 0),
    ("BUILDING_CAVE_OF_ANCESTORS", 0),
    ("BUILDING_CHANCEL_OF_GUARDIANS", 0),
    ("BUILDING_CITADEL_OF_LIGHT", 0),
    ("BUILDING_COMMAND_POST", 0),
    ("BUILDING_HALL_OF_MIRRORS", 0),
    ("BUILDING_JEWELER", 0),
    ("BUILDING_PALLENS_ENGINE", 0),
    ("BUILDING_PLANAR_GATE", 0),
    ("BUILDING_RELIQUARY", 0),
    ("BUILDING_SMUGGLERS_PORT", 0),
    ("BUILDING_TAILOR", 0),
    ("BUILDING_WARRENS", 0),
    ("BUILDING_MONUMENT", 0),
];

/// Python `sProd.index(name)`.
fn ix(name: &str) -> usize {
    static MAP: OnceLock<BTreeMap<&'static str, usize>> = OnceLock::new();
    let m = MAP.get_or_init(|| {
        let mut m = BTreeMap::new();
        for (i, (n, _)) in PROD.iter().enumerate() {
            m.entry(*n).or_insert(i);
        }
        m
    });
    *m.get(name).unwrap_or_else(|| panic!("AI_chooseProduction: {name} is not in sProd"))
}

/// The conquest-mode unit counts of the civilization blocks of `AI_chooseProduction`
/// (`pPlayer.getConquestUnitClassCount(class, pCity.area())`, BETTER AI).
struct Conquest {
    mages: i32,
    melee: i32,
    archers: i32,
    recon: i32,
    siege: i32,
    mounted: i32,
    priests: i32,
}

impl Conquest {
    /// The standard counts (`totalpriests` = the state religion's priests, `sReli`).
    fn new(g: &Game, p: PlayerId, area: AreaId, reli: Option<&str>) -> Conquest {
        let cc = |n: &str| g.player_get_conquest_unit_class_count(p, g.d.t::<UnitClassType>(n), area);
        Conquest {
            mages: cc("UNITCLASS_ADEPT") + cc("UNITCLASS_MAGE"),
            melee: cc("UNITCLASS_AXEMAN") + cc("UNITCLASS_CHAMPION"),
            archers: cc("UNITCLASS_LONGBOWMAN") + cc("UNITCLASS_ARCHER"),
            recon: cc("UNITCLASS_HUNTER") + cc("UNITCLASS_RANGER"),
            siege: cc("UNITCLASS_CATAPULT"),
            mounted: cc("UNITCLASS_HORSEMAN") + cc("UNITCLASS_HORSE_ARCHER") + cc("UNITCLASS_CHARIOT"),
            priests: match reli {
                Some(r) => cc(r),
                None => 0,
            },
        }
    }

    /// `totalunits` of the blocks that count every group.
    fn units(&self) -> i32 {
        self.mages + self.melee + self.archers + self.recon + self.siege + self.mounted + self.priests
    }
}

impl Cai {
    /// Python `CvGameUtils.AI_chooseTech(argsList = (ePlayer, bFree))` (CvGameUtils.py:594):
    /// the tech to research, or `None` (Python -1: the DLL then uses `AI_bestTech`).
    pub fn py_ai_choose_tech(&self, g: &Game, p: PlayerId, free: bool) -> Option<TechType> {
        let d = g.d;
        let player = &g.players[p];
        let team = player.team;
        let tn = |n: &str| d.t::<TechType>(n);
        let has = |n: &str| g.team_is_has_tech(team, Some(tn(n)));
        let has_t = |t: Option<TechType>| g.team_is_has_tech(team, t);
        // CvPlayer::canResearch(NO_TECH) is false (isHasTech(NO_TECH) is true).
        let can_research = |t: Option<TechType>| t.map_or(false, |t| g.player_can_research(p, t, false));
        let city = g.player_get_capital_city(p);
        let civ = g.player_get_civilization_type(p);
        let is_civ = |n: &str| civ == Some(d.t::<CivilizationType>(n));
        let fav_rel = player.favorite_religion;
        let has_trait = |n: &str| player.traits[d.t::<TraitType>(n)];
        let Some(leader_type) = g.player_get_leader_type(p) else {
            // PORT ADAPT: every player has a leader; gc.getLeaderHeadInfo(-1) would raise.
            return None;
        };
        let leader = &d.leader_heads[leader_type];
        let mut tech: Option<TechType> = None;

        let mut tier2 = false;
        let mut tier3 = false;
        let mut econ1 = false;
        let mut econ2 = false;

        if has("TECH_BRONZE_WORKING") {
            tier2 = true;
        }
        if has("TECH_ARCHERY") {
            tier2 = true;
        }
        if has("TECH_HUNTING") {
            tier2 = true;
        }
        if has("TECH_HORSEBACK_RIDING") {
            tier2 = true;
        }
        if is_civ("CIVILIZATION_LUCHUIRP") {
            if has("TECH_CONSTRUCTION") {
                tier2 = true;
            }
        }

        if has("TECH_IRON_WORKING") {
            tier3 = true;
        }
        if has("TECH_BOWYERS") {
            tier3 = true;
        }
        if has("TECH_STIRRUPS") {
            tier3 = true;
        }
        if has("TECH_ANIMAL_HANDLING") {
            tier3 = true;
        }

        if has("TECH_MINING") && has("TECH_CALENDAR") && has("TECH_EDUCATION") && has("TECH_EXPLORATION") {
            econ1 = true;
        }
        let mut count_econ_techs = 0;
        for n in ["TECH_TRADE", "TECH_SANITATION", "TECH_CURRENCY", "TECH_CONSTRUCTION", "TECH_TAXATION"] {
            if has(n) {
                count_econ_techs += 1;
            }
        }
        let mut econval = leader.economy_tech_value;
        if econval > 130 {
            econval = 130;
        }
        if (count_econ_techs * 95) + 150 > 4 * econval {
            econ2 = true;
        }

        if tier3 && econ2 {
            if has("TECH_SORCERY") {
                let mut counttowers = 0;
                for n in [
                    "BUILDINGCLASS_TOWER_OF_ALTERATION",
                    "BUILDINGCLASS_TOWER_OF_DIVINATION",
                    "BUILDINGCLASS_TOWER_OF_NECROMANCY",
                    "BUILDINGCLASS_TOWER_OF_THE_ELEMENTS",
                ] {
                    if player.building_class_count[d.t::<BuildingClassType>(n)] > 0 {
                        counttowers += 1;
                    }
                }
                // FIX: the Python returns Strength of Will even when the team already has it;
                // the DLL's pushResearch then fails and research stalls for good (the Python
                // answers the same every turn). Fall through to the other beelines instead.
                if counttowers > 2 && !has("TECH_STRENGTH_OF_WILL") {
                    return Some(tn("TECH_STRENGTH_OF_WILL"));
                }
            }

            // Altar Victory
            if leader.altar_victory {
                if has("TECH_RELIGIOUS_LAW") {
                    if !has("TECH_OMNISCIENCE") {
                        tech = Some(tn("TECH_OMNISCIENCE"));
                    }
                }
                if !has("TECH_RELIGIOUS_LAW") {
                    tech = Some(tn("TECH_RELIGIOUS_LAW"));
                }
            }

            if leader.culture_victory {
                if !has("TECH_MERCANTILISM") {
                    tech = Some(tn("TECH_MERCANTILISM"));
                }
            }
            // Magic Techs
            for n in ["TECH_ALTERATION", "TECH_DIVINATION", "TECH_ELEMENTALISM", "TECH_NECROMANCY"] {
                if !has(n) {
                    tech = Some(tn(n));
                }
            }

            // Late Wonder Beeline
            if let Some(wonder) = leader.favorite_late_wonder {
                // Python: gc.getBuildingInfo(-1) is None, so `.getPrereqAndTech()` raises and the
                // DLL keeps lResult = -1 (NO_TECH).
                let Some(temp) = civ.and_then(|civ| d.civ_building(civ, wonder)) else {
                    return None;
                };
                let newtech = d.buildings[temp].prereq_and_tech;
                if newtech.is_some() {
                    if !has_t(newtech) {
                        if can_research(newtech) {
                            tech = newtech;
                        }
                    }
                }
            }

            // Religion Heroes Beeline
            if let Some(rel) = fav_rel {
                if let Some(hero) = d.religions[rel].religion_hero2 {
                    // Python: gc.getUnitInfo(-1) is None and raises (DLL: NO_TECH).
                    let Some(t_unit) = d.unit_classes[hero].default_unit_index else {
                        return None;
                    };
                    let pre = d.units[t_unit].prereq_and_tech;
                    if pre.is_some() {
                        if !has_t(pre) {
                            if can_research(pre) {
                                tech = pre;
                            }
                        }
                    }
                }
            }

            if let Some(rel) = fav_rel {
                if let Some(hero) = d.religions[rel].religion_hero1 {
                    let Some(t_unit) = d.unit_classes[hero].default_unit_index else {
                        return None;
                    };
                    let pre = d.units[t_unit].prereq_and_tech;
                    if pre.is_some() {
                        if !has_t(pre) {
                            if can_research(pre) {
                                tech = pre;
                            }
                        }
                    }
                }
            }

            if let Some(rel) = fav_rel {
                if rel == d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES") {
                    if can_research(Some(tn("TECH_PRIESTHOOD"))) {
                        tech = Some(tn("TECH_PRIESTHOOD"));
                    }
                }
            }

            // Advanced Economic techs in case we haven't picked them up yet
            for n in ["TECH_TAXATION", "TECH_CURRENCY", "TECH_CONSTRUCTION"] {
                if !has(n) {
                    tech = Some(tn(n));
                }
            }

            // resource techs
            for n in ["TECH_HUNTING", "TECH_ANIMAL_HUSBANDRY"] {
                if !has(n) {
                    tech = Some(tn(n));
                }
            }
        }

        // more advanced beelines
        if tier2 && econ2 {
            // Civ Specific
            if is_civ("CIVILIZATION_AMURITES") {
                if !has("TECH_SORCERY") {
                    tech = Some(tn("TECH_SORCERY"));
                }
                if !has("TECH_BOWYERS") {
                    tech = Some(tn("TECH_BOWYERS"));
                }
            }

            if is_civ("CIVILIZATION_BALSERAPHS") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                }
            }

            if is_civ("CIVILIZATION_BANNOR") {
                if !has("TECH_FANATICISM") {
                    tech = Some(tn("TECH_FANATICISM"));
                }
            }

            if is_civ("CIVILIZATION_CALABIM") {
                if !has("TECH_FEUDALISM") {
                    tech = Some(tn("TECH_FEUDALISM"));
                }
            }

            if is_civ("CIVILIZATION_CLAN_OF_EMBERS") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                }
            }

            if is_civ("CIVILIZATION_DOVIELLO") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                }
            }

            if is_civ("CIVILIZATION_ELOHIM") {
                if !has("TECH_PRIESTHOOD") {
                    tech = Some(tn("TECH_PRIESTHOOD"));
                }
            }

            if is_civ("CIVILIZATION_GRIGORI") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                } else if !has("TECH_MEDICINE") {
                    tech = Some(tn("TECH_MEDICINE"));
                }
            }

            if is_civ("CIVILIZATION_HIPPUS") {
                if !has("TECH_STIRRUPS") {
                    tech = Some(tn("TECH_STIRRUPS"));
                }
            }

            if is_civ("CIVILIZATION_ILLIANS") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                }
            }

            if is_civ("CIVILIZATION_LUCHUIRP") {
                if has("TECH_TRADE") {
                    if !has("TECH_ELEMENTALISM") {
                        tech = Some(tn("TECH_ELEMENTALISM"));
                    } else if !has("TECH_SORCERY") {
                        tech = Some(tn("TECH_SORCERY"));
                    }
                }
            }
            if is_civ("CIVILIZATION_KHAZAD") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                }
            }

            if is_civ("CIVILIZATION_KURIOTATES") {
                if !has("TECH_STIRRUPS") {
                    tech = Some(tn("TECH_STIRRUPS"));
                }
            }

            if is_civ("CIVILIZATION_LANUN") {
                if !has("TECH_IRON_WORKING") {
                    tech = Some(tn("TECH_IRON_WORKING"));
                }
            }

            if is_civ("CIVILIZATION_LJOSALFAR") {
                if !has("TECH_BOWYERS") {
                    tech = Some(tn("TECH_BOWYERS"));
                }
            }

            if is_civ("CIVILIZATION_MALAKIM") {
                if !has("TECH_PRIESTHOOD") {
                    tech = Some(tn("TECH_PRIESTHOOD"));
                }
            }

            if is_civ("CIVILIZATION_SHEAIM") {
                if !has("TECH_SORCERY") {
                    tech = Some(tn("TECH_SORCERY"));
                }
                if !has("TECH_NECROMANCY") {
                    tech = Some(tn("TECH_NECROMANCY"));
                }
                if !has("TECH_CORRUPTION_OF_SPIRIT") {
                    tech = Some(tn("TECH_CORRUPTION_OF_SPIRIT"));
                }
            }

            if is_civ("CIVILIZATION_SIDAR") {
                if !has("TECH_POISONS") {
                    tech = Some(tn("TECH_POISONS"));
                } else if !has("TECH_ANIMAL_HANDLING") {
                    tech = Some(tn("TECH_ANIMAL_HANDLING"));
                }
            }

            if is_civ("CIVILIZATION_SVARTALFAR") {
                if !has("TECH_POISONS") {
                    tech = Some(tn("TECH_POISONS"));
                } else if !has("TECH_ANIMAL_HANDLING") {
                    tech = Some(tn("TECH_ANIMAL_HANDLING"));
                }
            }

            // Advanced Religion Beelines
            if let Some(rel) = fav_rel {
                let t2 = d.religions[rel].religion_tech2;
                if t2.is_some() {
                    if !has_t(t2) {
                        if can_research(t2) {
                            tech = t2;
                        }
                    }
                }
                let t1 = d.religions[rel].religion_tech1;
                if t1.is_some() {
                    if !has_t(t1) {
                        if can_research(t1) {
                            tech = t1;
                        }
                    }
                }
            }
        }

        //----------------------------
        // Establish a Good Economy
        //----------------------------
        if tier2 && !econ2 {
            for n in ["TECH_TAXATION", "TECH_CURRENCY", "TECH_CONSTRUCTION", "TECH_SANITATION", "TECH_TRADE"] {
                if !has(n) {
                    tech = Some(tn(n));
                }
            }
        }

        //----------------------------
        // Teching with early economy and military
        //----------------------------
        if tier2 && econ1 {
            // Early Wonder Beeline
            if let Some(wonder) = leader.favorite_early_wonder {
                // Python: gc.getBuildingInfo(-1) is None and raises (DLL: NO_TECH).
                let Some(temp) = civ.and_then(|civ| d.civ_building(civ, wonder)) else {
                    return None;
                };
                let newtech = d.buildings[temp].prereq_and_tech;
                if newtech.is_some() {
                    if !has_t(newtech) {
                        // C++ quirk: the Python tests canResearch(iTech) (the current pick),
                        // not canResearch(newtech).
                        if can_research(tech) {
                            tech = newtech;
                        }
                    }
                }
            }

            // Early Leader Beelines
            for fav in [leader.favorite_early_tech3, leader.favorite_early_tech2, leader.favorite_early_tech1] {
                if fav.is_some() {
                    if !has_t(fav) {
                        if can_research(fav) {
                            tech = fav;
                        }
                    }
                }
            }

            // Early Civ Beelines
            if is_civ("CIVILIZATION_CALABIM") {
                if has("TECH_EDUCATION") {
                    if !has("TECH_CODE_OF_LAWS") {
                        tech = Some(tn("TECH_CODE_OF_LAWS"));
                    }
                }
            }

            if is_civ("CIVILIZATION_CLAN_OF_EMBERS") {
                if player.num_cities() >= 3 {
                    if !has("TECH_MASONRY") {
                        tech = Some(tn("TECH_MASONRY"));
                    }
                }
            }

            if is_civ("CIVILIZATION_HIPPUS") {
                if has("TECH_EDUCATION") {
                    if !has("TECH_HORSEBACK_RIDING") {
                        tech = Some(tn("TECH_HORSEBACK_RIDING"));
                    } else if has("TECH_IRON_WORKING") {
                        if econ2 {
                            if !has("TECH_WARHORSES") {
                                tech = Some(tn("TECH_WARHORSES"));
                            }
                        }
                    }
                }
            }

            if is_civ("CIVILIZATION_LJOSALFAR") || is_civ("CIVILIZATION_SVARTALFAR") {
                if has("TECH_WAY_OF_THE_FORESTS") {
                    if !has("TECH_HIDDEN_PATHS") {
                        tech = Some(tn("TECH_HIDDEN_PATHS"));
                    }
                }
            }

            if is_civ("CIVILIZATION_LUCHUIRP") {
                if has("TECH_ANCIENT_CHANTS") {
                    if !has("TECH_CONSTRUCTION") {
                        tech = Some(tn("TECH_CONSTRUCTION"));
                    }
                }
            }

            if is_civ("CIVILIZATION_SHEAIM") {
                if !has("TECH_BRONZE_WORKING") {
                    tech = Some(tn("TECH_BRONZE_WORKING"));
                }
            }

            // Early Techs
            if !has("TECH_SAILING") {
                if g.player_count_num_coastal_cities(p) > 0 {
                    tech = Some(tn("TECH_SAILING"));
                }
            }
            if !has("TECH_ARCHERY") {
                tech = Some(tn("TECH_ARCHERY"));
            }
            if player.num_cities() >= 6 && !has("TECH_CODE_OF_LAWS") {
                if has_trait("TRAIT_FINANCIAL") {
                    tech = Some(tn("TECH_CODE_OF_LAWS"));
                }
            }

            if !has("TECH_ANIMAL_HUSBANDRY") {
                tech = Some(tn("TECH_ANIMAL_HUSBANDRY"));
            }

            if !has_trait("TRAIT_BARBARIAN") {
                if !has("TECH_WRITING") {
                    tech = Some(tn("TECH_WRITING"));
                }
            }
            for n in ["TECH_MYSTICISM", "TECH_FESTIVALS", "TECH_CARTOGRAPHY"] {
                if !has(n) {
                    tech = Some(tn(n));
                }
            }

            // Early Religion Beeline
            if !has_trait("TRAIT_AGNOSTIC") {
                let mut countreli = 0;
                for n in ["TECH_WAY_OF_THE_FORESTS", "TECH_WAY_OF_THE_EARTHMOTHER", "TECH_MESSAGE_FROM_THE_DEEP"] {
                    if has(n) {
                        countreli += 1;
                    }
                }
                if countreli == 0 {
                    let alignment = player.alignment;
                    if alignment == Some(d.t::<AlignmentType>("ALIGNMENT_GOOD")) {
                        tech = Some(tn("TECH_WAY_OF_THE_EARTHMOTHER"));
                    } else if alignment == Some(d.t::<AlignmentType>("ALIGNMENT_NEUTRAL")) {
                        tech = Some(tn("TECH_WAY_OF_THE_FORESTS"));
                    } else if alignment == Some(d.t::<AlignmentType>("ALIGNMENT_EVIL")) {
                        tech = Some(tn("TECH_MESSAGE_FROM_THE_DEEP"));
                    }

                    if let Some(rel) = fav_rel {
                        if rel == d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES") {
                            tech = Some(tn("TECH_WAY_OF_THE_FORESTS"));
                        }
                        if rel == d.t::<ReligionType>("RELIGION_RUNES_OF_KILMORPH") {
                            tech = Some(tn("TECH_WAY_OF_THE_EARTHMOTHER"));
                        }
                        if rel == d.t::<ReligionType>("RELIGION_OCTOPUS_OVERLORDS") {
                            tech = Some(tn("TECH_MESSAGE_FROM_THE_DEEP"));
                        }
                    }

                    if let Some(early_rel) = leader.favorite_early_religion {
                        let newtech = d.religions[early_rel].religion_tech1;
                        if !has_t(newtech) {
                            tech = newtech;
                        }
                    }

                    if is_civ("CIVILIZATION_LJOSALFAR") || is_civ("CIVILIZATION_SVARTALFAR") {
                        if can_research(tech) {
                            tech = Some(tn("TECH_WAY_OF_THE_FORESTS"));
                        }
                    }

                    if is_civ("CIVILIZATION_KHAZAD") {
                        if can_research(tech) {
                            tech = Some(tn("TECH_WAY_OF_THE_EARTHMOTHER"));
                        }
                    }
                }
            } else {
                // Early Happiness for Agnostics
                if !has("TECH_SANITATION") {
                    tech = Some(tn("TECH_SANITATION"));
                }
            }
        }

        //----------------------------
        // Early Economy
        //----------------------------
        if !econ1 {
            for n in ["TECH_EDUCATION", "TECH_CALENDAR", "TECH_MINING", "TECH_EXPLORATION"] {
                if !has(n) {
                    tech = Some(tn(n));
                }
            }
        }

        //----------------------------
        // Early Military Beeline
        //----------------------------
        if !tier2 {
            let temp = leader.favorite_early_mil_tech;
            if temp.is_some() {
                tech = temp;
            } else {
                tech = Some(tn("TECH_BRONZE_WORKING"));
                if is_civ("CIVILIZATION_LJOSALFAR") {
                    tech = Some(tn("TECH_HUNTING"));
                }
                if is_civ("CIVILIZATION_SVARTALFAR") {
                    tech = Some(tn("TECH_HUNTING"));
                }
                if is_civ("CIVILIZATION_LUCHUIRP") {
                    tech = Some(tn("TECH_CONSTRUCTION"));
                }
            }
        }

        //----------------------------
        // Calculate Starting resources
        //----------------------------
        if let (false, Some(city)) = (econ1, city) {
            const S_TECHS: [&str; 8] = [
                "TECH_AGRICULTURE",
                "TECH_CALENDAR",
                "TECH_CRAFTING",
                "TECH_EDUCATION",
                "TECH_HUNTING",
                "TECH_FISHING",
                "TECH_MINING",
                "TECH_ANIMAL_HUSBANDRY",
            ];
            const AGRICULTURE: usize = 0;
            const CALENDAR: usize = 1;
            const CRAFTING: usize = 2;
            const HUNTING: usize = 4;
            const FISHING: usize = 5;
            const MINING: usize = 6;
            const ANIMAL_HUSBANDRY: usize = 7;

            let mut values = [0i32; S_TECHS.len() + 1];
            let ix = g.cities[city].x;
            let iy = g.cities[city].y;

            if is_civ("CIVILIZATION_LANUN") {
                if g.city_is_coastal(city, 0) {
                    values[FISHING] += 150;
                }
            }

            // Loop invariants of the Python (it re-evaluates them for every plot).
            let mut build_in_forest = false;
            let mut build_in_jungle = false;
            if is_civ("CIVILIZATION_LJOSALFAR") || is_civ("CIVILIZATION_SVARTALFAR") {
                build_in_forest = true;
            }
            if has("TECH_MINING") {
                build_in_forest = true;
            }
            if has("TECH_BRONZE_WORKING") {
                build_in_jungle = true;
            }
            let forest = Some(d.t::<FeatureType>("FEATURE_FOREST"));
            let jungle = Some(d.t::<FeatureType>("FEATURE_JUNGLE"));
            let flood_plains = Some(d.t::<FeatureType>("FEATURE_FLOOD_PLAINS"));
            let desert = Some(d.t::<TerrainType>("TERRAIN_DESERT"));
            let coastal5 = g.city_is_coastal(city, 5);
            let lanun = is_civ("CIVILIZATION_LANUN");
            let hunting_turns = g.player_get_research_turns_left(p, tn("TECH_HUNTING"), true);

            for iix in (ix - 2)..(ix + 3) {
                for iiy in (iy - 2)..(iy + 3) {
                    let mut valid = true;
                    // C++ quirk: the Python compares the absolute plot coordinates with the
                    // corner offsets (0/5), so the 5x5 square keeps its corners (except near
                    // the map's (0,0) / (0,5) / (5,0) / (5,5) plots).
                    if iix == 0 && iiy == 0 {
                        valid = false;
                    }
                    if iix == 0 && iiy == 5 {
                        valid = false;
                    }
                    if iix == 5 && iiy == 0 {
                        valid = false;
                    }
                    if iix == 5 && iiy == 5 {
                        valid = false;
                    }
                    if !valid {
                        continue;
                    }
                    let Some(pl) = g.map.plot_xy(iix, iiy) else {
                        continue;
                    };
                    if g.plot_is_city(pl, false, None) {
                        continue;
                    }
                    let plot = &g.map.plots[pl];
                    // Fresh Water
                    if g.plot_is_fresh_water(pl) {
                        if !g.plot_is_hills(pl) {
                            let feature = plot.feature_type;
                            if plot.terrain_type != desert {
                                if feature != forest || build_in_forest {
                                    if feature != jungle || build_in_jungle {
                                        values[AGRICULTURE] += 70;
                                    }
                                } else if !build_in_forest {
                                    values[MINING] += 34;
                                }
                            }
                            if feature == flood_plains {
                                values[AGRICULTURE] += 250;
                            }
                        }
                    }

                    // Fishing
                    if coastal5 {
                        if g.plot_is_water(pl) {
                            values[FISHING] += 10;
                        }
                    }

                    if let Some(bonus) = g.plot_get_bonus_type(pl, None) {
                        let feature = plot.feature_type;
                        if !(feature == forest) || build_in_forest {
                            if !(feature == jungle) || build_in_jungle {
                                // (Each bonus matches at most one of the Python's `if`s.)
                                match d.type_name(bonus) {
                                    // Agri
                                    "BONUS_WHEAT" | "BONUS_CORN" | "BONUS_RICE" => values[AGRICULTURE] += 350,
                                    // Animal Husbandry
                                    "BONUS_COW" | "BONUS_SHEEP" | "BONUS_PIG" | "BONUS_HORSE" => values[ANIMAL_HUSBANDRY] += 58,
                                    // Crafting
                                    "BONUS_WINE" => values[CRAFTING] += 115,
                                    // Calendar
                                    "BONUS_DYE" | "BONUS_INCENSE" => values[CALENDAR] += 120,
                                    "BONUS_COTTON" | "BONUS_BANANA" | "BONUS_SILK" | "BONUS_REAGENTS" => values[CALENDAR] += 80,
                                    // Fishing
                                    "BONUS_CLAM" | "BONUS_CRAB" | "BONUS_FISH" => {
                                        if coastal5 {
                                            values[FISHING] += 40;
                                        }
                                    }
                                    "BONUS_PEARL" => {
                                        if coastal5 && lanun {
                                            values[FISHING] += 200;
                                        }
                                    }
                                    // Hunting
                                    "BONUS_DEER" => {
                                        if hunting_turns < 20 {
                                            values[HUNTING] += 80;
                                        }
                                    }
                                    "BONUS_FUR" | "BONUS_IVORY" => {
                                        if hunting_turns < 20 {
                                            values[HUNTING] += 110;
                                        }
                                    }
                                    // Mining
                                    "BONUS_GOLD" | "BONUS_GEMS" => values[MINING] += 200,
                                    _ => {}
                                }
                            }
                        } else {
                            values[MINING] += 50;
                        }
                    }
                }
            }

            if is_civ("CIVILIZATION_HIPPUS") {
                values[ANIMAL_HUSBANDRY] += 50;
            }
            if is_civ("CIVILIZATION_LANUN") {
                values[FISHING] *= 2;
            }

            let mut best_tech: Option<usize> = None;
            let mut best_tech_value = 99;
            for i in 0..S_TECHS.len() {
                if values[i] > best_tech_value {
                    if !has(S_TECHS[i]) {
                        best_tech_value = values[i];
                        best_tech = Some(i);
                    }
                }
            }

            let mut need_more_research = true;
            if let Some(i) = best_tech {
                tech = Some(tn(S_TECHS[i]));
                need_more_research = false;
            } else if !has("TECH_FESTIVALS") {
                tech = Some(tn("TECH_FESTIVALS"));
            } else if !tier2 {
                let mut iturns = 15;
                if g.game_turn() > 100 {
                    iturns += 10;
                }
                if let Some(mil) = leader.favorite_early_mil_tech {
                    if g.player_get_research_turns_left(p, mil, false) < iturns {
                        tech = Some(mil);
                        need_more_research = false;
                    }
                } else if has("TECH_MINING") {
                    if g.player_get_research_turns_left(p, tn("TECH_BRONZE_WORKING"), false) < iturns {
                        tech = Some(tn("TECH_BRONZE_WORKING"));
                        need_more_research = false;
                    }
                } else if has("TECH_ANIMAL_HUSBANDRY") {
                    if g.player_get_research_turns_left(p, tn("TECH_HORSEBACK_RIDING"), true) < iturns {
                        tech = Some(tn("TECH_HORSEBACK_RIDING"));
                        need_more_research = false;
                    }
                } else if has("TECH_EXPLORATION") {
                    if g.player_get_research_turns_left(p, tn("TECH_HUNTING"), true) < iturns {
                        tech = Some(tn("TECH_HUNTING"));
                        need_more_research = false;
                    }
                }
            }

            if need_more_research {
                if !has("TECH_EXPLORATION") {
                    tech = Some(tn("TECH_EXPLORATION"));
                } else if !has("TECH_FESTIVALS") {
                    tech = Some(tn("TECH_FESTIVALS"));
                } else if !has("TECH_MYSTICISM") {
                    tech = Some(tn("TECH_MYSTICISM"));
                } else if !has("TECH_EDUCATION") {
                    tech = Some(tn("TECH_EDUCATION"));
                } else if !has("TECH_CARTOGRAPHY") && player.num_cities() > 4 {
                    tech = Some(tn("TECH_CARTOGRAPHY"));
                } else if !tier2 {
                    if leader.favorite_early_mil_tech.is_some() {
                        tech = leader.favorite_early_mil_tech;
                    } else {
                        tech = Some(tn("TECH_BRONZE_WORKING"));
                    }
                }
            }
        }

        if player.num_cities() == 0 {
            return Some(tn("TECH_AGRICULTURE"));
        }

        if tech.is_some() {
            if !has_t(tech) {
                return tech;
            }
        }

        None
    }

    /// Python `CvGameUtils.AI_chooseProduction(argsList = (pCity,))` (CvGameUtils.py:1240):
    /// true = the Python chose (pushed an order; the DLL returns), false = the DLL chooses.
    pub fn py_ai_choose_production(&self, g: &mut Game, c: CityId) -> bool {
        // Barbarian Cities have their own Function
        if g.city_is_barbarian(c) {
            // PORT ADAPT: CustomFunctions.BarbCityProduction is ported in ffh-core; it draws
            // from the game's random stream (as the Python does), not the AI's.
            return g.cf_barb_city_production(c);
        }

        match self.py_choose_production_order(g, c) {
            Some((order, data)) => {
                g.city_push_order(c, order, data, -1, false, false, false, false);
                true
            }
            None => false,
        }
    }

    /// The evaluation part of `AI_chooseProduction` for non-barbarian cities: the order the
    /// Python pushes (`pCity.pushOrder(eOrder, iData, -1, False, False, False, False)`), or
    /// `None` when it returns False.
    fn py_choose_production_order(&self, g: &Game, c: CityId) -> Option<(OrderType, i32)> {
        let d = g.d;
        let p = g.cities[c].owner;
        let player = &g.players[p];
        let team = player.team;
        let plot = g.city_plot(c);

        // Automated Cities use another Function
        if player.human {
            return None;
        }

        // Choose Nothing in Disorder (can lead to weird results)
        if g.city_is_disorder(c) {
            return None;
        }

        let Some(civtype) = g.player_get_civilization_type(p) else {
            return None;
        };
        let Some(leader_type) = g.player_get_leader_type(p) else {
            // PORT ADAPT: every player has a leader; gc.getLeaderHeadInfo(-1) would raise.
            return None;
        };
        let leader = &d.leader_heads[leader_type];

        let uc = |n: &str| d.t::<UnitClassType>(n);
        let bc = |n: &str| d.t::<BuildingClassType>(n);
        let uccpm = |n: &str| g.player_get_unit_class_count_plus_making(p, uc(n));
        let bccpm = |n: &str| g.player_get_building_class_count_plus_making(p, bc(n));
        let is_civ = |n: &str| civtype == d.t::<CivilizationType>(n);
        let has_trait = |n: &str| player.traits[d.t::<TraitType>(n)];
        let rel = |n: &str| Some(d.t::<ReligionType>(n));

        // Values the Python re-reads many times (nothing changes in between).
        let num_cities = player.num_cities();
        let capital = g.city_is_capital(c);
        let pop = g.cities[c].population;
        let prod_diff = g.city_current_production_difference(c, false, false);
        let food_diff = g.city_food_difference(c, false);
        let prod_rank = g.city_find_base_yield_rate_rank(c, YieldType::YIELD_PRODUCTION);
        let conquest = player.conquest_mode;
        let area = g.city_area(c);
        let flag7 = g.player_count_group_flag_units(p, GROUPFLAG_DEFENSE_NEW);
        let flag10 = g.player_count_group_flag_units(p, GROUPFLAG_CONQUEST);

        // Var initialization
        // Used for City Specialization
        let mut patrol_spec = 0;
        let mut defense_spec = 0;

        // Used to decide if the Best Item is good enough. (The Python first computes
        // (1000 - 13 * getCurrentProductionDifference) * 0.8, then overwrites it.)
        let building_thres = (-flag7 * 50) as f64;

        // make sure no item is listed twice!
        let mut v = vec![0.0f64; PROD.len() + 1];
        macro_rules! val {
            ($n:expr) => {
                v[ix($n)]
            };
        }
        macro_rules! set {
            ($n:expr, $x:expr) => {{
                let x = ($x) as f64;
                v[ix($n)] = x;
            }};
        }
        macro_rules! add {
            ($n:expr, $x:expr) => {{
                let x = ($x) as f64;
                v[ix($n)] += x;
            }};
        }

        // City needs more Protection? (values 19001-20000)
        if self.city_ai_needed_perm_defense(g, c, 0) > 0 {
            set!("UNITCLASS_AXEMAN", 19500);
            set!("UNITCLASS_ARCHER", 20000);
            set!("UNITCLASS_HUNTER", 19500);
            set!("UNITCLASS_WARRIOR", 19002);
        }

        if self.city_ai_needed_perm_defense(g, c, 1) > 0 {
            set!("UNITCLASS_AXEMAN", 20000);
            set!("UNITCLASS_ARCHER", 19500);
            set!("UNITCLASS_HUNTER", 20000);
            set!("UNITCLASS_WARRIOR", 19003);
            if is_civ("CIVILIZATION_SHEAIM") {
                set!("UNITCLASS_AXEMAN", -20000);
            }
        }

        if self.city_ai_needed_perm_defense(g, c, 2) > 0 {
            set!("UNITCLASS_ADEPT", 19002);
        }

        if self.city_ai_needed_perm_defense(g, c, 3) > 0 {
            set!("UNITCLASS_PRIEST_OF_LEAVES", 19500);
            set!("UNITCLASS_PRIEST_OF_KILMORPH", 19500);
            set!("UNITCLASS_PRIEST_OF_THE_EMPYREAN", 19500);
            set!("UNITCLASS_PRIEST_OF_THE_OVERLORDS", 19500);
            set!("UNITCLASS_PRIEST_OF_THE_VEIL", 19500);
            set!("UNITCLASS_PRIEST_OF_THE_ORDER", 19500);
        }

        // WAR: Cities close by need more Protection? (values 18001-19000)
        let at_war = g.team_get_at_war_count(team, true) > 0;
        if at_war {
            if self.plot_ai_needed_perm_defense_reserve(g, plot, 0) > 0 {
                set!("UNITCLASS_AXEMAN", 18500);
                set!("UNITCLASS_ARCHER", 19000);
                set!("UNITCLASS_HUNTER", 18500);
                set!("UNITCLASS_WARRIOR", 18001);
            }

            if self.plot_ai_needed_perm_defense_reserve(g, plot, 1) > 0 {
                set!("UNITCLASS_AXEMAN", 19000);
                set!("UNITCLASS_ARCHER", 18500);
                set!("UNITCLASS_HUNTER", 19000);
                set!("UNITCLASS_WARRIOR", 18001);
            }

            if self.plot_ai_needed_perm_defense_reserve(g, plot, 2) > 0 {
                set!("UNITCLASS_ADEPT", 18001);
            }

            if self.plot_ai_needed_perm_defense_reserve(g, plot, 3) > 0 {
                set!("UNITCLASS_PRIEST_OF_LEAVES", 18500);
                set!("UNITCLASS_PRIEST_OF_KILMORPH", 18500);
                set!("UNITCLASS_PRIEST_OF_THE_EMPYREAN", 18500);
                set!("UNITCLASS_PRIEST_OF_THE_OVERLORDS", 18500);
                set!("UNITCLASS_PRIEST_OF_THE_VEIL", 18500);
                set!("UNITCLASS_PRIEST_OF_THE_ORDER", 18500);
                set!("UNITCLASS_GRIGORI_MEDIC", 18500);
            }
        }

        // WAR: Citypatrol need more Units? (values 17001-18000)
        if at_war {
            if self.city_ai_needed_patrol(g, c, 0) > 0 {
                set!("BUILDINGCLASS_TRAINING_YARD", 17002);
                set!("BUILDINGCLASS_ARCHERY_RANGE", 17003);
                set!("BUILDINGCLASS_HUNTING_LODGE", 17004);

                set!("UNITCLASS_AXEMAN", 17500);
                set!("UNITCLASS_ARCHER", 17500);
                set!("UNITCLASS_HUNTER", 18000);
                set!("UNITCLASS_WARRIOR", 17001);
            }

            if self.city_ai_needed_patrol(g, c, 1) > 0 {
                set!("UNITCLASS_ADEPT", 17001);
            }
        }

        // This is a City for Patrol Units?
        if self.city_ai_needed_city_patrol_production(g, c, 1) > 0 {
            patrol_spec = 13000;
            if capital {
                // make sure patrol is build before first settler
                patrol_spec += 4000;
            }
            set!("BUILDINGCLASS_TRAINING_YARD", patrol_spec + 2);
            set!("BUILDINGCLASS_ARCHERY_RANGE", patrol_spec + 3);
            set!("BUILDINGCLASS_HUNTING_LODGE", patrol_spec + 4);

            if self.city_ai_needed_patrol(g, c, 0) > 0 {
                set!("UNITCLASS_AXEMAN", patrol_spec + 500);
                set!("UNITCLASS_ARCHER", patrol_spec + 500);
                set!("UNITCLASS_HUNTER", patrol_spec + 500);
                set!("UNITCLASS_WARRIOR", patrol_spec + 1);
            }

            if self.city_ai_needed_patrol(g, c, 1) > 0 {
                set!("UNITCLASS_ADEPT", patrol_spec + 1);
            }
        }
        // Defense specialization:
        else if self.city_ai_needed_city_defense_production(g, c, 2) > 0 {
            defense_spec = 12000;
            add!("BUILDINGCLASS_ARCHERY_RANGE", defense_spec);
            add!("UNITCLASS_LONGBOWMAN", defense_spec);
        } else if self.city_ai_needed_city_defense_production(g, c, 1) > 0 {
            defense_spec = 12000;
            add!("BUILDINGCLASS_ARCHERY_RANGE", defense_spec);
            add!("UNITCLASS_ARCHER", defense_spec);
        }

        // Count how much Value the Specialization of a City has
        let mut specialization = patrol_spec;
        if specialization < defense_spec {
            specialization = defense_spec;
        }

        // Civ Needs more workboats?
        if let Some(water_area) = g.city_water_area(c) {
            if self.player_ai_total_water_area_unit_ais(g, p, water_area, UnitAIType::UNITAI_WORKER_SEA) == 0 {
                if self.city_ai_needed_sea_workers(g, c) > 0 {
                    set!("UNITCLASS_WORKBOAT", 19002);
                }
            }
        }

        // Civ needs more workers?
        let workers = uccpm("UNITCLASS_WORKER");
        if 2 * num_cities > workers {
            if capital || (g.city_current_production_difference(c, true, false) > 7 && pop > 3) {
                if workers < num_cities + num_cities - 1 - py_div(num_cities, 3) {
                    set!("UNITCLASS_WORKER", 16001);
                }
                if workers == 0 {
                    set!("UNITCLASS_WORKER", 19001);
                } else {
                    // Python 2: 3/2 == 1.
                    add!("UNITCLASS_WORKER", 300 + py_div(py_div(3, 2) * num_cities, 1 + workers) * 1000);
                }
            }
        }

        // Civ needs more Settlers?
        if !conquest || flag10 > 20 {
            if num_cities < player.max_cities || player.max_cities <= 0 {
                let mut valid = false;
                if g.city_current_production_difference(c, true, false) > 10 && pop > 4 {
                    if !capital {
                        valid = true;
                    }
                }
                if capital {
                    if g.city_happy_level(c) <= g.city_unhappy_level(c, 0) || food_diff < 2 {
                        valid = true;
                    }
                }
                if valid {
                    let mut settler_badly_needed = 1;
                    if num_cities > 3 {
                        settler_badly_needed = 2 + py_div(num_cities, 10);
                    }
                    if uccpm("UNITCLASS_SETTLER") < settler_badly_needed {
                        set!("UNITCLASS_SETTLER", 17000);
                    }
                }
            }
        }

        // Civ Needs more Barbsmashers?
        if !conquest {
            if uccpm("UNITCLASS_SETTLER") > 0 {
                if flag7 < 10 {
                    // 7 is GROUPFLAG_DEFENSE_NEW
                    add!("BUILDINGCLASS_TRAINING_YARD", 2502);
                    add!("BUILDINGCLASS_ARCHERY_RANGE", 2503);
                    add!("BUILDINGCLASS_HUNTING_LODGE", 2504);

                    add!("UNITCLASS_AXEMAN", 2500);
                    add!("UNITCLASS_ARCHER", 2500);
                    add!("UNITCLASS_HUNTER", 2500);
                    add!("UNITCLASS_WARRIOR", 2501);
                }
            }
        }

        // Civ can build a Religion Hero?
        let can_train = |u: Option<UnitType>| u.map_or(false, |u| g.city_can_train(c, u, true, false, false, false));
        if let Some(fav) = player.favorite_religion {
            for hero in [d.religions[fav].religion_hero1, d.religions[fav].religion_hero2] {
                let Some(hero) = hero else {
                    continue;
                };
                if g.player_get_unit_class_count_plus_making(p, hero) == 0 {
                    // (The class's default unit, not the civilization's.)
                    let t_unit = d.unit_classes[hero].default_unit_index;
                    if can_train(t_unit) {
                        if prod_rank < 3 {
                            return Some((OrderType::ORDER_TRAIN, t_unit.unwrap().i()));
                        }
                    }
                }
            }
        }

        // Civ can build its Hero?
        if prod_rank < 3 {
            let loki = d.t::<UnitType>("UNIT_LOKI");
            if let Some(hero) = d.civilizations[civtype].hero {
                if hero != loki {
                    if can_train(Some(hero)) {
                        return Some((OrderType::ORDER_TRAIN, hero.i()));
                    }
                }
            }
        }

        let count_naval_transporters = uccpm("UNITCLASS_GALLEY") + uccpm("UNITCLASS_CARAVEL") + uccpm("UNITCLASS_QUEEN_OF_THE_LINE");
        let count_naval_patrols = uccpm("UNITCLASS_TRIREME") + uccpm("UNITCLASS_FRIGATE") + uccpm("UNITCLASS_MAN_O_WAR");

        if player.unit_class_count[uc("UNITCLASS_SETTLER")] > 0 {
            // Need to add check that settlerships are only build for naval settle mission
            if count_naval_transporters < 3 {
                add!("UNITCLASS_GALLEY", 1500);
                add!("UNITCLASS_CARAVEL", 1500);
                add!("UNITCLASS_QUEEN_OF_THE_LINE", 1500);
            }
        }

        if count_naval_patrols < num_cities {
            add!("UNITCLASS_TRIREME", 1500);
            add!("UNITCLASS_FRIGATE", 1500);
            add!("UNITCLASS_MAN_O_WAR", 1500);
        }

        // Civ needs more terraformers?
        let mut countmages = 0;
        let mut count_priests = 0;
        let priest_of_leaves = d.t::<UnitType>("UNIT_PRIEST_OF_LEAVES");
        for &u in &player.units {
            let ai = g.units[u].ai_type;
            if ai == Some(UnitAIType::UNITAI_TERRAFORMER) {
                countmages += 1;
            }
            if ai == Some(UnitAIType::UNITAI_MANA_UPGRADE) {
                countmages += 1;
            }
            if g.units[u].unit_type == priest_of_leaves {
                if ai == Some(UnitAIType::UNITAI_TERRAFORMER) {
                    count_priests += 1;
                }
            }
        }

        if countmages < 2 {
            // need to write a Calculation for terraformer per area
            add!("UNITCLASS_ADEPT", 1000);
        }
        if count_priests < 2 {
            add!("UNITCLASS_PRIEST_OF_LEAVES", 1000);
        }

        // City has enough Culture?
        if g.city_culture(c, p) == 0 {
            set!("BUILDING_MONUMENT", 1100 + specialization);
        }

        // Every City should have atleast a training yard, archery range or hunting lodge
        let num_building_is_1 = |class: &str| match d.civ_building(civtype, bc(class)) {
            Some(b) => g.city_num_building(c, b) == 1,
            None => false,
        };
        let mut count = 0;
        for class in ["BUILDINGCLASS_TRAINING_YARD", "BUILDINGCLASS_ARCHERY_RANGE", "BUILDINGCLASS_HUNTING_LODGE"] {
            if num_building_is_1(class) {
                count += 1;
            }
        }
        // some Civs prefere a different Building
        if is_civ("CIVILIZATION_AMURITES") {
            if count == 0 {
                add!("BUILDINGCLASS_MAGE_GUILD", 1200);
                add!("BUILDINGCLASS_ARCHERY_RANGE", 1200);
                add!("BUILDINGCLASS_HUNTING_LODGE", 1200);
            }
        } else if count == 0 {
            add!("BUILDINGCLASS_TRAINING_YARD", 1200);
            add!("BUILDINGCLASS_ARCHERY_RANGE", 1200);
            add!("BUILDINGCLASS_HUNTING_LODGE", 1200);
        }

        if count > 0 {
            // Empire needs another Military Building? (The Python counts this city's stables,
            // siege workshops and mage guilds, but the count only gates `+= 0`.)
            add!("BUILDINGCLASS_STABLE", py_div(200 * num_cities, 1 + bccpm("BUILDINGCLASS_STABLE")));
            add!("BUILDINGCLASS_SIEGE_WORKSHOP", py_div(200 * num_cities, 1 + bccpm("BUILDINGCLASS_SIEGE_WORKSHOP")));
            add!("BUILDINGCLASS_MAGE_GUILD", py_div(200 * num_cities, 1 + bccpm("BUILDINGCLASS_MAGE_GUILD")));
        }

        // Temples and disciples
        let agnostic = has_trait("TRAIT_AGNOSTIC");
        let state_religion = g.player_get_state_religion(p);
        if !agnostic {
            let mut bonus = 0 + specialization;
            if state_religion == player.favorite_religion {
                bonus += 3000;
            }
            for (religion, temple, disciple) in [
                ("RELIGION_FELLOWSHIP_OF_LEAVES", "BUILDINGCLASS_TEMPLE_OF_LEAVES", "UNITCLASS_DISCIPLE_FELLOWSHIP_OF_LEAVES"),
                ("RELIGION_RUNES_OF_KILMORPH", "BUILDINGCLASS_TEMPLE_OF_KILMORPH", "UNITCLASS_DISCIPLE_RUNES_OF_KILMORPH"),
                ("RELIGION_THE_EMPYREAN", "BUILDINGCLASS_TEMPLE_OF_THE_EMPYREAN", "UNITCLASS_DISCIPLE_EMPYREAN"),
                ("RELIGION_OCTOPUS_OVERLORDS", "BUILDINGCLASS_TEMPLE_OF_THE_OVERLORDS", "UNITCLASS_DISCIPLE_OCTOPUS_OVERLORDS"),
                ("RELIGION_THE_ASHEN_VEIL", "BUILDINGCLASS_TEMPLE_OF_THE_VEIL", "UNITCLASS_DISCIPLE_THE_ASHEN_VEIL"),
                ("RELIGION_THE_ORDER", "BUILDINGCLASS_TEMPLE_OF_THE_ORDER", "UNITCLASS_DISCIPLE_THE_ORDER"),
            ] {
                if state_religion == rel(religion) {
                    add!(temple, 400 + 20 * prod_diff + bonus);
                    if uccpm(disciple) == 0 {
                        add!(disciple, 3000);
                    }
                    if religion == "RELIGION_THE_ASHEN_VEIL" {
                        set!("BUILDINGCLASS_PROPHECY_OF_RAGNAROK", 2000);
                    }
                }
            }

            if leader.religion_victory {
                // CyGame().getHolyCity(e).getOwner(): -1 for no holy city.
                let holy = g.get_holy_city(d.t::<ReligionType>("RELIGION_FELLOWSHIP_OF_LEAVES"));
                if holy.map(|h| g.cities[h].owner) == Some(p) {
                    if uccpm("UNITCLASS_DISCIPLE_FELLOWSHIP_OF_LEAVES") < num_cities * 3 {
                        add!("UNITCLASS_DISCIPLE_FELLOWSHIP_OF_LEAVES", 3000);
                    }
                }
            }
        }

        // City can build civ specific buildings?

        // init for Conquestmode
        let i_dif = 8 + 2 * g.handicap.i(); // used for Conquest Modus
        let min_city_size = 7;
        // sReli (iReli is its unit class).
        let mut reli: Option<&str> = None;
        let mut reli_mod = 0;
        if !agnostic {
            if state_religion == rel("RELIGION_FELLOWSHIP_OF_LEAVES") {
                reli = Some("UNITCLASS_PRIEST_OF_LEAVES");
                reli_mod += 2;
            }
            if state_religion == rel("RELIGION_RUNES_OF_KILMORPH") {
                reli = Some("UNITCLASS_PRIEST_OF_KILMORPH");
                reli_mod += 0;
            }
            if state_religion == rel("RELIGION_THE_EMPYREAN") {
                reli = Some("UNITCLASS_PRIEST_OF_THE_EMPYREAN");
                reli_mod += 0;
            }
            if state_religion == rel("RELIGION_OCTOPUS_OVERLORDS") {
                reli = Some("UNITCLASS_PRIEST_OF_THE_OVERLORDS");
                reli_mod += 1;
            }
            if state_religion == rel("RELIGION_THE_ASHEN_VEIL") {
                reli = Some("UNITCLASS_PRIEST_OF_THE_VEIL");
                reli_mod += 2;
            }
            if state_religion == rel("RELIGION_THE_ORDER") {
                reli = Some("UNITCLASS_PRIEST_OF_THE_ORDER");
                reli_mod += 0;
            }
        }

        // The conquest gate of every civilization block: a big (or starving) city and too few
        // conquest units (`countGroupFlagUnits(10)`, 10 is GROUPFLAG_CONQUEST).
        let conquest_city = |size: i32| (pop > size || food_diff < 1) && flag10 < num_cities * i_dif;
        // `if pPlayer.getNumCities() > getBuildingClassCountPlusMaking(class) * mult`.
        let few = |class: &str, mult: i32| num_cities > bccpm(class) * mult;

        // The standard building follow-ups of the conquest blocks.
        macro_rules! conquest_buildings {
            ($siege_mult:expr) => {
                if few("BUILDINGCLASS_HUNTING_LODGE", 5) {
                    set!("BUILDINGCLASS_HUNTING_LODGE", val!("UNITCLASS_HUNTER") + 10.0);
                }
                if few("BUILDINGCLASS_MAGE_GUILD", 5) {
                    set!("BUILDINGCLASS_MAGE_GUILD", val!("UNITCLASS_ADEPT") + 10.0);
                }
                if few("BUILDINGCLASS_SIEGE_WORKSHOP", $siege_mult) {
                    set!("BUILDINGCLASS_SIEGE_WORKSHOP", val!("UNITCLASS_CATAPULT") + 10.0);
                }
                if few("BUILDINGCLASS_STABLE", 5) {
                    set!("BUILDINGCLASS_STABLE", val!("UNITCLASS_HORSEMAN") + 10.0);
                }
                if few("BUILDINGCLASS_ARCHERY_RANGE", 5) {
                    set!("BUILDINGCLASS_ARCHERY_RANGE", val!("UNITCLASS_ARCHER") + 10.0);
                }
            };
        }
        // The standard unit values of the conquest blocks: adept, archer, axeman, hunter,
        // catapult, horseman weights, then the siege / mage minimums and the priests.
        macro_rules! conquest_units {
            ($k:expr, $cq:expr, $min_mages:expr) => {{
                let k: [i32; 6] = $k;
                let cq: &Conquest = $cq;
                let tu = cq.units();
                add!("UNITCLASS_ADEPT", 1200 + k[0] * (tu - cq.mages));
                add!("UNITCLASS_ARCHER", 1200 + k[1] * (tu - cq.archers));
                add!("UNITCLASS_AXEMAN", 1200 + k[2] * (tu - cq.melee));
                add!("UNITCLASS_HUNTER", 1200 + k[3] * (tu - cq.recon));
                add!("UNITCLASS_CATAPULT", 1200 + k[4] * (tu - cq.siege));
                add!("UNITCLASS_HORSEMAN", 1200 + k[5] * (tu - cq.mounted));

                if cq.siege < 5 {
                    add!("UNITCLASS_CATAPULT", 2000);
                }
                if cq.mages < $min_mages {
                    add!("UNITCLASS_ADEPT", 2000);
                }

                if let Some(r) = reli {
                    add!(r, 1200 + 10 * (tu - cq.priests));
                }
            }};
        }

        // Amurites
        if is_civ("CIVILIZATION_AMURITES") {
            if prod_rank < 3 {
                add!("UNITCLASS_GOVANNON", 19000);
                add!("BUILDINGCLASS_CATACOMB_LIBRALUS", 3000);
            }
            if g.city_num_building(c, d.t::<BuildingType>("BUILDING_MAGE_GUILD")) == 1 {
                add!("BUILDING_CAVE_OF_ANCESTORS", 630);
            }
            if conquest {
                set!("BUILDINGCLASS_SIEGE_WORKSHOP", -10000);
                if conquest_city(min_city_size + 3) {
                    let cq = Conquest::new(g, p, area, reli);
                    // No siege in the Amurites' total.
                    let tu = cq.mages + cq.melee + cq.archers + cq.recon + cq.mounted + cq.priests;
                    add!("UNITCLASS_ADEPT", 1200 + 7 * (tu - cq.mages));
                    add!("UNITCLASS_LONGBOWMAN", 1200 + 15 * (tu - cq.archers));
                    add!("UNITCLASS_AXEMAN", 1200 + 10 * (tu - cq.melee));
                    add!("UNITCLASS_HUNTER", 1200 + 10 * (tu - cq.recon));
                    add!("UNITCLASS_HORSEMAN", 1200 + 10 * (tu - cq.mounted));
                    if let Some(r) = reli {
                        add!(r, 1200 + 10 * (tu - cq.priests) * reli_mod);
                    }

                    set!("BUILDINGCLASS_ARCHERY_RANGE", 12000);
                    set!("BUILDINGCLASS_MAGE_GUILD", 12000);
                    if few("BUILDINGCLASS_HUNTING_LODGE", 5) {
                        set!("BUILDINGCLASS_HUNTING_LODGE", val!("UNITCLASS_HUNTER") + 10.0);
                    }
                    if few("BUILDINGCLASS_TRAINING_YARD", 5) {
                        set!("BUILDINGCLASS_TRAINING_YARD", val!("UNITCLASS_AXEMAN") + 10.0);
                    }
                    if few("BUILDINGCLASS_STABLE", 5) {
                        set!("BUILDINGCLASS_STABLE", val!("UNITCLASS_HORSEMAN") + 10.0);
                    }
                }
            }
        }

        // Balseraph
        if is_civ("CIVILIZATION_BALSERAPHS") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            // C++ quirk: the Python writes `iValue[HALL_OF_MIRRORS]==300` (a comparison, no-op).
            if capital {
                if g.team_is_has_tech(team, Some(d.t::<TechType>("TECH_FESTIVALS"))) {
                    add!("UNITCLASS_LOKI", 12000);
                }
            }
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 3, 14, 10, 11, 10], &cq, 3);
                    set!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    set!("UNITCLASS_FREAK", val!("UNITCLASS_AXEMAN") + 3.0);
                    conquest_buildings!(3);
                }
            }
        }

        // Bannor
        if is_civ("CIVILIZATION_BANNOR") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            if capital {
                add!("UNITCLASS_DONAL", 12000);
            }
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 8, 10, 10, 11, 12], &cq, 3);
                    set!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    conquest_buildings!(3);
                }
            }
        }

        // Calabim
        if is_civ("CIVILIZATION_CALABIM") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            add!("BUILDING_BREEDING_PIT", 750);
            add!("BUILDINGCLASS_COURTHOUSE", 800);
            if capital {
                add!("UNITCLASS_LOSHA", 12000);
            }
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 8, 12, 10, 9, 12], &cq, 3);
                    add!("BUILDINGCLASS_COURTHOUSE", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Clan of Embers
        if is_civ("CIVILIZATION_CLAN_OF_EMBERS") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            add!("BUILDING_WARRENS", 70 * prod_diff + specialization);
            if conquest {
                if conquest_city(min_city_size - 3) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 8, 15, 10, 8, 12], &cq, 3);
                    set!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    set!("BUILDING_WARRENS", 12001);
                    conquest_buildings!(5);
                }
            }
        }

        // Doviello
        if is_civ("CIVILIZATION_DOVIELLO") {
            if conquest {
                if conquest_city(min_city_size - 4) {
                    let mut cq = Conquest::new(g, p, area, reli);
                    // Better AI: Beastmen
                    cq.melee += g.player_get_conquest_unit_class_count(p, uc("UNITCLASS_WARRIOR"), area);
                    // Better AI: Beastmen (the Python adds it just before the axemen; the order
                    // of the `+=` does not matter)
                    add!("UNITCLASS_WARRIOR", 1199 + 15 * (cq.units() - cq.melee));
                    conquest_units!([5, 8, 15, 10, 9, 12], &cq, 3);
                    conquest_buildings!(5);
                }
            }
        }

        // Elohim
        if is_civ("CIVILIZATION_ELOHIM") {
            add!("BUILDINGCLASS_ARCHERY_RANGE", 250);
            add!("BUILDINGCLASS_LIBRARY", 200);
            add!("BUILDINGCLASS_PAGAN_TEMPLE", 5000);

            add!("BUILDING_RELIQUARY", 20 * prod_diff);
            add!("BUILDING_CHANCEL_OF_GUARDIANS", 20 * prod_diff);
            if conquest {
                if conquest_city(min_city_size + 3) {
                    let cq = Conquest::new(g, p, area, reli);
                    let totalmonk = g.player_get_conquest_unit_class_count(p, uc("UNITCLASS_MONK"), area);
                    // No melee, recon or monks in the Elohim's total.
                    let tu = cq.mages + cq.archers + cq.siege + cq.mounted + cq.priests;
                    add!("UNITCLASS_ADEPT", 1200 + 5 * (tu - cq.mages));
                    add!("UNITCLASS_ARCHER", 1200 + 8 * (tu - cq.archers));
                    add!("UNITCLASS_MONK", 1200 + 13 * (tu - totalmonk));
                    add!("UNITCLASS_CATAPULT", 1200 + 8 * (tu - cq.siege));
                    add!("UNITCLASS_HORSEMAN", 1200 + 10 * (tu - cq.mounted));

                    add!("BUILDING_RELIQUARY", 4000);

                    if cq.siege < 5 {
                        add!("UNITCLASS_CATAPULT", 2000);
                    }
                    if cq.mages < 3 {
                        add!("UNITCLASS_ADEPT", 2000);
                    }

                    if let Some(r) = reli {
                        add!(r, 1200 + 10 * (tu - cq.priests));
                    }
                    if few("BUILDINGCLASS_MAGE_GUILD", 5) {
                        set!("BUILDINGCLASS_MAGE_GUILD", val!("UNITCLASS_ADEPT") + 10.0);
                    }
                    if few("BUILDINGCLASS_SIEGE_WORKSHOP", 5) {
                        set!("BUILDINGCLASS_SIEGE_WORKSHOP", val!("UNITCLASS_CATAPULT") + 10.0);
                    }
                    if few("BUILDINGCLASS_STABLE", 5) {
                        set!("BUILDINGCLASS_STABLE", val!("UNITCLASS_HORSEMAN") + 10.0);
                    }
                    if few("BUILDINGCLASS_ARCHERY_RANGE", 5) {
                        set!("BUILDINGCLASS_ARCHERY_RANGE", val!("UNITCLASS_ARCHER") + 10.0);
                    }
                }
            }
        }

        // Grigori
        if is_civ("CIVILIZATION_GRIGORI") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            add!("BUILDINGCLASS_INN", 200);
            add!("BUILDING_ADVENTURERS_GUILD", 700);
            add!("BUILDINGCLASS_TAVERN", 30 * pop);
            add!("BUILDINGCLASS_INFIRMARY", py_div(200 * num_cities, 1 + bccpm("BUILDINGCLASS_INFIRMARY") + 15 * pop));
            if conquest {
                if conquest_city(min_city_size + 2) {
                    let mut cq = Conquest::new(g, p, area, None);
                    // The Grigori's "priests" are all their medics (not only conquest units).
                    cq.priests = uccpm("UNITCLASS_GRIGORI_MEDIC");
                    let tu = cq.units();
                    add!("UNITCLASS_ADEPT", 1200 + 5 * (tu - cq.mages));
                    add!("UNITCLASS_ARCHER", 1200 + 8 * (tu - cq.archers));
                    add!("UNITCLASS_AXEMAN", 1200 + 15 * (tu - cq.melee));
                    add!("UNITCLASS_HUNTER", 1200 + 10 * (tu - cq.recon));
                    add!("UNITCLASS_CATAPULT", 1200 + 8 * (tu - cq.siege));
                    add!("UNITCLASS_HORSEMAN", 1200 + 12 * (tu - cq.mounted));
                    add!("UNITCLASS_GRIGORI_MEDIC", 1200 + 10 * (tu - cq.priests));
                    if cq.siege < 5 {
                        add!("UNITCLASS_CATAPULT", 2000);
                    }
                    if cq.mages < 3 {
                        add!("UNITCLASS_ADEPT", 2000);
                    }

                    add!("BUILDING_ADVENTURERS_GUILD", 5000);
                    conquest_buildings!(5);
                    if few("BUILDINGCLASS_INFIRMARY", 5) {
                        set!("BUILDINGCLASS_INFIRMARY", val!("UNITCLASS_GRIGORI_MEDIC") + 10.0);
                    }
                }
            }
        }

        // Hippus
        if is_civ("CIVILIZATION_HIPPUS") {
            add!("BUILDINGCLASS_STABLE", 250);
            if conquest {
                if conquest_city(min_city_size - 2) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([8, 8, 11, 10, 9, 15], &cq, 3);
                    add!("BUILDINGCLASS_STABLE", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Illians
        if is_civ("CIVILIZATION_ILLIANS") {
            add!("BUILDINGCLASS_TRAINING_YARD", 100);
            add!("BUILDINGCLASS_PAGAN_TEMPLE", 20 * prod_diff + 20 * pop);
            if conquest {
                if conquest_city(min_city_size - 2) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 8, 15, 10, 9, 12], &cq, 3);
                    add!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        if is_civ("CIVILIZATION_INFERNAL") {
            set!("BUILDINGCLASS_CATACOMB_LIBRALUS", -10000);
        }

        // Kahzad
        if is_civ("CIVILIZATION_KHAZAD") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            if capital {
                add!("BUILDINGCLASS_BREWERY", 250);
            }
            add!("BUILDINGCLASS_FORGE", 400);
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 8, 15, 10, 11, 12], &cq, 3);
                    add!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Kuriotates
        if is_civ("CIVILIZATION_KURIOTATES") {
            add!("BUILDINGCLASS_TRAINING_YARD", -250);
            if g.player_get_num_available_bonuses(p, d.t::<BonusType>("BONUS_FINE_CLOTHES")) == 0 {
                add!("BUILDING_TAILOR", 300);
            }
            add!("BUILDING_TAILOR", 200 + 30 * pop + 30 * prod_diff);
            if g.player_get_num_available_bonuses(p, d.t::<BonusType>("BONUS_JEWELS")) == 0 {
                add!("BUILDING_JEWELER", 300);
            }
            add!("BUILDING_JEWELER", 200 + 30 * pop + 30 * prod_diff);
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([8, 8, 4, 7, 11, 15], &cq, 3);
                    add!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Lanun
        if is_civ("CIVILIZATION_LANUN") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            add!("BUILDINGCLASS_HARBOR", 100);
            // C++ quirk: the Python tests `pCity.getPopulation>4` (the method, not its value),
            // which Python 2 always finds true.
            if capital {
                add!("BUILDINGCLASS_HERON_THRONE", 200);
            }
            if uccpm("UNITCLASS_WORKBOAT") < 1 {
                add!("UNITCLASS_WORKBOAT", 1500);
            }
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 8, 15, 10, 9, 12], &cq, 3);
                    add!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Ljo
        if is_civ("CIVILIZATION_LJOSALFAR") {
            add!("BUILDINGCLASS_ARCHERY_RANGE", 250);
            if conquest {
                if conquest_city(min_city_size - 4) {
                    let cq = Conquest::new(g, p, area, reli);
                    let tu = cq.units();
                    add!("UNITCLASS_ADEPT", 1200 + 13 * (tu - cq.mages));
                    add!("UNITCLASS_ARCHER", 1200 + 15 * (tu - cq.archers));
                    add!("UNITCLASS_AXEMAN", 1200 + 11 * (tu - cq.melee));
                    add!("UNITCLASS_HUNTER", 1200 + 10 * (tu - cq.recon));
                    add!("UNITCLASS_CATAPULT", 1200 + -100 * (tu - cq.siege));
                    add!("UNITCLASS_HORSEMAN", 1200 + 12 * (tu - cq.mounted));

                    // (No siege minimum for the Ljosalfar.)
                    if cq.mages < 9 {
                        add!("UNITCLASS_ADEPT", 2000);
                    }

                    if let Some(r) = reli {
                        add!(r, 1200 + 10 * (tu - cq.priests));
                    }
                    add!("BUILDINGCLASS_ARCHERY_RANGE", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Luchu
        if is_civ("CIVILIZATION_LUCHUIRP") {
            add!("BUILDINGCLASS_TRAINING_YARD", 1000);
            add!("BUILDINGCLASS_MAGE_GUILD", prod_diff * 8);
            // Python float from here on.
            v[ix("UNITCLASS_ADEPT")] *= 1.2;
            add!("BUILDING_BLASTING_WORKSHOP", 400 + 20 * prod_diff);
            add!("BUILDING_PALLENS_ENGINE", 200 + 20 * prod_diff);
            add!("BUILDING_ADULARIA_CHAMBER", 200 + 20 * prod_diff);
            if conquest {
                if conquest_city(min_city_size - 4) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([8, 0, 15, 0, 9, 0], &cq, 5);
                    add!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    add!("BUILDING_BLASTING_WORKSHOP", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Malakim
        if is_civ("CIVILIZATION_MALAKIM") {
            add!("BUILDINGCLASS_TRAINING_YARD", 250);
            add!("BUILDINGCLASS_MAGE_GUILD", 200);
            add!("BUILDING_CITADEL_OF_LIGHT", 200 + 20 * prod_diff);
            add!("BUILDINGCLASS_PAGAN_TEMPLE", 100);
            add!("BUILDINGCLASS_ARCHERY_RANGE", 250);
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([5, 10, 15, 10, 9, 12], &cq, 3);
                    add!("BUILDINGCLASS_TRAINING_YARD", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Sheaim
        if is_civ("CIVILIZATION_SHEAIM") {
            add!("BUILDINGCLASS_MAGE_GUILD", 1000);
            if prod_rank < 3 {
                set!("UNITCLASS_GOVANNON", 19000);
                set!("BUILDINGCLASS_CATACOMB_LIBRALUS", 3000);
            }
            add!("BUILDING_PLANAR_GATE", -400 * 30 * prod_diff);
            set!("BUILDINGCLASS_PROPHECY_OF_RAGNAROK", 2000);
            if conquest {
                if conquest_city(min_city_size - 6) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([12, 10, 16, 10, 0, 12], &cq, 3);
                    add!("BUILDINGCLASS_MAGE_GUILD", 12020);
                    add!("BUILDINGCLASS_ARCHERY_RANGE", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Sidar
        if is_civ("CIVILIZATION_SIDAR") {
            add!("BUILDINGCLASS_HUNTING_LODGE", 200);
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([11, 10, 11, 15, 11, 12], &cq, 3);
                    add!("BUILDINGCLASS_HUNTING_LODGE", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Svartalfar
        if is_civ("CIVILIZATION_SVARTALFAR") {
            add!("BUILDINGCLASS_HUNTING_LODGE", 250);
            if conquest {
                if conquest_city(min_city_size) {
                    let cq = Conquest::new(g, p, area, reli);
                    conquest_units!([11, 10, 11, 15, 11, 12], &cq, 3);
                    add!("BUILDINGCLASS_HUNTING_LODGE", 12000);
                    conquest_buildings!(5);
                }
            }
        }

        // Trait specific buildings
        if has_trait("TRAIT_PHILOSOPHICAL") {
            add!("BUILDINGCLASS_LIBRARY", 400);
            add!("BUILDINGCLASS_ELDER_COUNCIL", 400);
        }
        if has_trait("TRAIT_SPIRITUAL") {
            add!("BUILDINGCLASS_TEMPLE_OF_KILMORPH", 200);
            add!("BUILDINGCLASS_TEMPLE_OF_LEAVES", 200);
            add!("BUILDINGCLASS_TEMPLE_OF_THE_EMPYREAN", 200);
            add!("BUILDINGCLASS_TEMPLE_OF_THE_OVERLORDS", 200);
            add!("BUILDINGCLASS_TEMPLE_OF_THE_VEIL", 200);
            add!("BUILDINGCLASS_TEMPLE_OF_THE_ORDER", 200);
        }
        if has_trait("TRAIT_AGGRESSIVE") {
            add!("BUILDINGCLASS_SIEGE_WORKSHOP", 200);
        }
        if has_trait("TRAIT_EXPANSIVE") {
            add!("BUILDINGCLASS_GRANARY", 2000);
            add!("BUILDINGCLASS_HARBOR", 2000);
        }
        if has_trait("TRAIT_INDUSTRIOUS") {
            add!("BUILDINGCLASS_FORGE", 200);
        }
        if has_trait("TRAIT_FINANCIAL") {
            add!("BUILDINGCLASS_MONEYCHANGER", 400);
            add!("BUILDINGCLASS_MARKET", 400);
        }
        if has_trait("TRAIT_ARCANE") {
            add!("BUILDINGCLASS_MAGE_GUILD", 200);
            // Python float from here on.
            v[ix("UNITCLASS_ADEPT")] *= 1.2;
        }
        if has_trait("TRAIT_DEFENDER") {
            add!("BUILDINGCLASS_PALISADE", 200);
            add!("BUILDINGCLASS_WALLS", 200);
        }

        // general happy buildings
        // Civic specific choices
        let happy_level = g.city_happy_level(c);
        let unhappy_level = g.city_unhappy_level(c, 0);
        let mut unhappy = 100 * (unhappy_level + 2 - happy_level);
        if unhappy_level + 1 > happy_level {
            // only apply Specialization bonus if Player is well established
            if num_cities > 3 || player.num_units() > 20 {
                unhappy += specialization;
            }
        }
        let cultural_values = player.civics[d.t::<CivicOptionType>("CIVICOPTION_CULTURAL_VALUES")];
        if cultural_values == Some(d.t::<CivicType>("CIVIC_NATIONHOOD")) {
            add!("BUILDINGCLASS_TRAINING_YARD", 200 + unhappy);
        }

        add!("BUILDINGCLASS_THEATRE", 500 + unhappy);
        if cultural_values == Some(d.t::<CivicType>("CIVIC_CONSUMPTION")) {
            add!("BUILDINGCLASS_THEATRE", 200);
            add!("BUILDINGCLASS_MARKET", 400);
        }

        add!("BUILDINGCLASS_PUBLIC_BATHS", 700 + unhappy);
        add!("BUILDINGCLASS_BASILICA", 100 + unhappy);
        add!("BUILDINGCLASS_CARNIVAL", 150 + unhappy);

        // Growth Buildings: Granary, Smokehouse
        let mut health_factor = (g.city_bad_health(c, false, 0) - g.city_good_health(c)) * 200;
        if health_factor < 0 {
            health_factor = 0;
        }
        let mut happy_factor = (happy_level - unhappy_level) * 200;
        if happy_factor < 0 {
            happy_factor = 0;
        }
        add!("BUILDINGCLASS_GRANARY", 200 + health_factor + happy_factor);
        add!("BUILDINGCLASS_SMOKEHOUSE", 200 + health_factor + happy_factor);

        if happy_factor > 400 && health_factor > 0 {
            add!("BUILDINGCLASS_GRANARY", 4000 + specialization);
            add!("BUILDINGCLASS_SMOKEHOUSE", 4000 + specialization);
        }

        let available = |n: &str| g.player_get_num_available_bonuses(p, d.t::<BonusType>(n)) > 0;
        for n in ["BONUS_WHEAT", "BONUS_CORN", "BONUS_RICE"] {
            if available(n) {
                add!("BUILDINGCLASS_GRANARY", 100);
            }
        }
        for n in ["BONUS_SHEEP", "BONUS_PIG", "BONUS_COW"] {
            if available(n) {
                add!("BUILDINGCLASS_SMOKEHOUSE", 100);
            }
        }

        let base_gold = g.city_base_commerce_rate(c, CommerceType::COMMERCE_GOLD);
        let base_research = g.city_base_commerce_rate(c, CommerceType::COMMERCE_RESEARCH);
        let maintenance_times100 = g.cities[c].maintenance;
        add!("BUILDING_SMUGGLERS_PORT", py_div(750, 16) * base_gold);
        add!("BUILDINGCLASS_BASILICA", 100 * maintenance_times100);

        // General Building Values
        // Commerce Buildings
        add!("BUILDINGCLASS_ELDER_COUNCIL", 700);
        if num_cities < 5 {
            add!("BUILDINGCLASS_ELDER_COUNCIL", 15000);
        }
        add!("BUILDINGCLASS_LIBRARY", py_div(700, 16) * base_research + specialization);
        add!("BUILDINGCLASS_ALCHEMY_LAB", py_div(700, 36) * base_research + specialization);

        if player.gold < 500 {
            add!("BUILDINGCLASS_MARKET", 900);
            if num_cities < 5 {
                add!("BUILDINGCLASS_MARKET", 15000);
            }
            add!("BUILDINGCLASS_MONEYCHANGER", py_div(900, 33) * base_gold);
            add!("BUILDINGCLASS_TAX_OFFICE", -200 + py_div(900, 27) * base_gold);
            add!("BUILDINGCLASS_GAMBLING_HOUSE", 200 + 15 * pop);
        }
        let trade_routes = g.city_trade_routes(c);
        let trade_modifier = g.city_total_trade_modifier(c, None);
        add!("BUILDINGCLASS_HARBOR", 100 * trade_routes);
        add!("BUILDINGCLASS_LIGHTHOUSE", trade_modifier + 50 * pop);
        add!("BUILDINGCLASS_INN", 100 * trade_routes + trade_modifier);
        add!("BUILDINGCLASS_TAVERN", 110 * trade_routes + trade_modifier);

        add!("BUILDINGCLASS_COURTHOUSE", maintenance_times100);
        add!("BUILDINGCLASS_DUNGEON", 200 * g.cities[c].war_weariness_modifier);
        // Production Buildings
        add!("BUILDINGCLASS_FORGE", 60 * prod_diff + specialization);
        if prod_diff > 15 {
            add!("BUILDINGCLASS_FORGE", 2000);
            if conquest {
                add!("BUILDINGCLASS_FORGE", 10000);
            }
            add!("BUILDING_COMMAND_POST", 250 + 15 * prod_diff);
            if conquest {
                add!("BUILDING_COMMAND_POST", 10000);
            }
        }

        // Defense Buildings
        add!("BUILDINGCLASS_PALISADE", 250 + 15 * pop);
        add!("BUILDINGCLASS_WALLS", 200 + 15 * pop);
        // Religion Buildings

        // Wonders and Rituals
        let iwondermod = 2000;
        let mut iconquestmod = 0;
        if conquest && flag10 < num_cities * i_dif {
            iconquestmod = -2000;
        }

        if num_cities > 3 {
            if bccpm("BUILDINGCLASS_BREWERY") == 0 {
                add!("BUILDINGCLASS_BREWERY", 250 + 15 * prod_diff);
            }
        }
        if bccpm("BUILDINGCLASS_HERON_THRONE") == 0 {
            add!("BUILDINGCLASS_HERON_THRONE", 250 + 15 * pop);
            if g.city_is_coastal(c, 10) {
                if num_cities > 2 {
                    add!("BUILDINGCLASS_HERON_THRONE", 1000);
                }
            }
        }
        if bccpm("BUILDINGCLASS_GREAT_LIGHTHOUSE") == 0 {
            add!("BUILDINGCLASS_GREAT_LIGHTHOUSE", 250 + 15 * pop);
        }
        if player.favorite_religion == rel("RELIGION_THE_ORDER") {
            if bccpm("BUILDINGCLASS_MERCURIAN_GATE") == 0 {
                add!("BUILDINGCLASS_MERCURIAN_GATE", iwondermod);
            }
        }
        if maintenance_times100 > 600 {
            if bccpm("BUILDINGCLASS_FORBIDDEN_PALACE") == 0 {
                add!("BUILDINGCLASS_FORBIDDEN_PALACE", iwondermod + iconquestmod);
            }
        }
        if g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_RESEARCH) < 3 {
            if bccpm("BUILDINGCLASS_CROWN_OF_AKHARIEN") == 0 {
                add!("BUILDINGCLASS_CROWN_OF_AKHARIEN", iwondermod + iconquestmod);
            }
        }
        if g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_GOLD) == 1 {
            if bccpm("BUILDINGCLASS_BAZAAR_OF_MAMMON") == 0 {
                add!("BUILDINGCLASS_BAZAAR_OF_MAMMON", iwondermod + iconquestmod);
                if g.city_is_any_holy_city(c) {
                    add!("BUILDINGCLASS_BAZAAR_OF_MAMMON", 2000);
                }
            }
        }
        if prod_rank < 3 {
            // (GUILD_OF_THE_NINE, CITY_OF_A_THOUSAND_SLUMS, MOKKAS_CAULDRON, SHRINE_OF_SIRONA
            // and CATACOMB_LIBRALUS get `+= 0` when not built.)
            for (class, bonus) in [
                ("BUILDINGCLASS_THEATRE_OF_DREAMS", iwondermod + iconquestmod),
                ("BUILDINGCLASS_PILLAR_OF_CHAINS", iwondermod + iconquestmod),
                ("BUILDINGCLASS_GUILD_OF_HAMMERS", iwondermod),
                ("BUILDINGCLASS_GRAND_MENAGERIE", iwondermod + iconquestmod),
                ("BUILDINGCLASS_CELESTIAL_COMPASS", iwondermod + iconquestmod),
                ("BUILDINGCLASS_RIDE_OF_THE_NINE_KINGS", iwondermod),
                ("BUILDINGCLASS_EYES_AND_EARS_NETWORK", iwondermod + iconquestmod),
                ("BUILDINGCLASS_SHRINE_OF_THE_CHAMPION", iwondermod),
                ("BUILDINGCLASS_AQUAE_SUCELLUS", iwondermod + iconquestmod),
                ("BUILDINGCLASS_HALL_OF_KINGS", iwondermod + iconquestmod),
                ("BUILDINGCLASS_GREAT_LIBRARY", iwondermod + iconquestmod),
                ("BUILDINGCLASS_TEMPLE_OF_TEMPORENCE", iwondermod + iconquestmod),
            ] {
                if bccpm(class) == 0 {
                    add!(class, bonus);
                }
            }
            // C++ quirk: the Python gates the Heroic Epic on the Temple of Temporence.
            if bccpm("BUILDINGCLASS_TEMPLE_OF_TEMPORENCE") == 0 {
                add!("BUILDINGCLASS_HEROIC_EPIC", iwondermod);
            }
            for (class, bonus) in [
                ("BUILDINGCLASS_NATIONAL_EPIC", iwondermod + iconquestmod),
                ("BUILDINGCLASS_TOWER_OF_COMPLACENCY", iwondermod),
                ("BUILDINGCLASS_TOWER_OF_EYES", iwondermod + iconquestmod),
                ("BUILDINGCLASS_SYLIVENS_PERFECT_LYRE", iwondermod + iconquestmod),
                ("BUILDINGCLASS_FORM_OF_THE_TITAN", iwondermod),
                ("BUILDINGCLASS_MINES_OF_GALDUR", iwondermod),
            ] {
                if bccpm(class) == 0 {
                    add!(class, bonus);
                }
            }

            add!("PROJECT_BLOOD_OF_THE_PHOENIX", iwondermod);
            add!("PROJECT_BIRTHRIGHT_REGAINED", iwondermod);

            if num_cities > 10 {
                add!("PROJECT_GENESIS", iwondermod);
            }
            add!("PROJECT_NATURES_REVOLT", iwondermod);
            if is_civ("CIVILIZATION_DOVIELLO") {
                add!("PROJECT_NATURES_REVOLT", 10000);
            }
            if is_civ("CIVILIZATION_LJOSALFAR") {
                add!("PROJECT_NATURES_REVOLT", 10000);
            }
            if is_civ("CIVILIZATION_SVARTALFAR") {
                add!("PROJECT_NATURES_REVOLT", 10000);
            }

            if leader.religion_victory {
                if state_religion == player.favorite_religion && conquest {
                    set!("PROJECT_PURGE_THE_UNFAITHFUL", 17000);
                }
            }

            if num_cities > 3 {
                add!("PROJECT_SAMHAIN", 17000);
            }
            if num_cities > 5 {
                add!("PROJECT_THE_WHITE_HAND", 17000);
            }
            if num_cities > 5 {
                add!("PROJECT_THE_DEEPENING", 17000);
            }
            set!("PROJECT_STIR_FROM_SLUMBER", 17000);
            set!("PROJECT_THE_DRAW", 17000);
            set!("PROJECT_ASCENSION", 17000);

            if is_civ("CIVILIZATION_SVARTALFAR") {
                add!("PROJECT_PACT_OF_THE_NILHORN", 10000);
            }
            if is_civ("CIVILIZATION_LJOSALFAR") {
                add!("PROJECT_PACT_OF_THE_NILHORN", 10000);
            }
        }

        // Buildings only for Big Cities
        add!("BUILDINGCLASS_AQUEDUCT", 15 * pop);
        add!("BUILDINGCLASS_GROVE", 15 * pop);
        add!("BUILDINGCLASS_HERBALIST", 150 + 5 * pop);

        // Victory Conditions

        // Altar Victory
        set!("BUILDINGCLASS_ALTAR_OF_THE_LUONNOTAR_FINAL", 35000);

        // Arcane Tower Victory
        if prod_rank < 3 {
            if leader.arcane_tower_victory {
                set!("PROJECT_RITES_OF_OGHMA", 17000);
            }
            for class in [
                "BUILDINGCLASS_TOWER_OF_ALTERATION",
                "BUILDINGCLASS_TOWER_OF_DIVINATION",
                "BUILDINGCLASS_TOWER_OF_NECROMANCY",
                "BUILDINGCLASS_TOWER_OF_THE_ELEMENTS",
            ] {
                if bccpm(class) == 0 {
                    set!(class, 17000);
                }
            }
            set!("BUILDINGCLASS_TOWER_OF_MASTERY", 35000);
        }

        // Culture Victory
        let mut culture_important = false;
        if (leader.culture_victory && conquest) && g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE) < 6 {
            culture_important = true;
        }
        if g.city_calculate_culture_percent(c, p) < 65 {
            culture_important = true;
        }
        if culture_important {
            for class in [
                "BUILDINGCLASS_CARNIVAL",
                "BUILDINGCLASS_GROVE",
                "BUILDINGCLASS_INN",
                "BUILDINGCLASS_LIBRARY",
                "BUILDINGCLASS_MONUMENT",
                "BUILDINGCLASS_PAGAN_TEMPLE",
                "BUILDINGCLASS_TAVERN",
                "BUILDINGCLASS_THEATRE",
                "BUILDINGCLASS_TEMPLE_OF_LEAVES",
                "BUILDINGCLASS_TEMPLE_OF_KILMORPH",
                "BUILDINGCLASS_TEMPLE_OF_THE_EMPYREAN",
                "BUILDINGCLASS_TEMPLE_OF_THE_OVERLORDS",
                "BUILDINGCLASS_TEMPLE_OF_THE_VEIL",
                "BUILDINGCLASS_TEMPLE_OF_THE_ORDER",
            ] {
                add!(class, 2000);
            }
        }

        // Religion Victory
        if leader.religion_victory {
            if !agnostic {
                if state_religion == player.favorite_religion {
                    if g.player_count_group_flag_units(p, 15) < py_div(num_cities, 5) {
                        for class in [
                            "UNITCLASS_PRIEST_OF_THE_VEIL",
                            "UNITCLASS_PRIEST_OF_THE_ORDER",
                            "UNITCLASS_PRIEST_OF_KILMORPH",
                            "UNITCLASS_PRIEST_OF_LEAVES",
                            "UNITCLASS_PRIEST_OF_THE_EMPYREAN",
                            "UNITCLASS_PRIEST_OF_THE_OVERLORDS",
                        ] {
                            if val!(class) < 4000.0 {
                                set!(class, 4000);
                            }
                        }
                    }
                }
            }
        }

        // Value higher Tier units more
        if val!("UNITCLASS_WARRIOR") > val!("UNITCLASS_AXEMAN") {
            set!("UNITCLASS_AXEMAN", val!("UNITCLASS_WARRIOR") + 2.0);
        }
        if val!("UNITCLASS_WARRIOR") > val!("UNITCLASS_ARCHER") {
            set!("UNITCLASS_ARCHER", val!("UNITCLASS_WARRIOR") + 2.0);
        }
        if val!("UNITCLASS_WARRIOR") > val!("UNITCLASS_HUNTER") {
            set!("UNITCLASS_HUNTER", val!("UNITCLASS_WARRIOR") + 2.0);
        }

        if val!("UNITCLASS_AXEMAN") > 0.0 {
            add!("UNITCLASS_CHAMPION", val!("UNITCLASS_AXEMAN") + 10.0);
        }

        if val!("UNITCLASS_ARCHER") > 0.0 {
            add!("UNITCLASS_LONGBOWMAN", val!("UNITCLASS_ARCHER") + 10.0);
        }

        if val!("UNITCLASS_HUNTER") > 0.0 {
            let rangers = uccpm("UNITCLASS_RANGER");
            let assassins = uccpm("UNITCLASS_ASSASSIN");
            add!("UNITCLASS_RANGER", val!("UNITCLASS_HUNTER") + 10.0 + assassins as f64);
            add!("UNITCLASS_ASSASSIN", val!("UNITCLASS_HUNTER") + 10.0 + rangers as f64);
        }

        if val!("UNITCLASS_HORSEMAN") > 0.0 {
            add!("UNITCLASS_CHARIOT", val!("UNITCLASS_HORSEMAN") + 10.0);
            add!("UNITCLASS_HORSE_ARCHER", val!("UNITCLASS_HORSEMAN") + 10.0);
        }

        if val!("UNITCLASS_CATAPULT") > 0.0 {
            add!("UNITCLASS_CANNON", val!("UNITCLASS_CATAPULT") + 5.0);
            add!("UNITCLASS_PRIEST_OF_THE_VEIL", val!("UNITCLASS_CATAPULT") + 10.0);
        }

        // The best item the city can build. Names the data lacks (Python -1: the machinist's
        // shop and the explorer) keep the value 0 and can never pass `> iBestBuildingValue`.
        let mut best: Option<(OrderType, i32)> = None;
        let mut best_value = 0.0f64;
        for (i, &(name, kind)) in PROD.iter().enumerate() {
            if v[i] > best_value {
                if v[i] > building_thres {
                    let item = match kind {
                        0 => d
                            .ty::<BuildingType>(name)
                            .filter(|&b| g.city_can_construct(c, b, true, false, false))
                            .map(|b| (OrderType::ORDER_CONSTRUCT, b.i())),
                        1 => d
                            .ty::<BuildingClassType>(name)
                            .and_then(|class| d.civ_building(civtype, class))
                            .filter(|&b| g.city_can_construct(c, b, true, false, false))
                            .map(|b| (OrderType::ORDER_CONSTRUCT, b.i())),
                        2 => d
                            .ty::<UnitClassType>(name)
                            .and_then(|class| d.civ_unit(civtype, class))
                            .filter(|&u| g.city_can_train(c, u, true, false, false, false))
                            .map(|u| (OrderType::ORDER_TRAIN, u.i())),
                        _ => d
                            .ty::<ProjectType>(name)
                            .filter(|&pr| g.city_can_create(c, pr, true, false))
                            .map(|pr| (OrderType::ORDER_CREATE, pr.i())),
                    };
                    if item.is_some() {
                        best_value = v[i];
                        best = item;
                    }
                }
            }
        }

        best
    }
}
