//! Telemetry: what the players did, with what, and why -- for analysing AI play
//! (docs/TELEMETRY.md).
//!
//! Off unless the runner turns it on (`g.telemetry = Telemetry::new(..)`; `ffh stats` does for
//! `FFH_TELEMETRY=1` or `FFH_TELEMETRY_FILE`). The rules record where players act -- missions,
//! casts, hurries, upgrades, promotions, deals, wars and war plans, cities founded, taken and
//! razed, combats, improvements, units/buildings/projects built, techs, civics, religions, unit
//! deaths by cause -- and a scan at the start of each player's turn records which spells each
//! unit knows and can cast. The AIs add why: [`Game::tel_intent`] (plans that hold until they
//! change), [`Game::tel_decision`] (a choice with its value and the alternatives),
//! [`Game::tel_cast_reason`] and [`Game::tel_war_reason`].
//!
//! Nothing reads it back during play: recording draws no random number and changes no game
//! state, so a game plays the same with it on or off (test `telemetry_does_not_change_play`).
//! It is not saved (a loaded game starts with it off).
//!
//! Outputs: aggregates per player ([`Game::telemetry_json`], which `ffh stats` embeds), and
//! optionally one JSON object per line for each event (casts, combats, wars, war plans, deals,
//! cities, hurries, upgrades, techs, civics, intents, decisions; missions, promotions,
//! improvements and builds are only counted).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::structs::TradeData;

/// The telemetry sink of a game (`Game::telemetry`); [`Telemetry::off`] by default.
#[derive(Default)]
pub struct Telemetry {
    inner: Option<Box<RefCell<Tel>>>,
}

impl Telemetry {
    pub fn off() -> Self {
        Telemetry { inner: None }
    }

    /// On. `events`, when given, receives one JSON object per line for each event.
    pub fn new(events: Option<Box<dyn Write + Send>>) -> Self {
        Telemetry { inner: Some(Box::new(RefCell::new(Tel { events, ..Tel::default() }))) }
    }

    #[inline]
    pub fn on(&self) -> bool {
        self.inner.is_some()
    }

    #[inline]
    fn with<R>(&self, f: impl FnOnce(&mut Tel) -> R) -> Option<R> {
        self.inner.as_ref().map(|c| f(&mut c.borrow_mut()))
    }

    /// Flushes the event writer.
    pub fn flush(&self) {
        self.with(|t| {
            if let Some(w) = t.events.as_mut() {
                let _ = w.flush();
            }
        });
    }
}

/// Per player and spell.
#[derive(Default)]
struct SpellStat {
    /// Unit-turns a unit knew the spell (met its unit-side prerequisites: promotions, unit type,
    /// class, combat type, civilization, religion, level) at the start of its owner's turn. A
    /// spell whose only prerequisite is the civilization (most world spells) counts once a turn.
    known: i64,
    /// Unit-turns a unit could cast it then (`canCast`; once a turn for civilization spells).
    castable: i64,
    /// Unit-turns a unit could still cast it when its owner's turn ended, having cast nothing
    /// that turn (opportunities passed up; once a turn for civilization spells).
    idle: i64,
    /// Turns at least one unit knew it / could cast it.
    turns_known: i64,
    turns_castable: i64,
    first_known: Option<i32>,
    first_castable: Option<i32>,
    casts: i64,
    first_cast: Option<i32>,
    last_cast: Option<i32>,
    /// Casts per hundred turns.
    by_century: [i64; 8],
    /// Casts by the reason the controller gave (`untagged` when it gave none).
    reasons: BTreeMap<String, i64>,
    /// Sum of the values the controller gave.
    value: i64,
    at_war: i64,
    in_city: i64,
    /// Enemy units (of teams at war with the caster's) within reach, summed over casts.
    enemies: i64,
    /// Effects on others' units within reach (the whole map for world spells): units gone, hit
    /// points taken, enemies that gained promotions (a curse's effect -- or a blessing that
    /// reached them too), units that changed to our side, non-enemies that gained promotions.
    kills: i64,
    damage: i64,
    cursed: i64,
    converted: i64,
    others_buffed: i64,
    summons: i64,
    /// Combats won by the units the spell summoned.
    summon_kills: i64,
    buffs: i64,
    healed: i64,
    own_lost: i64,
}

/// A war between two teams, from the declaration to the peace.
struct War {
    start: i32,
    end: Option<i32>,
    /// The declaring team first.
    teams: [TeamId; 2],
    /// The declarer's war plan when it declared.
    plan: &'static str,
    reason: Option<String>,
    /// Cities and their population each team took from the other, and combats each team won
    /// against the other.
    cities: [i64; 2],
    pop: [i64; 2],
    kills: [i64; 2],
}

