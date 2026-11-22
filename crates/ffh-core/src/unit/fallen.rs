//! This game's rules (docs/PORTING.md, "Changes from FFH2", Heroes): the dead the rules remember.
//!
//! * **Heroes return.** A hero (a unit holding `PROMOTION_HERO`) killed by harm -- a battle, a
//!   spell's damage, anything that takes its HP to 0 -- is not lost: it returns to its owner's
//!   capital [`HERO_RECOVERY_TURNS`] turns later (scaled to the game speed), with its promotions,
//!   experience and level, at [`HERO_RETURN_DAMAGE`] damage and with one Wound more
//!   (`PROMOTION_WOUNDED`, whose stat changes apply once per Wound, about one Combat promotion's
//!   worth each, for the rest of the game). A hero removed another way (upgraded, disbanded,
//!   captured, its civilization destroyed) is not remembered. An immortal hero is first reborn at
//!   once as before (FfH's immortality), spending it; its next death sends it to recover.
//! * **Resurrection's dead.** Any other unit of a civilization killed by harm is remembered for
//!   [`RESURRECTION_TURNS`] turns: the Resurrection spell (Life III) brings back the one its
//!   caster chooses, with its promotions and experience. Summons, equipment and world units are
//!   not remembered.

use crate::game::Game;
use crate::ids::*;
use crate::log::{LogMark, LogTag};
use crate::structs::SpellTarget;
use ffh_data::*;

/// Turns at normal speed a dead hero is away before it returns.
pub const HERO_RECOVERY_TURNS: i32 = 15;
/// Turns at normal speed a dead unit other than a hero stays within Resurrection's reach.
pub const RESURRECTION_TURNS: i32 = 20;
/// The damage (of 100 HP) a hero returns with.
pub const HERO_RETURN_DAMAGE: i32 = 75;

/// A dead unit the rules remember: enough to make it again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FallenUnit {
    /// The id it had (ids are never reused): the Resurrection choice's number.
    pub id: UnitId,
    pub unit_type: UnitType,
    /// Its own name (empty: its type's).
    pub name: String,
    pub ai_type: Option<UnitAIType>,
    pub experience: i32,
    pub level: i32,
    /// Its promotions, without equipment and temporary states (spells' buffs, poison, ...).
    pub promotions: Vec<PromotionType>,
    pub religion: Option<ReligionType>,
    /// Its Wounds when it died.
    pub wounds: i32,
    /// The game turn it died.
    pub died: i32,
    /// A hero's: the game turn it returns; -1 for the others.
    pub returns: i32,
}

/// `PROMOTION_WOUNDED`, cached.
pub fn wounded_promotion(d: &GameData) -> Option<PromotionType> {
    static P: std::sync::OnceLock<Option<PromotionType>> = std::sync::OnceLock::new();
    *P.get_or_init(|| d.ty::<PromotionType>("PROMOTION_WOUNDED"))
}

/// Is promotion `p` one a fallen unit keeps? Not equipment (it drops where the unit dies) and
/// not a passing state: a spell's buff, one removed by combat, healing or casting, one that
/// expires, a hold.
pub fn kept_promotion(d: &GameData, p: PromotionType) -> bool {
    let i = &d.promotions[p];
    !(i.equipment || i.dispellable || i.removed_by_combat || i.removed_when_healed || i.removed_by_casting || i.expire_chance > 0 || i.held)
        && Some(p) != wounded_promotion(d)
}

impl FallenUnit {
    /// Its name for people: its own, else its type's.
    pub fn display_name(&self, d: &GameData) -> String {
        if self.name.is_empty() {
            d.text.name(&d.units[self.unit_type].base.description).to_string()
        } else {
            self.name.clone()
        }
    }
}

impl Game {
    /// The turns of `turns` at normal speed in this game's speed.
    fn fallen_speed_turns(&self, turns: i32) -> i32 {
        crate::crusade::speed_turns(self, turns)
    }

    /// How many times promotion `p` counts on unit `u` while held: a Wounded unit's Wounds, else 1.
    pub fn unit_promotion_stacks(&self, u: UnitId, p: PromotionType) -> i32 {
        if Some(p) == wounded_promotion(self.d) {
            self.units[u].wounds.max(1)
        } else {
            1
        }
    }

