//! Battle reports and the battle log (plan 9.1, revised by 15.1 and 15.5; the battle UI design's
//! log, `battle_ui/final/DESIGN.md` section 10): saved state, written by the engagement's
//! post-battle step and read by the views, never by the rules.
//!
//! **Two layers.** A [`BattleReport`] is kept for each engagement of the current and the previous
//! game turn and for the last `ENGAGEMENT_REPORTS_KEPT` engagements: where, who fought (a
//! [`ReportSide`] each, with its real and shown owner), how many each side committed and lost,
//! and the [`Outcome`]: what the turn report's "Battles" list and the toasts show. Its
//! [`BattleLog`] is the full record the client replays (fighters with their A and Dv parts, the
//! pair table, every event with its rolls, the frozen forecast). The engagement writes a log only
//! for a battle a human could see; the log stays for every report of the current and the previous
//! turn and for the last `ENGAGEMENT_REPORT_LOGS_KEPT` logged reports, and older reports keep
//! their summary ([`Game::battle_reports_trim`]). Both caps give way to the two-turn window: a
//! turn's battles, however many (other players' wars included), never push out the reports or
//! logs a player's turn report is about to show.
//!
//! **DESIGN.md's `BattleLog` is the report and its log together**: `id`, `turn`, `plot`, `from`,
//! `outcome` and each side's players, army, garrison and stance are the report's (they outlive the
//! log); `slots`, the sides' picks ([`SidePick`]), `fighters`, `pairs`, `events` and `forecast`
//! are [`BattleLog`]'s. The view joins them. Other differences from DESIGN.md, all of form:
//! - numbers are `i32` (the engine's; the save's varints keep small ones small), fighter
//!   references are [`FighterIdx`] (`u16`, room for any stack) and a fighter's `b` is its index in
//!   [`BattleLog::fighters`] (not stored); sides are [`Side`]; the schema version is the view's;
//! - the round model's names are taken, so the log's are [`LogFighter`], [`LogPair`],
//!   [`LogEvent`] and [`LogDeath`], and DESIGN's `Phase` event enum is [`Stage`];
//! - `stance_known` is the viewer's (the view decides it: own team only in phase 6);
//! - the round model's [`Fate`] is each fighter's end ([`LogFighter::fate`]), [`LogEvent::End`]
//!   carries the volley rounds, [`Outcome::moved_in`] is a count (the units are in
//!   [`LogEvent::MoveIn`]), a capture or enslavement names the unit type it creates, a medic's
//!   heal names the medic's unit type (it need not be a fighter: adjacent medics heal too), the
//!   forecast's per-fighter tuple is [`ForecastFighter`], and [`TermSource`] adds `Own`,
//!   `VsBarbarian`, `VsDomain`, `UnitHillsAttack`, `Improvement` (a fort's defense) and
//!   `RangeDefense` (terms `unit_combat_terms` can produce).
//!
//! **Fighter order.** [`BattleLog::fighters`] holds the committed fighters first, in the round
//! model's order ([`super::Battle::fighters`]), then the reserves (eligible units on both plots that
//! were not committed, `slot: None`). So a committed fighter's index is its round-model
//! [`FighterIdx`], and the round model's references ([`Fate::Killed`]'s `by`, [`super::Death`],
//! [`super::Hit`]) read as log indices unchanged.
//!
//! **Hidden nationality (G5).** Every side and fighter keeps its real owner and the owner the
//! other side saw (`unit_get_visual_owner` at battle time). A viewer gets the real owners of its
//! own side and the shown ones of the other ([`BattleReport::owner_for`]), so a hidden-nationality
//! attacker stays the barbarian it looked like.

use ffh_data::*;

use super::battle::{FighterIdx, Side};
use super::resolve::{EngageResult, Fate, Phase};
use crate::game::Game;
use crate::ids::*;

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

/// A side of a report: who fought, with what stance, and what it lost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportSide {
    /// The side's player (the attacker: the launcher's owner; the defender: the city's owner, else
    /// the owner of its first committed unit) and the player the other side saw (hidden
    /// nationality, G5).
    pub player: PlayerId,
    pub shown_player: PlayerId,
    /// Every real owner of the side's units in the battle (committed or reserve) and, for the
    /// defender, the city's owner: the players the report is for. Real owners: a view shows the
    /// other side only `shown_player` (and the fighters' `shown_owner`).
    pub players: Vec<PlayerId>,
    /// The army whose stance the side fought with, if any, and the city whose garrison it was.
    pub army: Option<ArmyId>,
    pub garrison_of: Option<CityId>,
    /// The stance it fought with (plan 5.3).
    pub stance: StanceType,
    /// Units committed (sat-out ones included), withdrawn, sat out, fled after the battle.
    pub committed: i32,
    pub withdrew: i32,
    pub sat_out: i32,
    pub fled: i32,
    /// Units killed, by unit type (types ascending).
    pub lost: Vec<(UnitType, i32)>,
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
    /// Its defence modifier at the battle (after bombardment).
    pub defense_pct: i32,
}

