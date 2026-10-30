//! Cultural actions (this game's own rules, not FFH2's; `GAMEOPTION_CULTURE_ACTIONS`;
//! docs/CULTURE.md).
//!
//! Each city has a cultural queue next to its production queue. Its first order is the
//! *channel*: every turn the city diverts the action's rate R of its culture to it (that culture
//! reaches neither the city's culture, nor its plots, nor the culture victory), and after the
//! action's duration D (scaled by the game speed) the effect resolves on the target. A channel
//! whose city's culture rate falls below R, or whose target stops qualifying for a while (out of
//! range, share too low, an ally no longer allied), pauses: no progress, nothing diverted; after
//! `CULTURE_CHANNEL_STALL_TURNS` paused turns it is cancelled. Cancelling refunds nothing.
//!
//! The effectiveness E of an action on a foreign city is `Sc / (Sc + K·So + M)`: the caster's and
//! the owner's shares of the culture on the city's plot (its nationality), `K` = `CULTURE_RESIST_K`, `M` the city's defences against
//! hostile actions (a festival, the garrison). Here it is an integer percent.
//!
//! Every channel is public to the civilizations that have met its caster
//! ([`Game::culture_channels`]); the target's owner is told when a hostile channel starts and
//! when it resolves. Completed actions leave a memory of cultural hostility or friendship with
//! the target's owner (`Player::culture_memory`), which the AI's attitude reads.

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

/// What a cultural order is aimed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CultureTarget {
    City(CityId),
}

/// One order of a city's cultural queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureOrder {
    pub action: CulturalActionType,
    pub target: CultureTarget,
    /// The religion spread (Spread Religion); `None` for every other action.
    pub religion: Option<ReligionType>,
    /// Queue the order again once it completes (like production's repeat).
    pub repeat: bool,
    /// Turns channelled so far.
    pub progress: i32,
    /// Consecutive paused turns of the channel.
    pub stalled: i32,
    /// The game turn the order first became its city's channel (-1 while it has not).
    pub started_turn: i32,
}

/// A timed effect of a completed cultural action on a city (`City::culture_marks`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureMark {
    /// Festival, TradeFair or InciteUnrest.
    pub effect: CultureEffect,
    /// The caster (for a trade fair: whose cities' routes with this city gain).
    pub from: PlayerId,
    /// Happiness (festival), trade percent (fair) or unhappiness (unrest).
    pub amount: i32,
    /// Turns left, counted down in the city's turn.
    pub turns: i32,
}

