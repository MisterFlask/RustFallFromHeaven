//! Cultural channels (this game's own rules, not FFH2's; `GAMEOPTION_CULTURE_ACTIONS`;
//! docs/CULTURE.md).
//!
//! A city may channel `CULTURE_CHANNEL_PERCENT` of its culture into one standing order aimed at
//! a civilization, until its owner changes it ([`CultureChannel`]): fomenting revolt among its
//! people, or spreading one of the city's religions among its cities. The channelled culture
//! reaches neither the city's culture, nor its plots, nor the culture victory. Its effect on each
//! of the target's cities falls with the square of the distance ([`Game::culture_distance_effect`]).
//!
//! **Foment Revolt** adds revolt pressure (`City::culture_pressure`, per caster, in hundredths of
//! culture) to each of the target's cities in reach, every turn of the caster's city. In its own
//! turn a city resists it ([`Game::city_revolt_resistance`]: part of its own culture, its garrison)
//! and sheds `CULTURE_REVOLT_FADE_PERCENT` of it (at least a culture); what is left brings
//! unhappiness, then disorder
//! (a revolt: the occupation timer), and a caster whose pressure reaches `CULTURE_REVOLT_FLIP`
//! takes the city, as a culture revolt that flips it. Not an act of war; the target's leader
//! remembers it (`Player::culture_memory`).
//!
//! **Spread Religion** rolls once a turn for the target: the chance is the sum over its cities in
//! reach that could take the religion ([`Game::culture_channel_reach`]), capped; a success spreads
//! the religion to one of them, picked by their chances.
//!
//! Every channel is public to the civilizations that have met its caster
//! ([`Game::culture_channels`]).

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;

/// A city's standing cultural order (`City::culture_channel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureChannel {
    pub action: CulturalActionType,
    /// The civilization it is aimed at.
    pub target: PlayerId,
    /// The religion spread (Spread Religion); `None` for Foment Revolt.
    pub religion: Option<ReligionType>,
    /// The game turn it was set.
    pub since_turn: i32,
}

/// Why a channel cannot be set, or why one ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CultureRefusal {
    /// `GAMEOPTION_CULTURE_ACTIONS` is off.
    Off,
    /// The barbarians channel nothing.
    Barbarian,
    /// The target civilization is gone.
    Dead,
    /// The caster's team has not met the target.
    NotMet,
    /// Foment Revolt aimed at the caster's own team.
    OwnTeam,
    /// Spread Religion: no religion chosen, or the city lacks it.
    Religion(&'static str),
    /// None of the target's cities is within reach.
    NoneInReach,
    /// Spread Religion: none of the target's cities in reach could take the religion.
    NoneCanTake,
}

impl CultureRefusal {
    /// The reason for the player, in a sentence fragment.
    pub fn describe(&self) -> String {
        match self {
            CultureRefusal::Off => "cultural actions are off in this game".into(),
            CultureRefusal::Barbarian => "the barbarians channel nothing".into(),
            CultureRefusal::Dead => "that civilization is gone".into(),
            CultureRefusal::NotMet => "we have not met them".into(),
            CultureRefusal::OwnTeam => "not against our own team".into(),
            CultureRefusal::Religion(why) => (*why).into(),
            CultureRefusal::NoneInReach => "none of their cities is within reach".into(),
            CultureRefusal::NoneCanTake => "none of their cities in reach can take it (they have it, or their civics admit only their state religion)".into(),
        }
    }
}

/// One of the target's cities as a channel reaches it ([`Game::culture_channel_reach`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelReach {
    pub city: CityId,
    /// Plots from the channelling city.
    pub distance: i32,
    /// The distance effect, percent.
    pub effect: i32,
    /// Foment Revolt: the revolt pressure a turn it adds, in hundredths of culture. Spread
    /// Religion: the city's chance a turn, in hundredths of a percent (0 if it cannot take it).
    pub amount: i32,
}

/// A public channel ([`Game::culture_channels`]).
#[derive(Clone, Debug)]
pub struct Channel {
    pub city: CityId,
    pub caster: PlayerId,
    pub channel: CultureChannel,
}

