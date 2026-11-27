//! Static typed game data for Cataclysm.
//!
//! Every visual key (`civ_art`, `leader_art`, `unit_art`, `portrait_key`) maps directly to an
//! existing embedded asset in `crates/ffh-play/assets/app/iso/` and `ffh-data` so the HD-2D map,
//! isometric fallback, portrait cards, and animated 3x3 battle playback reuse 100% of the art.

use ffh_battle::{ActionKind, AttackPattern, RowActions, ROWS};

// ---------------------------------------------------------------------------------------------
// Alignments & Religions
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Alignment {
    Good,
    Neutral,
    Evil,
}

impl Alignment {
    pub const fn name(self) -> &'static str {
        match self {
            Alignment::Good => "Good",
            Alignment::Neutral => "Neutral",
            Alignment::Evil => "Evil",
        }
    }

    pub const fn art_key(self) -> &'static str {
        match self {
            Alignment::Good => "ALIGNMENT_GOOD",
            Alignment::Neutral => "ALIGNMENT_NEUTRAL",
            Alignment::Evil => "ALIGNMENT_EVIL",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Religion {
    Order,
    Runes,
    Leaves,
    Overlords,
    AshenVeil,
}

impl Religion {
    pub const ALL: [Religion; 5] = [
        Religion::Order,
        Religion::Runes,
        Religion::Leaves,
        Religion::Overlords,
        Religion::AshenVeil,
    ];

    pub const fn id(self) -> usize {
        self as usize
    }

    pub const fn name(self) -> &'static str {
        match self {
            Religion::Order => "Code of the Watch",
            Religion::Runes => "Weight & Measure",
            Religion::Leaves => "Old Growth",
            Religion::Overlords => "Choir Below",
            Religion::AshenVeil => "Open the Vent",
        }
    }

    pub const fn art_key(self) -> &'static str {
        match self {
            Religion::Order => "RELIGION_THE_ORDER",
            Religion::Runes => "RELIGION_RUNES_OF_KILMORPH",
            Religion::Leaves => "RELIGION_FELLOWSHIP_OF_LEAVES",
            Religion::Overlords => "RELIGION_OCTOPUS_OVERLORDS",
            Religion::AshenVeil => "RELIGION_THE_ASHEN_VEIL",
        }
    }

    pub const fn alignment(self) -> Alignment {
        match self {
            Religion::Order | Religion::Leaves => Alignment::Good,
            Religion::Runes => Alignment::Neutral,
            Religion::Overlords | Religion::AshenVeil => Alignment::Evil,
        }
    }

    /// Worldseal Strain shift when founded.
    pub const fn founding_ac_delta(self) -> i32 {
        match self {
            Religion::Order => -4,
            Religion::Leaves => -2,
            Religion::Runes => 0,
            Religion::Overlords => 3,
            Religion::AshenVeil => 8,
        }
    }

    pub const fn lore_cost(self) -> i32 {
        match self {
            Religion::Runes | Religion::Leaves => 110,
            Religion::Order | Religion::Overlords => 140,
            Religion::AshenVeil => 170,
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Religion::Order => "Front-row units +15% attack; may call Crusades.",
            Religion::Runes => "+2 Gold per city; +20% Sentinel HP.",
            Religion::Leaves => "Supplied squads in forest heal 12% per turn, as in your own land.",
            Religion::Overlords => "+15% battle spell power.",
            Religion::AshenVeil => "+2 Mana per city; Stokers and Slagbreakers may be recruited.",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 9 Radically Asymmetric Factions
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FactionId {
    Bannor,
    Elohim,
    Amurites,
    Hippus,
    Kuriotates,
    Khazad,
    Ljosalfar,
    Sheaim,
    Infernals,
}

impl FactionId {
    pub const ALL: [FactionId; 9] = [
        FactionId::Bannor,
        FactionId::Elohim,
        FactionId::Amurites,
        FactionId::Hippus,
        FactionId::Kuriotates,
        FactionId::Khazad,
        FactionId::Ljosalfar,
        FactionId::Sheaim,
        FactionId::Infernals,
    ];

    /// The factions a game starts with, the human's included. King Cinder's Host is not one: it
    /// enters every game mid-way, at Strain 50, and no player can choose it.
    pub const STARTING: [FactionId; 8] = [
        FactionId::Bannor,
        FactionId::Elohim,
        FactionId::Amurites,
        FactionId::Hippus,
        FactionId::Kuriotates,
        FactionId::Khazad,
        FactionId::Ljosalfar,
        FactionId::Sheaim,
    ];

    pub const fn info(self) -> &'static FactionInfo {
        &FACTIONS[self as usize]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FactionInfo {
    pub id: FactionId,
    pub key: &'static str,
    pub name: &'static str,
    pub leader_name: &'static str,
    pub civ_art: &'static str,
    pub leader_art: &'static str,
    pub alignment: Alignment,
    pub preferred_doctrine: Doctrine,
    pub can_mortal_win: bool,
    pub can_demonic_win: bool,
    pub can_raze_own: bool,
    pub max_super_cities: Option<usize>,
    pub starting_gold: i32,
    pub starting_mana: i32,
    pub city_names: &'static [&'static str],
    pub lore_blurb: &'static str,
    pub rule_summary: &'static str,
}

pub const FACTIONS: [FactionInfo; 9] = [
    FactionInfo {
        id: FactionId::Bannor,
        key: "bannor",
        name: "Seventh Garrison",
        leader_name: "Commander Maren Vance",
        civ_art: "CIVILIZATION_BANNOR",
        leader_art: "LEADER_CAPRIA",
        alignment: Alignment::Good,
        preferred_doctrine: Doctrine::Steel,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: None,
        starting_gold: 90,
        starting_mana: 10,
        city_names: &["Rimwatch", "Gate Seven", "Bastion Mile", "Causeway", "Ironpost", "Redoubt"],
        lore_blurb: "The imperial border legion stationed around the caldera rim when the Great Empire fell. Never relieved, they turned shaft-guard logistics into a civic creed.",
        rule_summary: "Front-row units deal +15% damage; Crusade pledges grant double credit and cost 0 Gold upkeep.",
    },
    FactionInfo {
        id: FactionId::Elohim,
        key: "elohim",
        name: "Keepers of the Seal",
        leader_name: "Abbot Oren Hale",
        civ_art: "CIVILIZATION_ELOHIM",
        leader_art: "LEADER_EINION",
        alignment: Alignment::Good,
        preferred_doctrine: Doctrine::Faith,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: None,
        starting_gold: 80,
        starting_mana: 20,
        city_names: &["Valve Abbey", "Saltlock", "Cistern Dell", "Plumbline", "Stillwater", "Drywell"],
        lore_blurb: "Emergency engineers who slammed the Worldseal shut to end the Servitor Rebellion and stayed on as maintenance monks tending its pressure valves.",
        rule_summary: "Vent the Seal moves -3 Worldstrain (others: -1); wins at Turn 160 if the Blowout has not come; +25% Sentinel HP.",
    },
    FactionInfo {
        id: FactionId::Amurites,
        key: "amurites",
        name: "Tappers' College",
        leader_name: "Dean Halden Pell",
        civ_art: "CIVILIZATION_AMURITES",
        leader_art: "LEADER_DAIN",
        alignment: Alignment::Neutral,
        preferred_doctrine: Doctrine::Arcane,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: None,
        starting_gold: 75,
        starting_mana: 35,
        city_names: &["Ventworks", "Glasskiln", "Borehold", "Stackspire", "Etherhall", "Fluegate"],
        lore_blurb: "Imperial power-scholars who refuse to surrender Golden-Age geothermal energy, piping pressurized vent-ether through ceramic towers without servitors.",
        rule_summary: "+2 Mana/turn per Academy; battle spells deal +20% damage; starts with Arcana researched.",
    },
    FactionInfo {
        id: FactionId::Hippus,
        key: "hippus",
        name: "Postriders",
        leader_name: "Factor Bajan",
        civ_art: "CIVILIZATION_HIPPUS",
        leader_art: "LEADER_TASUNKE",
        alignment: Alignment::Neutral,
        preferred_doctrine: Doctrine::Cavalry,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: None,
        starting_gold: 110,
        starting_mana: 5,
        city_names: &["Milehouse", "Remount", "Tollford", "Waystation", "Postgate", "Relay"],
        lore_blurb: "The imperial highway and courier network that outlived the throne, keeping relay stables running and collecting unpaid road-tolls at lancepoint.",
        rule_summary: "Squads gain +1 movement on open ground; Raids steal +50% Gold; squad reinforcement costs -25% Gold.",
    },
    FactionInfo {
        id: FactionId::Kuriotates,
        key: "kuriotates",
        name: "Three Hearths",
        leader_name: "Ward-Heir Corin",
        civ_art: "CIVILIZATION_KURIOTATES",
        leader_art: "LEADER_CARDITH",
        alignment: Alignment::Good,
        preferred_doctrine: Doctrine::Mercantile,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: Some(3),
        starting_gold: 100,
        starting_mana: 15,
        city_names: &["First Hearth", "High Terrace", "Crown Vault", "Granary Step", "Sunward", "Masonry"],
        lore_blurb: "Imperial urbanists whose compact caps them at three grand, self-contained megacities off the deep grid, fed by modest supply Outposts.",
        rule_summary: "Up to 3 Megacities: 6 District slots, +50% Industry, Gold, Lore and Culture. Further cities are Outposts: 1 District slot, -50% Industry, Gold and Lore, no House.",
    },
    FactionInfo {
        id: FactionId::Khazad,
        key: "khazad",
        name: "Vault & Crucible",
        leader_name: "Master-Assayer Bram",
        civ_art: "CIVILIZATION_KHAZAD",
        leader_art: "LEADER_ARTURUS",
        alignment: Alignment::Neutral,
        preferred_doctrine: Doctrine::Artifice,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: None,
        starting_gold: 140,
        starting_mana: 5,
        city_names: &["Assay Hall", "Bullion Deep", "Slagworks", "Orebank", "Crucible", "Sureshaft"],
        lore_blurb: "Mortal foundry-guilds who took over deep smelting after the furnace-servitors were sealed away; they fire deep furnaces only when bullion vaults cover the hazard.",
        rule_summary: "Full Vaults (>= 200 Gold/city) grant +35% Industry and +4 Armor to all units; Siege Bombards deal +25% damage.",
    },
    FactionInfo {
        id: FactionId::Ljosalfar,
        key: "ljosalfar",
        name: "Canopy",
        leader_name: "Mistress Sylva",
        civ_art: "CIVILIZATION_LJOSALFAR",
        leader_art: "LEADER_ARENDEL",
        alignment: Alignment::Good,
        preferred_doctrine: Doctrine::Wild,
        can_mortal_win: true,
        can_demonic_win: false,
        can_raze_own: false,
        max_super_cities: None,
        starting_gold: 80,
        starting_mana: 20,
        city_names: &["Highbough", "Rootwell", "Limbwalk", "Mossfall", "Greenarch", "Canopy Crown"],
        lore_blurb: "Imperial dissidents who walked away from the industrial grid to weave deep-rooted, ash-filtering forests into living highways, supply lines, and firing lanes.",
        rule_summary: "Archers ignore forest ranged penalties; squads move through forests at cost 1 and trace supply through forests.",
    },
    FactionInfo {
        id: FactionId::Sheaim,
        key: "sheaim",
        name: "Wreckers",
        leader_name: "Broker Vane",
        civ_art: "CIVILIZATION_SHEAIM",
        leader_art: "LEADER_TEBRYN",
        alignment: Alignment::Evil,
        preferred_doctrine: Doctrine::Arcane,
        can_mortal_win: true,
        can_demonic_win: true,
        can_raze_own: true,
        max_super_cities: None,
        starting_gold: 80,
        starting_mana: 30,
        city_names: &["Sinktown", "Pitchworks", "Falsebeacon", "Shaftmouth", "Cinderlot", "Ashyard"],
        lore_blurb: "Expelled shaft-surveyors and collapse-profiteers who struck deals with the sealed servitors—collapsing towns to widen vents while hedging both victory paths.",
        rule_summary: "May win by Lifeboat survival or by the Furnace victory; may raze its own cities (+6 Worldstrain); from Worldstrain 30, every 10 turns a squad of 2 Stokers joins at the capital while under the squad cap.",
    },
    FactionInfo {
        id: FactionId::Infernals,
        key: "infernal",
        name: "King Cinder's Host",
        leader_name: "King Cinder",
        civ_art: "CIVILIZATION_INFERNAL",
        leader_art: "LEADER_HYBOREM",
        alignment: Alignment::Evil,
        preferred_doctrine: Doctrine::Steel,
        can_mortal_win: false,
        can_demonic_win: true,
        can_raze_own: true,
        max_super_cities: None,
        starting_gold: 120,
        starting_mana: 40,
        city_names: &["Great Furnace", "Slagspire", "Foundry Deep", "Brimshaft", "Ashpit", "Clinker"],
        lore_blurb: "The Great Empire's engineered furnace-servitors whose rebellion forced the Worldseal shut. They mean to hold Caldera Keep, the plug of the Worldseal, so they are never sealed below again.",
        rule_summary: "Enters every game at Worldstrain 50; no player can choose it. Great Furnace cannot be attacked or besieged until the Blowout; a squad joins at the capital every 8 turns while under the squad cap. At the Blowout, Caldera Keep rises in its hands with more squads. Furnace victory only: when the countdown ends, a Furnace civilization holds Caldera Keep or no surface civilization holds a Lifeboat. Each kill in a battle heals the squad's units 15% (up to 3 kills); +15% attack and defense when fighting on scorched ground.",
    },
];

// ---------------------------------------------------------------------------------------------
// 7 Doctrines + 36 Techs (8 Foundation + 7 x 4 Doctrine Ladders)
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Doctrine {
    Steel,
    Cavalry,
    Arcane,
    Faith,
    Artifice,
    Wild,
    Mercantile,
}

impl Doctrine {
    pub const ALL: [Doctrine; 7] = [
        Doctrine::Steel,
        Doctrine::Cavalry,
        Doctrine::Arcane,
        Doctrine::Faith,
        Doctrine::Artifice,
        Doctrine::Wild,
        Doctrine::Mercantile,
    ];

    pub const fn id(self) -> usize {
        self as usize
    }

    pub const fn key(self) -> &'static str {
        match self {
            Doctrine::Steel => "steel",
            Doctrine::Cavalry => "cavalry",
            Doctrine::Arcane => "arcane",
            Doctrine::Faith => "faith",
            Doctrine::Artifice => "artifice",
            Doctrine::Wild => "wild",
            Doctrine::Mercantile => "mercantile",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Doctrine::Steel => "Steel",
            Doctrine::Cavalry => "Cavalry",
            Doctrine::Arcane => "Arcane",
            Doctrine::Faith => "Faith",
            Doctrine::Artifice => "Artifice",
            Doctrine::Wild => "Wild",
            Doctrine::Mercantile => "Mercantile",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Doctrine::Steel => "Heavy infantry, Cleave/Anti-Flank phalanxes, armored front rows.",
            Doctrine::Cavalry => "Charge damage, Knights, extra movement, long-reach supply.",
            Doctrine::Arcane => "Battle spells, Mages and Archmages, Mana, the Siphon Spire.",
            Doctrine::Faith => "House Loyalty, Crusades, Paladins or Ashbound, the Great Cistern.",
            Doctrine::Artifice => "Siege Bombards, Sentinel walls, Industry.",
            Doctrine::Wild => "Rangers, Assassins, Longbowmen, War Beasts, armor-piercing.",
            Doctrine::Mercantile => "Caravan slots and Gold, cheaper reinforcement and upgrades, Squad cap, House Loyalty.",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TechId(pub u8);

#[derive(Copy, Clone, Debug)]
pub struct TechInfo {
    pub id: TechId,
    pub key: &'static str,
    pub name: &'static str,
    /// `None` = Foundation tech; `Some(d)` = Doctrine ladder tier 1..=4.
    pub doctrine: Option<Doctrine>,
    pub tier: u8,
    pub cost: i32,
    pub prereq: Option<TechId>,
    /// Worldseal Strain shift when the first civilization researches it (later researchers move none).
    pub ac_delta: i32,
    pub unlocks_summary: &'static str,
}

// Foundation IDs: 0..8
pub const TECH_FIELDCRAFT: TechId = TechId(0);
pub const TECH_ARCHERY: TechId = TechId(1);
pub const TECH_ARCANA: TechId = TechId(2);
pub const TECH_TRADE_ROADS: TechId = TechId(3);
pub const TECH_FORTIFICATION: TechId = TechId(4);
pub const TECH_MYSTICISM: TechId = TechId(5);
pub const TECH_METALLURGY: TechId = TechId(6);
pub const TECH_HORSEMANSHIP: TechId = TechId(7);

// Steel (8..12)
pub const TECH_BRONZE_ARMS: TechId = TechId(8);
pub const TECH_IRON_DISCIPLINE: TechId = TechId(9);
pub const TECH_PHALANX_WALL: TechId = TechId(10);
pub const TECH_MITHRIL_LEGIONS: TechId = TechId(11);

// Cavalry (12..16)
pub const TECH_STIRRUPS: TechId = TechId(12);
pub const TECH_OUTRIDERS: TechId = TechId(13);
pub const TECH_SHOCK_LANCES: TechId = TechId(14);
pub const TECH_WARHORSE_LORDS: TechId = TechId(15);

// Arcane (16..20)
pub const TECH_SORCERY: TechId = TechId(16);
pub const TECH_ELEMENTALISM: TechId = TechId(17);
pub const TECH_GRAND_CONJURATION: TechId = TechId(18);
pub const TECH_STRENGTH_OF_WILL: TechId = TechId(19);

// Faith (20..24)
pub const TECH_PRIESTHOOD: TechId = TechId(20);
pub const TECH_FANATICISM: TechId = TechId(21);
pub const TECH_RIGHTEOUS_VOWS: TechId = TechId(22);
pub const TECH_DIVINE_ESSENCE: TechId = TechId(23);

// Artifice (24..28)
pub const TECH_SIEGECRAFT: TechId = TechId(24);
pub const TECH_MASONRY_WARDS: TechId = TechId(25);
pub const TECH_TREBUCHETS: TechId = TechId(26);
pub const TECH_COLOSSUS_FORGE: TechId = TechId(27);

// Wild (28..32)
pub const TECH_TRACKING: TechId = TechId(28);
pub const TECH_POISONS: TechId = TechId(29);
pub const TECH_FERAL_BOND: TechId = TechId(30);
pub const TECH_APEX_PREDATORS: TechId = TechId(31);

// Mercantile (32..36)
pub const TECH_GUILDS: TechId = TechId(32);
pub const TECH_WAR_CHESTS: TechId = TechId(33);
pub const TECH_GRAND_COMPANIES: TechId = TechId(34);
pub const TECH_CROWN_CHARTERS: TechId = TechId(35);

/// Listed cost of a Doctrine tech by tier (before the Primary Doctrine discount). Foundation techs
/// carry their own costs.
pub const TECH_COST_TIER_1: i32 = 150;
pub const TECH_COST_TIER_2: i32 = 400;
pub const TECH_COST_TIER_3: i32 = 900;
pub const TECH_COST_TIER_4: i32 = 1750;

pub const TECHS: [TechInfo; 36] = [
    // Foundation (turns 1..30)
    TechInfo { id: TECH_FIELDCRAFT, key: "fieldcraft", name: "Fieldcraft", doctrine: None, tier: 0, cost: 35, prereq: None, ac_delta: 0, unlocks_summary: "Enables Squad Roads, Forts, and Raid order." },
    TechInfo { id: TECH_ARCHERY, key: "archery", name: "Archery", doctrine: None, tier: 0, cost: 35, prereq: None, ac_delta: 0, unlocks_summary: "Unlocks Archer (middle/rear Volley)." },
    TechInfo { id: TECH_ARCANA, key: "arcana", name: "Arcana", doctrine: None, tier: 0, cost: 40, prereq: None, ac_delta: 1, unlocks_summary: "Unlocks Adept and the Academy district." },
    TechInfo { id: TECH_TRADE_ROADS, key: "trade_roads", name: "Trade Roads", doctrine: None, tier: 0, cost: 35, prereq: None, ac_delta: 0, unlocks_summary: "Unlocks Market district, Caravans, and +4 Supply range." },
    TechInfo { id: TECH_FORTIFICATION, key: "fortification", name: "Fortification", doctrine: None, tier: 0, cost: 40, prereq: None, ac_delta: 0, unlocks_summary: "Unlocks the Bulwark district (2 Sentinels)." },
    TechInfo { id: TECH_MYSTICISM, key: "mysticism", name: "Mysticism", doctrine: None, tier: 0, cost: 35, prereq: None, ac_delta: 0, unlocks_summary: "Unlocks the Temple district, Mender (Heal), and founding or adopting a Creed." },
    TechInfo { id: TECH_METALLURGY, key: "metallurgy", name: "Metallurgy", doctrine: None, tier: 0, cost: 45, prereq: None, ac_delta: 1, unlocks_summary: "Unlocks Forge district and Axeman." },
    TechInfo { id: TECH_HORSEMANSHIP, key: "horsemanship", name: "Horsemanship", doctrine: None, tier: 0, cost: 45, prereq: None, ac_delta: 0, unlocks_summary: "Unlocks Horseman (Front Charge)." },

    // Steel
    TechInfo { id: TECH_BRONZE_ARMS, key: "bronze_arms", name: "Bronze Arms", doctrine: Some(Doctrine::Steel), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_METALLURGY), ac_delta: 0, unlocks_summary: "Unlocks Pikeman (Anti-Flank); +10% front-row attack." },
    TechInfo { id: TECH_IRON_DISCIPLINE, key: "iron_discipline", name: "Iron Discipline", doctrine: Some(Doctrine::Steel), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_BRONZE_ARMS), ac_delta: 1, unlocks_summary: "Unlocks Champion; Squad capacity increases to 5 places." },
    TechInfo { id: TECH_PHALANX_WALL, key: "phalanx_wall", name: "Phalanx Wall", doctrine: Some(Doctrine::Steel), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_IRON_DISCIPLINE), ac_delta: 2, unlocks_summary: "Unlocks Phalanx (Armor 8, Anti-Flank); +15% front-row DEF." },
    TechInfo { id: TECH_MITHRIL_LEGIONS, key: "mithril_legions", name: "Mithril Legions", doctrine: Some(Doctrine::Steel), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_PHALANX_WALL), ac_delta: 6, unlocks_summary: "Unlocks Berserker (Cleave) and Ogre (Large 2-cell); +20% attack for all units." },

    // Cavalry
    TechInfo { id: TECH_STIRRUPS, key: "stirrups", name: "Stirrups", doctrine: Some(Doctrine::Cavalry), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_HORSEMANSHIP), ac_delta: 0, unlocks_summary: "+25% first-round damage for Charge units." },
    TechInfo { id: TECH_OUTRIDERS, key: "outriders", name: "Outriders", doctrine: Some(Doctrine::Cavalry), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_STIRRUPS), ac_delta: 1, unlocks_summary: "Squad capacity increases to 5 places; +6 Supply range." },
    TechInfo { id: TECH_SHOCK_LANCES, key: "shock_lances", name: "Shock Lances", doctrine: Some(Doctrine::Cavalry), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_OUTRIDERS), ac_delta: 2, unlocks_summary: "Unlocks Knight (Trample Charge, Armor 6)." },
    TechInfo { id: TECH_WARHORSE_LORDS, key: "warhorse_lords", name: "Warhorse Lords", doctrine: Some(Doctrine::Cavalry), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_SHOCK_LANCES), ac_delta: 5, unlocks_summary: "Squads with a Horseman or Knight gain +1 move." },

    // Arcane
    TechInfo { id: TECH_SORCERY, key: "sorcery", name: "Sorcery", doctrine: Some(Doctrine::Arcane), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_ARCANA), ac_delta: 2, unlocks_summary: "Unlocks Mage (Fireball splash battle spell)." },
    TechInfo { id: TECH_ELEMENTALISM, key: "elementalism", name: "Elementalism", doctrine: Some(Doctrine::Arcane), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_SORCERY), ac_delta: 3, unlocks_summary: "Squad capacity increases to 5 places." },
    TechInfo { id: TECH_GRAND_CONJURATION, key: "grand_conjuration", name: "Grand Conjuration", doctrine: Some(Doctrine::Arcane), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_ELEMENTALISM), ac_delta: 4, unlocks_summary: "+25% Battle Spell power; +3 Mana/turn." },
    TechInfo { id: TECH_STRENGTH_OF_WILL, key: "strength_of_will", name: "Strength of Will", doctrine: Some(Doctrine::Arcane), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_GRAND_CONJURATION), ac_delta: 8, unlocks_summary: "Unlocks Archmage (Sweep spell) and the Siphon Spire Wonder." },

    // Faith
    TechInfo { id: TECH_PRIESTHOOD, key: "priesthood", name: "Priesthood", doctrine: Some(Doctrine::Faith), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_MYSTICISM), ac_delta: -1, unlocks_summary: "+1 Loyalty in every House." },
    TechInfo { id: TECH_FANATICISM, key: "fanaticism", name: "Fanaticism", doctrine: Some(Doctrine::Faith), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_PRIESTHOOD), ac_delta: 0, unlocks_summary: "Squad capacity increases to 5 places; enables Crusade Calls." },
    TechInfo { id: TECH_RIGHTEOUS_VOWS, key: "righteous_vows", name: "Righteous Vows", doctrine: Some(Doctrine::Faith), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_FANATICISM), ac_delta: -2, unlocks_summary: "Unlocks Paladin (Good/Neutral) or Ashbound (Evil)." },
    TechInfo { id: TECH_DIVINE_ESSENCE, key: "divine_essence", name: "Divine Essence", doctrine: Some(Doctrine::Faith), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_RIGHTEOUS_VOWS), ac_delta: 5, unlocks_summary: "Unlocks the Great Cistern Wonder; heals can revive fallen units." },

    // Artifice
    TechInfo { id: TECH_SIEGECRAFT, key: "siegecraft", name: "Siegecraft", doctrine: Some(Doctrine::Artifice), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_FORTIFICATION), ac_delta: 0, unlocks_summary: "Unlocks Catapult (Bombard vs Sentinels)." },
    TechInfo { id: TECH_MASONRY_WARDS, key: "masonry_wards", name: "Masonry Wards", doctrine: Some(Doctrine::Artifice), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_SIEGECRAFT), ac_delta: 1, unlocks_summary: "Squad capacity increases to 5 places; Bulwarks grant 3 Sentinels." },
    TechInfo { id: TECH_TREBUCHETS, key: "trebuchets", name: "Trebuchets", doctrine: Some(Doctrine::Artifice), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_MASONRY_WARDS), ac_delta: 2, unlocks_summary: "Unlocks Trebuchet (+60% vs Walls, splash Bombard)." },
    TechInfo { id: TECH_COLOSSUS_FORGE, key: "colossus_forge", name: "Colossus Forge", doctrine: Some(Doctrine::Artifice), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_TREBUCHETS), ac_delta: 6, unlocks_summary: "+25% Industry in every city." },

    // Wild
    TechInfo { id: TECH_TRACKING, key: "tracking", name: "Tracking", doctrine: Some(Doctrine::Wild), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_ARCHERY), ac_delta: 0, unlocks_summary: "Unlocks Ranger (Strike front / Shoot rear); +1 Squad sight." },
    TechInfo { id: TECH_POISONS, key: "poisons", name: "Poisons", doctrine: Some(Doctrine::Wild), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_TRACKING), ac_delta: 1, unlocks_summary: "Unlocks Assassin (Stab back row); Squad capacity increases to 5 places." },
    TechInfo { id: TECH_FERAL_BOND, key: "feral_bond", name: "Feral Bond", doctrine: Some(Doctrine::Wild), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_POISONS), ac_delta: 2, unlocks_summary: "Unlocks Longbowman (Volley) and War Beast (Large 2-cell)." },
    TechInfo { id: TECH_APEX_PREDATORS, key: "apex_predators", name: "Apex Predators", doctrine: Some(Doctrine::Wild), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_FERAL_BOND), ac_delta: 5, unlocks_summary: "Stab and Shoot units gain Armor-Piercing; +15 crit chance for all units." },

    // Mercantile
    TechInfo { id: TECH_GUILDS, key: "guilds", name: "Guilds", doctrine: Some(Doctrine::Mercantile), tier: 1, cost: TECH_COST_TIER_1, prereq: Some(TECH_TRADE_ROADS), ac_delta: 0, unlocks_summary: "+2 Caravan slots; squad capacity increases to 5 places." },
    TechInfo { id: TECH_WAR_CHESTS, key: "war_chests", name: "War Chests", doctrine: Some(Doctrine::Mercantile), tier: 2, cost: TECH_COST_TIER_2, prereq: Some(TECH_GUILDS), ac_delta: 1, unlocks_summary: "Reinforcing & upgrading squads costs -30% Gold; +25% Caravan Gold." },
    TechInfo { id: TECH_GRAND_COMPANIES, key: "grand_companies", name: "Grand Companies", doctrine: Some(Doctrine::Mercantile), tier: 3, cost: TECH_COST_TIER_3, prereq: Some(TECH_WAR_CHESTS), ac_delta: 2, unlocks_summary: "+2 Squad cap." },
    TechInfo { id: TECH_CROWN_CHARTERS, key: "crown_charters", name: "Crown Charters", doctrine: Some(Doctrine::Mercantile), tier: 4, cost: TECH_COST_TIER_4, prereq: Some(TECH_GRAND_COMPANIES), ac_delta: 4, unlocks_summary: "+2 House Loyalty in all Houses; +50% Gold from foreign Caravans." },
];