    /// Unit `u`'s Wounds, for people: their effect in one line (`Wounds: 2. Attack -40%, ...`),
    /// and each stat's calculation (`Attack: -20% × 2 = -40%.`); `None` without Wounds.
    pub fn unit_wounds_calc(&self, u: UnitId) -> Option<(String, Vec<String>)> {
        let n = self.units[u].wounds;
        let w = wounded_promotion(self.d)?;
        if n <= 0 || !self.units[u].has_promotion[w] {
            return None;
        }
        let changes = &self.d.promotions[w].stat_changes;
        let mut parts = Vec::new();
        let mut calc = Vec::new();
        for s in StatType::ALL {
            let pct = changes[s.idx()].percent;
            if pct == 0 {
                continue;
            }
            let name = match s {
                StatType::Hp => "HP",
                StatType::Attack => "Attack",
                StatType::Defense => "Defense",
                StatType::Magic => "Magic",
                StatType::MagicDefense => "Magic defense",
                StatType::Accuracy => "Accuracy",
                StatType::Evasion => "Evasion",
                StatType::Crit => "Crit",
                StatType::Speed => "Speed",
            };
            parts.push(format!("{name} {}%", pct * n));
            calc.push(format!("{name}: {pct}% × {n} = {}%.", pct * n));
        }
        Some((format!("Wounds: {n}. {}.", parts.join(", ")), calc))
    }

    /// Sets unit `u`'s Wounds to `n` (holding `PROMOTION_WOUNDED` while `n > 0`).
    pub fn unit_set_wounds(&mut self, u: UnitId, n: i32) {
        let Some(w) = wounded_promotion(self.d) else { return };
        if self.units[u].has_promotion[w] {
            // Removing it takes off every Wound's stat changes and clears the count.
            self.unit_set_has_promotion(u, w, false);
        }
        self.units[u].wounds = n.max(0);
        if n > 0 {
            self.unit_set_has_promotion(u, w, true);
            if !self.units[u].has_promotion[w] {
                // Immune to it.
                self.units[u].wounds = 0;
            }
        }
    }

    /// Whether some player has a hero of class `class`, alive or recovering.
    pub fn hero_class_alive(&self, class: UnitClassType) -> bool {
        self.player_ids().any(|p| self.player_hero_class_alive(p, class))
    }

    /// Whether player `p` has a unit of class `class`, alive or recovering.
    pub fn player_hero_class_alive(&self, p: PlayerId, class: UnitClassType) -> bool {
        let d = self.d;
        self.players[p].unit_class_count[class] > 0
            || self.players[p].recovering.iter().any(|r| d.units[r.unit_type].unit_class_type == Some(class))
    }

    /// The record unit `u` leaves when it dies now, if the rules remember it: a hero or a unit
    /// Resurrection can bring back, killed by harm (its HP at 0), of a civilization (a lent unit's
    /// for its lender). Read in `unit_kill` before the unit's equipment drops.
    pub(crate) fn unit_fallen_record(&self, u: UnitId) -> Option<(PlayerId, FallenUnit)> {
        let d = self.d;
        let unit = &self.units[u];
        // (A Caravan re-forms by the trade rules, docs/TRADE.md 4.3: it is not remembered.)
        if !self.unit_is_dead(u) || unit.duration > 0 || unit.battle_summon || Some(unit.unit_type) == self.caravan_unit_type() {
            return None;
        }
        let owner = unit.lent.as_ref().map_or(unit.owner, |l| l.lender);
        if self.player_is_barbarian(owner) || !self.players[owner].alive {
            return None;
        }
        let info = &d.units[unit.unit_type];
        let hero = self.unit_is_hero(u);
        if !hero {
            // Equipment, world units and units with a national or team limit are not brought
            // back (a limit would be passed).
            if info.equipment_promotion.is_some() {
                return None;
            }
            if let Some(class) = info.unit_class_type {
                let c = &d.unit_classes[class];
                if c.max_global_instances > 0 || c.max_team_instances > 0 || c.max_player_instances > 0 {
                    return None;
                }
            }
        }
        let turn = self.game_turn();
        let rec = FallenUnit {
            id: u,
            unit_type: unit.unit_type,
            name: unit.name.clone(),
            ai_type: unit.ai_type,
            experience: unit.experience,
            level: unit.level,
            promotions: d.promotions.keys().filter(|&p| unit.has_promotion[p] && kept_promotion(d, p)).collect(),
            religion: unit.religion,
            wounds: unit.wounds,
            died: turn,
            returns: if hero { turn + self.fallen_speed_turns(HERO_RECOVERY_TURNS) } else { -1 },
        };
        Some((owner, rec))
    }

