//! Battle reports and the battle log of formation battles (spec section 2; docs/ENGAGEMENTS.md,
//! "The battle log"): saved state, written by the engagement's post-battle step and read by the
//! views, never by the rules.
//!
//! **Two layers.** A [`BattleReport`] is kept for each engagement of the current and the previous
//! game turn and for the last `ENGAGEMENT_REPORTS_KEPT` engagements: where, who fought (a
//! [`ReportSide`] each, with its real and shown owner), how many each side committed, lost, saw
//! escape and kept standing, and the [`Outcome`] (whether the plot fell). Its [`BattleLog`] is the
//! full record the client replays: the fighters on their grids, the ground and its rules, the
//! slots and lanes, every event in order (the morale check and the defensive strikes, every
//! action of the battle as the resolver fought it, the end, the post-battle steps) and the pairs
//! that hit. The engagement writes a log only for a battle a human could see; the log stays for
//! every report of the current and the previous turn and for the last
//! `ENGAGEMENT_REPORT_LOGS_KEPT` logged reports, and older reports keep their summary
//! ([`Game::battle_reports_trim`]). Both caps give way to the two-turn window.
//!
//! **Fighter order.** [`BattleLog::fighters`] holds the committed units first, in the
//! engagement's order (the attacker's formation by lane then row, then the defender's), sat-out
//! units in their place, then the reserves (the other units on both plots the battle met,
//! `place: None`). The resolver fights the committed units that did not sit out, in that order;
//! the log's actions ([`LogEvent::Action`]) name fighters by their log index.
//!
//! **Hidden nationality (G5).** Every side and fighter keeps its real owner and the owner the
//! other side saw (`unit_get_visual_owner` at battle time). A viewer gets the real owners of its
//! own side and the shown ones of the other ([`BattleReport::owner_for`]).

use ffh_data::*;

use super::formation::{Event, FighterIdx, Ground, GroundRule, Side};
use crate::game::Game;
use crate::ids::*;

/// How an engagement ended for the plot (spec 1.7; decided after the post-battle steps).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EngageResult {
    /// The plot fell: no defender is left on it (the launching group moved in, if it could).
    Taken,
    /// The defender's line is gone (every committed defender fell, escaped or sat out), but
    /// defenders are left on the plot: it holds.
    Cleared,
    /// The attacker's line is gone (every committed attacker fell, escaped or sat out).
    Repulsed,
    /// Both lines still stand after the last round.
    Held,
}

/// One engagement's report (`Game::battle_reports`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleReport {
    /// Never reused (`Game::battle_report_next`).
    pub id: u32,
    /// The game turn it was fought.
    pub turn: i32,
    /// The defended plot and the attacking plot.
    pub plot: PlotIdx,
    pub from: PlotIdx,
    /// The defended plot as it was (terrain, hills, the river crossing, the city).
    pub place: PlotSnap,
    /// `[attacker, defender]` (index by [`Side::index`]).
    pub sides: [ReportSide; 2],
    pub outcome: Outcome,
    /// The full record, for battles a human could see while it is kept (module comment).
    pub log: Option<Box<BattleLog>>,
}

/// A side of a report: who fought and what became of its units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportSide {
    /// The side's player (the attacker: the launcher's owner; the defender: the city's owner, else
    /// the owner of its first committed unit) and the player the other side saw (hidden
    /// nationality, G5).
    pub player: PlayerId,
    pub shown_player: PlayerId,
    /// Every real owner of the side's units in the battle (committed or reserve) and, for the
    /// defender, the city's owner: the players the report is for.
    pub players: Vec<PlayerId>,
    /// The most slots it could fill (its ordinary and river slots; the log's [`SideSlots`] say
    /// why), its lanes, and the slots its formation used (a large unit fills two).
    pub slots: i32,
    pub lanes: i32,
    pub used: i32,
    /// Units in its formation, then of them: sat out (the morale check), fallen, escaped
    /// (withdrew) and standing at the end: `committed = sat_out + fallen + escaped + standing`.
    pub committed: i32,
    pub sat_out: i32,
    pub fallen: i32,
    pub escaped: i32,
    pub standing: i32,
    /// Units killed, by unit type (types ascending).
    pub lost: Vec<(UnitType, i32)>,
    /// Of the committed units, those called into the battle (summons.rs: summons, and units a kill
    /// created that entered the field): beyond its slots, not in `used`.
    pub summons: i32,
    /// This side's ships that fired into the battle from the coast (docs/NAVAL.md, 4.7), each
    /// with the damage its shot dealt (centi-HP), in firing order.
    pub shore: Vec<(UnitType, i32)>,
}