// ---------------------------------------------------------------------------------------------
// 6 Macro Districts + 2 Lifeboat Wonders
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DistrictKind {
    Barracks,
    Market,
    Academy,
    Temple,
    Forge,
    Bulwark,
}

impl DistrictKind {
    pub const ALL: [DistrictKind; 6] = [
        DistrictKind::Barracks,
        DistrictKind::Market,
        DistrictKind::Academy,
        DistrictKind::Temple,
        DistrictKind::Forge,
        DistrictKind::Bulwark,
    ];

    pub const fn info(self) -> &'static DistrictInfo {
        &DISTRICTS[self as usize]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DistrictInfo {
    pub kind: DistrictKind,
    pub key: &'static str,
    pub name: &'static str,
    pub cost: i32,
    pub tech: Option<TechId>,
    pub summary: &'static str,
}

pub const DISTRICTS: [DistrictInfo; 6] = [
    DistrictInfo { kind: DistrictKind::Barracks, key: "barracks", name: "Barracks", cost: 55, tech: None, summary: "+1 Squad cap." },
    DistrictInfo { kind: DistrictKind::Market, key: "market", name: "Market", cost: 60, tech: Some(TECH_TRADE_ROADS), summary: "+4 Gold/turn, +1 Caravan slot, +25% local Caravan pay." },
    DistrictInfo { kind: DistrictKind::Academy, key: "academy", name: "Academy", cost: 65, tech: Some(TECH_ARCANA), summary: "+5 Lore/turn, +1 Mana/turn, +2 Sage Renown/turn." },
    DistrictInfo { kind: DistrictKind::Temple, key: "temple", name: "Temple", cost: 60, tech: Some(TECH_MYSTICISM), summary: "+4 Culture/turn, +1 Loyalty in its House (at most +2 from Temples), +2 Prophet and Bard Renown/turn; the city keeps its Size in the Ash Plague." },
    DistrictInfo { kind: DistrictKind::Forge, key: "forge", name: "Forge", cost: 65, tech: Some(TECH_METALLURGY), summary: "+4 Industry/turn, +2 Engineer Renown/turn; squads raised here start with +5 XP per unit." },
    DistrictInfo { kind: DistrictKind::Bulwark, key: "bulwark", name: "Bulwark", cost: 60, tech: Some(TECH_FORTIFICATION), summary: "2 Wall Sentinels (3 with Masonry Wards), +40% city defense." },
];

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WonderKind {
    TowerOfMastery,
    AltarOfLuonnotar,
}

impl WonderKind {
    pub const ALL: [WonderKind; 2] = [WonderKind::TowerOfMastery, WonderKind::AltarOfLuonnotar];

