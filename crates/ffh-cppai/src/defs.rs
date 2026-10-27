//! Constants of the C++ AI: `AI_Defines.h` and the BETTER AI (Sephi) block of `CvDefines.h`.
//! (`GROUPFLAG_*` and other rules-visible defines live in `ffh_core::defs`.)

pub use ffh_core::defs::NUM_CITY_PLOTS;

pub const DEFAULT_PLAYER_CLOSENESS: i32 = 6;
pub const AI_DAGGER_THRESHOLD: i32 = 100;

pub const AI_DEFAULT_STRATEGY: i32 = 1 << 0;
pub const AI_STRATEGY_DAGGER: i32 = 1 << 1;
pub const AI_STRATEGY_SLEDGEHAMMER: i32 = 1 << 2;
pub const AI_STRATEGY_CASTLE: i32 = 1 << 3;
pub const AI_STRATEGY_FASTMOVERS: i32 = 1 << 4;
pub const AI_STRATEGY_SLOWMOVERS: i32 = 1 << 5;
pub const AI_STRATEGY_CULTURE1: i32 = 1 << 6;
pub const AI_STRATEGY_CULTURE2: i32 = 1 << 7;
pub const AI_STRATEGY_CULTURE3: i32 = 1 << 8;
pub const AI_STRATEGY_CULTURE4: i32 = 1 << 9;
pub const AI_STRATEGY_MISSIONARY: i32 = 1 << 10;
pub const AI_STRATEGY_CRUSH: i32 = 1 << 11;
pub const AI_STRATEGY_PRODUCTION: i32 = 1 << 12;
pub const AI_STRATEGY_PEACE: i32 = 1 << 13;
pub const AI_STRATEGY_GET_BETTER_UNITS: i32 = 1 << 14;
pub const AI_STRATEGY_LAND_BLITZ: i32 = 1 << 15;
pub const AI_STRATEGY_AIR_BLITZ: i32 = 1 << 16;
pub const AI_STRATEGY_LAST_STAND: i32 = 1 << 17;
pub const AI_STRATEGY_FINAL_WAR: i32 = 1 << 18;
pub const AI_STRATEGY_OWABWNW: i32 = 1 << 19;
pub const AI_STRATEGY_BIG_ESPIONAGE: i32 = 1 << 20;

pub const AI_CITY_ROLE_VALID: i32 = 1 << 1;
pub const AI_CITY_ROLE_BIG_CULTURE: i32 = 1 << 2;
pub const AI_CITY_ROLE_BIG_PRODUCTION: i32 = 1 << 3;
pub const AI_CITY_ROLE_BIG_MILITARY: i32 = 1 << 4;
pub const AI_CITY_ROLE_SCIENCE: i32 = 1 << 5;
pub const AI_CITY_ROLE_GOLD: i32 = 1 << 6;
pub const AI_CITY_ROLE_PRODUCTION: i32 = 1 << 7;
pub const AI_CITY_ROLE_SPECIALIST: i32 = 1 << 8;
pub const AI_CITY_ROLE_FISHING: i32 = 1 << 9;
pub const AI_CITY_ROLE_STAGING: i32 = 1 << 10;
pub const AI_CITY_ROLE_LICHPIN: i32 = 1 << 11;

pub const NUM_PERMDEFENDER_TYPES: usize = 4;
pub const NUM_PATROL_TYPES: usize = 2;
pub const NUM_CITYDEFENSE_TIERS: usize = 3;
pub const NUM_PATROL_TIERS: usize = 3;

pub use ffh_core::defs::GROUPFLAG_NONE;
pub const GROUPFLAG_PERMDEFENSE: i32 = 2;
pub const GROUPFLAG_PERMDEFENSE_NEW: i32 = 3;
pub const GROUPFLAG_PATROL: i32 = 4;
pub const GROUPFLAG_PATROL_NEW: i32 = 5;
pub const GROUPFLAG_EXPLORE: i32 = 6;
pub const GROUPFLAG_DEFENSE_NEW: i32 = 7;
pub const GROUPFLAG_DEFENSE: i32 = 8;
pub const GROUPFLAG_HERO: i32 = 9;
pub const GROUPFLAG_CONQUEST: i32 = 10;
pub const GROUPFLAG_SETTLERGROUP: i32 = 11;
pub const GROUPFLAG_HNGROUP: i32 = 12;
pub const GROUPFLAG_FEASTING: i32 = 13;
pub const GROUPFLAG_INQUISITION: i32 = 14;
pub const GROUPFLAG_NAVAL_SETTLE: i32 = 20;
pub const GROUPFLAG_NAVAL_SETTLE_NEW: i32 = 21;
pub const GROUPFLAG_NAVAL_SETTLE_PICKUP: i32 = 22;
pub const GROUPFLAG_NAVAL_SETTLE_REINFORCEMENTS: i32 = 23;
pub const GROUPFLAG_NAVAL_INVASION: i32 = 24;
pub const GROUPFLAG_NAVAL_INVASION_NEW: i32 = 25;
pub const GROUPFLAG_ANIMAL_CONSTRUCTBUILDING: i32 = 40;

// CvPlayerAI.cpp
pub const FOUND_RANGE: i32 = 7;
pub use ffh_core::path::DANGER_RANGE;
pub const GREATER_FOUND_RANGE: i32 = 5;
pub const CIVIC_CHANGE_DELAY: i32 = 25;
pub const RELIGION_CHANGE_DELAY: i32 = 15;

// CvCityAI.cpp
pub const BUILDINGFOCUS_FOOD: i32 = 1 << 1;
pub const BUILDINGFOCUS_PRODUCTION: i32 = 1 << 2;
pub const BUILDINGFOCUS_GOLD: i32 = 1 << 3;
pub const BUILDINGFOCUS_RESEARCH: i32 = 1 << 4;
pub const BUILDINGFOCUS_CULTURE: i32 = 1 << 5;
pub const BUILDINGFOCUS_DEFENSE: i32 = 1 << 6;
pub const BUILDINGFOCUS_HAPPY: i32 = 1 << 7;
pub const BUILDINGFOCUS_HEALTHY: i32 = 1 << 8;
pub const BUILDINGFOCUS_EXPERIENCE: i32 = 1 << 9;
pub const BUILDINGFOCUS_MAINTENANCE: i32 = 1 << 10;
pub const BUILDINGFOCUS_SPECIALIST: i32 = 1 << 11;
pub const BUILDINGFOCUS_ESPIONAGE: i32 = 1 << 12;
pub const BUILDINGFOCUS_BIGCULTURE: i32 = 1 << 13;
pub const BUILDINGFOCUS_WORLDWONDER: i32 = 1 << 14;
pub const BUILDINGFOCUS_DOMAINSEA: i32 = 1 << 15;
pub const BUILDINGFOCUS_WONDEROK: i32 = 1 << 16;
pub const BUILDINGFOCUS_CAPITAL: i32 = 1 << 17;