/// Why an order cannot be queued, or why a channel pauses or ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CultureRefusal {
    /// `GAMEOPTION_CULTURE_ACTIONS` is off.
    Off,
    /// The target city no longer exists.
    TargetGone,
    /// The target is of the wrong kind for the action (whose city it is, an unmet civilization,
    /// a city the caster has not seen).
    Target(&'static str),
    /// The casting city's culture level is below the tier's.
    Level(Option<CultureLevelType>),
    /// The casting city makes `have` culture a turn; the action needs `need`.
    Rate { have: i32, need: i32 },
    /// The target is `dist` plots away; the tier reaches `max`.
    Range { dist: i32, max: i32 },
    /// The caster's share of the target's culture is `have`%; the tier needs `need`%.
    Share { have: i32, need: i32 },
    /// The effectiveness is `have`%; actions need at least `min`%.
    Effect { have: i32, min: i32 },
    /// Spread Religion: the casting city lacks the religion, the target has it, or the
    /// target's owner admits no religion but its state religion.
    Religion(&'static str),
    /// A trade fair needs trade routes between the two civilizations.
    NoTrade,
    /// Another city of the caster already channels this order.
    Duplicate,
    /// The queue is full, or already holds the order.
    Queue(&'static str),
}

impl CultureRefusal {
    /// A refusal that cannot pass by itself: the channel is cancelled at once rather than paused.
    pub fn permanent(&self) -> bool {
        matches!(self, CultureRefusal::Off | CultureRefusal::TargetGone | CultureRefusal::Target(_) | CultureRefusal::Religion(_))
    }

    /// The reason for the player, in a sentence fragment ("needs 4 culture a turn (has 2)").
    pub fn describe(&self, d: &GameData) -> String {
        match self {
            CultureRefusal::Off => "cultural actions are off in this game".into(),
            CultureRefusal::TargetGone => "the target is gone".into(),
            CultureRefusal::Target(why) => (*why).into(),
            CultureRefusal::Level(l) => match l {
                Some(l) => format!("needs a {} city", d.text.name(&d.culture_levels[*l].base.description)),
                None => "needs a higher culture level".into(),
            },
            CultureRefusal::Rate { have, need } => format!("needs {need} culture a turn (has {have})"),
            CultureRefusal::Range { dist, max } => format!("out of range ({dist} plots; reaches {max})"),
            CultureRefusal::Share { have, need } => format!("our culture there is {have}% (needs {need}%)"),
            CultureRefusal::Effect { have, min } => format!("effectiveness {have}% (needs {min}%)"),
            CultureRefusal::Religion(why) => (*why).into(),
            CultureRefusal::NoTrade => "no trade routes with them (open borders, no war)".into(),
            CultureRefusal::Duplicate => "another of our cities already channels it".into(),
            CultureRefusal::Queue(why) => (*why).into(),
        }
    }
}

/// What an order would do now ([`Game::culture_check`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CulturePreview {
    /// Effectiveness E in percent (100 when the action is not scaled by it).
    pub effectiveness: i32,
    /// Hostile to the target's owner.
    pub hostile: bool,
    /// The magnitude on completion: happiness (festival), trade percent (fair), chance in
    /// percent (spread), unhappiness (unrest).
    pub magnitude: i32,
    /// Turns the effect lasts (0 for a one-off: a spread).
    pub effect_turns: i32,
    /// R and D (scaled by the game speed).
    pub rate: i32,
    pub duration: i32,
}

/// What happened to a channel ([`Game::culture_event`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CultureEventKind {
    Start,
    /// Completed: whether the effect took (a spread can fail) and its magnitude.
    Complete { success: bool, magnitude: i32 },
    Cancel,
}

/// A public channel: a city's first cultural order ([`Game::culture_channels`]).
#[derive(Clone, Debug)]
pub struct Channel {
    pub city: CityId,
    pub caster: PlayerId,
    pub order: CultureOrder,
    /// D, scaled by the game speed.
    pub duration: i32,
}

impl Game {
    /// Whether this game has cultural actions.
    #[inline]
    pub fn culture_actions_on(&self) -> bool {
        self.is_option(GameOptionType::GAMEOPTION_CULTURE_ACTIONS)
    }

    /// A number of turns at normal speed, scaled by the game speed like build times
    /// (`iTrainPercent`), rounded, at least 1.
    pub fn culture_speed_turns(&self, turns: i32) -> i32 {
        let pct = self.d.game_speeds[self.setup.game_speed].train_percent;
        ((turns * pct + 50) / 100).max(1)
    }

    /// The channel's duration D of `action`, scaled by the game speed.
    pub fn culture_action_duration(&self, action: CulturalActionType) -> i32 {
        self.culture_speed_turns(self.d.cultural_actions[action].duration)
    }

    /// The culture level a tier needs (`None` for tier 1: any).
    pub fn culture_tier_level(&self, tier: i32) -> Option<CultureLevelType> {
        let defines = &self.d.defines;
        match tier {
            2 => opt_from_i32(defines.culture_tier2_level),
            3 => opt_from_i32(defines.culture_tier3_level),
            _ => None,
        }
    }

    /// A tier's reach in plots from the casting city.
    pub fn culture_tier_range(&self, tier: i32) -> i32 {
        let defines = &self.d.defines;
        match tier {
            2 => defines.culture_range_t2,
            3 => defines.culture_range_t3,
            _ => defines.culture_range_t1,
        }
    }

    /// A tier's minimum caster share (percent) in a foreign target.
    pub fn culture_tier_min_share(&self, tier: i32) -> i32 {
        let defines = &self.d.defines;
        match tier {
            2 => defines.culture_min_share_t2,
            3 => defines.culture_min_share_t3,
            _ => defines.culture_min_share_t1,
        }
    }