/// The defended plot (DESIGN `PlotSnap`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotSnap {
    pub x: i32,
    pub y: i32,
    pub terrain: Option<TerrainType>,
    pub feature: Option<FeatureType>,
    pub hills: bool,
    /// The attack crossed a river.
    pub river_crossing: bool,
    pub city: Option<CitySnap>,
}

/// The defended city (DESIGN `CitySnap`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CitySnap {
    pub id: CityId,
    pub name: String,
    pub owner: PlayerId,
    pub size: i32,
    /// Its defence modifier at the battle (after bombardment): it cost the attacker slots.
    pub defense_pct: i32,
}

/// How an engagement ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub result: EngageResult,
    /// Rounds begun (0: a side had nobody on the field).
    pub rounds: i32,
    /// The attacker moved in (and took the city, if one).
    pub plot_taken: bool,
    pub city_captured: Option<CityId>,
    /// Units that moved in (they are listed in [`LogEvent::MoveIn`]).
    pub moved_in: i32,
    /// The defender's units still on the plot after the battle (its line's survivors that did
    /// not flee, the units that sat out and its reserve), its Sentinels standing counted.
    pub defenders_left: i32,
    /// The city's Sentinels that fought, and of them those breached (docs/SIEGES.md).
    pub sentinels: i32,
    pub sentinels_breached: i32,
}

/// The full record of an engagement (spec section 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleLog {
    /// `[attacker, defender]`.
    pub slots: [SideSlots; 2],
    pub lanes: [u8; 2],
    /// The defended plot's ground and the rules of it that apply.
    pub ground: Ground,
    pub rules: Vec<GroundRule>,
    /// The side's formation was its order's (the player's or the AI's own), not the rules' auto
    /// formation.
    pub ordered: [bool; 2],
    /// Each side's battle tactic (docs/TACTICS.md; `Game::engage_side_tactic`): what arranged its
    /// auto formation (an ordered side's is its body's, unused).
    pub tactics: [BattleTacticType; 2],
    /// Committed units in the engagement's order, then the reserves (module comment).
    pub fighters: Vec<LogFighter>,
    /// A and Dv of every (hitter, target) pair that hit.
    pub pairs: Vec<LogPair>,
    /// In resolution order: the order of the RNG draws.
    pub events: Vec<LogEvent>,
    /// What the attacker's plan showed when it attacked (a human attacker's).
    pub forecast: Option<FrozenForecast>,
}

/// A side's slots and why (DESIGN `SideSlots`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SideSlots {
    /// `ENGAGEMENT_SLOTS_BASE`.
    pub base: i32,
    /// In the order applied.
    pub changes: Vec<SlotChange>,
    /// Ordinary slots.
    pub total: i32,
    /// Slots only river-capable (`bRiver`) units fill (Q9).
    pub river_only: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotChange {
    pub src: SlotSource,
    pub delta: i32,
}

/// Why a side's slots changed (DESIGN `SlotSource`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotSource {
    /// The defended plot's `iEngagementSlotsChange`.
    Terrain(TerrainType),
    Feature(FeatureType),
    Hills,
    /// The attacker, across a river.
    River,
    /// Both sides: the defended city's Sentinels that fight, one slot each (docs/SIEGES.md).
    Sentinels(i32),
    /// Raised to `ENGAGEMENT_SLOTS_MIN`.
    Minimum,
    /// Phase 7: extra slots used by summons.
    Summons(i32),
    /// The attacker: the defended plot's improvement (`engagement_attacker_slots_change`, a
    /// card's: the Trench).
    Improvement(ImprovementType),
}

/// What became of a committed unit.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LogFate {
    /// On the field at the end.
    Standing,
    /// Killed by log fighter `by` in round `round`.
    Fell { by: FighterIdx, round: u8 },
    /// Left the battle alive in round `round` (its withdrawal took the place of a killing hit).
    Withdrew { round: u8 },
    /// Failed the morale check: it did not fight.
    SatOut,
}