/// How an engagement ended (DESIGN `Outcome`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub result: EngageResult,
    pub volley_rounds: i32,
    pub rounds: i32,
    /// The attacker moved in (and took the city, if one).
    pub plot_taken: bool,
    pub city_captured: Option<CityId>,
    /// Units that moved in (they are listed in [`LogEvent::MoveIn`]).
    pub moved_in: i32,
    /// Uncommitted defenders still on the plot.
    pub defenders_left: i32,
}

/// The full record of an engagement (DESIGN `BattleLog`, less what the report holds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleLog {
    /// `[attacker, defender]`.
    pub slots: [SideSlots; 2],
    pub picks: [SidePick; 2],
    /// Committed fighters in the round model's order, then the reserves (module comment).
    pub fighters: Vec<LogFighter>,
    /// A and Dv of every (hitter, target) pair that hit.
    pub pairs: Vec<LogPair>,
    /// In resolution order: the order of the RNG draws (DESIGN 10.1).
    pub events: Vec<LogEvent>,
    /// What the attacker's plan showed when it attacked.
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
    /// The attacker: one per `ENGAGEMENT_SLOTS_DEFENSE_DIVISOR` percent; `next_at` is the
    /// percent at which one more is lost.
    CityDefense { pct: i32, next_at: i32 },
    /// Defence knocked down this turn (it returns slots).
    Bombarded { pct: i32 },
    /// Raised to `ENGAGEMENT_SLOTS_MIN`.
    Minimum,
    /// Phase 7: extra slots used by summons.
    Summons(i32),
}

/// How a side's force was picked (DESIGN `SideSnap`'s pick fields).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidePick {
    /// The stance's own pick.
    pub auto_pick: Vec<FighterIdx>,
    pub picked_by: Pick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Auto,
    /// The player swapped units: `(out, in)` pairs.
    Player { swaps: Vec<(FighterIdx, FighterIdx)> },
}

/// A unit on either plot (DESIGN `Fighter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogFighter {
    /// The unit (ids are never reused; it may be gone).
    pub unit: UnitId,
    pub unit_type: UnitType,
    /// Its own name (heroes, renamed units; empty for most).
    pub name: String,
    /// Its real owner and the owner the other side saw.
    pub owner: PlayerId,
    pub shown_owner: PlayerId,
    pub side: Side,
    /// Its slot; `None`: on the plot, not committed (a reserve).
    pub slot: Option<i32>,
    pub level: i32,
    pub xp: i32,
    pub hero: bool,
    pub role: CombatRoleType,
    pub pos: Position,
    pub screen: bool,
    pub ignores_shield: bool,
    pub assassin_profile: bool,
    pub guardsman: bool,
    /// Max HP, and HP at the start (centi-HP, before the opening).
    pub max_hp: i32,
    pub hp_start: i32,
    /// First strikes and their chance (the roll is a [`LogEvent::VolleyRoll`]).
    pub volleys: i32,
    pub volley_chance: i32,
    /// 0 for defenders in a city (Q3).
    pub withdraw_pct: i32,
    /// Defensive strike: (chance, damage percent).
    pub def_strike: Option<(i32, i32)>,
    /// Collateral: (damage percent, limit percent, max units).
    pub collateral: Option<(i32, i32, i32)>,
    /// 100 = none.
    pub combat_limit: i32,
    pub immune: Immunities,
    pub fear: bool,
    /// Against an ordinary opponent (the card's headline).
    pub a: Stat,
    pub dv: Stat,
    /// How it ended (the round model's; `None` for a reserve).
    pub fate: Option<Fate>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Position {
    Front,
    Back,
}

/// A bit set of what a fighter is immune to.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Immunities(pub u8);

impl Immunities {
    pub const FIRST_STRIKES: u8 = 1;
    pub const DEF_STRIKE: u8 = 2;
    pub const FEAR: u8 = 4;
    pub const COLLATERAL: u8 = 8;

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
    pub role_pct: i32,
    pub terms: Vec<Term>,
    pub total_x100: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub src: TermSource,
    pub pct: i32,
}

/// Every term `unit_combat_terms` can produce (plan 4.2; DESIGN `TermSource`).
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
}

/// A pair that hit (DESIGN `Pair`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogPair {
    pub h: FighterIdx,
    pub t: FighterIdx,
    /// A of `h` against `t`, Dv of `t` against `h` (x100).
    pub a_x100: i32,
    pub dv_x100: i32,
    /// Centi-HP: `B x A / (A + Dv)`.
    pub mean: i32,
    /// The pair-only terms (against a unit combat, class, promotion).
    pub a_extra: Vec<Term>,
    pub dv_extra: Vec<Term>,
}

/// A stage of the engagement (DESIGN's `Phase` event enum).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    PreBattle,
    Morale,
    Opening,
    Volley(i32),
    Round(i32),
    Post,
}