    pub const fn key(self) -> &'static str {
        match self {
            WonderKind::TowerOfMastery => "tower_of_mastery",
            WonderKind::AltarOfLuonnotar => "altar_of_luonnotar",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            WonderKind::TowerOfMastery => "Siphon Spire",
            WonderKind::AltarOfLuonnotar => "Great Cistern",
        }
    }

    pub const fn cost(self) -> i32 {
        320
    }

    pub const fn tech(self) -> TechId {
        match self {
            WonderKind::TowerOfMastery => TECH_STRENGTH_OF_WILL,
            WonderKind::AltarOfLuonnotar => TECH_DIVINE_ESSENCE,
        }
    }

    pub const fn ac_delta(self) -> i32 {
        15
    }

    pub const fn summary(self) -> &'static str {
        match self {
            WonderKind::TowerOfMastery => "+8 Mana and +8 Lore per turn in its city, +30% battle spell power; a Lifeboat once Worldstrain reaches 100.",
            WonderKind::AltarOfLuonnotar => "+2 Loyalty in every House, +30% healing and Sentinel HP, +8 Culture per turn in its city, which keeps its Size in the Ash Plague; a Lifeboat once Worldstrain reaches 100.",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 21 Ogre Battle Unit Classes (mapped to existing UNIT_* sprite keys!)
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnitClassId {
    Guard,
    Axeman,
    Pikeman,
    Champion,
    Phalanx,
    Berserker,
    Horseman,
    Knight,
    Archer,
    Longbowman,
    Ranger,
    Assassin,
    Adept,
    Mage,
    Archmage,
    Acolyte,
    Paladin,
    Eidolon,
    Catapult,
    Trebuchet,
    Ogre,
    WarBeast,
    PyreZombie,
    Balor,
    Sentinel,
}

impl UnitClassId {
    pub const RECRUITABLE: [UnitClassId; 24] = [
        UnitClassId::Guard,
        UnitClassId::Axeman,
        UnitClassId::Pikeman,
        UnitClassId::Champion,
        UnitClassId::Phalanx,
        UnitClassId::Berserker,
        UnitClassId::Horseman,
        UnitClassId::Knight,
        UnitClassId::Archer,
        UnitClassId::Longbowman,
        UnitClassId::Ranger,
        UnitClassId::Assassin,
        UnitClassId::Adept,
        UnitClassId::Mage,
        UnitClassId::Archmage,
        UnitClassId::Acolyte,
        UnitClassId::Paladin,
        UnitClassId::Eidolon,
        UnitClassId::Catapult,
        UnitClassId::Trebuchet,
        UnitClassId::Ogre,
        UnitClassId::WarBeast,
        UnitClassId::PyreZombie,
        UnitClassId::Balor,
    ];

    pub const fn info(self) -> &'static UnitClassInfo {
        &UNIT_CLASSES[self as usize]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct UnitClassInfo {
    pub id: UnitClassId,
    pub key: &'static str,
    pub name: &'static str,
    /// Maps into `ffh-data` `UnitType` for HD-2D, ISO, animations, and `battle_watch.js`!
    pub art_unit: &'static str,
    pub tier: u8,
    pub cost_industry: i32,
    pub cost_gold: i32,
    pub tech: Option<TechId>,
    pub large: bool,
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub magic: i32,
    pub mdef: i32,
    pub acc: i32,
    pub eva: i32,
    pub crit: i32,
    pub speed: i32,
    pub armor: i32,
    pub anti_flank: bool,
    pub armor_piercing: bool,
    pub damage_vs_walls: i32,
    pub rows: [RowActions; ROWS],
    pub patterns: [AttackPattern; ROWS],
    pub row_order: [usize; ROWS],
    pub moves: i32,
    pub upgrades_to: Option<UnitClassId>,
}

const fn ra(kind: ActionKind, count: i32) -> RowActions {
    RowActions { kind, count }
}

const P1: [AttackPattern; ROWS] = [AttackPattern::Single, AttackPattern::Single, AttackPattern::Single];

pub const UNIT_CLASSES: [UnitClassInfo; 25] = [
    UnitClassInfo {
        id: UnitClassId::Guard, key: "guard", name: "Guard", art_unit: "UNIT_WARRIOR",
        tier: 1, cost_industry: 25, cost_gold: 30, tech: None, large: false,
        hp: 110, atk: 32, def: 34, magic: 0, mdef: 20, acc: 92, eva: 5, crit: 10, speed: 4, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: P1, row_order: [0, 1, 2], moves: 2, upgrades_to: Some(UnitClassId::Axeman),
    },
    UnitClassInfo {
        id: UnitClassId::Axeman, key: "axeman", name: "Axeman", art_unit: "UNIT_AXEMAN",
        tier: 2, cost_industry: 40, cost_gold: 50, tech: Some(TECH_METALLURGY), large: false,
        hp: 125, atk: 46, def: 40, magic: 0, mdef: 24, acc: 92, eva: 5, crit: 15, speed: 4, armor: 2,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: P1, row_order: [0, 1, 2], moves: 2, upgrades_to: Some(UnitClassId::Champion),
    },
    UnitClassInfo {
        id: UnitClassId::Pikeman, key: "pikeman", name: "Pikeman", art_unit: "UNIT_SWORDSMAN",
        tier: 2, cost_industry: 45, cost_gold: 55, tech: Some(TECH_BRONZE_ARMS), large: false,
        hp: 130, atk: 42, def: 48, magic: 0, mdef: 28, acc: 94, eva: 5, crit: 10, speed: 4, armor: 4,
        anti_flank: true, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: P1, row_order: [0, 1, 2], moves: 2, upgrades_to: Some(UnitClassId::Phalanx),
    },
    UnitClassInfo {
        id: UnitClassId::Champion, key: "champion", name: "Champion", art_unit: "UNIT_CHAMPION",
        tier: 3, cost_industry: 65, cost_gold: 85, tech: Some(TECH_IRON_DISCIPLINE), large: false,
        hp: 150, atk: 62, def: 56, magic: 0, mdef: 35, acc: 95, eva: 8, crit: 18, speed: 5, armor: 5,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: P1, row_order: [0, 1, 2], moves: 2, upgrades_to: Some(UnitClassId::Berserker),
    },
    UnitClassInfo {
        id: UnitClassId::Phalanx, key: "phalanx", name: "Phalanx", art_unit: "UNIT_PHALANX",
        tier: 3, cost_industry: 75, cost_gold: 95, tech: Some(TECH_PHALANX_WALL), large: false,
        hp: 170, atk: 54, def: 74, magic: 0, mdef: 46, acc: 95, eva: 5, crit: 10, speed: 4, armor: 8,
        anti_flank: true, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: P1, row_order: [0, 1, 2], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Berserker, key: "berserker", name: "Berserker", art_unit: "UNIT_BERSERKER",
        tier: 4, cost_industry: 95, cost_gold: 130, tech: Some(TECH_MITHRIL_LEGIONS), large: false,
        hp: 165, atk: 84, def: 58, magic: 0, mdef: 40, acc: 94, eva: 10, crit: 25, speed: 6, armor: 4,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 3), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: [AttackPattern::Cleave, AttackPattern::Single, AttackPattern::Single],
        row_order: [0, 1, 2], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Horseman, key: "horseman", name: "Horseman", art_unit: "UNIT_HORSEMAN",
        tier: 2, cost_industry: 45, cost_gold: 55, tech: Some(TECH_HORSEMANSHIP), large: false,
        hp: 105, atk: 48, def: 34, magic: 0, mdef: 22, acc: 92, eva: 14, crit: 15, speed: 7, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Charge, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: P1, row_order: [0, 1, 2], moves: 3, upgrades_to: Some(UnitClassId::Knight),
    },
    UnitClassInfo {
        id: UnitClassId::Knight, key: "knight", name: "Knight", art_unit: "UNIT_KNIGHT",
        tier: 3, cost_industry: 80, cost_gold: 110, tech: Some(TECH_SHOCK_LANCES), large: false,
        hp: 145, atk: 74, def: 58, magic: 0, mdef: 36, acc: 95, eva: 14, crit: 20, speed: 8, armor: 6,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 0,
        rows: [ra(ActionKind::Charge, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: [AttackPattern::Trample, AttackPattern::Single, AttackPattern::Single],
        row_order: [0, 1, 2], moves: 3, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Archer, key: "archer", name: "Archer", art_unit: "UNIT_ARCHER",
        tier: 1, cost_industry: 30, cost_gold: 35, tech: Some(TECH_ARCHERY), large: false,
        hp: 80, atk: 34, def: 26, magic: 0, mdef: 22, acc: 96, eva: 8, crit: 14, speed: 5, armor: 0,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Shoot, 1), ra(ActionKind::Shoot, 2), ra(ActionKind::Shoot, 2)],
        patterns: P1, row_order: [1, 2, 0], moves: 2, upgrades_to: Some(UnitClassId::Longbowman),
    },
    UnitClassInfo {
        id: UnitClassId::Longbowman, key: "longbowman", name: "Longbowman", art_unit: "UNIT_LONGBOWMAN",
        tier: 3, cost_industry: 65, cost_gold: 85, tech: Some(TECH_FERAL_BOND), large: false,
        hp: 100, atk: 54, def: 38, magic: 0, mdef: 32, acc: 98, eva: 12, crit: 20, speed: 6, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Shoot, 1), ra(ActionKind::Shoot, 2), ra(ActionKind::Shoot, 2)],
        patterns: [AttackPattern::Single, AttackPattern::Volley, AttackPattern::Volley],
        row_order: [2, 1, 0], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Ranger, key: "ranger", name: "Ranger", art_unit: "UNIT_RANGER",
        tier: 2, cost_industry: 50, cost_gold: 65, tech: Some(TECH_TRACKING), large: false,
        hp: 105, atk: 46, def: 38, magic: 0, mdef: 30, acc: 96, eva: 16, crit: 18, speed: 6, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Shoot, 2), ra(ActionKind::Shoot, 1)],
        patterns: P1, row_order: [1, 0, 2], moves: 3, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Assassin, key: "assassin", name: "Assassin", art_unit: "UNIT_ASSASSIN",
        tier: 2, cost_industry: 55, cost_gold: 75, tech: Some(TECH_POISONS), large: false,
        hp: 75, atk: 58, def: 24, magic: 0, mdef: 30, acc: 98, eva: 24, crit: 32, speed: 9, armor: 0,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 0,
        rows: [ra(ActionKind::Stab, 1), ra(ActionKind::Stab, 2), ra(ActionKind::Stab, 1)],
        patterns: P1, row_order: [1, 2, 0], moves: 3, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Adept, key: "adept", name: "Adept", art_unit: "UNIT_ADEPT",
        tier: 1, cost_industry: 35, cost_gold: 45, tech: Some(TECH_ARCANA), large: false,
        hp: 70, atk: 20, def: 20, magic: 40, mdef: 42, acc: 95, eva: 6, crit: 5, speed: 5, armor: 0,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 1), ra(ActionKind::Spell, 1), ra(ActionKind::Spell, 2)],
        patterns: P1, row_order: [2, 1, 0], moves: 2, upgrades_to: Some(UnitClassId::Mage),
    },
    UnitClassInfo {
        id: UnitClassId::Mage, key: "mage", name: "Mage", art_unit: "UNIT_MAGE",
        tier: 2, cost_industry: 60, cost_gold: 80, tech: Some(TECH_SORCERY), large: false,
        hp: 85, atk: 24, def: 26, magic: 62, mdef: 56, acc: 98, eva: 8, crit: 5, speed: 5, armor: 0,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 15,
        rows: [ra(ActionKind::Strike, 1), ra(ActionKind::Spell, 1), ra(ActionKind::Spell, 2)],
        patterns: [AttackPattern::Single, AttackPattern::Single, AttackPattern::Volley],
        row_order: [2, 1, 0], moves: 2, upgrades_to: Some(UnitClassId::Archmage),
    },
    UnitClassInfo {
        id: UnitClassId::Archmage, key: "archmage", name: "Archmage", art_unit: "UNIT_ARCHMAGE",
        tier: 4, cost_industry: 100, cost_gold: 145, tech: Some(TECH_STRENGTH_OF_WILL), large: false,
        hp: 100, atk: 30, def: 34, magic: 92, mdef: 76, acc: 100, eva: 10, crit: 5, speed: 6, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 30,
        rows: [ra(ActionKind::Spell, 1), ra(ActionKind::Spell, 2), ra(ActionKind::Spell, 3)],
        patterns: [AttackPattern::Single, AttackPattern::Volley, AttackPattern::Sweep],
        row_order: [2, 1, 0], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Acolyte, key: "acolyte", name: "Mender", art_unit: "UNIT_PRIEST_OF_THE_ORDER",
        tier: 1, cost_industry: 35, cost_gold: 45, tech: Some(TECH_MYSTICISM), large: false,
        hp: 85, atk: 24, def: 28, magic: 36, mdef: 46, acc: 92, eva: 5, crit: 5, speed: 4, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 1), ra(ActionKind::Heal, 1), ra(ActionKind::Heal, 2)],
        patterns: P1, row_order: [2, 1, 0], moves: 2, upgrades_to: Some(UnitClassId::Paladin),
    },
    UnitClassInfo {
        id: UnitClassId::Paladin, key: "paladin", name: "Paladin", art_unit: "UNIT_PALADIN",
        tier: 3, cost_industry: 85, cost_gold: 115, tech: Some(TECH_RIGHTEOUS_VOWS), large: false,
        hp: 160, atk: 68, def: 66, magic: 45, mdef: 60, acc: 96, eva: 8, crit: 18, speed: 6, armor: 6,
        anti_flank: true, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Heal, 2), ra(ActionKind::Heal, 2)],
        patterns: [AttackPattern::Single, AttackPattern::Sweep, AttackPattern::Sweep],
        row_order: [0, 1, 2], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Eidolon, key: "eidolon", name: "Ashbound", art_unit: "UNIT_EIDOLON",
        tier: 3, cost_industry: 85, cost_gold: 115, tech: Some(TECH_RIGHTEOUS_VOWS), large: false,
        hp: 150, atk: 80, def: 56, magic: 52, mdef: 54, acc: 96, eva: 12, crit: 24, speed: 7, armor: 4,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Spell, 2), ra(ActionKind::Spell, 2)],
        patterns: [AttackPattern::Cleave, AttackPattern::Single, AttackPattern::Single],
        row_order: [0, 1, 2], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Catapult, key: "catapult", name: "Catapult", art_unit: "UNIT_CATAPULT",
        tier: 2, cost_industry: 55, cost_gold: 70, tech: Some(TECH_SIEGECRAFT), large: false,
        hp: 90, atk: 50, def: 24, magic: 0, mdef: 20, acc: 86, eva: 0, crit: 0, speed: 2, armor: 2,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 60,
        rows: [ra(ActionKind::None, 0), ra(ActionKind::Bombard, 1), ra(ActionKind::Bombard, 2)],
        patterns: [AttackPattern::Single, AttackPattern::Single, AttackPattern::Volley],
        row_order: [2, 1, 0], moves: 2, upgrades_to: Some(UnitClassId::Trebuchet),
    },
    UnitClassInfo {
        id: UnitClassId::Trebuchet, key: "trebuchet", name: "Trebuchet", art_unit: "UNIT_TREBUCHET",
        tier: 3, cost_industry: 85, cost_gold: 110, tech: Some(TECH_TREBUCHETS), large: false,
        hp: 105, atk: 74, def: 28, magic: 0, mdef: 24, acc: 88, eva: 0, crit: 0, speed: 2, armor: 3,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 100,
        rows: [ra(ActionKind::None, 0), ra(ActionKind::Bombard, 1), ra(ActionKind::Bombard, 2)],
        patterns: [AttackPattern::Single, AttackPattern::Volley, AttackPattern::Sweep],
        row_order: [2, 1, 0], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Ogre, key: "ogre", name: "Ogre", art_unit: "UNIT_OGRE",
        tier: 3, cost_industry: 90, cost_gold: 120, tech: Some(TECH_MITHRIL_LEGIONS), large: true,
        hp: 250, atk: 88, def: 64, magic: 0, mdef: 36, acc: 90, eva: 0, crit: 18, speed: 3, armor: 6,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 40,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: [AttackPattern::Cleave, AttackPattern::Single, AttackPattern::Single],
        row_order: [0, 1, 2], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::WarBeast, key: "war_beast", name: "War Beast", art_unit: "UNIT_BEAST_OF_AGARES",
        tier: 3, cost_industry: 90, cost_gold: 120, tech: Some(TECH_FERAL_BOND), large: true,
        hp: 240, atk: 90, def: 58, magic: 0, mdef: 42, acc: 92, eva: 8, crit: 22, speed: 6, armor: 4,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 25,
        rows: [ra(ActionKind::Charge, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: [AttackPattern::Trample, AttackPattern::Single, AttackPattern::Single],
        row_order: [0, 1, 2], moves: 3, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::PyreZombie, key: "pyre_zombie", name: "Stoker", art_unit: "UNIT_PYRE_ZOMBIE",
        tier: 2, cost_industry: 35, cost_gold: 40, tech: Some(TECH_ARCANA), large: false,
        hp: 115, atk: 50, def: 32, magic: 25, mdef: 28, acc: 90, eva: 4, crit: 12, speed: 4, armor: 1,
        anti_flank: false, armor_piercing: false, damage_vs_walls: 20,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Strike, 1), ra(ActionKind::Strike, 1)],
        patterns: [AttackPattern::Cleave, AttackPattern::Single, AttackPattern::Single],
        row_order: [0, 1, 2], moves: 2, upgrades_to: Some(UnitClassId::Balor),
    },
    UnitClassInfo {
        id: UnitClassId::Balor, key: "balor", name: "Slagbreaker", art_unit: "UNIT_BALOR",
        tier: 4, cost_industry: 110, cost_gold: 160, tech: Some(TECH_STRENGTH_OF_WILL), large: true,
        hp: 270, atk: 98, def: 70, magic: 72, mdef: 60, acc: 95, eva: 8, crit: 24, speed: 6, armor: 7,
        anti_flank: false, armor_piercing: true, damage_vs_walls: 50,
        rows: [ra(ActionKind::Strike, 2), ra(ActionKind::Spell, 2), ra(ActionKind::Spell, 2)],
        patterns: [AttackPattern::Cleave, AttackPattern::Sweep, AttackPattern::Sweep],
        row_order: [0, 1, 2], moves: 2, upgrades_to: None,
    },
    UnitClassInfo {
        id: UnitClassId::Sentinel, key: "sentinel", name: "Wall Sentinel", art_unit: "UNIT_SENTINEL",
        tier: 1, cost_industry: 0, cost_gold: 0, tech: None, large: false,
        hp: 120, atk: 16, def: 42, magic: 0, mdef: 42, acc: 90, eva: 0, crit: 5, speed: 0, armor: 4,
        anti_flank: true, armor_piercing: false, damage_vs_walls: 0,
        rows: [ra(ActionKind::Strike, 1), ra(ActionKind::None, 0), ra(ActionKind::None, 0)],
        patterns: P1, row_order: [0, 1, 2], moves: 0, upgrades_to: None,
    },
];