/// A unit on either plot (spec section 2's fighter).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogFighter {
    /// The unit (ids are never reused; it may be gone); for a Sentinel, no unit
    /// ([`LogFighter::SENTINEL_UNIT`]).
    pub unit: UnitId,
    /// A Sentinel of the defended city (docs/SIEGES.md): `unit_type` is `UNIT_SENTINEL`, `owner`
    /// the city's.
    pub sentinel: bool,
    pub unit_type: UnitType,
    /// Its own name (heroes, renamed units; empty for most).
    pub name: String,
    /// Its real owner and the owner the other side saw.
    pub owner: PlayerId,
    pub shown_owner: PlayerId,
    pub side: Side,
    /// Its cell in its side's grid, `(lane, row)` (a large unit: its first lane); `None`: on the
    /// plot, not in the formation (a reserve).
    pub place: Option<(u8, u8)>,
    pub large: bool,
    pub level: i32,
    pub xp: i32,
    pub hero: bool,
    /// Max HP, and HP before the opening and when the rounds begin (centi-HP).
    pub max_hp: i32,
    pub hp_start: i32,
    pub hp_battle: i32,
    /// Its speed in the battle (the unit's, with first strikes and the chance first strikes it
    /// rolled), its first strikes, their chance and what it rolled.
    pub speed: i32,
    pub first_strikes: i32,
    pub chance_first_strikes: i32,
    pub rolled_first_strikes: i32,
    /// What it does from each row (front, middle, rear), with its extra actions.
    pub actions: [RowActions; ROWS],
    /// The damage spell it casts with SPELL actions (`None`: the magic bolt, or it casts none).
    pub spell: Option<SpellType>,
    /// Its accuracy, evasion and crit chance (docs/UNIT_STATS.md).
    pub accuracy: i32,
    pub evasion: i32,
    pub crit: i32,
    pub anti_flank: bool,
    pub flanker: bool,
    /// 0 for defenders in a city (Q3).
    pub withdraw_pct: i32,
    /// 100 = none (attackers only).
    pub combat_limit: i32,
    /// It splashes its hits, never past this damage (percent).
    pub collateral_limit: Option<i32>,
    /// Defensive strike: (chance, damage percent).
    pub def_strike: Option<(i32, i32)>,
    pub immune: Immunities,
    pub fear: bool,
    /// Against an ordinary opponent (the card's headline).
    pub a: Stat,
    pub dv: Stat,
    /// Its magic and magic defence (SPELL: docs/UNIT_STATS.md).
    pub mag: Stat,
    pub mdef: Stat,
    /// How it ended (`None` for a reserve), and the damage it dealt, took and healed in the
    /// battle (centi-HP).
    pub fate: Option<LogFate>,
    pub dealt: i32,
    pub taken: i32,
    pub healed: i32,
    /// A unit called into the battle (summons.rs): the fighter that called it (a summon's caster;
    /// the unit whose kill may create it), and whether a kill brings it (`on_kill`: out of the
    /// line until an event's `entered` does; one none brought never fought).
    pub summoner: Option<FighterIdx>,
    pub on_kill: bool,
}

impl LogFighter {
    /// The `unit` of a Sentinel's log fighter: no unit has it.
    pub const SENTINEL_UNIT: UnitId = UnitId(u32::MAX);
}

/// A bit set of what a fighter is immune to.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Immunities(pub u8);

impl Immunities {
    pub const DEF_STRIKE: u8 = 2;
    pub const FEAR: u8 = 4;

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }

    pub fn with(self, bit: u8, on: bool) -> Immunities {
        if on {
            Immunities(self.0 | bit)
        } else {
            Immunities(self.0 & !bit)
        }
    }
}

/// A strength and its parts (DESIGN `Stat`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stat {
    pub base_x100: i32,
    pub terms: Vec<Term>,
    pub total_x100: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub src: TermSource,
    pub pct: i32,
}

/// Every term `unit_combat_terms` can produce (the duel's and the engagement's; plan 4.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TermSource {
    /// The unit's own combat percentage, when not broken down by promotion.
    Own,
    Promotion(PromotionType),
    Spell(SpellType),
    BattleEffect(SpellType),
    Hills,
    Terrain(TerrainType),
    Feature(FeatureType),
    Fortify,
    CityDefense,
    UnitCityDefense,
    UnitHillsDefense,
    UnitTerrainDefense(TerrainType),
    UnitFeatureDefense(FeatureType),
    CityAttack,
    UnitTerrainAttack(TerrainType),
    UnitFeatureAttack(FeatureType),
    Amphibious,
    VsUnitCombat(UnitCombatType),
    VsUnitClass(UnitClassType),
    VsPromotion(PromotionType),
    VsAnimal,
    /// `CITY_BARBARIAN_DEFENSE_MODIFIER`.
    VsBarbarian,
    VsDomain(DomainType),
    InBorders,
    GlobalCounter,
    Handicap,
    /// Phase 7: an army commander's aura.
    Aura(UnitId),
    Typed(DamageType),
    /// The unit's hills attack bonus (`iHillsAttack`; no FfH unit or promotion has one).
    UnitHillsAttack,
    /// The defended plot's improvement (a fort's defense).
    Improvement(ImprovementType),
    /// FfH's range defense of friendly improvements near a plot without a city.
    RangeDefense,
    /// This game's: a warship's wear at sea (docs/NAVAL.md, 3.4).
    SeaWear,
}

