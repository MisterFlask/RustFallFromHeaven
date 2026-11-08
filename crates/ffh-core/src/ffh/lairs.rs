//! Lair exploration (`spellExploreLair`, `spellExploreLairEpic` and the `exploreLair*` functions
//! of CustomFunctions.py) and the odds of lairs and goody huts as the interface shows them.
//!
//! An exploration rolls a tier (`rand(100) + the unit's level`), then picks one outcome from the
//! tier's list. Each tier's list is built by a read-only function (`lair_*_outcomes`,
//! `lair_big_bad_lists`) that the roll and [`Game::lair_explore_odds`] share, so the odds shown
//! are the lists the roll draws from. The lists of the two "big" tiers depend on the grace
//! period (`cf_grace`, a random draw per check, FFH2's order kept): their builders take the
//! grace check as a closure, the roll answering it with the game RNG and the odds with every
//! combination of answers, weighted.
//!
//! Odds model the RNG as uniform (`CvRandom::get(n)` is uniform to within 1/65536 per value).

use crate::game::Game;
use crate::ids::*;
use ffh_data::*;

/// The five result tiers of an exploration (`exploreLairBigBad` .. `exploreLairBigGood`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LairTier {
    BigBad,
    Bad,
    Neutral,
    Good,
    BigGood,
}

impl LairTier {
    pub const ALL: [LairTier; 5] = [LairTier::BigBad, LairTier::Bad, LairTier::Neutral, LairTier::Good, LairTier::BigGood];

    /// The tier's key in the interface's protocol.
    pub fn key(self) -> &'static str {
        match self {
            LairTier::BigBad => "big_bad",
            LairTier::Bad => "bad",
            LairTier::Neutral => "neutral",
            LairTier::Good => "good",
            LairTier::BigGood => "big_good",
        }
    }

    /// The `[lo, hi)` range of `rand(100) + level` that rolls this tier (`epic`: Explore Lair on
    /// an epic site, big bad below 54, big good from 54); `None` when it cannot be rolled.
    fn roll_range(self, epic: bool) -> Option<(i32, i32)> {
        const LOW: i32 = i32::MIN / 2;
        const HIGH: i32 = i32::MAX / 2;
        if epic {
            return match self {
                LairTier::BigBad => Some((LOW, 54)),
                LairTier::BigGood => Some((54, HIGH)),
                _ => None,
            };
        }
        Some(match self {
            LairTier::BigBad => (LOW, 14),
            LairTier::Bad => (14, 44),
            LairTier::Neutral => (44, 74),
            LairTier::Good => (74, 94),
            LairTier::BigGood => (94, HIGH),
        })
    }

    /// The tier a roll of `rnd` (`rand(100) + level`) falls in.
    pub fn of_roll(rnd: i32, epic: bool) -> LairTier {
        LairTier::ALL
            .into_iter()
            .find(|t| t.roll_range(epic).is_some_and(|(lo, hi)| lo <= rnd && rnd < hi))
            .expect("every roll has a tier")
    }
}

/// The big bad result's three lists (`exploreLairBigBad`): the monster, its henchmen (0-4 of one
/// kind) and its promotion are each picked from one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LairBigBadLists {
    pub monsters: Vec<&'static str>,
    pub henchmen: Vec<&'static str>,
    pub promotions: Vec<&'static str>,
}

/// The key of the big bad tier's single outcome (a named monster and its henchmen).
pub const BIG_BAD_MONSTER: &str = "MONSTER";

/// In % the chance that the lair is destroyed after outcome `key` of `tier` (what the Python
/// `exploreLair*` functions return; `spellExploreLair` removes the lair when it beats
/// `rand(100)`).
pub fn lair_destroy_chance(tier: LairTier, key: &str) -> i32 {
    match tier {
        LairTier::BigBad => 0,
        LairTier::Bad => match key {
            "DEATH" => 0,
            "COLLAPSE" => 100,
            "SPAWN_DROWN" | "SPAWN_SCORPION_BAD" | "SPAWN_SEA_SERPENT" | "SPAWN_SPECTRE" | "SPAWN_SPIDER" => 50,
            "CRAZED" | "DEMONIC_POSSESSION" | "DISEASED" | "ENRAGED" | "PLAGUED" | "POISONED" | "WITHERED" | "RUSTED" => 80,
            _ => 100,
        },
        LairTier::Neutral => match key {
            "CAGE" | "PORTAL" => 0,
            "MUTATED" => 50,
            k if k.starts_with("SPAWN_") => 50,
            _ => 100,
        },
        LairTier::Good => match key {
            "HIGH_GOLD" => 90,
            "TREASURE" | "SPIRIT_GUIDE" => 80,
            _ => 100,
        },
        LairTier::BigGood => 100,
    }
}

/// Whether outcome `key` replaces the lair itself (the cage trap turns the lair into a cage).
pub fn lair_outcome_replaces_lair(tier: LairTier, key: &str) -> bool {
    tier == LairTier::Neutral && key == "CAGE"
}

/// One outcome of an exploration and its chance (parts per million of the whole exploration).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LairOutcomeOdds {
    pub tier: LairTier,
    pub key: &'static str,
    pub ppm: i64,
    /// The chance in % that the lair is destroyed after it ([`lair_destroy_chance`]).
    pub destroy_pct: i32,
}

/// The odds of exploring a lair with a unit ([`Game::lair_explore_odds`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LairOdds {
    /// The exploration spell's result (`spellExploreLair` or, on an epic site, `...Epic`).
    pub epic: bool,
    /// The unit's level (added to the roll).
    pub level: i32,
    /// Each tier the roll can reach and its chance (ppm).
    pub tiers: Vec<(LairTier, i64)>,
    /// Every outcome with a chance, by tier, in the order of the tier's list.
    pub outcomes: Vec<LairOutcomeOdds>,
    /// The chance (ppm) that the lair is still there after the exploration.
    pub lair_survives_ppm: i64,
    /// The chance (ppm) that a grace check passes now (`cf_grace`: early in the game the big
    /// results leave out their strongest entries).
    pub grace_ppm: i64,
    /// Every monster, henchman and promotion the big bad result can pick from now (the union
    /// over the grace checks).
    pub big_bad: LairBigBadLists,
}

/// The odds of entering a goody hut ([`Game::goody_hut_odds`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodyOdds {
    /// Each goody of the difficulty's list once, in list order: its chance (ppm), its share of
    /// the list (slots of `slots_total`), and whether the entering unit could receive it now.
    pub outcomes: Vec<GoodyOutcomeOdds>,
    pub slots_total: i32,
    /// The draws made before the hut gives nothing (`NUM_DO_GOODY_ATTEMPTS`).
    pub attempts: i32,
    /// The chance (ppm) that every draw is refused and the hut gives nothing.
    pub nothing_ppm: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoodyOutcomeOdds {
    pub goody: GoodyType,
    pub ppm: i64,
    pub slots: i32,
    pub allowed: bool,
}