#[derive(Default)]
struct Tel {
    events: Option<Box<dyn Write + Send>>,
    /// (player, category, key) -> [count, sum].
    counts: BTreeMap<(PlayerId, &'static str, &'static str), [i64; 2]>,
    spells: BTreeMap<(PlayerId, SpellType), SpellStat>,
    wars: Vec<War>,
    /// (lower team, higher team) -> index in `wars`.
    open_wars: BTreeMap<(TeamId, TeamId), usize>,
    war_reasons: BTreeMap<(TeamId, TeamId), String>,
    /// Units summoned by a spell, for their later kills.
    summons: BTreeMap<UnitId, SpellType>,
    /// (player, kind) -> [(turn, value)] (plans as they change).
    timelines: BTreeMap<(PlayerId, &'static str), Vec<(i32, String)>>,
    /// (player, kind, choice) -> [count, value sum].
    decisions: BTreeMap<(PlayerId, &'static str, String), [i64; 2]>,
    /// (player, kind, why) -> count.
    whys: BTreeMap<(PlayerId, &'static str, &'static str), i64>,
    /// What the rules are doing (the cause given to unit deaths meanwhile).
    ctx: Vec<&'static str>,
    kill_cause: BTreeMap<UnitId, &'static str>,
    cast_reason: Option<(UnitId, String, i64)>,
    cast_depth: u32,
    /// Spells with unit-side prerequisites a unit type may meet (by unit type), built on the
    /// first scan; true for those whose only prerequisite is the civilization.
    cand: Vec<Vec<(SpellType, bool)>>,
}

impl Tel {
    fn event(&mut self, line: impl FnOnce() -> String) {
        if let Some(w) = self.events.as_mut() {
            let _ = writeln!(w, "{}", line());
        }
    }

    fn count(&mut self, p: PlayerId, cat: &'static str, key: &'static str, sum: i64) {
        let e = self.counts.entry((p, cat, key)).or_insert([0, 0]);
        e[0] += 1;
        e[1] += sum;
    }

    fn war(&mut self, a: TeamId, b: TeamId) -> Option<(&mut War, usize)> {
        let i = *self.open_wars.get(&(a.min(b), a.max(b)))?;
        let w = &mut self.wars[i];
        let side = if w.teams[0] == a { 0 } else { 1 };
        Some((w, side))
    }
}

// ----------------------------------------------------------------------------
// JSON
// ----------------------------------------------------------------------------

/// A JSON object written field by field.
pub struct Obj(String);

impl Obj {
    pub fn new() -> Obj {
        Obj(String::from("{"))
    }

    fn key(&mut self, k: &str) {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        push_json_str(&mut self.0, k);
        self.0.push(':');
    }

    pub fn s(mut self, k: &str, v: &str) -> Obj {
        self.key(k);
        push_json_str(&mut self.0, v);
        self
    }

    pub fn i(mut self, k: &str, v: i64) -> Obj {
        self.key(k);
        self.0.push_str(&v.to_string());
        self
    }

    pub fn b(mut self, k: &str, v: bool) -> Obj {
        self.key(k);
        self.0.push_str(if v { "true" } else { "false" });
        self
    }

    pub fn opt(self, k: &str, v: Option<i64>) -> Obj {
        match v {
            Some(v) => self.i(k, v),
            None => self.raw(k, "null"),
        }
    }

    /// A value that is already JSON.
    pub fn raw(mut self, k: &str, json: &str) -> Obj {
        self.key(k);
        self.0.push_str(json);
        self
    }

    pub fn end(mut self) -> String {
        self.0.push('}');
        self.0
    }
}

impl Default for Obj {
    fn default() -> Self {
        Obj::new()
    }
}

pub fn push_json_str(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn json_str(s: &str) -> String {
    let mut out = String::new();
    push_json_str(&mut out, s);
    out
}

/// Alternatives for [`Game::tel_decision`]: `[["name", value], ..]`.
pub fn tel_alts<'a>(items: impl IntoIterator<Item = (&'a str, i64)>) -> String {
    let v: Vec<String> = items.into_iter().map(|(n, v)| format!("[{},{}]", json_str(n), v)).collect();
    format!("[{}]", v.join(","))
}

fn ev(turn: i32, kind: &str, p: PlayerId) -> Obj {
    Obj::new().i("t", turn as i64).s("e", kind).i("p", p.0 as i64)
}

fn enum_name(names: &'static [&'static str], i: usize) -> &'static str {
    names.get(i).copied().unwrap_or("?")
}

const ODDS_BUCKETS: [&str; 10] = ["00", "10", "20", "30", "40", "50", "60", "70", "80", "90"];

// ----------------------------------------------------------------------------
// Recording (rules side)
// ----------------------------------------------------------------------------

/// What [`Game::tel_cast_begin`] saw before a cast.
pub(crate) struct CastSnap {
    owner: PlayerId,
    spell: SpellType,
    /// Units within reach: id, owner, damage, promotions held, enemy of the caster's team.
    units: Vec<(UnitId, PlayerId, i32, u32, bool)>,
    next_unit: UnitId,
    nested: bool,
}