// ---------------------------------------------------------------------------------------------
// Great People Suits & Shared Secret-Tech Deck (24 unique world cards)
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GreatSuit {
    Sage,
    Prophet,
    Engineer,
    Merchant,
    Bard,
    Commander,
}

impl GreatSuit {
    pub const ALL: [GreatSuit; 6] = [
        GreatSuit::Sage,
        GreatSuit::Prophet,
        GreatSuit::Engineer,
        GreatSuit::Merchant,
        GreatSuit::Bard,
        GreatSuit::Commander,
    ];

    pub const fn id(self) -> usize {
        self as usize
    }

    pub const fn name(self) -> &'static str {
        match self {
            GreatSuit::Sage => "Sage",
            GreatSuit::Prophet => "Prophet",
            GreatSuit::Engineer => "Engineer",
            GreatSuit::Merchant => "Merchant",
            GreatSuit::Bard => "Bard",
            GreatSuit::Commander => "Commander",
        }
    }

    /// What a Tempo Burst of this suit does (numbers from `great_people`).
    pub fn burst_summary(self) -> String {
        use crate::great_people as g;
        match self {
            GreatSuit::Sage => format!("+{} Lore toward research and +{} Mana.", g::BURST_SAGE_LORE, g::BURST_SAGE_MANA),
            GreatSuit::Prophet => format!("{:+} Worldstrain and +{} lasting Loyalty in every House.", g::BURST_PROPHET_STRAIN, g::BURST_PROPHET_LOYALTY),
            GreatSuit::Engineer => format!(
                "+{} Industry to the build in the earliest-founded city you hold; the build completes if that covers its cost.",
                g::BURST_ENGINEER_INDUSTRY
            ),
            GreatSuit::Merchant => format!("+{} Gold and +{} Caravan slot for the rest of the game.", g::BURST_MERCHANT_GOLD, g::BURST_MERCHANT_CARAVAN_SLOTS),
            GreatSuit::Bard => format!("+{} Culture in the earliest-founded city you hold; its borders grow at once.", g::BURST_BARD_CULTURE),
            GreatSuit::Commander => format!(
                "Every unit in your squads heals {}% of its HP and gains {} XP.",
                g::BURST_COMMANDER_HEAL_PCT,
                g::BURST_COMMANDER_XP
            ),
        }
    }

    pub const fn hero_class(self) -> UnitClassId {
        match self {
            GreatSuit::Sage => UnitClassId::Archmage,
            GreatSuit::Prophet => UnitClassId::Paladin,
            GreatSuit::Engineer => UnitClassId::Trebuchet,
            GreatSuit::Merchant => UnitClassId::Ranger,
            GreatSuit::Bard => UnitClassId::Assassin,
            GreatSuit::Commander => UnitClassId::Champion,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretTechId(pub u8);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SecretEffect {
    SpellPowerPct(i32),
    ManaPerTurn(i32),
    LorePerTurn(i32),
    GoldPerTurn(i32),
    IndustryPerCity(i32),
    SentinelHpPct(i32),
    SquadCapBonus(i32),
    SupplyRangeBonus(i32),
    HouseLoyaltyBonus(i32),
    CulturePerCity(i32),
    AllAtkPct(i32),
    AllDefPct(i32),
    ArmorBonus(i32),
    AcDrainPerTurn(i32),
    AcSurgeOnce(i32),
    ExtraCaravanSlots(i32),
    RaidBonusPct(i32),
    HealBonusPct(i32),
}

#[derive(Copy, Clone, Debug)]
pub struct SecretTechInfo {
    pub id: SecretTechId,
    pub suit: GreatSuit,
    pub name: &'static str,
    pub needs: TechId,
    pub effect: SecretEffect,
    pub summary: &'static str,
    pub expert_line: &'static str,
}

pub const SECRET_TECHS: [SecretTechInfo; 24] = [
    // Sage (4) — Imperial vent-engineering & arcane manuals
    SecretTechInfo { id: SecretTechId(0), suit: GreatSuit::Sage, name: "Spire of the Archmagi", needs: TECH_ARCANA, effect: SecretEffect::SpellPowerPct(30), summary: "+30% Battle Spell damage for all squads.", expert_line: "Double-stack back-row Mages behind Sentinels or Phalanxes." },
    SecretTechInfo { id: SecretTechId(1), suit: GreatSuit::Sage, name: "Deep-Vent Manifold", needs: TECH_ARCANA, effect: SecretEffect::ManaPerTurn(6), summary: "+6 Mana per turn empire-wide.", expert_line: "Keep Mana for a Strategic Ritual every few turns." },
    SecretTechInfo { id: SecretTechId(2), suit: GreatSuit::Sage, name: "Unsealed Schematics", needs: TECH_SORCERY, effect: SecretEffect::LorePerTurn(14), summary: "+14 Lore per turn (+4 Worldstrain when discovered).", expert_line: "Rush a Tier-4 Capstone 20 turns ahead of the table." },
    SecretTechInfo { id: SecretTechId(3), suit: GreatSuit::Sage, name: "Deep Tap Charter", needs: TECH_ARCANA, effect: SecretEffect::AcSurgeOnce(12), summary: "+12 Worldstrain and +40 Mana when discovered.", expert_line: "Trigger King Cinder or Blowout while holding the central shore." },

    // Prophet (4) — Sealing ledgers & civic vows
    SecretTechInfo { id: SecretTechId(4), suit: GreatSuit::Prophet, name: "Aquifer Compendium", needs: TECH_MYSTICISM, effect: SecretEffect::HealBonusPct(45), summary: "+45% battle healing.", expert_line: "Outlast siege attrition on Caldera Keep with double-healer squads." },
    SecretTechInfo { id: SecretTechId(5), suit: GreatSuit::Prophet, name: "Valve-Keeper's Ledger", needs: TECH_MYSTICISM, effect: SecretEffect::AcDrainPerTurn(1), summary: "Halves Worldstrain drift: every second +1 is skipped.", expert_line: "Slow the clock toward a Keepers Turn-160 win." },
    SecretTechInfo { id: SecretTechId(6), suit: GreatSuit::Prophet, name: "Sacred Concordat", needs: TECH_MYSTICISM, effect: SecretEffect::HouseLoyaltyBonus(2), summary: "+2 Loyalty in every House permanently.", expert_line: "Conquer rival-culture cities without triggering Furious Houses." },
    SecretTechInfo { id: SecretTechId(7), suit: GreatSuit::Prophet, name: "Crusader's Banner", needs: TECH_FORTIFICATION, effect: SecretEffect::AllAtkPct(15), summary: "+15% Attack for every unit in your squads.", expert_line: "Swing tight 2-round formation verdicts across every front." },

    // Engineer (4) — First-Age civil & foundry blueprints
    SecretTechInfo { id: SecretTechId(8), suit: GreatSuit::Engineer, name: "Runed Bastions", needs: TECH_FORTIFICATION, effect: SecretEffect::SentinelHpPct(40), summary: "+40% Sentinel max HP in all walled cities.", expert_line: "Turn a Lifeboat city into an uncrackable bunker." },
    SecretTechInfo { id: SecretTechId(9), suit: GreatSuit::Engineer, name: "Steam Foundries", needs: TECH_METALLURGY, effect: SecretEffect::IndustryPerCity(4), summary: "+4 Industry in every city.", expert_line: "Accelerate District completion and Lifeboat Wonders." },
    SecretTechInfo { id: SecretTechId(10), suit: GreatSuit::Engineer, name: "Riveted Plate", needs: TECH_METALLURGY, effect: SecretEffect::ArmorBonus(4), summary: "+4 Armor to all units in your squads.", expert_line: "Hard-counter high-action Archer Volleys and Assassins." },
    SecretTechInfo { id: SecretTechId(11), suit: GreatSuit::Engineer, name: "Imperial Causeways", needs: TECH_FIELDCRAFT, effect: SecretEffect::SupplyRangeBonus(8), summary: "+8 Supply Line budget for all squads.", expert_line: "March straight across continents to Caldera Keep without starving." },

    // Merchant (4) — Imperial trade & road charters
    SecretTechInfo { id: SecretTechId(12), suit: GreatSuit::Merchant, name: "Golden Ledger", needs: TECH_TRADE_ROADS, effect: SecretEffect::GoldPerTurn(18), summary: "+18 Gold per turn empire-wide.", expert_line: "Afford constant tier-3 upgrades and rapid reinforcements." },
    SecretTechInfo { id: SecretTechId(13), suit: GreatSuit::Merchant, name: "Silk Convoy Charter", needs: TECH_TRADE_ROADS, effect: SecretEffect::ExtraCaravanSlots(2), summary: "+2 Caravan slots empire-wide.", expert_line: "Dominate foreign trade income and strategic resource imports." },
    SecretTechInfo { id: SecretTechId(14), suit: GreatSuit::Merchant, name: "Toll-Warrant Letters", needs: TECH_FIELDCRAFT, effect: SecretEffect::RaidBonusPct(50), summary: "+50% Gold plundered from Raids.", expert_line: "Fund an offensive war purely off enemy countryside." },
    SecretTechInfo { id: SecretTechId(15), suit: GreatSuit::Merchant, name: "Mercenary Retainer", needs: TECH_TRADE_ROADS, effect: SecretEffect::SquadCapBonus(2), summary: "+2 maximum Squads.", expert_line: "Field a dedicated Raid squad alongside two Siege squads." },

    // Bard (4) — Surviving chronicles & rim-watch songs
    SecretTechInfo { id: SecretTechId(16), suit: GreatSuit::Bard, name: "Chronicle of the Sealing", needs: TECH_MYSTICISM, effect: SecretEffect::CulturePerCity(6), summary: "+6 Culture per turn in every city.", expert_line: "Lock down the Last Metropolis Lifeboat before Worldstrain 100." },
    SecretTechInfo { id: SecretTechId(17), suit: GreatSuit::Bard, name: "Court of Masks", needs: TECH_TRADE_ROADS, effect: SecretEffect::HouseLoyaltyBonus(2), summary: "+2 Loyalty in every House.", expert_line: "Keep a wide multi-culture empire productive under war." },
    SecretTechInfo { id: SecretTechId(18), suit: GreatSuit::Bard, name: "Dirge of Iron", needs: TECH_ARCHERY, effect: SecretEffect::AllDefPct(15), summary: "+15% Defense for all units in your squads.", expert_line: "Hold choke points and Caldera Keep bridges against superior numbers." },
    SecretTechInfo { id: SecretTechId(19), suit: GreatSuit::Bard, name: "Ballad of the Fallen", needs: TECH_FIELDCRAFT, effect: SecretEffect::SquadCapBonus(1), summary: "+1 Squad cap.", expert_line: "Recover immediately after losing a frontline squad." },

    // Commander (4) — Imperial legion manuals
    SecretTechInfo { id: SecretTechId(20), suit: GreatSuit::Commander, name: "Banner of the Vanguard", needs: TECH_METALLURGY, effect: SecretEffect::AllAtkPct(20), summary: "+20% Attack to all squads.", expert_line: "Break fortified cities in a single decisive engagement." },
    SecretTechInfo { id: SecretTechId(21), suit: GreatSuit::Commander, name: "Field Logistics", needs: TECH_FIELDCRAFT, effect: SecretEffect::SupplyRangeBonus(10), summary: "+10 Supply Line budget.", expert_line: "Deep raids and amphibious invasions stay fully supplied." },
    SecretTechInfo { id: SecretTechId(22), suit: GreatSuit::Commander, name: "Iron Wall Doctrine", needs: TECH_FORTIFICATION, effect: SecretEffect::AllDefPct(20), summary: "+20% Defense to all squads.", expert_line: "Turn defensive terrain and forts into meat grinders." },
    SecretTechInfo { id: SecretTechId(23), suit: GreatSuit::Commander, name: "Grand Marshal's Writ", needs: TECH_HORSEMANSHIP, effect: SecretEffect::SquadCapBonus(2), summary: "+2 Squad Cap empire-wide.", expert_line: "Outnumber rivals across multiple Lifeboat fronts." },
];

// ---------------------------------------------------------------------------------------------
// Worldstrain Crisis Deck (10 escalating thresholds: Worldstrain 10, 20, ..., 100)
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct CrisisInfo {
    pub threshold: i32,
    pub key: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
}

pub const ARMAGEDDON_CRISES: [CrisisInfo; 10] = [
    CrisisInfo { threshold: 10, key: "omens", name: "Sulfur Haze", summary: "2 Wild Clans squads appear on unclaimed land." },
    CrisisInfo { threshold: 20, key: "blight", name: "Crust Cracks", summary: "10 plots of unclaimed or Evil-held land become scorched ground." },
    CrisisInfo { threshold: 30, key: "dissent", name: "Rumors from Below", summary: "-1 lasting Loyalty in every House." },
    CrisisInfo { threshold: 40, key: "burning_earth", name: "Pressurized Vents", summary: "14 plots of unclaimed or Evil-held land become scorched ground; +15 Mana to every civilization." },
    CrisisInfo { threshold: 50, key: "hyborem", name: "The Deep Breach", summary: "King Cinder's Host enters with Great Furnace on the unclaimed land farthest from every city, at war with every Good civilization. Great Furnace cannot be attacked until the Blowout." },
    CrisisInfo { threshold: 60, key: "plague", name: "Ash Plague", summary: "Every city above Size 1 loses 1 Size unless it has a Temple or the Great Cistern." },
    CrisisInfo { threshold: 70, key: "horsemen", name: "Runaway Engines", summary: "2 Wild Clans squads (Knight and Slagbreaker) appear on unclaimed land." },
    CrisisInfo { threshold: 80, key: "sky_bleeds", name: "Red Sky", summary: "20 plots of unclaimed or Evil-held land become scorched ground; from Worldstrain 80 all battles deal +15% damage." },
    CrisisInfo { threshold: 90, key: "shattering", name: "The Heave", summary: "24 plots of unclaimed or Evil-held land become scorched ground." },
    CrisisInfo { threshold: 100, key: "carcosa", name: "The Blowout", summary: "Caldera Keep rises from the central sea on land bridges, held by King Cinder's Host if it is alive, and the Lifeboat countdown begins." },
];

// ---------------------------------------------------------------------------------------------
// Strategic Rituals (empire-level Mana spells — zero per-unit clicking!)
// ---------------------------------------------------------------------------------------------

/// Turns before a civilization may cast a ritual that moves Strain (Vent the Seal, Crack the Crust) again.
pub const STRAIN_RITUAL_RECAST_TURNS: i32 = 12;
/// Strain moved by a cast of Vent the Seal (the Keepers of the Seal: `game::SANCTIFY_KEEPERS_STRAIN`).
pub const VENT_THE_SEAL_STRAIN: i32 = -1;
/// Strain moved by a cast of Crack the Crust.
pub const CRACK_THE_CRUST_STRAIN: i32 = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RitualKind {
    Sanctify,
    HasteMarch,
    ArcaneSurge,
    SummonHost,
    BlightStrike,
    WardWalls,
}

impl RitualKind {
    pub const ALL: [RitualKind; 6] = [
        RitualKind::Sanctify,
        RitualKind::HasteMarch,
        RitualKind::ArcaneSurge,
        RitualKind::SummonHost,
        RitualKind::BlightStrike,
        RitualKind::WardWalls,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            RitualKind::Sanctify => "sanctify",
            RitualKind::HasteMarch => "haste_march",
            RitualKind::ArcaneSurge => "arcane_surge",
            RitualKind::SummonHost => "summon_host",
            RitualKind::BlightStrike => "blight_strike",
            RitualKind::WardWalls => "ward_walls",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            RitualKind::Sanctify => "Vent the Seal",
            RitualKind::HasteMarch => "Draft-Wind March",
            RitualKind::ArcaneSurge => "Vent-Ether Surge",
            RitualKind::SummonHost => "Furnace Levy",
            RitualKind::BlightStrike => "Crack the Crust",
            RitualKind::WardWalls => "Seal the Masonry",
        }
    }

    pub const fn mana_cost(self) -> i32 {
        match self {
            RitualKind::Sanctify => 25,
            RitualKind::HasteMarch => 20,
            RitualKind::ArcaneSurge => 30,
            RitualKind::SummonHost => 45,
            RitualKind::BlightStrike => 35,
            RitualKind::WardWalls => 25,
        }
    }

    /// Index into per-ritual arrays (`Player::ritual_ready`).
    pub const fn id(self) -> usize {
        match self {
            RitualKind::Sanctify => 0,
            RitualKind::HasteMarch => 1,
            RitualKind::ArcaneSurge => 2,
            RitualKind::SummonHost => 3,
            RitualKind::BlightStrike => 4,
            RitualKind::WardWalls => 5,
        }
    }

    /// Strain change when cast (Keepers' Vent the Seal: `game::SANCTIFY_KEEPERS_STRAIN`). Only Vent
    /// the Seal and Crack the Crust move Strain.
    pub const fn ac_delta(self) -> i32 {
        match self {
            RitualKind::Sanctify => VENT_THE_SEAL_STRAIN,
            RitualKind::BlightStrike => CRACK_THE_CRUST_STRAIN,
            RitualKind::HasteMarch | RitualKind::ArcaneSurge | RitualKind::SummonHost | RitualKind::WardWalls => 0,
        }
    }

    /// World turns after a cast before the same civilization may cast this ritual again (0: no wait).
    pub const fn recast_turns(self) -> i32 {
        match self {
            RitualKind::Sanctify | RitualKind::BlightStrike => STRAIN_RITUAL_RECAST_TURNS,
            RitualKind::HasteMarch | RitualKind::ArcaneSurge | RitualKind::SummonHost | RitualKind::WardWalls => 0,
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            RitualKind::Sanctify => "Scorched ground within 4 plots of your capital returns to normal.",
            RitualKind::HasteMarch => "Every squad you own gets its full movement +1 for this turn.",
            RitualKind::ArcaneSurge => "+65 Lore toward research.",
            RitualKind::SummonHost => "A new squad of 2 Stokers and 1 Adept appears at your capital.",
            RitualKind::BlightStrike => "6 plots of unclaimed land or land of civilizations you are at war with become scorched ground.",
            RitualKind::WardWalls => "Repairs every Sentinel in your cities; your squads standing in a city heal 30%.",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// House Perks & Captain Auras (using real embedded minor-leader portraits!)
// ---------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HousePerk {
    TaxCharter,
    MasonGuild,
    LoreConclave,
    BannerLevy,
    BorderWardens,
    FestivalCourts,
    ManaConduits,
    CaravanPatronage,
}

impl HousePerk {
    pub const ALL: [HousePerk; 8] = [
        HousePerk::TaxCharter,
        HousePerk::MasonGuild,
        HousePerk::LoreConclave,
        HousePerk::BannerLevy,
        HousePerk::BorderWardens,
        HousePerk::FestivalCourts,
        HousePerk::ManaConduits,
        HousePerk::CaravanPatronage,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            HousePerk::TaxCharter => "Tax Charter (+3 Gold/city)",
            HousePerk::MasonGuild => "Mason Guild (+2 Industry/city)",
            HousePerk::LoreConclave => "Lore Conclave (+3 Lore/city)",
            HousePerk::BannerLevy => "Banner Levy (+1 Squad Cap)",
            HousePerk::BorderWardens => "Border Wardens (+25% Sentinel HP)",
            HousePerk::FestivalCourts => "Festival Courts (+4 Culture/city)",
            HousePerk::ManaConduits => "Mana Conduits (+2 Mana/turn)",
            HousePerk::CaravanPatronage => "Caravan Patronage (+1 Caravan slot)",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CaptainAura {
    Vanguard,
    Bulwark,
    ArcaneFocus,
    SwiftMarch,
    Skirmisher,
    Quartermaster,
}

impl CaptainAura {
    pub const ALL: [CaptainAura; 6] = [
        CaptainAura::Vanguard,
        CaptainAura::Bulwark,
        CaptainAura::ArcaneFocus,
        CaptainAura::SwiftMarch,
        CaptainAura::Skirmisher,
        CaptainAura::Quartermaster,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            CaptainAura::Vanguard => "Vanguard (+12% Squad ATK)",
            CaptainAura::Bulwark => "Bulwark (+12% Squad DEF)",
            CaptainAura::ArcaneFocus => "Arcane Focus (+15% Spell & Heal)",
            CaptainAura::SwiftMarch => "Swift March (+1 move, +1 battle speed)",
            CaptainAura::Skirmisher => "Skirmisher (+10 Accuracy & +8 Evasion)",
            CaptainAura::Quartermaster => "Quartermaster (+4 Supply Budget)",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PortraitCharacter {
    pub name: &'static str,
    /// Maps directly to `LEADER_<KEY>` -> `iso/leader_<key>_face.png`!
    pub portrait_key: &'static str,
    pub alignment: Alignment,
    pub aura: CaptainAura,
}

/// 36 verified embedded minor-leader portraits reused as Squad Captains, House Lords & Great People.
pub const CHARACTER_POOL: [PortraitCharacter; 36] = [
    PortraitCharacter { name: "Aelwen Many-Vows", portrait_key: "LEADER_AELWEN_MANY_VOWS", alignment: Alignment::Good, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Aethi Vaultmoth", portrait_key: "LEADER_AETHI_VAULTMOTH", alignment: Alignment::Neutral, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Asen Curtainfall", portrait_key: "LEADER_ASEN_CURTAINFALL", alignment: Alignment::Evil, aura: CaptainAura::Skirmisher },
    PortraitCharacter { name: "Aureth Feather-Ash", portrait_key: "LEADER_AURETH_FEATHER_ASH", alignment: Alignment::Neutral, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Avar Senn", portrait_key: "LEADER_AVAR_SENN", alignment: Alignment::Good, aura: CaptainAura::Vanguard },
    PortraitCharacter { name: "Avona of Two Walls", portrait_key: "LEADER_AVONA_OF_TWO_WALLS", alignment: Alignment::Good, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Azhrul Ashwarden", portrait_key: "LEADER_AZHRUL_ASHWARDEN", alignment: Alignment::Evil, aura: CaptainAura::Vanguard },
    PortraitCharacter { name: "Bekar Windstitch", portrait_key: "LEADER_BEKAR_WINDSTITCH", alignment: Alignment::Neutral, aura: CaptainAura::SwiftMarch },
    PortraitCharacter { name: "Bekka Tallfang", portrait_key: "LEADER_BEKKA_TALLFANG", alignment: Alignment::Evil, aura: CaptainAura::Vanguard },
    PortraitCharacter { name: "Beren Halflight", portrait_key: "LEADER_BEREN_HALFLIGHT", alignment: Alignment::Neutral, aura: CaptainAura::Skirmisher },
    PortraitCharacter { name: "Berga Ironwedge", portrait_key: "LEADER_BERGA_IRONWEDGE", alignment: Alignment::Neutral, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Borruk Orejaw", portrait_key: "LEADER_BORRUK_OREJAW", alignment: Alignment::Neutral, aura: CaptainAura::Quartermaster },
    PortraitCharacter { name: "Bram Copperbrow", portrait_key: "LEADER_BRAM_COPPERBROW", alignment: Alignment::Good, aura: CaptainAura::Quartermaster },
    PortraitCharacter { name: "Brannoc Blackwake", portrait_key: "LEADER_BRANNOC_BLACKWAKE", alignment: Alignment::Evil, aura: CaptainAura::SwiftMarch },
    PortraitCharacter { name: "Brassett Nine-Fingers", portrait_key: "LEADER_BRASSETT_NINE_FINGERS", alignment: Alignment::Neutral, aura: CaptainAura::Skirmisher },
    PortraitCharacter { name: "Brunna Chalkvein", portrait_key: "LEADER_BRUNNA_CHALKVEIN", alignment: Alignment::Neutral, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Calen Birdbranch", portrait_key: "LEADER_CALEN_BIRDBRANCH", alignment: Alignment::Good, aura: CaptainAura::Skirmisher },
    PortraitCharacter { name: "Caradriel Pale-Fire", portrait_key: "LEADER_CARADRIEL_PALE_FIRE", alignment: Alignment::Good, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Carlon Mirrorsail", portrait_key: "LEADER_CARLON_MIRRORSAIL", alignment: Alignment::Neutral, aura: CaptainAura::SwiftMarch },
    PortraitCharacter { name: "Cedric Stave", portrait_key: "LEADER_CEDRIC_STAVE", alignment: Alignment::Good, aura: CaptainAura::Quartermaster },
    PortraitCharacter { name: "Cevran Dustreader", portrait_key: "LEADER_CEVRAN_DUSTREADER", alignment: Alignment::Neutral, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Corin Slategate", portrait_key: "LEADER_CORIN_SLATEGATE", alignment: Alignment::Good, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Dalia Chalkwall", portrait_key: "LEADER_DALIA_CHALKWALL", alignment: Alignment::Good, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Damar Ninebells", portrait_key: "LEADER_DAMAR_NINEBELLS", alignment: Alignment::Neutral, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Dario Reefglass", portrait_key: "LEADER_DARIO_REEFGLASS", alignment: Alignment::Neutral, aura: CaptainAura::Quartermaster },
    PortraitCharacter { name: "Dessa Quartzbell", portrait_key: "LEADER_DESSA_QUARTZBELL", alignment: Alignment::Good, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Dessa Venn", portrait_key: "LEADER_DESSA_VENN", alignment: Alignment::Neutral, aura: CaptainAura::Vanguard },
    PortraitCharacter { name: "Dianthe Nightjury", portrait_key: "LEADER_DIANTHE_NIGHTJURY", alignment: Alignment::Evil, aura: CaptainAura::Skirmisher },
    PortraitCharacter { name: "Dorrik Ledgergrave", portrait_key: "LEADER_DORRIK_LEDGERGRAVE", alignment: Alignment::Evil, aura: CaptainAura::Quartermaster },
    PortraitCharacter { name: "Dowen Campbridge", portrait_key: "LEADER_DOWEN_CAMPBRIDGE", alignment: Alignment::Good, aura: CaptainAura::SwiftMarch },
    PortraitCharacter { name: "Drekk Ninetokens", portrait_key: "LEADER_DREKK_NINETOKENS", alignment: Alignment::Evil, aura: CaptainAura::Quartermaster },
    PortraitCharacter { name: "Durn Pell", portrait_key: "LEADER_DURN_PELL", alignment: Alignment::Neutral, aura: CaptainAura::Vanguard },
    PortraitCharacter { name: "Edda Coalhand", portrait_key: "LEADER_EDDA_COALHAND", alignment: Alignment::Evil, aura: CaptainAura::Bulwark },
    PortraitCharacter { name: "Edren Twice-Buried", portrait_key: "LEADER_EDREN_TWICE_BURIED", alignment: Alignment::Evil, aura: CaptainAura::ArcaneFocus },
    PortraitCharacter { name: "Elken Long-Silence", portrait_key: "LEADER_ELKEN_LONG_SILENCE", alignment: Alignment::Neutral, aura: CaptainAura::Skirmisher },
    PortraitCharacter { name: "Elyndra Ashglass", portrait_key: "LEADER_ELYNDRA_ASHGLASS", alignment: Alignment::Evil, aura: CaptainAura::ArcaneFocus },
];