/// Parts per 10^12 to parts per million, rounded.
fn ppt_to_ppm(ppt: i128) -> i64 {
    ((ppt + 500_000) / 1_000_000) as i64
}

const PPT: i128 = 1_000_000_000_000;
/// The most grace checks a list builder makes (big bad: 3).
const MAX_GRACE_CALLS: u32 = 4;

impl Game {
    /// The bound of the grace check: `cf_grace` passes when `rand(bound) + bound` exceeds the
    /// game turn.
    pub fn grace_bound(&self) -> i32 {
        let grace = 20 * (self.game_speed_type().i() + 1);
        let diff = self.d.handicaps.len() as i32 + 1 - self.handicap.i();
        grace * diff
    }

    /// How many of the `grace_bound()` values of the grace roll pass the check now.
    fn grace_passing_values(&self) -> i32 {
        let bound = self.grace_bound();
        (2 * bound - 1 - self.game_turn()).clamp(0, bound)
    }

    /// Runs a list builder with the grace checks answered by the game RNG, in the order the
    /// builder asks (the builder only reads the game).
    fn with_grace_rng<R>(&mut self, f: impl FnOnce(&Game, &mut dyn FnMut() -> bool) -> R) -> R {
        let bound = self.grace_bound();
        let turn = self.game_turn();
        let mut rng = std::mem::take(&mut self.soren_rand);
        let out = {
            let mut grace = || rng.get_i(bound, "grace") + bound > turn;
            f(self, &mut grace)
        };
        self.soren_rand = rng;
        out
    }

    /// Runs `build` once per combination of grace answers it asks for: `(weight, result)`, the
    /// weights out of `grace_bound()^MAX_GRACE_CALLS`.
    fn over_grace<T>(&self, mut build: impl FnMut(&mut dyn FnMut() -> bool) -> T) -> Vec<(i128, T)> {
        let bound = self.grace_bound() as i128;
        let pass = self.grace_passing_values() as i128;
        let mut out = Vec::new();
        for mask in 0u32..(1 << MAX_GRACE_CALLS) {
            let mut calls = 0u32;
            let result = build(&mut || {
                let answer = mask >> calls & 1 == 1;
                calls += 1;
                answer
            });
            assert!(calls <= MAX_GRACE_CALLS, "a lair list makes at most {MAX_GRACE_CALLS} grace checks");
            // Each combination once: the bits past the checks made are 0.
            if mask >> calls != 0 {
                continue;
            }
            let mut weight: i128 = bound.pow(MAX_GRACE_CALLS - calls);
            for i in 0..calls {
                weight *= if mask >> i & 1 == 1 { pass } else { bound - pass };
            }
            if weight > 0 {
                out.push((weight, result));
            }
        }
        out
    }

    // ------------------------------------------------------------------ outcome lists