/// How far revolt pressure has gone in a city (`City::culture_revolt_level`).
pub const REVOLT_CALM: i32 = 0;
/// Unhappiness.
pub const REVOLT_UNREST: i32 = 1;
/// Disorder.
pub const REVOLT_REVOLT: i32 = 2;
/// A caster's pressure is two thirds of the way to taking the city.
pub const REVOLT_NEAR_FLIP: i32 = 3;

impl Game {
    /// Whether this game has cultural channels.
    #[inline]
    pub fn culture_actions_on(&self) -> bool {
        self.is_option(GameOptionType::GAMEOPTION_CULTURE_ACTIONS)
    }

    /// The distance effect (percent) of a channel on a city `dist` plots away: in full within
    /// `CULTURE_CHANNEL_FULL_RANGE`, then falling with the square of the distance, nothing beyond
    /// `CULTURE_CHANNEL_RANGE`.
    pub fn culture_distance_effect(&self, dist: i32) -> i32 {
        let defines = &self.d.defines;
        if dist > defines.culture_channel_range {
            return 0;
        }
        let full = defines.culture_channel_full_range.max(1);
        if dist <= full {
            return 100;
        }
        100 * full * full / (dist * dist)
    }

    /// The culture a turn city `c` would channel now: `CULTURE_CHANNEL_PERCENT` of its culture
    /// rate, rounded down.
    pub fn city_channel_amount(&self, c: CityId) -> i32 {
        self.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE).max(0) * self.d.defines.culture_channel_percent / 100
    }

    /// Whether `caster`'s `action` with `religion` aimed at `target` is hostile to it.
    pub fn culture_is_hostile(&self, action: CulturalActionType, caster: PlayerId, target: PlayerId, religion: Option<ReligionType>) -> bool {
        if self.players[target].team == self.players[caster].team {
            return false;
        }
        match self.d.cultural_actions[action].hostile {
            CultureHostility::Never => false,
            CultureHostility::Always => true,
            CultureHostility::DifferentStateReligion => match self.player_get_state_religion(target) {
                Some(state) => Some(state) != religion,
                None => false,
            },
        }
    }

    /// Land units of city `c`'s team on its plot able to defend it: each resists revolt pressure.
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

    /// Whether city `t` could take religion `r` from a channel: it lacks it, and its owner's civics
    /// admit it (a foreign owner allowing only its state religion takes only that one).
    pub fn culture_can_take_religion(&self, t: CityId, r: ReligionType, caster: PlayerId) -> bool {
        let owner = self.cities[t].owner;
        if self.cities[t].has_religion[r] {
            return false;
        }
        let own_team = self.players[owner].team == self.players[caster].team;
        own_team || !self.player_is_no_non_state_religion_spread(owner) || self.player_get_state_religion(owner) == Some(r)
    }

    /// Each of `target`'s cities that city `c` would reach with `action` (and `religion`): its
    /// distance, the distance effect and what the channel would do there a turn, in city id order.
    /// Cities out of reach are left out.
    pub fn culture_channel_reach(&self, c: CityId, action: CulturalActionType, target: PlayerId, religion: Option<ReligionType>) -> Vec<ChannelReach> {
        let defines = &self.d.defines;
        let caster = self.cities[c].owner;
        let amount = self.city_channel_amount(c);
        let (cx, cy) = (self.cities[c].x, self.cities[c].y);
        let mut out = Vec::new();
        for &t in &self.players[target].cities {
            if !self.cities.contains(t) || t == c {
                continue;
            }
            let distance = self.plot_distance(cx, cy, self.cities[t].x, self.cities[t].y);
            let effect = self.culture_distance_effect(distance);
            if effect <= 0 {
                continue;
            }
            let per_turn = match self.d.cultural_actions[action].effect {
                CultureEffect::FomentRevolt => amount * effect,
                CultureEffect::SpreadReligion => match religion {
                    Some(r) if self.culture_can_take_religion(t, r, caster) => {
                        let present = self.d.religions.keys().filter(|&o| self.cities[t].has_religion[o]).count() as i32;
                        amount * defines.culture_spread_per_culture * effect / 100 / (1 + present)
                    }
                    _ => 0,
                },
            };
            out.push(ChannelReach { city: t, distance, effect, amount: per_turn });
        }
        out
    }

    /// Spread Religion's chance a turn (hundredths of a percent) from its reach: the cities'
    /// chances added up, at most `CULTURE_SPREAD_MAX`.
    pub fn culture_spread_chance(&self, reach: &[ChannelReach]) -> i32 {
        reach.iter().map(|r| r.amount).sum::<i32>().min(self.d.defines.culture_spread_max.max(0))
    }

    /// Whether city `c` may channel `action` (with `religion`) into `target` now.
    pub fn culture_channel_check(&self, c: CityId, action: CulturalActionType, target: PlayerId, religion: Option<ReligionType>) -> Result<(), CultureRefusal> {
        if !self.culture_actions_on() {
            return Err(CultureRefusal::Off);
        }
        let caster = self.cities[c].owner;
        if caster == self.barbarian_player() {
            return Err(CultureRefusal::Barbarian);
        }
        if !self.players[target].alive || self.players[target].cities.is_empty() {
            return Err(CultureRefusal::Dead);
        }
        let (ct, tt) = (self.players[caster].team, self.players[target].team);
        if ct != tt && target != self.barbarian_player() && !self.teams[ct].has_met[tt] {
            return Err(CultureRefusal::NotMet);
        }
        match self.d.cultural_actions[action].effect {
            CultureEffect::FomentRevolt => {
                if ct == tt {
                    return Err(CultureRefusal::OwnTeam);
                }
                if self.culture_channel_reach(c, action, target, None).is_empty() {
                    return Err(CultureRefusal::NoneInReach);
                }
            }
            CultureEffect::SpreadReligion => {
                let Some(r) = religion else {
                    return Err(CultureRefusal::Religion("no religion chosen"));
                };
                if !self.cities[c].has_religion[r] {
                    return Err(CultureRefusal::Religion("the city lacks that religion"));
                }
                // Whether a city could take it, not the chance now: a turn without culture (in
                // disorder) does not end the channel.
                let reach = self.culture_channel_reach(c, action, target, religion);
                if reach.is_empty() {
                    return Err(CultureRefusal::NoneInReach);
                }
                if !reach.iter().any(|x| self.culture_can_take_religion(x.city, r, caster)) {
                    return Err(CultureRefusal::NoneCanTake);
                }
            }
        }
        Ok(())
    }

    /// City `c` channels `order` (action, target, religion) from now on, or nothing (`None`):
    /// always its owner's decision (a governor never sets one). Setting the channel it already has
    /// keeps it (and its start turn).
    pub fn city_set_culture_channel(&mut self, c: CityId, order: Option<(CulturalActionType, PlayerId, Option<ReligionType>)>) -> Result<(), CultureRefusal> {
        let current = self.cities[c].culture_channel.as_ref().map(|ch| (ch.action, ch.target, ch.religion));
        if current == order {
            return Ok(());
        }
        if let Some((action, target, religion)) = order {
            self.culture_channel_check(c, action, target, religion)?;
        }
        if current.is_some() {
            self.culture_channel_end(c, None);
        }
        if let Some((action, target, religion)) = order {
            let turn = self.game_turn();
            self.cities[c].culture_channel = Some(CultureChannel { action, target, religion, since_turn: turn });
            self.culture_channel_started(c);
        }
        Ok(())
    }

    /// City `c`'s channel this turn (its turn, before its culture is added): it diverts its
    /// culture, and Foment Revolt adds pressure to the target's cities, Spread Religion rolls. A
    /// channel with nothing left to do ends. Sets `culture_diverted`.
    pub fn city_do_culture_channel(&mut self, c: CityId) {
        self.cities[c].culture_diverted = 0;
        let Some(ch) = self.cities[c].culture_channel.clone() else { return };
        if let Err(why) = self.culture_channel_check(c, ch.action, ch.target, ch.religion) {
            self.culture_channel_end(c, Some(why));
            return;
        }
        let amount = self.city_channel_amount(c);
        self.cities[c].culture_diverted = amount;
        if amount <= 0 {
            return;
        }
        let caster = self.cities[c].owner;
        let reach = self.culture_channel_reach(c, ch.action, ch.target, ch.religion);
        match self.d.cultural_actions[ch.action].effect {
            CultureEffect::FomentRevolt => {
                for x in &reach {
                    self.cities[x.city].culture_pressure[caster] += x.amount;
                }
                self.culture_remember(c, &ch);
            }
            CultureEffect::SpreadReligion => {
                let chance = self.culture_spread_chance(&reach);
                if chance > 0 && self.soren_rand_num(10000, "Cultural Spread Religion") < chance {
                    let total: i32 = reach.iter().map(|x| x.amount).sum();
                    let mut pick = self.soren_rand_num(total.max(1), "Cultural Spread Religion city");
                    let mut chosen = None;
                    for x in reach.iter().filter(|x| x.amount > 0) {
                        if pick < x.amount {
                            chosen = Some(*x);
                            break;
                        }
                        pick -= x.amount;
                    }
                    if let (Some(x), Some(r)) = (chosen, ch.religion) {
                        self.culture_spread(c, &ch, x, r, chance);
                    }
                }
            }
        }
    }

    /// A channel's hostile turn: every `CULTURE_REVOLT_MEMORY_TURNS` turns of it, the target's
    /// leader remembers a point of cultural hostility.
    fn culture_remember(&mut self, c: CityId, ch: &CultureChannel) {
        let caster = self.cities[c].owner;
        if !self.culture_is_hostile(ch.action, caster, ch.target, ch.religion) {
            return;
        }
        let every = self.d.defines.culture_revolt_memory_turns.max(1);
        let turns = self.game_turn() - ch.since_turn;
        if turns > 0 && turns % every == 0 {
            self.culture_add_memory(ch.target, caster, 0);
        }
    }

    /// `of` remembers a point of cultural hostility (`slot` 0) or friendship (1) towards `towards`.
    fn culture_add_memory(&mut self, of: PlayerId, towards: PlayerId, slot: usize) {
        let cap = self.d.defines.culture_memory_max.max(1);
        let m = &mut self.players[of].culture_memory[towards][slot];
        *m = (*m + 1).min(cap);
    }

    /// Religion `r` spreads to `x.city` through city `c`'s channel (as a missionary's spread, with
    /// FfH's religion hooks): both sides are told, and the target's leader remembers it when it
    /// has a state religion (friendship for its own, hostility for another).
    fn culture_spread(&mut self, c: CityId, ch: &CultureChannel, x: ChannelReach, r: ReligionType, chance: i32) {
        let d = self.d;
        let turn = self.game_turn();
        let caster = self.cities[c].owner;
        let t = x.city;
        let owner = self.cities[t].owner;
        self.city_set_has_religion(t, r, true, true, false);
        if !self.cities.contains(t) {
            return;
        }
        let religion = d.text.name(&d.religions[r].base.description).to_string();
        let city = d.text.name(&self.cities[c].name).to_string();
        let target = d.text.name(&self.cities[t].name).to_string();
        let at = (self.cities[t].x, self.cities[t].y);
        let pct = format_chance(chance);
        let civ = self.player_get_civilization_adjective(owner);
        self.log.msg_at(turn, Some(caster), at, || format!("{religion} spread to {target} ({civ}) through {city}'s culture ({pct} a turn)."));
        let foreign = self.players[owner].team != self.players[caster].team;
        if foreign {
            let by = self.player_get_civilization_adjective(caster);
            self.log.msg_at(turn, Some(owner), at, || format!("{religion} spread to {target} through the culture of {city} ({by})."));
            if let Some(state) = self.player_get_state_religion(owner) {
                self.culture_add_memory(owner, caster, if state == r { 1 } else { 0 });
            }
        }
        self.tel_culture(caster, "culture_spread", ch.action, ch.target, 1);
    }

    /// City `c`'s channel ends (`why`: it could no longer go on; `None`: its owner changed it).
    fn culture_channel_end(&mut self, c: CityId, why: Option<CultureRefusal>) {
        let Some(ch) = self.cities[c].culture_channel.take() else { return };
        self.cities[c].culture_diverted = 0;
        let caster = self.cities[c].owner;
        if let Some(why) = why {
            let turn = self.game_turn();
            let city = self.d.text.name(&self.cities[c].name).to_string();
            let what = self.culture_channel_name(&ch);
            let reason = why.describe();
            self.log.msg_at(turn, Some(caster), (self.cities[c].x, self.cities[c].y), || format!("{city} stops channelling {what}: {reason}."));
        }
        if self.culture_is_hostile(ch.action, caster, ch.target, ch.religion) && self.players[ch.target].alive {
            let turn = self.game_turn();
            let city = self.d.text.name(&self.cities[c].name).to_string();
            let civ = self.player_get_civilization_adjective(caster);
            let what = self.d.text.name(&self.d.cultural_actions[ch.action].base.description).to_string();
            self.log.msg(turn, Some(ch.target), || format!("{city} ({civ}) no longer channels {what} against us."));
        }
        self.tel_culture(caster, "culture_channel_end", ch.action, ch.target, 1);
    }

    /// City `c`'s channel was just set: the target is told when it is hostile.
    fn culture_channel_started(&mut self, c: CityId) {
        let Some(ch) = self.cities[c].culture_channel.clone() else { return };
        let caster = self.cities[c].owner;
        if self.culture_is_hostile(ch.action, caster, ch.target, ch.religion) {
            let turn = self.game_turn();
            let city = self.d.text.name(&self.cities[c].name).to_string();
            let civ = self.player_get_civilization_adjective(caster);
            let what = self.culture_channel_name(&ch);
            self.log.msg(turn, Some(ch.target), || format!("{city} ({civ}) channels its culture into {what} against us."));
        }
        self.tel_culture(caster, "culture_channel", ch.action, ch.target, 1);
    }

    /// "Foment Revolt", "Spread Religion (Buddhism)".
    pub fn culture_channel_name(&self, ch: &CultureChannel) -> String {
        let d = self.d;
        let action = d.text.name(&d.cultural_actions[ch.action].base.description).to_string();
        match ch.religion {
            Some(r) => format!("{action} ({})", d.text.name(&d.religions[r].base.description)),
            None => action,
        }
    }

    /// The revolt pressure city `c` resists each turn, in hundredths of culture:
    /// `CULTURE_REVOLT_RESIST_PERCENT` of its own culture rate and `CULTURE_REVOLT_GARRISON` per
    /// defender.
    pub fn city_revolt_resistance(&self, c: CityId) -> i32 {
        let defines = &self.d.defines;
        let own = self.city_commerce_rate(c, CommerceType::COMMERCE_CULTURE).max(0) * defines.culture_revolt_resist_percent;
        own + 100 * defines.culture_revolt_garrison * self.culture_garrison_count(c)
    }

    /// The revolt pressure on city `c` from every caster, in hundredths of culture.
    pub fn city_revolt_pressure(&self, c: CityId) -> i32 {
        self.cities[c].culture_pressure.iter().sum()
    }

    /// The caster with the most revolt pressure on city `c` (the lowest id on ties) and its
    /// pressure, if any.
    pub fn city_revolt_leader(&self, c: CityId) -> Option<(PlayerId, i32)> {
        let mut best: Option<(PlayerId, i32)> = None;
        for p in self.player_ids() {
            let v = self.cities[c].culture_pressure[p];
            if v > 0 && best.map_or(true, |(_, b)| v > b) {
                best = Some((p, v));
            }
        }
        best
    }

    /// How far revolt pressure `total` (and the leader's `lead`, hundredths) goes: the
    /// `REVOLT_*` levels.
    pub fn culture_revolt_level_of(&self, total: i32, lead: i32) -> i32 {
        let defines = &self.d.defines;
        if lead * 3 >= 100 * defines.culture_revolt_flip * 2 {
            REVOLT_NEAR_FLIP
        } else if total >= 100 * defines.culture_revolt_disorder {
            REVOLT_REVOLT
        } else if total >= 100 * defines.culture_revolt_unhappy_step.max(1) {
            REVOLT_UNREST
        } else {
            REVOLT_CALM
        }
    }

    /// Unhappiness from revolt pressure in city `c`: a point per `CULTURE_REVOLT_UNHAPPY_STEP`,
    /// at most `CULTURE_REVOLT_UNHAPPY_MAX`.
    pub fn city_culture_unhappiness(&self, c: CityId) -> i32 {
        let defines = &self.d.defines;
        let total = self.city_revolt_pressure(c);
        (total / (100 * defines.culture_revolt_unhappy_step.max(1))).min(defines.culture_revolt_unhappy_max.max(0))
    }

    /// City `c`'s revolt pressure in its turn: each caster's resisted and faded; a revolt
    /// (disorder) while the total is at `CULTURE_REVOLT_DISORDER`; the city joins a caster whose
    /// pressure reaches `CULTURE_REVOLT_FLIP` (not a capital, not with
    /// `GAMEOPTION_NO_CITY_FLIPPING`). The owner and the leading caster hear of each change.
    pub fn city_do_revolt_pressure(&mut self, c: CityId) {
        if self.city_revolt_pressure(c) <= 0 && self.cities[c].culture_revolt_level == REVOLT_CALM {
            return;
        }
        let d = self.d;
        let defines = &d.defines;
        let resist = self.city_revolt_resistance(c);
        let fade = defines.culture_revolt_fade_percent.max(0);
        let before_unhappy = self.city_culture_unhappiness(c);
        for p in self.player_ids() {
            let v = self.cities[c].culture_pressure[p];
            if v > 0 {
                // At least a culture a turn fades, so the last of it goes too.
                self.cities[c].culture_pressure[p] = (v - resist - (v * fade / 100).max(100)).max(0);
            }
        }
        // A caster that is gone or now of the city's team presses no more.
        let team = self.city_team(c);
        for p in self.player_ids() {
            if self.cities[c].culture_pressure[p] > 0 && (!self.players[p].alive || self.players[p].team == team) {
                self.cities[c].culture_pressure[p] = 0;
            }
        }
        let total = self.city_revolt_pressure(c);
        let leader = self.city_revolt_leader(c);
        if self.city_culture_unhappiness(c) != before_unhappy {
            self.cities[c].ai_assign_work_dirty = true;
        }
        if total >= 100 * defines.culture_revolt_disorder && self.cities[c].occupation_timer < 1 {
            self.city_set_occupation_timer(c, 1);
        }
        let level = self.culture_revolt_level_of(total, leader.map_or(0, |l| l.1));
        let old = self.cities[c].culture_revolt_level;
        self.cities[c].culture_revolt_level = level;
        if level != old {
            self.culture_revolt_news(c, old, level, leader.map(|l| l.0));
        }
        if let Some((p, v)) = leader {
            let flip_ok = !self.city_is_capital(c) && !self.is_option(GameOptionType::GAMEOPTION_NO_CITY_FLIPPING);
            if flip_ok && v >= 100 * defines.culture_revolt_flip {
                self.culture_flip(c, p);
            }
        }
    }

    /// City `c`'s revolt level changed from `old` to `level`: its owner and the leading caster
    /// hear of it.
    fn culture_revolt_news(&mut self, c: CityId, old: i32, level: i32, leader: Option<PlayerId>) {
        let turn = self.game_turn();
        let owner = self.cities[c].owner;
        let name = self.d.text.name(&self.cities[c].name).to_string();
        let at = (self.cities[c].x, self.cities[c].y);
        let by = leader.map(|p| self.player_get_civilization_adjective(p)).unwrap_or_default();
        let (to_owner, to_caster) = match level {
            REVOLT_CALM => (format!("The unrest in {name} has passed."), format!("{name} is calm again.")),
            REVOLT_UNREST if level > old => (format!("{by} culture stirs unrest in {name}."), format!("Our culture stirs unrest in {name}.")),
            REVOLT_UNREST => (format!("{name} is no longer in revolt, though still restless."), format!("{name} is no longer in revolt.")),
            REVOLT_REVOLT if level > old => (format!("{name} is in revolt, stirred by {by} culture."), format!("{name} is in revolt, stirred by our culture.")),
            REVOLT_REVOLT => (format!("{name} is no longer about to join the {by}."), format!("{name} is no longer about to join us.")),
            _ => (format!("{name} is close to joining the {by}! A garrison and our own culture resist it."), format!("{name} is close to joining us.")),
        };
        self.log.msg_at(turn, Some(owner), at, || to_owner);
        if let Some(p) = leader.filter(|&p| p != owner) {
            self.log.msg_at(turn, Some(p), at, || to_caster);
            let tag = match level {
                REVOLT_CALM => "calm",
                REVOLT_UNREST => "unrest",
                REVOLT_REVOLT => "revolt",
                _ => "near_flip",
            };
            self.tel_revolt(p, owner, c, tag);
        }
    }

    /// City `t` joins `caster` as in a culture revolt that flips the city (`CvPlot::doCulture`):
    /// barbarian units on the plot move away, defenders lose half their hit points, then the plot
    /// (and with it the city) changes hands; in a one-city challenge a human caster's flip
    /// destroys the city instead.
    fn culture_flip(&mut self, t: CityId, caster: PlayerId) {
        let turn = self.game_turn();
        let owner = self.cities[t].owner;
        let name = self.d.text.name(&self.cities[t].name).to_string();
        let at = (self.cities[t].x, self.cities[t].y);
        let by = self.player_get_civilization_adjective(caster);
        self.log.msg_at(turn, Some(owner), at, || format!("{name} has joined the {by}, won by their culture."));
        self.log.msg_at(turn, Some(caster), at, || format!("{name} has joined us, won by our culture."));
        self.tel_revolt(caster, owner, t, "flip");
        let plot = self.city_plot(t);
        for u in self.map.plots[plot].units.clone() {
            if !self.units.contains(u) {
                continue;
            }
            if self.unit_is_barbarian(u) {
                self.unit_jump_to_nearest_valid_plot(u);
            } else if self.unit_can_defend(u, None) {
                let damage = self.unit_curr_hit_points(u) / 2;
                self.unit_change_damage(u, damage, Some(caster));
            }
        }
        if !self.cities.contains(t) {
            return;
        }
        if self.is_option(GameOptionType::GAMEOPTION_ONE_CITY_CHALLENGE) && self.players[caster].human {
            self.city_kill(t, true);
        } else {
            self.plot_set_owner(plot, Some(caster), true, true);
        }
    }

    /// City `c` is going (destroyed, or captured: the capturer gets a new city): its channel ends.
    /// (The pressure on it goes with it; channels aimed at its owner go on.)
    pub fn city_culture_lost(&mut self, c: CityId) {
        if self.cities[c].culture_channel.is_some() {
            let caster = self.cities[c].owner;
            if let Some(ch) = self.cities[c].culture_channel.take() {
                self.tel_culture(caster, "culture_channel_end", ch.action, ch.target, 1);
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

    /// Every channel in the game, in city id order.
    pub fn culture_channels(&self) -> Vec<Channel> {
        self.cities.iter().filter_map(|(c, city)| city.culture_channel.clone().map(|channel| Channel { city: c, caster: city.owner, channel })).collect()
    }

    /// Whether team `team` sees `caster`'s channels: its own, and those of civilizations it has
    /// met.
    pub fn culture_channel_visible(&self, team: TeamId, caster: PlayerId) -> bool {
        let ct = self.players[caster].team;
        ct == team || self.teams[team].has_met[ct]
    }
}

/// A chance in hundredths of a percent as the player reads it: "12%", "0.5%".
pub fn format_chance(bp: i32) -> String {
    if bp >= 1000 || bp % 100 == 0 {
        format!("{}%", (bp + 50) / 100)
    } else {
        format!("{}.{}%", bp / 100, (bp % 100) / 10)
    }
}