    /// Whether `caster`'s team counts `owner`'s as an ally for friendly actions: the same team
    /// (a permanent alliance is one team) or a defensive pact.
    pub fn culture_is_ally(&self, caster: PlayerId, owner: PlayerId) -> bool {
        let (a, b) = (self.players[caster].team, self.players[owner].team);
        a == b || self.teams[a].defensive_pact[b]
    }

    /// Whether `action` with `religion` on city `target` would be hostile to its owner.
    pub fn culture_is_hostile(&self, action: CulturalActionType, caster: PlayerId, target: CityId, religion: Option<ReligionType>) -> bool {
        let owner = self.cities[target].owner;
        if self.players[owner].team == self.players[caster].team {
            return false;
        }
        match self.d.cultural_actions[action].hostile {
            CultureHostility::Never => false,
            CultureHostility::Always => true,
            CultureHostility::DifferentStateReligion => match self.player_get_state_religion(owner) {
                Some(state) => Some(state) != religion,
                None => false,
            },
        }
    }

    /// Land combat units of the city's team on its plot: each resists the actions that the
    /// garrison resists.
    pub fn culture_garrison_count(&self, c: CityId) -> i32 {
        let team = self.city_team(c);
        let plot = self.city_plot(c);
        self.map.plots[plot]
            .units
            .iter()
            .filter(|&&u| {
                self.unit_get_team(u) == team
                    && self.d.units[self.units[u].unit_type].domain_type == Some(DomainType::DOMAIN_LAND)
                    && self.unit_can_defend(u, Some(plot))
            })
            .count() as i32
    }

    /// The defensive modifiers M of city `c` against a hostile `action`, in share points
    /// (100 = a whole share): a festival, and each garrison unit for the actions the garrison
    /// resists.
    pub fn culture_resistance(&self, c: CityId, action: CulturalActionType) -> i32 {
        let defines = &self.d.defines;
        let mut m = 0;
        if self.cities[c].culture_marks.iter().any(|k| k.effect == CultureEffect::Festival) {
            m += defines.culture_festival_resist;
        }
        if self.d.cultural_actions[action].effect == CultureEffect::InciteUnrest {
            m += defines.culture_garrison_resist * self.culture_garrison_count(c);
        }
        m
    }

    /// `player`'s share (percent) of the culture on city `c`'s plot: its nationality there, as
    /// the city screen shows it.
    pub fn culture_share(&self, c: CityId, player: PlayerId) -> i32 {
        self.plot_calculate_culture_percent(self.city_plot(c), player)
    }

    /// The effectiveness E (percent) of `caster`'s `action` on foreign city `target`:
    /// `100·Sc / (Sc + K·So + M)` over the culture on the city's plot (M only against a hostile
    /// action).
    pub fn culture_effectiveness(&self, action: CulturalActionType, caster: PlayerId, target: CityId, hostile: bool) -> i32 {
        let plot = self.city_plot(target);
        let culture = &self.map.plots[plot].culture;
        let sc = culture[caster] as i64;
        if sc <= 0 {
            return 0;
        }
        let so = culture[self.cities[target].owner] as i64;
        let total = self.plot_count_total_culture(plot).max(1) as i64;
        let k = self.d.defines.culture_resist_k as i64;
        let m = if hostile { self.culture_resistance(target, action) as i64 } else { 0 };
        // Sc / (Sc + K/100·So + M/100·total), all in hundredths.
        let denom = 100 * sc + k * so + m * total;
        ((100 * 100 * sc) / denom.max(1)) as i32
    }