    /// Remembers `rec` for player `p` (`unit_kill`, once the unit is gone).
    pub(crate) fn player_add_fallen(&mut self, p: PlayerId, rec: FallenUnit) {
        let turn = self.game_turn();
        if rec.returns >= 0 {
            let name = rec.display_name(self.d);
            let left = rec.returns - turn;
            let at = self.player_get_capital_city(p).map(|c| (self.cities[c].x, self.cities[c].y));
            let text = move || format!("{name} has fallen. Returns to the capital in {left} turns, Wounded.");
            match at {
                Some(at) => self.log.msg_marked(turn, Some(p), at, LogMark::Loss, LogTag::None, text),
                None => self.log.msg(turn, Some(p), text),
            }
            self.players[p].recovering.push(rec);
        } else {
            self.players[p].fallen.push(rec);
        }
    }

    /// At the start of player `p`'s turn: its heroes due return to its capital (its first city
    /// without one; they wait while it has none), and the dead out of Resurrection's reach are
    /// forgotten.
    pub fn player_do_fallen(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        let reach = self.fallen_speed_turns(RESURRECTION_TURNS);
        self.players[p].fallen.retain(|r| turn - r.died <= reach);
        if self.players[p].recovering.iter().all(|r| r.returns > turn) {
            return;
        }
        let Some(city) = self.player_get_capital_city(p).or_else(|| self.players[p].cities.first().copied()) else { return };
        let mut i = 0;
        while i < self.players[p].recovering.len() {
            if self.players[p].recovering[i].returns <= turn {
                let rec = self.players[p].recovering.remove(i);
                self.player_return_hero(p, rec, city);
            } else {
                i += 1;
            }
        }
    }

    /// Makes `rec` again for player `p` in city `city`: its type, name, experience, level and
    /// promotions, with `wounds` Wounds; returns the new unit.
    pub(crate) fn player_remake_fallen(&mut self, p: PlayerId, rec: &FallenUnit, x: i32, y: i32, wounds: i32) -> UnitId {
        let u = self.player_init_unit(p, rec.unit_type, x, y, rec.ai_type, None);
        if !rec.name.is_empty() {
            self.unit_set_name(u, rec.name.clone());
        }
        for &pr in &rec.promotions {
            if !self.units[u].has_promotion[pr] {
                self.unit_set_has_promotion(u, pr, true);
            }
        }
        self.unit_set_level(u, rec.level.max(self.units[u].level));
        self.unit_set_experience(u, rec.experience.max(self.units[u].experience), -1);
        if rec.religion.is_some() {
            self.units[u].religion = rec.religion;
        }
        self.unit_set_wounds(u, wounds);
        self.unit_test_promotion_ready(u);
        u
    }

    /// Hero `rec` returns to player `p`'s city `city`, Wounded once more.
    fn player_return_hero(&mut self, p: PlayerId, rec: FallenUnit, city: CityId) {
        let (x, y) = (self.cities[city].x, self.cities[city].y);
        let u = self.player_remake_fallen(p, &rec, x, y, rec.wounds + 1);
        self.unit_set_damage(u, HERO_RETURN_DAMAGE, None);
        let name = rec.display_name(self.d);
        let wounds = self.units[u].wounds;
        let city_name = self.cities[city].name.clone();
        let turn = self.game_turn();
        self.log.msg_marked(turn, Some(p), (x, y), LogMark::Event, LogTag::None, || format!("{name} returns to {city_name}. Wounds: {wounds}."));
    }

    /// The dead Resurrection cast by `caster` may bring back: its owner's remembered dead, but
    /// not one another of its units is already resurrecting.
    pub fn resurrection_candidates(&self, caster: UnitId) -> Vec<&FallenUnit> {
        let p = self.units[caster].owner;
        let taken: Vec<i32> = self.players[p]
            .units
            .iter()
            .filter(|&&o| o != caster)
            .filter_map(|&o| match self.units[o].delayed_spell_target {
                Some(SpellTarget::Choice(i)) if self.units[o].delayed_spell.is_some() => Some(i),
                _ => None,
            })
            .collect();
        self.players[p].fallen.iter().filter(|r| !taken.contains(&(r.id.0 as i32))).collect()
    }

    /// Resurrection: caster `caster`'s owner gets back its dead unit numbered `choice` (its old
    /// id) on the caster's plot, with its promotions and experience; false if it is not there.
    pub fn resurrect_fallen(&mut self, caster: UnitId, choice: i32) -> bool {
        let p = self.units[caster].owner;
        let Some(i) = self.players[p].fallen.iter().position(|r| r.id.0 as i32 == choice) else { return false };
        let rec = self.players[p].fallen.remove(i);
        let (x, y) = (self.units[caster].x, self.units[caster].y);
        self.player_remake_fallen(p, &rec, x, y, rec.wounds);
        let name = rec.display_name(self.d);
        let turn = self.game_turn();
        self.log.msg_marked(turn, Some(p), (x, y), LogMark::Event, LogTag::None, || format!("Resurrection: {name} returns."));
        true
    }
}