impl From<Phase> for Stage {
    fn from(p: Phase) -> Stage {
        match p {
            Phase::Opening => Stage::Opening,
            Phase::Volley(k) => Stage::Volley(k),
            Phase::Round(n) => Stage::Round(n),
        }
    }
}

/// An event of the log (DESIGN `Event`), in resolution order. HP changes only at `DefStrike`,
/// `Collateral`, `Cast`, `RoundEnd` and `Heal`; liveness only at `RoundEnd`, `Withdraw` and
/// `SatOut` (DESIGN 10.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogEvent {
    Stage { stage: Stage },
    /// Phase 7.
    Cast { caster: FighterIdx, spell: SpellType, targets: Vec<FighterIdx>, effect: CastEffect },
    /// Fear (Q8): the roll out of 40.
    Morale { unit: FighterIdx, source: FighterIdx, chance: i32, roll: i32, passed: bool },
    SatOut { unit: FighterIdx, why: SatOutWhy },
    /// Only when the volley chance is above 0.
    VolleyRoll { unit: FighterIdx, chance: i32, roll: i32, total: i32 },
    /// `rolled`: the strikers that rolled.
    DefStrike { striker: FighterIdx, target: FighterIdx, rolled: i32, chance: i32, roll: i32, pct: i32, dmg: i32 },
    Collateral { source: FighterIdx, target: FighterIdx, pct: i32, limit_pct: i32, protection_pct: i32, dmg: i32 },
    /// `step`: the index in the hitter's target order; `blocked`: the earlier steps and why they
    /// were passed; `eligible`: the set the target was drawn from; `r` in 80..=120; `capped` by
    /// the hitter's combat limit.
    Hit { h: FighterIdx, t: FighterIdx, rule: TargetRule, step: i32, blocked: Vec<(i32, Block)>, eligible: Vec<FighterIdx>, r: i32, dmg: i32, capped: bool },
    /// The round's hits apply here, all at once.
    RoundEnd { deaths: Vec<LogDeath> },
    Withdraw { unit: FighterIdx, hp_pct: i32, chance: i32, roll: i32, ok: bool, fatal: bool, hp_kept: Option<i32> },
    /// The side's last front-position fighter fell.
    ShieldDown { side: Side },
    End { result: EngageResult, volley_rounds: i32, rounds: i32 },
    Heal { unit: FighterIdx, amount: i32, source: HealSource },
    Xp { unit: FighterIdx, amount: i32, from: Vec<XpSource> },
    KillEffect { killer: FighterIdx, victim: FighterIdx, kind: KillKind },
    /// A withdrawn defender outside a city.
    Flee { unit: FighterIdx, to: PlotIdx },
    /// Q8, after a won attack outside a city.
    Rout { units: Vec<FighterIdx>, to: Vec<PlotIdx> },
    MoveIn { units: Vec<FighterIdx>, city: Option<CityId> },
}

/// A death at a round's end (DESIGN `Death`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogDeath {
    pub b: FighterIdx,
    pub killer: Option<FighterIdx>,
    pub assists: Vec<FighterIdx>,
    /// Damage beyond 0, centi-HP.
    pub overkill: i32,
}

/// The step of the hitter's order that chose the target (DESIGN `TargetRule`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TargetRule {
    FrontFirst,
    FrontRankDown,
    FlankerScreensFirst,
    FlankerAround,
    FlankerFrontLast,
    AssassinSupportFirst,
    AssassinBack,
    AssassinFrontLast,
    /// Phase 7: a Guardsman draws assassin-profile hitters (Q6).
    BodyguardFirst,
}

/// Why a step of the order was passed: the round model's.
pub use super::resolve::Block;

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
    /// An attacker with a combat limit whose every target is at it.
    EveryTargetCapped,
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

/// The plan as the attacker saw it (DESIGN `FrozenForecast`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenForecast {
    pub source: ForecastSource,
    pub win_permille: i32,
    pub draw_permille: i32,
    /// p10, p50, p90.
    pub rounds: (i32, i32, i32),
    pub losses_if_win_x10: i32,
    pub per: Vec<ForecastFighter>,
    /// Each side's mean HP after each step (opening, round 1, ...).
    pub curve: [Vec<i32>; 2],
    /// What the What-if list showed: the plan and its win chance (per mille).
    pub alternatives: Vec<(AltKind, i32)>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForecastSource {
    /// The player's preview: runs of the real resolver on a private RNG.
    Sampled { runs: i32 },
    /// The AI's moment integrator.
    Expected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForecastFighter {
    pub b: FighterIdx,
    pub fall_pct: i32,
    pub withdraw_pct: i32,
    pub death_round: Option<i32>,
}

/// A What-if plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AltKind {
    ThisPlan,
    StancePick,
    Swap { out: FighterIdx, add: FighterIdx },
    Stance(StanceType),
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