    /// Whether city `c` could channel `action` on `target` now, and what it would do. `queueing`
    /// also checks the queue (full, already holding the order) and asks for the full rate; a
    /// channel checks itself without them each turn (a missing rate pauses it).
    pub fn culture_check(&self, c: CityId, action: CulturalActionType, target: CultureTarget, religion: Option<ReligionType>, queueing: bool) -> Result<CulturePreview, CultureRefusal> {
        let d = self.d;
        let defines = &d.defines;
        if !self.culture_actions_on() {
            return Err(CultureRefusal::Off);
        }
        let info = &d.cultural_actions[action];
        let caster = self.cities[c].owner;
        if caster == self.barbarian_player() {
            return Err(CultureRefusal::Target("barbarians take no cultural actions"));
        }
        let CultureTarget::City(t) = target;
        if !self.cities.contains(t) {
            return Err(CultureRefusal::TargetGone);
        }
        let owner = self.cities[t].owner;
        let caster_team = self.players[caster].team;
        let owner_team = self.players[owner].team;
        let own_team = caster_team == owner_team;

        // The target's kind.
        match info.target {
            CultureTargetKind::OwnOrAllyCity => {
                if !self.culture_is_ally(caster, owner) {
                    return Err(CultureRefusal::Target("only our cities and those of our defensive-pact partners"));
                }
            }
            CultureTargetKind::ForeignCity => {
                if own_team {
                    return Err(CultureRefusal::Target("only foreign cities"));
                }
            }
            CultureTargetKind::AnyCity => {}
        }
        if !own_team {
            if owner != self.barbarian_player() && !self.teams[caster_team].has_met[owner_team] {
                return Err(CultureRefusal::Target("we have not met them"));
            }
            if !self.city_is_revealed(t, caster_team, false) {
                return Err(CultureRefusal::Target("we have not seen that city"));
            }
        }

        // The action's own conditions.
        match info.effect {
            CultureEffect::SpreadReligion => {
                let Some(r) = religion else {
                    return Err(CultureRefusal::Religion("no religion chosen"));
                };
                if !self.cities[c].has_religion[r] {
                    return Err(CultureRefusal::Religion("the casting city lacks that religion"));
                }
                if self.cities[t].has_religion[r] {
                    return Err(CultureRefusal::Religion("the target already has that religion"));
                }
                if !own_team && self.player_is_no_non_state_religion_spread(owner) && self.player_get_state_religion(owner) != Some(r) {
                    return Err(CultureRefusal::Religion("their civics admit no religion but their state religion"));
                }
            }
            CultureEffect::TradeFair => {
                if self.at_war(caster_team, owner_team) || !self.player_can_have_trade_routes_with(caster, owner) {
                    return Err(CultureRefusal::NoTrade);
                }
            }
            CultureEffect::Festival | CultureEffect::InciteUnrest => {}
        }

        // The casting city: tier, rate, range.
        if let Some(level) = self.culture_tier_level(info.tier) {
            if self.cities[c].culture_level.map_or(true, |l| l < level) {
                return Err(CultureRefusal::Level(Some(level)));
            }
        }
        let rate = self.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE);
        if rate < info.rate {
            return Err(CultureRefusal::Rate { have: rate, need: info.rate });
        }
        let (cx, cy) = (self.cities[c].x, self.cities[c].y);
        let dist = self.plot_distance(cx, cy, self.cities[t].x, self.cities[t].y);
        let max = self.culture_tier_range(info.tier);
        if dist > max {
            return Err(CultureRefusal::Range { dist, max });
        }

        // Presence and effectiveness abroad.
        let hostile = self.culture_is_hostile(action, caster, t, religion);
        let mut effectiveness = 100;
        if !own_team {
            let share = self.culture_share(t, caster);
            let need = self.culture_tier_min_share(info.tier);
            if share < need {
                return Err(CultureRefusal::Share { have: share, need });
            }
            if info.scaled_by_effect {
                effectiveness = self.culture_effectiveness(action, caster, t, hostile);
                if effectiveness < defines.culture_min_effect {
                    return Err(CultureRefusal::Effect { have: effectiveness, min: defines.culture_min_effect });
                }
            }
        }