impl Game {
    fn tel_name_unit(&self, t: UnitType) -> &'static str {
        self.d.units[t].base.type_.as_str()
    }

    /// Rules context for the cause of unit deaths (`combat`, `spell`, `scrap`, ...).
    pub(crate) fn tel_ctx_push(&self, c: &'static str) {
        self.telemetry.with(|t| t.ctx.push(c));
    }

    pub(crate) fn tel_ctx_pop(&self) {
        self.telemetry.with(|t| t.ctx.pop());
    }

    pub(crate) fn tel_mission(&self, g: GroupId, mission: MissionType, mission_ai: Option<MissionAIType>) {
        let p = self.groups[g].owner;
        let name = self.d.missions[mission].base.type_.as_str();
        self.telemetry.with(|t| {
            t.count(p, "mission", name, 0);
            if let Some(m) = mission_ai {
                t.count(p, "mission_ai", enum_name(MissionAIType::NAMES, m.idx()), 0);
            }
        });
    }

    fn tel_production_name(&self, c: CityId) -> &'static str {
        let d = self.d;
        if let Some(u) = self.city_production_unit(c) {
            d.units[u].base.type_.as_str()
        } else if let Some(b) = self.city_production_building(c) {
            d.buildings[b].base.type_.as_str()
        } else if let Some(pr) = self.city_production_project(c) {
            d.projects[pr].base.type_.as_str()
        } else if let Some(pr) = self.city_production_process(c) {
            d.processes[pr].base.type_.as_str()
        } else {
            "NONE"
        }
    }

    pub(crate) fn tel_hurry(&self, c: CityId, hurry: HurryType, gold: i32, pop: i32) {
        let p = self.cities[c].owner;
        let name = self.d.hurries[hurry].base.type_.as_str();
        let what = self.tel_production_name(c);
        let turn = self.game_turn();
        let city_pop = self.cities[c].population;
        self.telemetry.with(|t| {
            t.count(p, "hurry", name, gold as i64);
            if pop != 0 {
                t.count(p, "hurry_pop", name, pop as i64);
            }
            t.count(p, "hurried", what, 0);
            t.event(|| {
                ev(turn, "hurry", p).s("how", name).s("what", what).i("gold", gold as i64).i("pop", pop as i64).i("city_pop", city_pop as i64).end()
            });
        });
    }

    pub(crate) fn tel_upgrade(&self, u: UnitId, to: UnitType, price: i32) {
        let p = self.units[u].owner;
        let from = self.tel_name_unit(self.units[u].unit_type);
        let to = self.tel_name_unit(to);
        let turn = self.game_turn();
        let level = self.units[u].level;
        self.telemetry.with(|t| {
            t.count(p, "upgrade_to", to, price as i64);
            t.count(p, "upgrade_from", from, 0);
            t.event(|| ev(turn, "upgrade", p).s("from", from).s("to", to).i("gold", price as i64).i("level", level as i64).end());
        });
    }

    pub(crate) fn tel_promote(&self, u: UnitId, promotion: PromotionType) {
        let p = self.units[u].owner;
        let name = self.d.promotions[promotion].base.type_.as_str();
        self.telemetry.with(|t| t.count(p, "promote", name, 0));
    }

    pub(crate) fn tel_deal(&self, who: PlayerId, other: PlayerId, ours: &[TradeData], theirs: &[TradeData]) {
        let turn = self.game_turn();
        let names = |l: &[TradeData]| -> Vec<(&'static str, i32)> {
            l.iter().map(|it| (enum_name(TradeableItem::NAMES, it.item_type.idx()), it.data)).collect()
        };
        let (a, b) = (names(ours), names(theirs));
        self.telemetry.with(|t| {
            for &(n, data) in &a {
                let gold = if n == "TRADE_GOLD" || n == "TRADE_GOLD_PER_TURN" { data as i64 } else { 0 };
                t.count(who, "deal_gave", n, gold);
                t.count(other, "deal_got", n, gold);
            }
            for &(n, data) in &b {
                let gold = if n == "TRADE_GOLD" || n == "TRADE_GOLD_PER_TURN" { data as i64 } else { 0 };
                t.count(other, "deal_gave", n, gold);
                t.count(who, "deal_got", n, gold);
            }
            t.count(who, "deal", "n", 0);
            t.count(other, "deal", "n", 0);
            let list = |l: &[(&'static str, i32)]| tel_alts(l.iter().map(|&(n, d)| (n, d as i64)));
            t.event(|| ev(turn, "deal", who).i("with", other.0 as i64).raw("gave", &list(&a)).raw("got", &list(&b)).end());
        });
    }

    fn tel_team_players(&self, t: TeamId) -> String {
        let v: Vec<String> = self.player_ids().filter(|&p| self.players[p].team == t).map(|p| p.0.to_string()).collect();
        format!("[{}]", v.join(","))
    }

    pub(crate) fn tel_declare_war(&self, t: TeamId, other: TeamId, plan: Option<WarPlanType>) {
        let turn = self.game_turn();
        let plan = plan.or(self.teams[t].ai_war_plan[other]);
        let plan = plan.map_or("NONE", |w| enum_name(WarPlanType::NAMES, w.idx()));
        let leader = self.team_get_leader_id(t);
        let players = (self.tel_team_players(t), self.tel_team_players(other));
        self.telemetry.with(|tl| {
            let key = (t.min(other), t.max(other));
            if tl.open_wars.contains_key(&key) {
                return;
            }
            let reason = tl.war_reasons.remove(&(t, other));
            tl.open_wars.insert(key, tl.wars.len());
            if let Some(p) = leader {
                tl.count(p, "war_declared", plan, 0);
            }
            tl.event(|| {
                let o = ev(turn, "war", leader.unwrap_or(PlayerId(0)))
                    .i("team", t.0 as i64)
                    .i("vs", other.0 as i64)
                    .raw("players", &players.0)
                    .raw("vs_players", &players.1)
                    .s("plan", plan);
                match &reason {
                    Some(r) => o.s("reason", r),
                    None => o,
                }
                .end()
            });
            tl.wars.push(War { start: turn, end: None, teams: [t, other], plan, reason, cities: [0; 2], pop: [0; 2], kills: [0; 2] });
        });
    }

    pub(crate) fn tel_make_peace(&self, t: TeamId, other: TeamId) {
        let turn = self.game_turn();
        let leader = self.team_get_leader_id(t);
        self.telemetry.with(|tl| {
            let Some(i) = tl.open_wars.remove(&(t.min(other), t.max(other))) else {
                return;
            };
            let w = &mut tl.wars[i];
            w.end = Some(turn);
            let (start, teams, cities, kills) = (w.start, w.teams, w.cities, w.kills);
            tl.event(|| {
                ev(turn, "peace", leader.unwrap_or(PlayerId(0)))
                    .i("team", t.0 as i64)
                    .i("vs", other.0 as i64)
                    .i("started", start as i64)
                    .i("declarer", teams[0].0 as i64)
                    .raw("cities", &format!("[{},{}]", cities[0], cities[1]))
                    .raw("kills", &format!("[{},{}]", kills[0], kills[1]))
                    .end()
            });
        });
    }

    pub(crate) fn tel_war_plan(&self, t: TeamId, other: TeamId, plan: Option<WarPlanType>) {
        let turn = self.game_turn();
        let name = plan.map_or("NONE", |w| enum_name(WarPlanType::NAMES, w.idx()));
        let Some(leader) = self.team_get_leader_id(t) else {
            return;
        };
        self.telemetry.with(|tl| {
            tl.count(leader, "war_plan", name, 0);
            tl.event(|| ev(turn, "war_plan", leader).i("team", t.0 as i64).i("vs", other.0 as i64).s("plan", name).end());
        });
    }

    /// Team `t` becomes (`value`) or stops being `master`'s vassal.
    pub(crate) fn tel_vassal(&self, t: TeamId, master: TeamId, value: bool, capitulated: bool) {
        let turn = self.game_turn();
        let how = match (value, capitulated) {
            (true, true) => "capitulated",
            (true, false) => "became",
            (false, _) => "freed",
        };
        let members: Vec<PlayerId> = self.player_ids().filter(|&p| self.players[p].team == t && self.players[p].alive).collect();
        let leader = self.team_get_leader_id(master);
        let (cities, power) = (self.teams[t].num_cities, self.team_get_power(t, false));
        let master_power = self.team_get_power(master, false);
        for &p in &members {
            self.tel_intent(p, "vassal_of", || if value { format!("team {}", master.0) } else { "free".to_string() });
        }
        self.telemetry.with(|tl| {
            for &p in &members {
                tl.count(p, "vassal", how, 0);
                tl.event(|| {
                    ev(turn, "vassal", p)
                        .s("how", how)
                        .i("master", master.0 as i64)
                        .i("cities", cities as i64)
                        .i("power", power as i64)
                        .i("master_power", master_power as i64)
                        .end()
                });
            }
            if let Some(l) = leader {
                tl.count(l, "vassal_master", if value { "gained" } else { "lost" }, 0);
            }
        });
    }

    pub(crate) fn tel_found(&self, p: PlayerId, c: CityId) {
        let turn = self.game_turn();
        let (x, y) = self.map.xy(self.city_plot(c));
        let n = self.players[p].cities.len();
        self.telemetry.with(|t| {
            t.count(p, "city", "founded", 0);
            t.event(|| ev(turn, "found", p).i("x", x as i64).i("y", y as i64).i("cities", n as i64).end());
        });
    }

    pub(crate) fn tel_acquire(&self, p: PlayerId, c: CityId, conquest: bool, trade: bool) {
        let turn = self.game_turn();
        let old = self.cities[c].owner;
        let pop = self.cities[c].population;
        let (x, y) = self.map.xy(self.city_plot(c));
        let how = if conquest {
            "conquest"
        } else if trade {
            "trade"
        } else {
            "other"
        };
        let (ta, tb) = (self.players[p].team, self.players[old].team);
        let barb = self.player_is_barbarian(old);
        self.telemetry.with(|t| {
            t.count(p, "city_taken", if barb && conquest { "from_barbarians" } else { how }, pop as i64);
            t.count(old, "city_lost", how, pop as i64);
            if conquest {
                if let Some((w, side)) = t.war(ta, tb) {
                    w.cities[side] += 1;
                    w.pop[side] += pop as i64;
                }
            }
            t.event(|| ev(turn, "acquire", p).i("from", old.0 as i64).s("how", how).i("pop", pop as i64).i("x", x as i64).i("y", y as i64).end());
        });
    }

    pub(crate) fn tel_raze(&self, p: PlayerId, c: CityId) {
        let turn = self.game_turn();
        let pop = self.cities[c].population;
        self.telemetry.with(|t| {
            t.count(p, "city", "razed", pop as i64);
            t.event(|| ev(turn, "raze", p).i("pop", pop as i64).end());
        });
    }

    /// The first `kill` of a unit: its cause is the rules context, else `killed` (by a player)
    /// or `other`.
    pub(crate) fn tel_kill_mark(&self, u: UnitId, killer: Option<PlayerId>) {
        self.telemetry.with(|t| {
            if !t.kill_cause.contains_key(&u) {
                let cause = t.ctx.last().copied().unwrap_or(if killer.is_some() { "killed" } else { "other" });
                t.kill_cause.insert(u, cause);
            }
        });
    }

    /// A unit leaves the game (the final `kill`).
    pub(crate) fn tel_unit_died(&self, u: UnitId) {
        let p = self.units[u].owner;
        let ty = self.tel_name_unit(self.units[u].unit_type);
        let combat = self.d.units[self.units[u].unit_type].combat > 0;
        self.telemetry.with(|t| {
            let cause = t.kill_cause.remove(&u).or_else(|| t.ctx.last().copied()).unwrap_or("other");
            t.count(p, "died", cause, 0);
            if combat && matches!(cause, "combat" | "spell" | "killed") {
                t.count(p, "lost_unit", ty, 0);
            }
            t.summons.remove(&u);
        });
    }

    /// An immortal unit was reborn instead of dying.
    pub(crate) fn tel_unit_reborn(&self, u: UnitId) {
        let p = self.units[u].owner;
        self.telemetry.with(|t| {
            t.kill_cause.remove(&u);
            t.count(p, "died", "reborn", 0);
        });
    }

    /// A combat, before its losers are removed: `odds` (per mille) of the attacker as the
    /// battle began; `undefended` when the defender could not defend (it just dies).
    pub(crate) fn tel_combat(&self, att: UnitId, def: UnitId, odds: i32, undefended: bool, plot: PlotIdx) {
        let d = self.d;
        let turn = self.game_turn();
        let (pa, pd) = (self.units[att].owner, self.units[def].owner);
        let (ta, td) = (self.players[pa].team, self.players[pd].team);
        let (att_dead, def_dead) = (self.unit_is_dead(att), self.unit_is_dead(def));
        let res = if att_dead {
            "loss"
        } else if def_dead {
            "win"
        } else {
            "withdraw"
        };
        let (ua, ud) = (self.tel_name_unit(self.units[att].unit_type), self.tel_name_unit(self.units[def].unit_type));
        let (la, ld) = (self.units[att].level, self.units[def].level);
        let city = self.plot_is_city(plot, false, None);
        let (x, y) = self.map.xy(plot);
        let barb = self.player_is_barbarian(pd);
        let animal = d.units[self.units[def].unit_type].animal;
        let max_hp = d.defines.max_hit_points;
        let (dmg_a, dmg_d) = (self.units[att].damage.min(max_hp), self.units[def].damage.min(max_hp));
        self.telemetry.with(|t| {
            let bucket = ODDS_BUCKETS[((odds.clamp(0, 999)) / 100) as usize];
            if undefended {
                t.count(pa, "attack", "undefended", 0);
            } else {
                t.count(pa, "attack", res, 0);
                t.count(pa, "attack_odds", bucket, (res == "win") as i64);
                t.count(pa, "attack_odds_exp", bucket, odds as i64);
                t.count(pd, "defend", if att_dead { "win" } else if def_dead { "loss" } else { "withdraw" }, 0);
                t.count(pd, "defend_in_city", if city { "yes" } else { "no" }, def_dead as i64);
                if barb {
                    t.count(pa, "attack_vs", if animal { "animal" } else { "barbarian" }, (res == "win") as i64);
                } else {
                    t.count(pa, "attack_vs", "civ", (res == "win") as i64);
                }
            }
            let winner = if att_dead {
                Some((def, td, ta, ud))
            } else if def_dead {
                Some((att, ta, td, ua))
            } else {
                None
            };
            if let Some((w, tw, tl, wname)) = winner {
                let wp = if w == att { pa } else { pd };
                t.count(wp, "kill_by", wname, 0);
                if let Some(&spell) = t.summons.get(&w) {
                    t.spells.entry((wp, spell)).or_default().summon_kills += 1;
                }
                if let Some((war, side)) = t.war(tw, tl) {
                    war.kills[side] += 1;
                }
                let loser = if w == att { def } else { att };
                t.kill_cause.insert(loser, "combat");
            }
            t.event(|| {
                ev(turn, "combat", pa)
                    .i("vs", pd.0 as i64)
                    .s("att", ua)
                    .s("def", ud)
                    .i("lvl", la as i64)
                    .i("def_lvl", ld as i64)
                    .i("odds", odds as i64)
                    .s("res", if undefended { "undefended" } else { res })
                    .i("dmg", dmg_a as i64)
                    .i("def_dmg", dmg_d as i64)
                    .b("city", city)
                    .i("x", x as i64)
                    .i("y", y as i64)
                    .end()
            });
        });
    }

    pub(crate) fn tel_improvement(&self, u: UnitId, build: BuildType) {
        let p = self.units[u].owner;
        let name = self.d.builds[build].base.type_.as_str();
        self.telemetry.with(|t| t.count(p, "build", name, 0));
    }

    pub(crate) fn tel_built_unit(&self, u: UnitId) {
        let p = self.units[u].owner;
        let name = self.tel_name_unit(self.units[u].unit_type);
        self.telemetry.with(|t| t.count(p, "built_unit", name, 0));
    }

    pub(crate) fn tel_built_building(&self, c: CityId, b: BuildingType) {
        let p = self.cities[c].owner;
        let name = self.d.buildings[b].base.type_.as_str();
        self.telemetry.with(|t| t.count(p, "built_building", name, 0));
    }

    pub(crate) fn tel_built_project(&self, c: CityId, pr: ProjectType) {
        let p = self.cities[c].owner;
        let name = self.d.projects[pr].base.type_.as_str();
        let turn = self.game_turn();
        self.telemetry.with(|t| {
            t.count(p, "built_project", name, 0);
            t.event(|| ev(turn, "project", p).s("project", name).end());
        });
    }

    pub(crate) fn tel_tech(&self, team: TeamId, tech: TechType) {
        let turn = self.game_turn();
        let name = self.d.techs[tech].base.type_.as_str();
        let members: Vec<PlayerId> = self.player_ids().filter(|&p| self.players[p].team == team && self.players[p].alive).collect();
        self.telemetry.with(|t| {
            for &p in &members {
                t.count(p, "tech", "n", 0);
                t.event(|| ev(turn, "tech", p).s("tech", name).end());
            }
        });
    }

    pub(crate) fn tel_revolution(&self, p: PlayerId, new_civics: &IdxVec<CivicOptionType, Option<CivicType>>) {
        let d = self.d;
        let turn = self.game_turn();
        let changed: Vec<&'static str> = d
            .civic_options
            .keys()
            .filter(|&o| self.players[p].civics[o] != new_civics[o])
            .filter_map(|o| new_civics[o].map(|c| d.civics[c].base.type_.as_str()))
            .collect();
        let anarchy = self.player_get_civic_anarchy_length(p, new_civics);
        self.telemetry.with(|t| {
            t.count(p, "revolution", "n", anarchy as i64);
            for &c in &changed {
                t.count(p, "civic", c, 0);
            }
            t.event(|| ev(turn, "civics", p).raw("to", &tel_alts(changed.iter().map(|&c| (c, 0)))).i("anarchy", anarchy as i64).end());
        });
    }

    pub(crate) fn tel_convert(&self, p: PlayerId, religion: Option<ReligionType>) {
        let turn = self.game_turn();
        let name = religion.map_or("NONE", |r| self.d.religions[r].base.type_.as_str());
        self.telemetry.with(|t| {
            t.count(p, "convert", name, 0);
            t.event(|| ev(turn, "religion", p).s("to", name).end());
        });
    }

    // ---- spells ----

    fn tel_reach(&self, u: UnitId, spell: SpellType) -> Vec<PlotIdx> {
        let info = &self.d.spells[spell];
        if info.global {
            return (0..self.map.plots.len()).map(PlotIdx::from_usize).collect();
        }
        let Some(plot) = self.unit_plot_opt(u) else {
            return Vec::new();
        };
        // Python spells reach up to two plots (`spellCrush`); built-in effects reach `iRange`.
        let r = info.range.max(2);
        let (x, y) = self.map.xy(plot);
        let mut out = Vec::new();
        for dx in -r..=r {
            for dy in -r..=r {
                if let Some(pi) = self.map.plot_xy(x + dx, y + dy) {
                    out.push(pi);
                }
            }
        }
        out
    }

    /// Before `unit_cast` runs.
    pub(crate) fn tel_cast_begin(&self, u: UnitId, spell: SpellType) -> CastSnap {
        let owner = self.units[u].owner;
        let nested = self.telemetry.with(|t| {
            t.cast_depth += 1;
            t.cast_depth > 1
        }) == Some(true);
        let next_unit = self.units.next_id();
        let mut snap = CastSnap { owner, spell, units: Vec::new(), next_unit, nested };
        let turn = self.game_turn();
        let team = self.players[owner].team;
        let unit_type = self.tel_name_unit(self.units[u].unit_type);
        let level = self.units[u].level;
        let spell_name = self.d.spells[spell].base.type_.as_str();
        let plot = self.unit_plot_opt(u);
        let city = plot.is_some_and(|pl| self.plot_is_city(pl, false, None));
        let at_war = self.team_get_at_war_count(team, true) > 0;
        let mut enemies = 0i64;
        if !nested {
            for pl in self.tel_reach(u, spell) {
                for &v in &self.map.plots[pl].units {
                    let unit = &self.units[v];
                    let promos = unit.has_promotion.iter().filter(|&&h| h).count() as u32;
                    let enemy = self.at_war(team, self.players[unit.owner].team);
                    snap.units.push((v, unit.owner, unit.damage, promos, enemy));
                    enemies += enemy as i64;
                }
            }
        }
        let (x, y) = plot.map_or((-1, -1), |pl| self.map.xy(pl));
        self.telemetry.with(|t| {
            let (reason, value) = if nested {
                ("nested".to_string(), 0)
            } else {
                match t.cast_reason.take() {
                    Some((ru, r, v)) if ru == u => (r, v),
                    _ => ("untagged".to_string(), 0),
                }
            };
            let s = t.spells.entry((owner, spell)).or_default();
            s.casts += 1;
            s.first_cast.get_or_insert(turn);
            s.last_cast = Some(turn);
            s.by_century[(turn.max(0) / 100).min(7) as usize] += 1;
            *s.reasons.entry(reason.clone()).or_insert(0) += 1;
            s.value += value;
            s.at_war += at_war as i64;
            s.in_city += city as i64;
            s.enemies += enemies;
            t.event(|| {
                ev(turn, "cast", owner)
                    .s("spell", spell_name)
                    .s("unit", unit_type)
                    .i("lvl", level as i64)
                    .i("x", x as i64)
                    .i("y", y as i64)
                    .b("city", city)
                    .b("war", at_war)
                    .i("enemies", enemies)
                    .s("reason", &reason)
                    .i("value", value)
                    .end()
            });
        });
        snap
    }

    /// After `unit_cast` ran: what it did to the units it reached.
    pub(crate) fn tel_cast_end(&self, snap: CastSnap) {
        let max_hp = self.d.defines.max_hit_points;
        // kills, damage, cursed, converted, summons, buffs, healed, own_lost, others_buffed
        let mut out = [0i64; 9];
        let mut summoned = Vec::new();
        if !snap.nested {
            for &(v, owner, damage, promos, enemy) in &snap.units {
                let own = owner == snap.owner;
                match self.units.get(v) {
                    None => out[if own { 7 } else { 0 }] += 1,
                    Some(unit) => {
                        let now_promos = unit.has_promotion.iter().filter(|&&h| h).count() as u32;
                        let dd = unit.damage.min(max_hp) - damage.min(max_hp);
                        if !own && unit.owner == snap.owner {
                            out[3] += 1;
                        } else if own {
                            if dd < 0 {
                                out[6] += -dd as i64;
                            }
                            if now_promos > promos {
                                out[5] += 1;
                            }
                        } else {
                            if dd > 0 {
                                out[1] += dd as i64;
                            }
                            if now_promos > promos {
                                out[if enemy { 2 } else { 8 }] += 1;
                            }
                        }
                    }
                }
            }
            for i in snap.next_unit.0..self.units.next_id().0 {
                if self.units.get(UnitId(i)).is_some_and(|unit| unit.owner == snap.owner) {
                    out[4] += 1;
                    summoned.push(UnitId(i));
                }
            }
        }
        let turn = self.game_turn();
        let spell_name = self.d.spells[snap.spell].base.type_.as_str();
        self.telemetry.with(|t| {
            t.cast_depth = t.cast_depth.saturating_sub(1);
            if snap.nested {
                return;
            }
            for v in summoned {
                t.summons.insert(v, snap.spell);
            }
            let s = t.spells.entry((snap.owner, snap.spell)).or_default();
            s.kills += out[0];
            s.damage += out[1];
            s.cursed += out[2];
            s.converted += out[3];
            s.summons += out[4];
            s.buffs += out[5];
            s.healed += out[6];
            s.own_lost += out[7];
            s.others_buffed += out[8];
            if out.iter().any(|&v| v != 0) {
                t.event(|| {
                    ev(turn, "cast_result", snap.owner)
                        .s("spell", spell_name)
                        .i("kills", out[0])
                        .i("damage", out[1])
                        .i("cursed", out[2])
                        .i("converted", out[3])
                        .i("summons", out[4])
                        .i("buffs", out[5])
                        .i("healed", out[6])
                        .i("own_lost", out[7])
                        .i("others_buffed", out[8])
                        .end()
                });
            }
        });
    }

    fn tel_spell_candidates(&self) -> Vec<Vec<(SpellType, bool)>> {
        let d = self.d;
        d.units
            .keys()
            .map(|ut| {
                let info = &d.units[ut];
                d.spells
                    .keys()
                    .filter(|&s| {
                        let sp = &d.spells[s];
                        let unit_side = sp.promotion_prereq1.is_some()
                            || sp.promotion_prereq2.is_some()
                            || sp.unit_prereq.is_some()
                            || sp.unit_class_prereq.is_some()
                            || sp.unit_combat_prereq.is_some()
                            || sp.civilization_prereq.is_some();
                        unit_side
                            && sp.unit_prereq.map_or(true, |t| t == ut)
                            && sp.unit_class_prereq.map_or(true, |c| info.unit_class_type == Some(c))
                    })
                    .map(|s| {
                        let sp = &d.spells[s];
                        let civ_only = sp.promotion_prereq1.is_none()
                            && sp.promotion_prereq2.is_none()
                            && sp.unit_prereq.is_none()
                            && sp.unit_class_prereq.is_none()
                            && sp.unit_combat_prereq.is_none();
                        (s, civ_only)
                    })
                    .collect()
            })
            .collect()
    }

    /// Does the unit meet the spell's unit-side prerequisites (it "knows" the spell)?
    fn tel_knows(&self, u: UnitId, spell: SpellType, human: bool) -> bool {
        let sp = &self.d.spells[spell];
        let unit = &self.units[u];
        (human || sp.allow_ai)
            && sp.promotion_prereq1.map_or(true, |p| unit.has_promotion[p])
            && sp.promotion_prereq2.map_or(true, |p| unit.has_promotion[p])
            && sp.unit_combat_prereq.map_or(true, |c| self.unit_get_unit_combat_type(u) == Some(c))
            && sp.civilization_prereq.map_or(true, |c| self.unit_get_civilization_type(u) == c)
            && sp.religion_prereq.map_or(true, |r| unit.religion == Some(r))
            && unit.level >= sp.caster_min_level
    }

    /// At the start of a player's turn (its units reset, before it moves): which spells its
    /// units know and can cast. A caster is a unit that knows a spell a promotion gives. At the
    /// end of its turn (`end`): which spells the units that cast nothing could still cast.
    pub(crate) fn tel_spell_scan(&self, p: PlayerId, end: bool) {
        let Some(mut cand) = self.telemetry.with(|t| std::mem::take(&mut t.cand)) else {
            return;
        };
        if cand.is_empty() {
            cand = self.tel_spell_candidates();
        }
        let turn = self.game_turn();
        let human = self.players[p].human;
        // spell -> [knowers, casters]
        let mut seen: BTreeMap<SpellType, [i64; 2]> = BTreeMap::new();
        let mut casters = 0i64;
        // At the end: casters that cast nothing and could still cast a spell of their own.
        let mut idle_casters = 0i64;
        for &u in &self.players[p].units {
            if self.unit_plot_opt(u).is_none() || (end && self.units[u].has_casted) {
                continue;
            }
            let mut caster = false;
            let mut castable_own = false;
            for &(s, civ_only) in &cand[self.units[u].unit_type.idx()] {
                if !self.tel_knows(u, s, human) {
                    continue;
                }
                caster |= self.d.spells[s].promotion_prereq1.is_some();
                let e = seen.entry(s).or_insert([0, 0]);
                if civ_only {
                    e[0] = 1;
                    if e[1] == 0 && self.unit_can_cast(u, s, false) {
                        e[1] = 1;
                    }
                } else {
                    e[0] += 1;
                    if self.unit_can_cast(u, s, false) {
                        e[1] += 1;
                        castable_own = true;
                    }
                }
            }
            casters += caster as i64;
            idle_casters += (caster && castable_own) as i64;
        }
        self.telemetry.with(|t| {
            t.cand = cand;
            if end {
                for (s, [_, c]) in seen {
                    if c > 0 {
                        t.spells.entry((p, s)).or_default().idle += c;
                    }
                }
                if idle_casters > 0 {
                    t.count(p, "casters", "idle_castable", idle_casters);
                }
                return;
            }
            if casters > 0 {
                t.count(p, "casters", "unit_turns", casters);
            }
            for (s, [k, c]) in seen {
                let st = t.spells.entry((p, s)).or_default();
                st.known += k;
                st.castable += c;
                st.turns_known += 1;
                st.first_known.get_or_insert(turn);
                if c > 0 {
                    st.turns_castable += 1;
                    st.first_castable.get_or_insert(turn);
                }
            }
        });
    }
}

// ----------------------------------------------------------------------------
// Recording (AI side) and output
// ----------------------------------------------------------------------------

impl Game {
    /// An AI plan that holds until it changes (a grand goal, a war target, a city's plan): kept
    /// as a timeline per player and kind when `value` differs from the last one.
    pub fn tel_intent(&self, p: PlayerId, kind: &'static str, value: impl FnOnce() -> String) {
        if !self.telemetry.on() {
            return;
        }
        let v = value();
        let turn = self.game_turn();
        self.telemetry.with(|t| {
            let line = t.timelines.entry((p, kind)).or_default();
            if line.last().is_some_and(|(_, last)| *last == v) {
                return;
            }
            if line.len() < 1000 {
                line.push((turn, v.clone()));
            }
            t.count(p, "intent_changes", kind, 0);
            t.event(|| ev(turn, "intent", p).s("kind", kind).s("value", &v).end());
        });
    }

    /// An AI decision: what it chose, the rule that chose it (`why`, may be empty), the chosen
    /// option's value, and the alternatives it weighed (JSON, e.g. [`tel_alts`]; evaluated only
    /// when events are written).
    pub fn tel_decision(&self, p: PlayerId, kind: &'static str, choice: &str, why: &'static str, value: i64, alts: impl FnOnce() -> String) {
        if !self.telemetry.on() {
            return;
        }
        let turn = self.game_turn();
        let has_events = self.telemetry.with(|t| t.events.is_some()) == Some(true);
        let alts = if has_events { alts() } else { String::new() };
        self.telemetry.with(|t| {
            let e = t.decisions.entry((p, kind, choice.to_string())).or_insert([0, 0]);
            e[0] += 1;
            e[1] += value;
            if !why.is_empty() {
                *t.whys.entry((p, kind, why)).or_insert(0) += 1;
            }
            t.event(|| {
                let o = ev(turn, "decision", p).s("kind", kind).s("choice", choice).s("why", why).i("value", value);
                if alts.is_empty() { o } else { o.raw("alts", &alts) }.end()
            });
        });
    }

    /// Why the controller is about to cast with `u` (read by the cast that follows).
    pub fn tel_cast_reason(&self, u: UnitId, reason: &str, value: i64) {
        self.telemetry.with(|t| t.cast_reason = Some((u, reason.to_string(), value)));
    }

    /// Why team `t` prepares or declares war on `target`; the declaration takes every reason
    /// given since the last one ("; "-joined).
    pub fn tel_war_reason(&self, t: TeamId, target: TeamId, reason: impl FnOnce() -> String) {
        if !self.telemetry.on() {
            return;
        }
        let r = reason();
        self.telemetry.with(|tl| {
            let e = tl.war_reasons.entry((t, target)).or_default();
            if !e.is_empty() {
                e.push_str("; ");
            }
            e.push_str(&r);
        });
    }

    /// The aggregates as one JSON object (None when telemetry is off): per player, `counts`
    /// (category -> key -> [count, sum]), `spells`, `timelines`, `decisions` (kind -> choice ->
    /// [count, value sum]) and `whys` (kind -> rule -> count); and `wars`.
    pub fn telemetry_json(&self) -> Option<String> {
        let d = self.d;
        let final_turn = self.game_turn();
        self.telemetry.with(|t| {
            let mut players: BTreeMap<PlayerId, Vec<(&'static str, String)>> = BTreeMap::new();
            // counts
            let mut cats: BTreeMap<(PlayerId, &'static str), Vec<String>> = BTreeMap::new();
            for (&(p, cat, key), v) in &t.counts {
                cats.entry((p, cat)).or_default().push(format!("{}:[{},{}]", json_str(key), v[0], v[1]));
            }
            let mut counts: BTreeMap<PlayerId, Vec<String>> = BTreeMap::new();
            for ((p, cat), items) in cats {
                counts.entry(p).or_default().push(format!("{}:{{{}}}", json_str(cat), items.join(",")));
            }
            for (p, v) in counts {
                players.entry(p).or_default().push(("counts", format!("{{{}}}", v.join(","))));
            }
            // spells
            let mut spells: BTreeMap<PlayerId, Vec<String>> = BTreeMap::new();
            for (&(p, s), st) in &t.spells {
                let reasons: Vec<String> = st.reasons.iter().map(|(r, n)| format!("{}:{}", json_str(r), n)).collect();
                let by: Vec<String> = st.by_century.iter().map(|n| n.to_string()).collect();
                let o = Obj::new()
                    .i("known", st.known)
                    .i("castable", st.castable)
                    .i("idle", st.idle)
                    .i("turns_known", st.turns_known)
                    .i("turns_castable", st.turns_castable)
                    .opt("first_known", st.first_known.map(i64::from))
                    .opt("first_castable", st.first_castable.map(i64::from))
                    .i("casts", st.casts)
                    .opt("first_cast", st.first_cast.map(i64::from))
                    .opt("last_cast", st.last_cast.map(i64::from))
                    .raw("by_century", &format!("[{}]", by.join(",")))
                    .raw("reasons", &format!("{{{}}}", reasons.join(",")))
                    .i("value", st.value)
                    .i("at_war", st.at_war)
                    .i("in_city", st.in_city)
                    .i("enemies", st.enemies)
                    .i("kills", st.kills)
                    .i("damage", st.damage)
                    .i("cursed", st.cursed)
                    .i("converted", st.converted)
                    .i("others_buffed", st.others_buffed)
                    .i("summons", st.summons)
                    .i("summon_kills", st.summon_kills)
                    .i("buffs", st.buffs)
                    .i("healed", st.healed)
                    .i("own_lost", st.own_lost)
                    .b("world", d.spells[s].global)
                    .end();
                spells.entry(p).or_default().push(format!("{}:{}", json_str(&d.spells[s].base.type_), o));
            }
            for (p, v) in spells {
                players.entry(p).or_default().push(("spells", format!("{{{}}}", v.join(","))));
            }
            // timelines
            let mut tl: BTreeMap<PlayerId, Vec<String>> = BTreeMap::new();
            for (&(p, kind), line) in &t.timelines {
                let items: Vec<String> = line.iter().map(|(turn, v)| format!("[{},{}]", turn, json_str(v))).collect();
                tl.entry(p).or_default().push(format!("{}:[{}]", json_str(kind), items.join(",")));
            }
            for (p, v) in tl {
                players.entry(p).or_default().push(("timelines", format!("{{{}}}", v.join(","))));
            }
            // decisions
            let mut dk: BTreeMap<(PlayerId, &'static str), Vec<String>> = BTreeMap::new();
            for ((p, kind, choice), v) in &t.decisions {
                dk.entry((*p, *kind)).or_default().push(format!("{}:[{},{}]", json_str(choice), v[0], v[1]));
            }
            let mut dec: BTreeMap<PlayerId, Vec<String>> = BTreeMap::new();
            for ((p, kind), items) in dk {
                dec.entry(p).or_default().push(format!("{}:{{{}}}", json_str(kind), items.join(",")));
            }
            for (p, v) in dec {
                players.entry(p).or_default().push(("decisions", format!("{{{}}}", v.join(","))));
            }
            let mut wk: BTreeMap<(PlayerId, &'static str), Vec<String>> = BTreeMap::new();
            for (&(p, kind, why), n) in &t.whys {
                wk.entry((p, kind)).or_default().push(format!("{}:{}", json_str(why), n));
            }
            let mut whys: BTreeMap<PlayerId, Vec<String>> = BTreeMap::new();
            for ((p, kind), items) in wk {
                whys.entry(p).or_default().push(format!("{}:{{{}}}", json_str(kind), items.join(",")));
            }
            for (p, v) in whys {
                players.entry(p).or_default().push(("whys", format!("{{{}}}", v.join(","))));
            }
            let pl: Vec<String> = players
                .into_iter()
                .map(|(p, fields)| {
                    let mut o = Obj::new();
                    for (k, v) in fields {
                        o = o.raw(k, &v);
                    }
                    format!("\"{}\":{}", p.0, o.end())
                })
                .collect();
            let wars: Vec<String> = t
                .wars
                .iter()
                .map(|w| {
                    let o = Obj::new()
                        .i("start", w.start as i64)
                        .opt("end", w.end.map(i64::from))
                        .i("turns", (w.end.unwrap_or(final_turn) - w.start) as i64)
                        .i("a", w.teams[0].0 as i64)
                        .i("b", w.teams[1].0 as i64)
                        .raw("a_players", &self.tel_team_players(w.teams[0]))
                        .raw("b_players", &self.tel_team_players(w.teams[1]))
                        .b("barb", self.team_is_barbarian(w.teams[0]) || self.team_is_barbarian(w.teams[1]))
                        .s("plan", w.plan);
                    match &w.reason {
                        Some(r) => o.s("reason", r),
                        None => o,
                    }
                    .raw("cities", &format!("[{},{}]", w.cities[0], w.cities[1]))
                    .raw("pop", &format!("[{},{}]", w.pop[0], w.pop[1]))
                    .raw("kills", &format!("[{},{}]", w.kills[0], w.kills[1]))
                    .end()
                })
                .collect();
            Obj::new().i("version", 1).raw("players", &format!("{{{}}}", pl.join(","))).raw("wars", &format!("[{}]", wars.join(","))).end()
        })
    }
}