    /// `exploreLairBigBad`'s lists for a lair on `plot`; `grace` answers its grace checks.
    pub fn lair_big_bad_lists(&self, plot: PlotIdx, grace: &mut dyn FnMut() -> bool) -> LairBigBadLists {
        let d = self.d;
        let mut list: Vec<&'static str> = vec!["UNIT_AZER"];
        let mut promo_list: Vec<&'static str> = vec![
            "PROMOTION_MUTATED",
            "PROMOTION_CANNIBALIZE",
            "PROMOTION_MOBILITY1",
            "PROMOTION_STRONG",
            "PROMOTION_BLITZ",
            "PROMOTION_COMMAND1",
            "PROMOTION_HEROIC_STRENGTH",
            "PROMOTION_HEROIC_DEFENSE",
            "PROMOTION_MAGIC_IMMUNE",
            "PROMOTION_STONESKIN",
            "PROMOTION_VALOR",
            "PROMOTION_VILE_TOUCH",
        ];
        let mut henchman_list: Vec<&'static str> = vec!["UNIT_AZER", "UNIT_GRIFFON"];
        if !grace() {
            list.push("UNIT_AIR_ELEMENTAL");
        }
        if !self.plot_is_water(plot) {
            list.extend_from_slice(&["UNIT_ASSASSIN", "UNIT_OGRE", "UNIT_GIANT_SPIDER", "UNIT_HILL_GIANT", "UNIT_SPECTRE", "UNIT_SCORPION"]);
            henchman_list.extend_from_slice(&[
                "UNIT_AXEMAN",
                "UNIT_WOLF",
                "UNIT_CHAOS_MARAUDER",
                "UNIT_WOLF_RIDER",
                "UNIT_MISTFORM",
                "UNIT_LION",
                "UNIT_TIGER",
                "UNIT_BABY_SPIDER",
                "UNIT_FAWN",
                "UNIT_SCORPION",
            ]);
            if !grace() {
                list.extend_from_slice(&[
                    "UNIT_EARTH_ELEMENTAL",
                    "UNIT_FIRE_ELEMENTAL",
                    "UNIT_GARGOYLE",
                    "UNIT_VAMPIRE",
                    "UNIT_MYCONID",
                    "UNIT_EIDOLON",
                    "UNIT_LICH",
                    "UNIT_OGRE_WARCHIEF",
                    "UNIT_SATYR",
                    "UNIT_WEREWOLF",
                ]);
                promo_list.extend_from_slice(&["PROMOTION_FIRE2", "PROMOTION_AIR2", "PROMOTION_HERO", "PROMOTION_MARKSMAN", "PROMOTION_SHADOWWALK"]);
                henchman_list.push("UNIT_OGRE");
                if self.map.plots[plot].feature_type == Some(d.t::<FeatureType>("FEATURE_FOREST")) {
                    list.push("UNIT_TREANT");
                }
            }
            if self.map.plots[plot].terrain_type == Some(d.t::<TerrainType>("TERRAIN_SNOW")) {
                henchman_list.extend_from_slice(&["UNIT_FROSTLING_ARCHER", "UNIT_FROSTLING_WOLF_RIDER", "UNIT_POLAR_BEAR"]);
            }
            if self.map.plots[plot].improvement_type == Some(d.t::<ImprovementType>("IMPROVEMENT_BARROW")) {
                promo_list.push("PROMOTION_DEATH2");
                henchman_list.extend_from_slice(&["UNIT_SKELETON", "UNIT_PYRE_ZOMBIE"]);
                if !grace() {
                    list.push("UNIT_WRAITH");
                }
            }
            if self.map.plots[plot].improvement_type == Some(d.t::<ImprovementType>("IMPROVEMENT_RUINS")) {
                promo_list.push("PROMOTION_POISONED_BLADE");
                henchman_list.extend_from_slice(&["UNIT_LIZARDMAN", "UNIT_GORILLA"]);
                if !grace() {
                    list.push("UNIT_MANTICORE");
                }
            }
            if self.get_global_counter() > 40 {
                list.extend_from_slice(&["UNIT_PIT_BEAST", "UNIT_DEATH_KNIGHT", "UNIT_BALOR"]);
                promo_list.push("PROMOTION_FEAR");
                henchman_list.extend_from_slice(&["UNIT_IMP", "UNIT_HELLHOUND"]);
            }
        }
        if self.plot_is_water(plot) {
            list.extend_from_slice(&["UNIT_SEA_SERPENT", "UNIT_STYGIAN_GUARD", "UNIT_PIRATE"]);
            henchman_list.push("UNIT_DROWN");
            if !grace() {
                list.extend_from_slice(&["UNIT_WATER_ELEMENTAL", "UNIT_KRAKEN"]);
            }
        }
        LairBigBadLists { monsters: list, henchmen: henchman_list, promotions: promo_list }
    }

    /// `exploreLairBad`'s list for `caster` exploring a lair on `plot`.
    pub fn lair_bad_outcomes(&self, caster: UnitId, plot: PlotIdx) -> Vec<&'static str> {
        let d = self.d;
        let mut list: Vec<&'static str> = vec!["COLLAPSE"];
        if self.units[caster].level == 1 {
            list.push("DEATH");
        }
        if self.unit_is_alive(caster) {
            list.extend_from_slice(&["CRAZED", "DEMONIC_POSSESSION", "DISEASED", "ENRAGED", "PLAGUED", "POISONED", "WITHERED"]);
        }
        if self.unit_get_unit_combat_type(caster) == Some(d.t::<UnitCombatType>("UNITCOMBAT_MELEE")) {
            list.push("RUSTED");
        }
        if self.plot_is_water(plot) {
            list.extend_from_slice(&["SPAWN_DROWN", "SPAWN_SEA_SERPENT"]);
        }
        if !self.plot_is_water(plot) {
            list.extend_from_slice(&["SPAWN_SPIDER", "SPAWN_SPECTRE"]);
        }
        if self.map.plots[plot].improvement_type == Some(d.t::<ImprovementType>("IMPROVEMENT_GOBLIN_FORT")) {
            list.extend_from_slice(&["SPAWN_SCORPION_BAD", "SPAWN_SCORPION_BAD", "SPAWN_SCORPION_BAD"]);
        }
        list
    }

    /// `exploreLairNeutral`'s list for `caster` exploring a lair on `plot`.
    pub fn lair_neutral_outcomes(&self, caster: UnitId, plot: PlotIdx) -> Vec<&'static str> {
        let d = self.d;
        let mut list: Vec<&'static str> = vec!["NOTHING"];
        if !self.plot_is_water(plot) {
            list.extend_from_slice(&["SPAWN_SKELETON", "SPAWN_LIZARDMAN", "SPAWN_SPIDER", "PORTAL", "DEPTHS", "DWARF_VS_LIZARDMEN", "CAGE"]);
            if self.map.plots[plot].terrain_type == Some(d.t::<TerrainType>("TERRAIN_SNOW")) {
                list.push("SPAWN_FROSTLING");
            }
            let improvement = self.map.plots[plot].improvement_type;
            if improvement == Some(d.t::<ImprovementType>("IMPROVEMENT_BARROW")) {
                list.extend_from_slice(&["SPAWN_SKELETON", "SPAWN_SKELETON"]);
            }
            if improvement == Some(d.t::<ImprovementType>("IMPROVEMENT_RUINS")) {
                list.extend_from_slice(&["SPAWN_LIZARDMAN", "SPAWN_LIZARDMAN"]);
            }
            if improvement == Some(d.t::<ImprovementType>("IMPROVEMENT_GOBLIN_FORT")) {
                list.extend_from_slice(&["SPAWN_SCORPION", "SPAWN_SCORPION", "SPAWN_SCORPION"]);
            }
        }
        if self.plot_is_water(plot) {
            list.push("SPAWN_DROWN");
        }
        if self.unit_is_alive(caster) && !self.units[caster].has_promotion[d.t::<PromotionType>("PROMOTION_MUTATED")] {
            list.push("MUTATED");
        }
        list
    }

    /// `exploreLairGood`'s list for `caster` exploring a lair on `plot`.
    pub fn lair_good_outcomes(&self, caster: UnitId, plot: PlotIdx) -> Vec<&'static str> {
        let d = self.d;
        let p = self.units[caster].owner;
        let has = |name: &str| self.units[caster].has_promotion[d.t::<PromotionType>(name)];
        let combat = self.unit_get_unit_combat_type(caster);
        let is_combat = |name: &str| combat == Some(d.t::<UnitCombatType>(name));

        let mut list: Vec<&'static str> = vec!["HIGH_GOLD", "TREASURE", "EXPERIENCE"];
        if self.unit_is_alive(caster) && !has("PROMOTION_SPIRIT_GUIDE") {
            list.push("SPIRIT_GUIDE");
        }
        if !self.plot_is_water(plot) {
            list.extend_from_slice(&["ITEM_HEALING_SALVE", "SUPPLIES"]);
            if self.player_is_has_tech(p, d.t::<TechType>("TECH_MYSTICISM")) {
                list.extend_from_slice(&[
                    "PRISONER_DISCIPLE_ASHEN",
                    "PRISONER_DISCIPLE_EMPYREAN",
                    "PRISONER_DISCIPLE_LEAVES",
                    "PRISONER_DISCIPLE_OVERLORDS",
                    "PRISONER_DISCIPLE_RUNES",
                    "PRISONER_DISCIPLE_ORDER",
                ]);
            }
        }
        if is_combat("UNITCOMBAT_MELEE") && !has("PROMOTION_ENCHANTED_BLADE") {
            list.push("ENCHANTED_BLADE");
        }
        if is_combat("UNITCOMBAT_ADEPT") && !has("PROMOTION_SPELLSTAFF") {
            list.push("SPELLSTAFF");
        }
        if is_combat("UNITCOMBAT_RECON") && !has("PROMOTION_POISONED_BLADE") {
            list.push("POISONED_BLADE");
        }
        if is_combat("UNITCOMBAT_ARCHER") && !has("PROMOTION_FLAMING_ARROWS") {
            list.push("FLAMING_ARROWS");
        }
        if is_combat("UNITCOMBAT_DISCIPLE") && !has("PROMOTION_SHIELD_OF_FAITH") {
            list.push("SHIELD_OF_FAITH");
        }
        let weapon_tier = d.units[self.units[caster].unit_type].weapon_tier;
        if weapon_tier >= 1 && !has("PROMOTION_MITHRIL_WEAPONS") {
            if weapon_tier >= 3 && self.player_is_has_tech(p, d.t::<TechType>("TECH_IRON_WORKING")) {
                list.push("MITHRIL_WEAPONS");
            }
            if !has("PROMOTION_IRON_WEAPONS") {
                if weapon_tier >= 2 && self.player_is_has_tech(p, d.t::<TechType>("TECH_BRONZE_WORKING")) {
                    list.push("IRON_WEAPONS");
                }
                if !has("PROMOTION_BRONZE_WEAPONS") {
                    list.push("BRONZE_WEAPONS");
                }
            }
        }
        list
    }

    /// `exploreLairBigGood`'s list for `caster` exploring a lair on `plot`; `grace` answers its
    /// grace check.
    pub fn lair_big_good_outcomes(&self, caster: UnitId, plot: PlotIdx, grace: &mut dyn FnMut() -> bool) -> Vec<&'static str> {
        let d = self.d;
        let p = self.units[caster].owner;
        let mut list: Vec<&'static str> = vec!["TREASURE_VAULT", "GOLDEN_AGE"];
        if self.player_can_receive_goody(p, plot, d.t::<GoodyType>("GOODY_GRAVE_TECH"), Some(caster)) {
            list.push("TECH");
        }
        if !self.plot_is_water(plot) {
            list.extend_from_slice(&[
                "ITEM_JADE_TORC",
                "ITEM_ROD_OF_WINDS",
                "ITEM_TIMOR_MASK",
                "PRISONER_ADVENTURER",
                "PRISONER_ARTIST",
                "PRISONER_COMMANDER",
                "PRISONER_ENGINEER",
                "PRISONER_MERCHANT",
                "PRISONER_PROPHET",
                "PRISONER_SCIENTIST",
            ]);
            if self.map.plots[plot].bonus_type.is_none() {
                list.push("BONUS_MANA");
                if self.player_is_has_tech(p, d.t::<TechType>("TECH_MINING")) {
                    list.extend_from_slice(&["BONUS_COPPER", "BONUS_GEMS", "BONUS_GOLD"]);
                }
                if self.player_is_has_tech(p, d.t::<TechType>("TECH_SMELTING")) {
                    list.push("BONUS_IRON");
                }
            }
        }
        if self.plot_is_water(plot) {
            list.push("PRISONER_SEA_SERPENT");
            if self.map.plots[plot].bonus_type.is_none() {
                list.extend_from_slice(&["BONUS_CLAM", "BONUS_CRAB", "BONUS_FISH"]);
            }
        }
        if !grace() {
            list.extend_from_slice(&["PRISONER_ANGEL", "PRISONER_MONK", "PRISONER_ASSASSIN", "PRISONER_CHAMPION", "PRISONER_MAGE"]);
        }
        list
    }

    /// The list of `tier` (the big bad tier: its single outcome) for `caster` on `plot`.
    fn lair_tier_outcomes(&self, tier: LairTier, caster: UnitId, plot: PlotIdx, grace: &mut dyn FnMut() -> bool) -> Vec<&'static str> {
        match tier {
            LairTier::BigBad => vec![BIG_BAD_MONSTER],
            LairTier::Bad => self.lair_bad_outcomes(caster, plot),
            LairTier::Neutral => self.lair_neutral_outcomes(caster, plot),
            LairTier::Good => self.lair_good_outcomes(caster, plot),
            LairTier::BigGood => self.lair_big_good_outcomes(caster, plot, grace),
        }
    }

    // ------------------------------------------------------------------ the roll

    /// `spellExploreLair` / `spellExploreLairEpic` up to the tier's result: rolls the tier and
    /// applies one of its outcomes. Returns the chance in % that the lair is destroyed.
    pub fn lair_explore(&mut self, caster: UnitId, epic: bool) -> i32 {
        let rnd = self.soren_rand_num(100, "Explore Lair") + self.units[caster].level;
        self.lair_explore_tier(caster, LairTier::of_roll(rnd, epic))
    }

    /// One result of `tier` for `caster` on its plot (`exploreLairBigBad` .. `exploreLairBigGood`).
    /// Returns the chance in % that the lair is destroyed.
    pub fn lair_explore_tier(&mut self, caster: UnitId, tier: LairTier) -> i32 {
        let plot = self.unit_plot(caster);
        if tier == LairTier::BigBad {
            let lists = self.with_grace_rng(|g, grace| g.lair_big_bad_lists(plot, grace));
            self.lair_apply_big_bad(caster, plot, &lists);
            return lair_destroy_chance(tier, BIG_BAD_MONSTER);
        }
        let list = self.with_grace_rng(|g, grace| g.lair_tier_outcomes(tier, caster, plot, grace));
        let key = self.cf_pick(&list, "Pick Goody");
        self.lair_apply(tier, caster, plot, key);
        lair_destroy_chance(tier, key)
    }

    /// The big bad result: a named barbarian monster with a promotion appears next to the
    /// caster, with 0-4 henchmen.
    fn lair_apply_big_bad(&mut self, caster: UnitId, _plot: PlotIdx, lists: &LairBigBadLists) {
        let d = self.d;
        let p = self.units[caster].owner;
        let monster = self.cf_pick(&lists.monsters, "Pick Monster");
        let henchman = self.cf_pick(&lists.henchmen, "Pick Henchman");
        let promo = self.cf_pick(&lists.promotions, "Pick Promotion");
        let unit = d.t::<UnitType>(monster);
        let henchman = d.t::<UnitType>(henchman);
        if let Some(new_unit) = self.cf_add_unit_fixed(caster, unit) {
            self.unit_set_has_promotion(new_unit, d.t::<PromotionType>(promo), true);
            let name = self.cf_marnok_name_generator(new_unit);
            self.unit_set_name(new_unit, name);
            self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_BIGBAD");
            let barbarian = self.barbarian_player();
            let (x, y) = (self.units[new_unit].x, self.units[new_unit].y);
            for _ in 0..self.soren_rand_num(5, "Pick Henchmen") {
                self.player_init_unit(barbarian, henchman, x, y, None, Some(DirectionType::DIRECTION_SOUTH));
            }
        }
    }

    /// The goody a result receives (`pPlayer.receiveGoody`), for the results that are one.
    pub fn lair_outcome_goody(tier: LairTier, key: &str) -> Option<&'static str> {
        Some(match (tier, key) {
            (LairTier::Bad | LairTier::Neutral, "SPAWN_DROWN") => "GOODY_EXPLORE_LAIR_SPAWN_DROWN",
            (LairTier::Bad, "SPAWN_SCORPION_BAD") => "GOODY_EXPLORE_LAIR_SPAWN_SCORPION_BAD",
            (LairTier::Bad, "SPAWN_SEA_SERPENT") => "GOODY_EXPLORE_LAIR_SPAWN_SEA_SERPENT",
            (LairTier::Bad, "SPAWN_SPECTRE") => "GOODY_GRAVE_SPECTRE",
            (LairTier::Bad | LairTier::Neutral, "SPAWN_SPIDER") => "GOODY_EXPLORE_LAIR_SPAWN_SPIDER",
            (LairTier::Neutral, "SPAWN_FROSTLING") => "GOODY_EXPLORE_LAIR_SPAWN_FROSTLING",
            (LairTier::Neutral, "SPAWN_LIZARDMAN") => "GOODY_EXPLORE_LAIR_SPAWN_LIZARDMAN",
            (LairTier::Neutral, "SPAWN_SCORPION") => "GOODY_EXPLORE_LAIR_SPAWN_SCORPION",
            (LairTier::Neutral, "SPAWN_SKELETON") => "GOODY_EXPLORE_LAIR_SPAWN_SKELETON",
            (LairTier::Good, "HIGH_GOLD") => "GOODY_EXPLORE_LAIR_HIGH_GOLD",
            (LairTier::Good, "SUPPLIES") => "GOODY_EXPLORE_LAIR_SUPPLIES",
            (LairTier::Good, "EXPERIENCE") => "GOODY_EXPLORE_LAIR_EXPERIENCE",
            (LairTier::Good, "ITEM_HEALING_SALVE") => "GOODY_EXPLORE_LAIR_ITEM_HEALING_SALVE",
            // The two potion results are never put in the list (dead code in the Python).
            (LairTier::Good, "ITEM_POTION_OF_INVISIBILITY") => "GOODY_EXPLORE_LAIR_ITEM_POTION_OF_INVISIBILITY",
            (LairTier::Good, "ITEM_POTION_OF_RESTORATION") => "GOODY_EXPLORE_LAIR_ITEM_POTION_OF_RESTORATION",
            (LairTier::Good, "PRISONER_DISCIPLE_ASHEN") => "GOODY_EXPLORE_LAIR_PRISONER_DISCIPLE_ASHEN",
            (LairTier::Good, "PRISONER_DISCIPLE_EMPYREAN") => "GOODY_EXPLORE_LAIR_PRISONER_DISCIPLE_EMPYREAN",
            (LairTier::Good, "PRISONER_DISCIPLE_LEAVES") => "GOODY_EXPLORE_LAIR_PRISONER_DISCIPLE_LEAVES",
            (LairTier::Good, "PRISONER_DISCIPLE_OVERLORDS") => "GOODY_EXPLORE_LAIR_PRISONER_DISCIPLE_OVERLORDS",
            (LairTier::Good, "PRISONER_DISCIPLE_RUNES") => "GOODY_EXPLORE_LAIR_PRISONER_DISCIPLE_RUNES",
            (LairTier::Good, "PRISONER_DISCIPLE_ORDER") => "GOODY_EXPLORE_LAIR_PRISONER_DISCIPLE_ORDER",
            (LairTier::BigGood, "TREASURE_VAULT") => "GOODY_EXPLORE_LAIR_TREASURE_VAULT",
            (LairTier::BigGood, "ITEM_JADE_TORC") => "GOODY_EXPLORE_LAIR_ITEM_JADE_TORC",
            (LairTier::BigGood, "ITEM_ROD_OF_WINDS") => "GOODY_EXPLORE_LAIR_ITEM_ROD_OF_WINDS",
            (LairTier::BigGood, "ITEM_TIMOR_MASK") => "GOODY_EXPLORE_LAIR_ITEM_TIMOR_MASK",
            (LairTier::BigGood, "PRISONER_ADVENTURER") => "GOODY_EXPLORE_LAIR_PRISONER_ADVENTURER",
            (LairTier::BigGood, "PRISONER_ANGEL") => "GOODY_EXPLORE_LAIR_PRISONER_ANGEL",
            (LairTier::BigGood, "PRISONER_ARTIST") => "GOODY_EXPLORE_LAIR_PRISONER_ARTIST",
            (LairTier::BigGood, "PRISONER_ASSASSIN") => "GOODY_EXPLORE_LAIR_PRISONER_ASSASSIN",
            (LairTier::BigGood, "PRISONER_CHAMPION") => "GOODY_EXPLORE_LAIR_PRISONER_CHAMPION",
            (LairTier::BigGood, "PRISONER_COMMANDER") => "GOODY_EXPLORE_LAIR_PRISONER_COMMANDER",
            (LairTier::BigGood, "PRISONER_ENGINEER") => "GOODY_EXPLORE_LAIR_PRISONER_ENGINEER",
            (LairTier::BigGood, "PRISONER_MAGE") => "GOODY_EXPLORE_LAIR_PRISONER_MAGE",
            (LairTier::BigGood, "PRISONER_MERCHANT") => "GOODY_EXPLORE_LAIR_PRISONER_MERCHANT",
            (LairTier::BigGood, "PRISONER_MONK") => "GOODY_EXPLORE_LAIR_PRISONER_MONK",
            (LairTier::BigGood, "PRISONER_PROPHET") => "GOODY_EXPLORE_LAIR_PRISONER_PROPHET",
            (LairTier::BigGood, "PRISONER_SEA_SERPENT") => "GOODY_EXPLORE_LAIR_PRISONER_SEA_SERPENT",
            (LairTier::BigGood, "PRISONER_SCIENTIST") => "GOODY_EXPLORE_LAIR_PRISONER_SCIENTIST",
            (LairTier::BigGood, "TECH") => "GOODY_GRAVE_TECH",
            _ => return None,
        })
    }

    /// Applies result `key` of `tier` (not the big bad one) to `caster` on `plot`.
    fn lair_apply(&mut self, tier: LairTier, caster: UnitId, plot: PlotIdx, key: &'static str) {
        let d = self.d;
        let p = self.units[caster].owner;
        if let Some(goody) = Self::lair_outcome_goody(tier, key) {
            self.cf_receive_goody(p, plot, goody, caster);
            return;
        }
        let promotion = |name: &str| d.t::<PromotionType>(name);
        let has = |g: &Game, name: &str| g.units[caster].has_promotion[d.t::<PromotionType>(name)];
        // A promotion and its message (`TXT_KEY_MESSAGE_EXPLORE_LAIR_<key>`).
        let grant = |g: &mut Game, promo: &str, msg: &'static str| {
            g.unit_set_has_promotion(caster, promotion(promo), true);
            g.cf_msg(p, msg);
        };
        match (tier, key) {
            (LairTier::Bad, "DEATH") => {
                // Python quirk: `caster.kill(True, 0)` names player 0 as the killer.
                self.unit_kill(caster, true, Some(PlayerId(0)));
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_DEATH");
            }
            (LairTier::Bad, "COLLAPSE") => {
                self.unit_do_damage(caster, 50, 90, None, Some(d.t::<DamageType>("DAMAGE_PHYSICAL")), false);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_COLLAPSE");
            }
            (LairTier::Bad, "CRAZED") => grant(self, "PROMOTION_CRAZED", "TXT_KEY_MESSAGE_EXPLORE_LAIR_CRAZED"),
            (LairTier::Bad, "DEMONIC_POSSESSION") => {
                self.unit_set_has_promotion(caster, promotion("PROMOTION_ENRAGED"), true);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_CRAZED"), true);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_DEMON"), true);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_POSSESSED");
            }
            (LairTier::Bad, "DISEASED") => grant(self, "PROMOTION_DISEASED", "TXT_KEY_MESSAGE_EXPLORE_LAIR_DISEASED"),
            (LairTier::Bad, "ENRAGED") => grant(self, "PROMOTION_ENRAGED", "TXT_KEY_MESSAGE_EXPLORE_LAIR_ENRAGED"),
            (LairTier::Bad, "PLAGUED") => grant(self, "PROMOTION_PLAGUED", "TXT_KEY_MESSAGE_EXPLORE_LAIR_PLAGUED"),
            (LairTier::Bad, "POISONED") => {
                self.unit_set_has_promotion(caster, promotion("PROMOTION_POISONED"), true);
                self.unit_do_damage(caster, 25, 90, None, Some(d.t::<DamageType>("DAMAGE_POISON")), false);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_POISONED");
            }
            (LairTier::Bad, "WITHERED") => grant(self, "PROMOTION_WITHERED", "TXT_KEY_MESSAGE_EXPLORE_LAIR_WITHERED"),
            (LairTier::Bad, "RUSTED") => {
                self.unit_set_has_promotion(caster, promotion("PROMOTION_RUSTED"), true);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_BRONZE_WEAPONS"), false);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_IRON_WEAPONS"), false);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_MITHRIL_WEAPONS"), false);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_RUSTED");
            }
            (LairTier::Neutral, "CAGE") => {
                self.plot_set_improvement_type(plot, Some(d.t::<ImprovementType>("IMPROVEMENT_CAGE")));
                let held = d.t::<PromotionType>("PROMOTION_HELD");
                for i in (0..=self.map.plots[plot].units.len()).rev() {
                    if let Some(&unit) = self.map.plots[plot].units.get(i) {
                        self.unit_set_has_promotion(unit, held, true);
                    }
                }
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_CAGE");
            }
            (LairTier::Neutral, "DEPTHS") => self.cf_lair_event(caster, "EVENTTRIGGER_EXPLORE_LAIR_DEPTHS"),
            (LairTier::Neutral, "DWARF_VS_LIZARDMEN") => self.cf_lair_event(caster, "EVENTTRIGGER_EXPLORE_LAIR_DWARF_VS_LIZARDMEN"),
            (LairTier::Neutral, "PORTAL") => self.cf_lair_event(caster, "EVENTTRIGGER_EXPLORE_LAIR_PORTAL"),
            (LairTier::Neutral, "MUTATED") => grant(self, "PROMOTION_MUTATED", "TXT_KEY_MESSAGE_EXPLORE_LAIR_MUTATED"),
            (LairTier::Neutral, "NOTHING") => self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_NOTHING"),
            (LairTier::Good, "TREASURE") => self.cf_place_treasure(p, d.t::<UnitType>("EQUIPMENT_TREASURE")),
            (LairTier::Good, "SPIRIT_GUIDE") => grant(self, "PROMOTION_SPIRIT_GUIDE", "TXT_KEY_MESSAGE_EXPLORE_LAIR_SPIRIT_GUIDE"),
            (LairTier::Good, "ENCHANTED_BLADE") => grant(self, "PROMOTION_ENCHANTED_BLADE", "TXT_KEY_MESSAGE_EXPLORE_LAIR_ENCHANTED_BLADE"),
            (LairTier::Good, "SPELLSTAFF") => grant(self, "PROMOTION_SPELLSTAFF", "TXT_KEY_MESSAGE_EXPLORE_LAIR_SPELLSTAFF"),
            (LairTier::Good, "POISONED_BLADE") => grant(self, "PROMOTION_POISONED_BLADE", "TXT_KEY_MESSAGE_EXPLORE_LAIR_POISONED_BLADE"),
            (LairTier::Good, "FLAMING_ARROWS") => grant(self, "PROMOTION_FLAMING_ARROWS", "TXT_KEY_MESSAGE_EXPLORE_LAIR_FLAMING_ARROWS"),
            (LairTier::Good, "SHIELD_OF_FAITH") => grant(self, "PROMOTION_SHIELD_OF_FAITH", "TXT_KEY_MESSAGE_EXPLORE_LAIR_SHIELD_OF_FAITH"),
            (LairTier::Good, "BRONZE_WEAPONS") => {
                self.unit_set_has_promotion(caster, promotion("PROMOTION_BRONZE_WEAPONS"), true);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_BRONZE_WEAPONS");
                if has(self, "PROMOTION_RUSTED") {
                    self.unit_set_has_promotion(caster, promotion("PROMOTION_RUSTED"), false);
                }
            }
            (LairTier::Good, "IRON_WEAPONS") => {
                self.unit_set_has_promotion(caster, promotion("PROMOTION_IRON_WEAPONS"), true);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_BRONZE_WEAPONS"), false);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_IRON_WEAPONS");
                if has(self, "PROMOTION_RUSTED") {
                    self.unit_set_has_promotion(caster, promotion("PROMOTION_RUSTED"), false);
                }
            }
            (LairTier::Good, "MITHRIL_WEAPONS") => {
                self.unit_set_has_promotion(caster, promotion("PROMOTION_MITHRIL_WEAPONS"), true);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_IRON_WEAPONS"), false);
                self.unit_set_has_promotion(caster, promotion("PROMOTION_BRONZE_WEAPONS"), false);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_MITHRIL_WEAPONS");
                if has(self, "PROMOTION_RUSTED") {
                    self.unit_set_has_promotion(caster, promotion("PROMOTION_RUSTED"), false);
                }
            }
            (LairTier::BigGood, "BONUS_CLAM" | "BONUS_COPPER" | "BONUS_CRAB" | "BONUS_FISH" | "BONUS_GOLD" | "BONUS_GEMS" | "BONUS_IRON" | "BONUS_MANA") => {
                // These results are the bonus type names.
                self.plot_set_bonus_type(plot, Some(d.t::<BonusType>(key)));
            }
            (LairTier::BigGood, "GOLDEN_AGE") => {
                let length = self.golden_age_length();
                self.player_change_golden_age_turns(p, length);
                self.cf_msg(p, "TXT_KEY_MESSAGE_EXPLORE_LAIR_GOLDEN_AGE");
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------ odds

    /// Whether Explore Lair on `plot` is the epic kind (`spellExploreLairEpic`), or `None` when no
    /// Explore Lair spell targets the plot's improvement.
    pub fn lair_explore_kind(&self, plot: PlotIdx) -> Option<bool> {
        self.lair_explore_kind_of(self.map.plots[plot].improvement_type?)
    }

    /// Whether Explore Lair on `improvement` is the epic kind, or `None` when no Explore Lair
    /// spell targets it.
    pub fn lair_explore_kind_of(&self, improvement: ImprovementType) -> Option<bool> {
        self.d
            .spells
            .iter()
            .filter(|s| s.improvement_prereq == Some(improvement))
            .find_map(|s| match s.py_result.split('(').next().unwrap_or("").trim() {
                "spellExploreLair" => Some(false),
                "spellExploreLairEpic" => Some(true),
                _ => None,
            })
    }

    /// The odds of `unit` exploring the lair on `plot` (wherever the unit stands now): `None` when
    /// no Explore Lair spell targets the plot's improvement. The lists are those of the unit and
    /// the plot as they are now.
    pub fn lair_explore_odds(&self, unit: UnitId, plot: PlotIdx) -> Option<LairOdds> {
        let epic = self.lair_explore_kind(plot)?;
        let level = self.units[unit].level;
        let mut tiers = Vec::new();
        let mut outcomes = Vec::new();
        let mut survives_ppt: i128 = 0;
        let bound = self.grace_bound() as i128;
        let grace_den = bound.pow(MAX_GRACE_CALLS);
        for tier in LairTier::ALL {
            let Some((lo, hi)) = tier.roll_range(epic) else { continue };
            // The rolls r in 0..100 with lo <= r + level < hi.
            let count = ((hi - level).min(100) - (lo - level).max(0)).max(0) as i128;
            if count == 0 {
                continue;
            }
            tiers.push((tier, ppt_to_ppm(count * PPT / 100)));
            // Each outcome's weight within the tier, out of PPT.
            let mut within: Vec<(&'static str, i128)> = Vec::new();
            for (weight, list) in self.over_grace(|grace| self.lair_tier_outcomes(tier, unit, plot, grace)) {
                let len = list.len() as i128;
                for &key in &list {
                    let share = weight * PPT / (grace_den * len);
                    match within.iter_mut().find(|(k, _)| *k == key) {
                        Some(entry) => entry.1 += share,
                        None => within.push((key, share)),
                    }
                }
            }
            for (key, share) in within {
                let ppt = count * share / 100;
                let destroy_pct = lair_destroy_chance(tier, key);
                if !lair_outcome_replaces_lair(tier, key) {
                    survives_ppt += ppt * (100 - destroy_pct) as i128 / 100;
                }
                outcomes.push(LairOutcomeOdds { tier, key, ppm: ppt_to_ppm(ppt), destroy_pct });
            }
        }
        let mut big_bad = LairBigBadLists::default();
        for (_, lists) in self.over_grace(|grace| self.lair_big_bad_lists(plot, grace)) {
            for (all, these) in [(&mut big_bad.monsters, lists.monsters), (&mut big_bad.henchmen, lists.henchmen), (&mut big_bad.promotions, lists.promotions)] {
                for name in these {
                    if !all.contains(&name) {
                        all.push(name);
                    }
                }
            }
        }
        let grace_ppm = ppt_to_ppm(self.grace_passing_values() as i128 * PPT / bound);
        Some(LairOdds { epic, level, tiers, outcomes, lair_survives_ppm: ppt_to_ppm(survives_ppt), grace_ppm, big_bad })
    }

    /// The odds of player `p`'s `unit` (if any) entering the goody hut on `plot` now
    /// (`doGoody`: up to `NUM_DO_GOODY_ATTEMPTS` draws from the difficulty's list, each kept only
    /// if `canReceiveGoody` allows it).
    pub fn goody_hut_odds(&self, p: PlayerId, plot: PlotIdx, unit: Option<UnitId>) -> GoodyOdds {
        let d = self.d;
        let attempts = d.defines.num_do_goody_attempts.max(0);
        let list: &[GoodyType] = match self.player_get_handicap_type(p) {
            Some(h) if !self.player_is_barbarian(p) => &d.handicaps[h].goodies,
            _ => &[],
        };
        let total = list.len() as i128;
        let mut outcomes: Vec<GoodyOutcomeOdds> = Vec::new();
        for &goody in list {
            match outcomes.iter_mut().find(|o| o.goody == goody) {
                Some(o) => o.slots += 1,
                None => outcomes.push(GoodyOutcomeOdds { goody, ppm: 0, slots: 1, allowed: self.player_can_receive_goody(p, plot, goody, unit) }),
            }
        }
        if total == 0 {
            return GoodyOdds { outcomes, slots_total: 0, attempts, nothing_ppm: 1_000_000 };
        }
        let allowed: i128 = outcomes.iter().filter(|o| o.allowed).map(|o| o.slots as i128).sum();
        let refused = total - allowed;
        // sum_{k < attempts} refused^k * total^(attempts-1-k): the draws that reach attempt k+1.
        let reach: i128 = (0..attempts as u32).map(|k| refused.pow(k) * total.pow(attempts as u32 - 1 - k)).sum();
        let den = total.pow(attempts as u32);
        for o in &mut outcomes {
            if o.allowed {
                o.ppm = ppt_to_ppm(o.slots as i128 * reach * PPT / den);
            }
        }
        let nothing_ppm = ppt_to_ppm(refused.pow(attempts as u32) * PPT / den);
        GoodyOdds { outcomes, slots_total: total as i32, attempts, nothing_ppm }
    }

    /// The gold a goody gives, `(min, max)` after the game speed, or `None` when it gives none.
    pub fn goody_gold_range(&self, goody: GoodyType) -> Option<(i32, i32)> {
        let info = &self.d.goodies[goody];
        let speed = self.d.game_speeds[self.game_speed_type()].growth_percent;
        let max = info.gold + (info.gold_rand1 - 1).max(0) + (info.gold_rand2 - 1).max(0);
        if info.gold == 0 && max == 0 {
            return None;
        }
        Some((info.gold * speed / 100, max * speed / 100))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    fn open_land(g: &Game) -> Vec<PlotIdx> {
        g.map
            .plot_ids()
            .filter(|&p| !g.plot_is_water(p) && !g.plot_is_impassable(p) && g.map.plots[p].plot_city.is_none() && g.map.plots[p].units.is_empty())
            .collect()
    }

    fn first_civ(g: &Game) -> PlayerId {
        g.player_ids().find(|&p| g.players[p].alive && !g.player_is_barbarian(p)).expect("a civilization")
    }

    /// A unit of the first civilization standing on a lair of `improvement` on open land.
    fn on_lair(g: &mut Game, improvement: &str, unit: &str) -> (UnitId, PlotIdx) {
        let d = g.d;
        let p = first_civ(g);
        let plot = open_land(g)[0];
        let (x, y) = g.map.xy(plot);
        let u = g.player_init_unit(p, d.t::<UnitType>(unit), x, y, None, None);
        g.plot_set_improvement_type(plot, Some(d.t::<ImprovementType>(improvement)));
        (u, plot)
    }

    #[test]
    fn tiers_follow_the_level() {
        let mut g = game();
        let (u, plot) = on_lair(&mut g, "IMPROVEMENT_BARROW", "UNIT_WARRIOR");
        let odds = g.lair_explore_odds(u, plot).expect("a barrow can be explored");
        assert!(!odds.epic);
        let tier = |o: &LairOdds, t: LairTier| o.tiers.iter().find(|x| x.0 == t).map_or(0, |x| x.1);
        assert_eq!(odds.level, 1);
        assert_eq!(tier(&odds, LairTier::BigBad), 130_000);
        assert_eq!(tier(&odds, LairTier::BigGood), 70_000);
        g.units[u].level = 20;
        let odds = g.lair_explore_odds(u, plot).unwrap();
        assert_eq!(tier(&odds, LairTier::BigBad), 0);
        assert_eq!(tier(&odds, LairTier::Bad), 240_000);
        assert_eq!(tier(&odds, LairTier::BigGood), 260_000);
        // Level 1 alone can die.
        assert!(odds.outcomes.iter().all(|o| o.key != "DEATH"));
    }

    #[test]
    fn outcome_odds_sum_to_one() {
        for lair in ["IMPROVEMENT_BARROW", "IMPROVEMENT_RUINS", "IMPROVEMENT_GOBLIN_FORT", "IMPROVEMENT_DUNGEON"] {
            let mut g = game();
            let (u, plot) = on_lair(&mut g, lair, "UNIT_WARRIOR");
            let odds = g.lair_explore_odds(u, plot).unwrap();
            let sum: i64 = odds.outcomes.iter().map(|o| o.ppm).sum();
            assert!((sum - 1_000_000).abs() <= 100, "{lair}: {sum}");
            let tiers: i64 = odds.tiers.iter().map(|t| t.1).sum();
            assert_eq!(tiers, 1_000_000, "{lair}");
            assert!(odds.lair_survives_ppm > 0 && odds.lair_survives_ppm < 1_000_000);
            assert!(!odds.big_bad.monsters.is_empty());
        }
    }

    /// The roll draws from the lists the odds are computed from, with the same frequencies.
    #[test]
    fn the_roll_matches_the_odds() {
        let mut g = game();
        let (u, plot) = on_lair(&mut g, "IMPROVEMENT_GOBLIN_FORT", "UNIT_WARRIOR");
        g.units[u].level = 3;
        let odds = g.lair_explore_odds(u, plot).unwrap();
        let mut counts: Vec<(LairTier, &'static str, i64)> = Vec::new();
        const N: i64 = 40_000;
        for seed in 0..N {
            g.soren_rand = Rng::new(seed as u32 * 7919 + 13);
            let rnd = g.soren_rand_num(100, "Explore Lair") + g.units[u].level;
            let tier = LairTier::of_roll(rnd, odds.epic);
            let key = if tier == LairTier::BigBad {
                BIG_BAD_MONSTER
            } else {
                let list = g.with_grace_rng(|g, grace| g.lair_tier_outcomes(tier, u, plot, grace));
                g.cf_pick(&list, "Pick Goody")
            };
            match counts.iter_mut().find(|c| c.0 == tier && c.1 == key) {
                Some(c) => c.2 += 1,
                None => counts.push((tier, key, 1)),
            }
        }
        for (tier, key, n) in &counts {
            let o = odds.outcomes.iter().find(|o| o.tier == *tier && o.key == *key).unwrap_or_else(|| panic!("{key} rolled but not in the odds"));
            let freq = n * 1_000_000 / N;
            assert!((freq - o.ppm).abs() < 4_000, "{key}: rolled {freq} ppm, odds {}", o.ppm);
        }
    }

    #[test]
    fn goody_odds_sum_to_one_and_respect_conditions() {
        let mut g = game();
        let p = first_civ(&g);
        let plot = open_land(&g)[0];
        let (x, y) = g.map.xy(plot);
        let u = g.player_init_unit(p, g.d.t::<UnitType>("UNIT_WARRIOR"), x, y, None, None);
        let odds = g.goody_hut_odds(p, plot, Some(u));
        let sum: i64 = odds.outcomes.iter().map(|o| o.ppm).sum::<i64>() + odds.nothing_ppm;
        assert!((sum - 1_000_000).abs() <= 20, "{sum}");
        assert_eq!(odds.slots_total, 20);
        // A healthy unit is not healed.
        let healing = g.d.t::<GoodyType>("GOODY_HEALING");
        if let Some(o) = odds.outcomes.iter().find(|o| o.goody == healing) {
            assert!(!o.allowed && o.ppm == 0);
        }
        let low = g.d.t::<GoodyType>("GOODY_LOW_GOLD");
        let (min, max) = g.goody_gold_range(low).unwrap();
        assert!(min > 0 && max > min);
        assert_eq!(g.goody_gold_range(healing), None);
    }

    #[test]
    fn grace_bound_is_the_grace_check() {
        let g = game();
        let bound = g.grace_bound();
        assert!(bound > 0);
        // Turn 0: every roll passes.
        assert_eq!(g.grace_passing_values(), bound);
    }
}