        // One channel of an order per civilization.
        let me = (action, target, religion);
        let duplicate = self.players[caster].cities.iter().any(|&o| {
            o != c && self.cities[o].culture_queue.first().is_some_and(|q| (q.action, q.target, q.religion) == me)
        });
        if duplicate {
            return Err(CultureRefusal::Duplicate);
        }
        if queueing {
            let queue = &self.cities[c].culture_queue;
            if queue.len() as i32 >= defines.culture_queue_max.max(1) {
                return Err(CultureRefusal::Queue("the cultural queue is full"));
            }
            if queue.iter().any(|q| (q.action, q.target, q.religion) == me) {
                return Err(CultureRefusal::Queue("already in the queue"));
            }
        }

        let scale = |n: i32| (n * effectiveness + 50) / 100;
        let (magnitude, effect_turns) = match info.effect {
            CultureEffect::Festival => (defines.culture_festival_happy, self.culture_speed_turns(defines.culture_festival_turns)),
            CultureEffect::TradeFair => (scale(defines.culture_trade_fair_pct).max(1), self.culture_speed_turns(defines.culture_trade_fair_turns)),
            CultureEffect::SpreadReligion => (scale(defines.culture_spread_base).clamp(0, 100), 0),
            CultureEffect::InciteUnrest => (scale(defines.culture_unrest_unhappy).max(1), self.culture_speed_turns(defines.culture_unrest_turns)),
        };
        Ok(CulturePreview { effectiveness, hostile, magnitude, effect_turns, rate: info.rate, duration: self.culture_action_duration(action) })
    }

    /// Queues `action` on `target` in city `c` (at the end, or `first`: the current channel then
    /// waits, keeping its progress). An order that becomes the channel starts at once.
    pub fn city_push_culture_order(&mut self, c: CityId, action: CulturalActionType, target: CultureTarget, religion: Option<ReligionType>, repeat: bool, first: bool) -> Result<(), CultureRefusal> {
        self.culture_check(c, action, target, religion, true)?;
        let order = CultureOrder { action, target, religion, repeat, progress: 0, stalled: 0, started_turn: -1 };
        let queue = &mut self.cities[c].culture_queue;
        if first {
            queue.insert(0, order);
        } else {
            queue.push(order);
        }
        self.culture_start_head(c);
        Ok(())
    }

    /// Removes order `index` of city `c`'s cultural queue (a started channel is cancelled).
    pub fn city_pop_culture_order(&mut self, c: CityId, index: usize) {
        if index >= self.cities[c].culture_queue.len() {
            return;
        }
        let order = self.cities[c].culture_queue.remove(index);
        if order.started_turn >= 0 {
            self.culture_event(c, &order, CultureEventKind::Cancel);
        }
        if index == 0 {
            self.culture_start_head(c);
        }
    }

    /// Sets the repeat flag of order `index`.
    pub fn city_set_culture_order_repeat(&mut self, c: CityId, index: usize, repeat: bool) {
        if let Some(o) = self.cities[c].culture_queue.get_mut(index) {
            o.repeat = repeat;
        }
    }

    /// The head of city `c`'s queue became the channel: the first time, it starts (public).
    fn culture_start_head(&mut self, c: CityId) {
        let turn = self.game_turn();
        let Some(head) = self.cities[c].culture_queue.first_mut() else { return };
        head.stalled = 0;
        if head.started_turn >= 0 {
            return;
        }
        head.started_turn = turn;
        let order = head.clone();
        self.culture_event(c, &order, CultureEventKind::Start);
    }

    /// The channel of city `c` this turn (its turn, before its culture is added): progress
    /// or pause, completion, cancellation. Sets `culture_diverted`.
    pub fn city_do_culture_channel(&mut self, c: CityId) {
        self.cities[c].culture_diverted = 0;
        if self.cities[c].culture_queue.is_empty() {
            return;
        }
        let stall_turns = self.d.defines.culture_channel_stall_turns.max(1);
        let order = self.cities[c].culture_queue[0].clone();
        match self.culture_check(c, order.action, order.target, order.religion, false) {
            Err(r) if r.permanent() => {
                self.cities[c].culture_queue.remove(0);
                self.culture_event(c, &order, CultureEventKind::Cancel);
                self.culture_start_head(c);
            }
            Err(_) => {
                let head = &mut self.cities[c].culture_queue[0];
                head.stalled += 1;
                if head.stalled >= stall_turns {
                    self.cities[c].culture_queue.remove(0);
                    self.culture_event(c, &order, CultureEventKind::Cancel);
                    self.culture_start_head(c);
                }
            }
            Ok(preview) => {
                self.cities[c].culture_diverted = preview.rate;
                let head = &mut self.cities[c].culture_queue[0];
                head.stalled = 0;
                head.progress += 1;
                if head.progress >= preview.duration {
                    let done = self.cities[c].culture_queue.remove(0);
                    self.culture_complete(c, &done, preview);
                    if done.repeat && self.cities.contains(c) {
                        let again = CultureOrder { progress: 0, stalled: 0, started_turn: -1, ..done };
                        self.cities[c].culture_queue.push(again);
                    }
                    if self.cities.contains(c) {
                        self.culture_start_head(c);
                    }
                }
            }
        }
    }

    /// Resolves a completed order of city `c` with what it does now (`p`).
    fn culture_complete(&mut self, c: CityId, order: &CultureOrder, p: CulturePreview) {
        let CultureTarget::City(t) = order.target;
        let caster = self.cities[c].owner;
        let effect = self.d.cultural_actions[order.action].effect;
        let mut success = true;
        match effect {
            CultureEffect::Festival | CultureEffect::TradeFair | CultureEffect::InciteUnrest => {
                self.city_add_culture_mark(t, CultureMark { effect, from: caster, amount: p.magnitude, turns: p.effect_turns });
            }
            CultureEffect::SpreadReligion => {
                let r = order.religion.expect("checked: a spread has a religion");
                success = self.soren_rand_num(100, "Cultural Spread Religion") < p.magnitude;
                if success {
                    // The religion is in the casting city, so it is founded: as a missionary's
                    // spread (CvUnit::spread), with FfH's religion hooks.
                    self.city_set_has_religion(t, r, true, true, false);
                }
            }
        }
        self.culture_event(c, order, CultureEventKind::Complete { success, magnitude: p.magnitude });
    }

    /// Adds a timed mark to city `c`: one from the same caster with the same effect is
    /// replaced (refreshed), others stack.
    pub fn city_add_culture_mark(&mut self, c: CityId, mark: CultureMark) {
        let marks = &mut self.cities[c].culture_marks;
        marks.retain(|k| !(k.effect == mark.effect && k.from == mark.from));
        let fair = mark.effect == CultureEffect::TradeFair;
        let from = mark.from;
        marks.push(mark);
        self.cities[c].ai_assign_work_dirty = true;
        if fair {
            let owner = self.cities[c].owner;
            self.player_update_trade_routes(owner);
            if from != owner {
                self.player_update_trade_routes(from);
            }
        }
    }

    /// City `c`'s marks age by a turn (its turn); expired ones go.
    pub fn city_do_culture_marks(&mut self, c: CityId) {
        if self.cities[c].culture_marks.is_empty() {
            return;
        }
        let mut fairs_ended = Vec::new();
        let marks = &mut self.cities[c].culture_marks;
        for k in marks.iter_mut() {
            k.turns -= 1;
        }
        marks.retain(|k| {
            if k.turns > 0 {
                return true;
            }
            if k.effect == CultureEffect::TradeFair {
                fairs_ended.push(k.from);
            }
            false
        });
        self.cities[c].ai_assign_work_dirty = true;
        if !fairs_ended.is_empty() {
            let owner = self.cities[c].owner;
            self.player_update_trade_routes(owner);
            for from in fairs_ended {
                if from != owner && self.players[from].alive {
                    self.player_update_trade_routes(from);
                }
            }
        }
    }

    /// Happiness from festivals in city `c` (they do not stack).
    pub fn city_culture_happiness(&self, c: CityId) -> i32 {
        self.cities[c].culture_marks.iter().filter(|k| k.effect == CultureEffect::Festival).map(|k| k.amount).max().unwrap_or(0)
    }

    /// Unhappiness incited in city `c` (one mark per caster; casters add up).
    pub fn city_culture_unhappiness(&self, c: CityId) -> i32 {
        self.cities[c].culture_marks.iter().filter(|k| k.effect == CultureEffect::InciteUnrest).map(|k| k.amount).sum()
    }

    /// The trade fair bonus (percent) of the route from city `c` to `other`: a fair in either
    /// city held by the other city's owner (the larger, if both).
    pub fn city_trade_fair_modifier(&self, c: CityId, other: CityId) -> i32 {
        let fair = |at: CityId, by: PlayerId| {
            self.cities[at].culture_marks.iter().filter(|k| k.effect == CultureEffect::TradeFair && k.from == by).map(|k| k.amount).max().unwrap_or(0)
        };
        fair(c, self.cities[other].owner).max(fair(other, self.cities[c].owner))
    }

    /// City `c` is going (destroyed, or captured: the capturer gets a new city): its channel
    /// ends, and so do the channels aimed at it.
    pub fn city_culture_lost(&mut self, c: CityId) {
        if let Some(order) = self.cities[c].culture_queue.first().cloned() {
            if order.started_turn >= 0 {
                self.culture_event(c, &order, CultureEventKind::Cancel);
            }
        }
        self.cities[c].culture_queue.clear();
        let others: Vec<CityId> = self.cities.iter().map(|(o, _)| o).filter(|&o| o != c).collect();
        for o in others {
            let aimed = |q: &CultureOrder| q.target == CultureTarget::City(c);
            if !self.cities[o].culture_queue.iter().any(aimed) {
                continue;
            }
            let head_hit = self.cities[o].culture_queue.first().is_some_and(aimed);
            let head = self.cities[o].culture_queue[0].clone();
            self.cities[o].culture_queue.retain(|q| !aimed(q));
            if head_hit {
                if head.started_turn >= 0 {
                    self.culture_event(o, &head, CultureEventKind::Cancel);
                }
                self.culture_start_head(o);
            }
        }
    }

    /// The memories of cultural actions fade: every `CULTURE_MEMORY_DECAY_TURNS` turns, a point
    /// of each (player `p`'s turn).
    pub fn player_do_culture_memory(&mut self, p: PlayerId) {
        let every = self.d.defines.culture_memory_decay_turns;
        if every <= 0 || self.game_turn() % every != 0 {
            return;
        }
        for m in self.players[p].culture_memory.iter_mut() {
            for v in m.iter_mut() {
                *v = (*v - 1).max(0);
            }
        }
    }

    /// Every channel in the game (each city's first cultural order), in city id order.
    pub fn culture_channels(&self) -> Vec<Channel> {
        let mut out = Vec::new();
        for (c, city) in self.cities.iter() {
            if let Some(order) = city.culture_queue.first() {
                out.push(Channel { city: c, caster: city.owner, order: order.clone(), duration: self.culture_action_duration(order.action) });
            }
        }
        out
    }

    /// Whether team `team` sees `caster`'s channels: its own, and those of civilizations it has
    /// met.
    pub fn culture_channel_visible(&self, team: TeamId, caster: PlayerId) -> bool {
        let ct = self.players[caster].team;
        ct == team || self.teams[team].has_met[ct]
    }

    /// The hostile channels aimed at city `t`.
    pub fn culture_hostile_channels_at(&self, t: CityId) -> Vec<Channel> {
        self.culture_channels()
            .into_iter()
            .filter(|ch| ch.order.target == CultureTarget::City(t) && self.culture_is_hostile(ch.order.action, ch.caster, t, ch.order.religion))
            .collect()
    }

    /// A channel started, completed or was cancelled: the messages to the caster and the
    /// target's owner, the target owner's memory of it, telemetry. (The hook for anything that
    /// reacts to cultural actions: a diplomacy or threat system would subscribe here.)
    pub fn culture_event(&mut self, c: CityId, order: &CultureOrder, kind: CultureEventKind) {
        let d = self.d;
        let turn = self.game_turn();
        let caster = self.cities[c].owner;
        let info = &d.cultural_actions[order.action];
        let action = d.text.name(&info.base.description).to_string();
        let CultureTarget::City(t) = order.target;
        let target_alive = self.cities.contains(t);
        let target_name = if target_alive { d.text.name(&self.cities[t].name).to_string() } else { "a lost city".to_string() };
        let city_name = d.text.name(&self.cities[c].name).to_string();
        let owner = target_alive.then(|| self.cities[t].owner);
        let hostile = target_alive && self.culture_is_hostile(order.action, caster, t, order.religion);
        let civ = self.player_get_civilization_adjective(caster);
        let religion = order.religion.map(|r| d.text.name(&d.religions[r].base.description).to_string());
        let what = match &religion {
            Some(r) => format!("{action} ({r})"),
            None => action.clone(),
        };
        let at = target_alive.then(|| (self.cities[t].x, self.cities[t].y));
        let foreign_owner = owner.filter(|&o| self.players[o].team != self.players[caster].team);

        let (to_caster, to_owner) = match kind {
            CultureEventKind::Start => {
                let dur = self.culture_action_duration(order.action);
                (None, hostile.then(|| format!("{city_name} ({civ}) has begun {what} against {target_name}: {dur} turns. A festival or a garrison resists it.")))
            }
            CultureEventKind::Cancel => (
                Some(format!("{city_name}'s {what} on {target_name} was cancelled.")),
                hostile.then(|| format!("{city_name} ({civ}) gave up {what} against {target_name}.")),
            ),
            CultureEventKind::Complete { success, magnitude } => {
                let result = self.culture_result_text(info.effect, success, magnitude, order);
                (
                    Some(format!("{city_name}'s {what} on {target_name}: {result}.")),
                    foreign_owner.map(|_| format!("{city_name} ({civ}) completed {what} on {target_name}: {result}.")),
                )
            }
        };
        if let Some(text) = to_caster {
            match at {
                Some(at) => self.log.msg_at(turn, Some(caster), at, || text),
                None => self.log.msg(turn, Some(caster), || text),
            }
        }
        if let (Some(text), Some(o)) = (to_owner, foreign_owner) {
            match at {
                Some(at) => self.log.msg_at(turn, Some(o), at, || text),
                None => self.log.msg(turn, Some(o), || text),
            }
        }

        // The target owner's memory of a completed action.
        if let (CultureEventKind::Complete { .. }, Some(o)) = (kind, foreign_owner) {
            let friendly = match info.effect {
                CultureEffect::Festival | CultureEffect::TradeFair => true,
                CultureEffect::SpreadReligion => order.religion.is_some() && self.player_get_state_religion(o) == order.religion,
                CultureEffect::InciteUnrest => false,
            };
            let slot = if hostile { Some(0) } else if friendly { Some(1) } else { None };
            if let Some(slot) = slot {
                let cap = d.defines.culture_memory_max.max(1);
                let m = &mut self.players[o].culture_memory[caster][slot];
                *m = (*m + info.memory_weight).min(cap);
            }
        }

        self.tel_culture(c, order, kind, hostile, owner);
    }

    /// A completed action's result for the messages ("+2 unhappiness for 5 turns").
    fn culture_result_text(&self, effect: CultureEffect, success: bool, magnitude: i32, order: &CultureOrder) -> String {
        let d = self.d;
        let defines = &d.defines;
        match effect {
            CultureEffect::Festival => format!("+{magnitude} happiness and resistance to hostile actions for {} turns", self.culture_speed_turns(defines.culture_festival_turns)),
            CultureEffect::TradeFair => format!("trade routes between the city and the caster's cities +{magnitude}% for {} turns", self.culture_speed_turns(defines.culture_trade_fair_turns)),
            CultureEffect::SpreadReligion => {
                let r = order.religion.map(|r| d.text.name(&d.religions[r].base.description).to_string()).unwrap_or_default();
                if success { format!("{r} spread ({magnitude}% chance)") } else { format!("{r} failed to spread ({magnitude}% chance)") }
            }
            CultureEffect::InciteUnrest => format!("+{magnitude} unhappiness for {} turns", self.culture_speed_turns(defines.culture_unrest_turns)),
        }
    }
}