/// A pair that hit (DESIGN `Pair`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogPair {
    pub h: FighterIdx,
    pub t: FighterIdx,
    /// A of `h` against `t`, Dv of `t` against `h` (x100).
    pub a_x100: i32,
    pub dv_x100: i32,
    /// Centi-HP: `ENGAGEMENT_BASE_DAMAGE x A / (A + Dv)`, before the hit's percentages.
    pub mean: i32,
    /// For `h`'s spells: its magic, `t`'s magic defence (x100), the percentage of the spell's
    /// damage `t` takes (its resistance to the spell's damage type) and the mean, centi-HP,
    /// before the spell's power and the hit's percentages.
    pub ma_x100: i32,
    pub md_x100: i32,
    pub spell_pct: i32,
    pub magic_mean: i32,
    /// The pair-only terms (against a unit combat, class, promotion).
    pub a_extra: Vec<Term>,
    pub dv_extra: Vec<Term>,
}

/// A stage of the engagement.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    PreBattle,
    Morale,
    Opening,
    /// The rounds (each action names its round and pass).
    Battle,
    Post,
}

/// An event of the log, in resolution order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogEvent {
    Stage { stage: Stage },
    /// Phase 7.
    Cast { caster: FighterIdx, spell: SpellType, targets: Vec<FighterIdx>, effect: CastEffect },
    /// Fear (Q8): the roll out of 40.
    Morale { unit: FighterIdx, source: FighterIdx, chance: i32, roll: i32, passed: bool },
    SatOut { unit: FighterIdx, why: SatOutWhy },
    /// A defensive strike (Q5) on an attacker before the battle. `rolled`: the strikers that
    /// rolled.
    DefStrike { striker: FighterIdx, target: FighterIdx, rolled: i32, chance: i32, roll: i32, pct: i32, dmg: i32 },
    /// One action of the battle as the resolver fought it (fighters by log index; a spell by its
    /// `SpellType` index): `{round, pass, actor, kind, spell, target, hits, heal, knocked, note}`.
    Action(Event),
    /// The battle's end: the rounds begun and which lines are gone (attacker, defender).
    End { rounds: i32, down: [bool; 2] },
    Heal { unit: FighterIdx, amount: i32, source: HealSource },
    Xp { unit: FighterIdx, amount: i32, from: Vec<XpSource> },
    KillEffect { killer: FighterIdx, victim: FighterIdx, kind: KillKind },
    /// A withdrawn defender outside a city.
    Flee { unit: FighterIdx, to: PlotIdx },
    MoveIn { units: Vec<FighterIdx>, city: Option<CityId> },
    /// The defended city's building struck before the battle (`pre_battle_damage`, a card's: the
    /// Thunderhead Tower): the damage each fighter took, centi-HP.
    BuildingStrike { building: BuildingType, hits: Vec<(FighterIdx, i32)> },
    /// Supply (docs/SUPPLY.md): the fighters of starving armies, before the battle, and the
    /// percent change of the damage they deal (`STARVATION_DAMAGE_DEALT_PERCENT`).
    Starvation { units: Vec<FighterIdx>, dealt_pct: i32 },
    /// A ship's shot from the coast before the battle (docs/NAVAL.md, 4.7): its type, owner and
    /// side, and the damage each fighter took, centi-HP.
    ShoreFire { ship: UnitType, owner: PlayerId, side: Side, hits: Vec<(FighterIdx, i32)> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CastEffect {
    Mod { a_pct: i32, dv_pct: i32 },
    Damage(Vec<(FighterIdx, i32)>),
    Summon(FighterIdx),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SatOutWhy {
    /// Failed its morale check.
    Afraid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HealSource {
    /// A medic of the unit's side (`b` when it is in the log) on its plot or, `adjacent`, next
    /// to it (Q12).
    Medic { b: Option<FighterIdx>, unit_type: UnitType, adjacent: bool },
    OnKill,
    Spell(SpellType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XpSource {
    /// A kill it helped: its share of the damage the victim took (per mille).
    Kill { victim: FighterIdx, share_permille: i32 },
    Withdrew,
    DefStrike,
}

/// A per-kill effect (G2), with the unit type it creates where it creates one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KillKind {
    Capture(UnitType),
    Enslave(UnitType),
    Convert(UnitType),
    Create(UnitType),
    Gold(i32),
    Afflict(PromotionType),
    Promotion(PromotionType),
}

/// The plan as the attacker saw it (frozen from the attack preview, `Game::engage_preview`, when
/// a human attacker attacks).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenForecast {
    /// The battles the preview played.
    pub runs: i32,
    /// The chance (per mille) the defender's line is wiped out, the plot falls, the attacker's
    /// line is wiped out.
    pub wiped_permille: i32,
    pub falls_permille: i32,
    pub repulsed_permille: i32,
    /// Expected units fallen per side (attacker, defender), x100.
    pub lost_x100: [i32; 2],
    /// Each committed fighter of the plan (both sides).
    pub per: Vec<ForecastFighter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForecastFighter {
    pub b: FighterIdx,
    /// The chance it falls and escapes (percent).
    pub fall_pct: i32,
    pub escape_pct: i32,
}

impl BattleReport {
    /// The side `p` fought on, if any.
    pub fn side_of(&self, p: PlayerId) -> Option<Side> {
        [Side::Attacker, Side::Defender].into_iter().find(|s| self.sides[s.index()].players.contains(&p))
    }

    /// Whether the report is `p`'s to see: `p` fought on a side.
    pub fn involves(&self, p: PlayerId) -> bool {
        self.side_of(p).is_some()
    }

    /// The owner to show `viewer` for a side's player or unit: its real owner on the viewer's
    /// own side, the shown owner on the other (or for a viewer of neither side).
    pub fn owner_for(&self, viewer: PlayerId, side: Side, owner: PlayerId, shown: PlayerId) -> PlayerId {
        if self.side_of(viewer) == Some(side) {
            owner
        } else {
            shown
        }
    }
}

impl Game {
    /// Records an engagement's report: gives it the next id and the current turn, and trims the
    /// reports and their logs ([`Game::battle_reports_trim`]). Returns the id.
    pub fn battle_report_add(&mut self, mut report: BattleReport) -> u32 {
        let id = self.battle_report_next;
        self.battle_report_next += 1;
        report.id = id;
        report.turn = self.game_turn();
        self.battle_reports.push(report);
        self.battle_reports_trim();
        id
    }

    /// Drops the reports that are neither of the current or the previous turn nor among the last
    /// `ENGAGEMENT_REPORTS_KEPT` (the oldest first), and the logs of reports that are neither of
    /// those two turns nor among the last `ENGAGEMENT_REPORT_LOGS_KEPT` logged ones. So the two
    /// turns' reports and logs stay whatever their number, and the caps bound the older ones.
    /// Runs after each new report and at each new game turn.
    pub fn battle_reports_trim(&mut self) {
        let recent = self.game_turn() - 1;
        let kept = self.d.defines.engagement_reports_kept.max(1) as usize;
        if self.battle_reports.len() > kept {
            // Reports are in turn order: the older-than-the-window ones are a prefix.
            let older = self.battle_reports.iter().take_while(|r| r.turn < recent).count();
            let extra = (self.battle_reports.len() - kept).min(older);
            self.battle_reports.drain(..extra);
        }
        let logs_kept = self.d.defines.engagement_report_logs_kept.max(0) as usize;
        let mut logged = 0;
        for r in self.battle_reports.iter_mut().rev() {
            if r.log.is_none() {
                continue;
            }
            logged += 1;
            if logged > logs_kept && r.turn < recent {
                r.log = None;
            }
        }
    }

    /// The report with id `id`, while it is kept.
    pub fn battle_report(&self, id: u32) -> Option<&BattleReport> {
        // Ids grow with the list's order: a binary search.
        self.battle_reports.binary_search_by_key(&id, |r| r.id).ok().map(|i| &self.battle_reports[i])
    }

    /// The kept reports `p` fought in, oldest first.
    pub fn battle_reports_of(&self, p: PlayerId) -> impl Iterator<Item = &BattleReport> {
        self.battle_reports.iter().filter(move |r| r.involves(p))
    }
}
