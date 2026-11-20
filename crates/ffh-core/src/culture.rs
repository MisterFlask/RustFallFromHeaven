//! Cultural channels (this game's own rules, not FFH2's; docs/CULTURE.md and docs/PERSONS.md,
//! "The fourth build").
//!
//! A civilization that can have Houses (`Game::player_has_houses`) has at most one cultural
//! channel (`Player::culture_channel`), a standing order until it changes it: **Court** a House of
//! another realm, **Guard** one of its own Houses, or **Spread Religion** among a civilization's
//! cities ([`ChannelOrder`]). While it runs, each of its cities diverts `CULTURE_CHANNEL_PERCENT`
//! of its culture rate, in hundredths, in its turn ([`Game::city_culture_diverted_times100`]): the
//! diverted culture reaches neither the city's culture (its level, borders, the culture victory)
//! nor its plots, and a city in disorder diverts nothing. What a turn's cities divert is the
//! channel's *inflow* (`Player::culture_inflow`, hundredths, kept for the turn).
//!
//! **Reach**: the effect falls with the distance from the caster's nearest city to the target's
//! nearest city ([`Game::culture_house_reach`]): in full within `CULTURE_CHANNEL_FULL_RANGE`, then
//! with the square of the distance, nothing beyond `CULTURE_CHANNEL_RANGE`
//! ([`Game::culture_distance_effect`]).
//!
//! **Court** and **Guard** act on the House's favour in its liege's turn (`persons/favour.rs`); a
//! Court channel leaves the liege a point of `MEMORY_COURTED_HOUSE` every
//! `CULTURE_COURT_MEMORY_TURNS` turns, up to 8 ([`Game::player_do_culture_channel`]). **Spread
//! Religion** rolls once a turn, in the caster's turn after its cities: the chance is the sum over
//! the target's cities in reach that could take the religion (each at its distance from the
//! caster's nearest city), capped; a success spreads the religion to one of them, picked by their
//! chances.
//!
//! One function checks an order ([`Game::culture_channel_check`], the refusal in the screen's
//! words) and one sets it ([`Game::player_set_culture_channel`]); in its owner's turn a channel
//! whose target is gone or out of reach, or a spread with nothing left to do, ends and its caster
//! is told why. Every channel is public to the civilizations that have met its caster
//! ([`Game::culture_channels`], [`Game::culture_channel_visible`]); its target (a House's liege,
//! for Court) is told when it starts and when it stops.
//!
//! The read API for the screens and the AI: [`Game::culture_channel_preview`] (what an order does:
//! its inflow, reach and effect), [`Game::culture_channel_options`] (the targets an action may
//! take, each with its one line and refusal), [`Game::culture_channels_between_others`]; the
//! courtships and guards of a House are in `persons/favour.rs`.

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::persons::{and_list, CourtshipView, GuardView};

/// What a channel is aimed at: a House (Court, Guard) or a civilization (Spread Religion).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChannelTarget {
    House(HouseId),
    Player(PlayerId),
}

/// An order for a civilization's channel: what [`Game::player_set_culture_channel`] takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelOrder {
    pub action: CulturalActionType,
    pub target: ChannelTarget,
    /// The religion spread (Spread Religion); `None` for Court and Guard.
    pub religion: Option<ReligionType>,
}

/// A civilization's standing cultural order (`Player::culture_channel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CultureChannel {
    pub action: CulturalActionType,
    pub target: ChannelTarget,
    pub religion: Option<ReligionType>,
    /// The game turn it was set.
    pub since_turn: i32,
}

impl CultureChannel {
    /// Its order.
    pub fn order(&self) -> ChannelOrder {
        ChannelOrder { action: self.action, target: self.target, religion: self.religion }
    }
}

/// How far a channel reaches: the plot distance between the nearest cities and its effect
/// (percent: [`Game::culture_distance_effect`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reach {
    pub distance: i32,
    pub effect: i32,
}

/// One of the target's cities as a spread reaches it ([`Game::culture_spread_reach`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelReach {
    pub city: CityId,
    /// Plots from the caster's nearest city.
    pub distance: i32,
    /// The distance effect, percent.
    pub effect: i32,
    /// The city's chance a turn, in hundredths of a percent (0 if it cannot take the religion).
    pub amount: i32,
}

/// A public channel ([`Game::culture_channels`]).
#[derive(Clone, Debug)]
pub struct Channel {
    pub caster: PlayerId,
    pub channel: CultureChannel,
}

/// What an order does ([`Game::culture_channel_preview`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelEffect {
    /// Court: the courtship of the House at the inflow now (its favour, the requirement and its
    /// parts, the turns to the split, the cities that would go and those the hold contest keeps).
    Court(CourtshipView),
    /// Guard: what it takes from each courtship of the House.
    Guard(GuardView),
    /// Spread Religion: the chance a turn (hundredths of a percent, capped) and the target's cities
    /// in reach, each with its share.
    Spread { chance: i32, cities: Vec<ChannelReach> },
    /// The order is refused (`ChannelPreview::refusal`) and nothing can be read of it.
    None,
}

/// An order previewed ([`Game::culture_channel_preview`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelPreview {
    pub order: ChannelOrder,
    /// Why it cannot be set now (`None`: it can, or it stands).
    pub refusal: Option<String>,
    /// The inflow it would have now, hundredths of culture a turn
    /// ([`Game::player_culture_inflow_now`]).
    pub inflow: i32,
    /// Court and Guard: from the caster's nearest city to the House's nearest; Spread Religion:
    /// to the target's nearest city.
    pub reach: Option<Reach>,
    pub effect: ChannelEffect,
    /// What it does, in a line: `Favour 0 of 188, a split in ~31 turns (Ventry, Corona).`
    pub line: String,
}

/// A target an action may take ([`Game::culture_channel_options`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelOption {
    pub target: ChannelTarget,
    /// `House Aldric (Bannor)`, `Sheaim`.
    pub label: String,
    /// What the order would do ([`ChannelPreview::line`]); empty when refused.
    pub line: String,
    /// Why it cannot be set (`None`: it can).
    pub refusal: Option<String>,
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // Reach
    // -----------------------------------------------------------------------------------------

    /// The distance effect (percent) of a channel `dist` plots away: in full within
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

    /// Player `p`'s cities that stand, in its order.
    fn culture_cities_of(&self, p: PlayerId) -> impl Iterator<Item = CityId> + '_ {
        self.players[p].cities.iter().copied().filter(|&c| self.cities.contains(c))
    }

    /// The plot distance from player `from`'s nearest city to city `t` (`None`: `from` has no
    /// city).
    pub fn culture_city_distance(&self, from: PlayerId, t: CityId) -> Option<i32> {
        let (x, y) = (self.cities[t].x, self.cities[t].y);
        self.culture_cities_of(from).map(|c| self.plot_distance(self.cities[c].x, self.cities[c].y, x, y)).min()
    }

    /// The reach from player `from`'s nearest city to the nearest of `to` (`None`: no city on
    /// either side).
    pub fn culture_reach_to(&self, from: PlayerId, to: &[CityId]) -> Option<Reach> {
        let distance = to.iter().filter_map(|&t| self.culture_city_distance(from, t)).min()?;
        Some(Reach { distance, effect: self.culture_distance_effect(distance) })
    }

    /// The reach from player `from`'s nearest city to House `h`'s nearest city.
    pub fn culture_house_reach(&self, from: PlayerId, h: HouseId) -> Option<Reach> {
        self.culture_reach_to(from, &self.house_cities(h))
    }

    // -----------------------------------------------------------------------------------------
    // The inflow
    // -----------------------------------------------------------------------------------------

    /// The culture city `c` diverts to its owner's channel in its turn, in hundredths:
    /// `CULTURE_CHANNEL_PERCENT` of its culture rate; nothing without a channel or in disorder.
    pub fn city_culture_diverted_times100(&self, c: CityId) -> i32 {
        let owner = self.cities[c].owner;
        if self.players[owner].culture_channel.is_none() {
            return 0;
        }
        self.city_culture_diverted_if_channelling(c)
    }

    /// What city `c` would divert to a channel of its owner's (hundredths; nothing in disorder).
    fn city_culture_diverted_if_channelling(&self, c: CityId) -> i32 {
        if self.city_is_disorder(c) {
            return 0;
        }
        self.city_commerce_rate_times100(c, CommerceType::COMMERCE_CULTURE).max(0) * self.d.defines.culture_channel_percent / 100
    }

    /// The inflow a channel of player `p` would have at its cities' culture now (hundredths a
    /// turn; pure): what the screens and the predictions read. The rules use what the cities
    /// diverted in their last turn (`Player::culture_inflow`).
    pub fn player_culture_inflow_now(&self, p: PlayerId) -> i32 {
        self.culture_cities_of(p).map(|c| self.city_culture_diverted_if_channelling(c)).sum()
    }

    // -----------------------------------------------------------------------------------------
    // The check, the setting
    // -----------------------------------------------------------------------------------------

    /// What `action` does.
    pub fn culture_effect(&self, action: CulturalActionType) -> CultureEffect {
        self.d.cultural_actions[action].effect
    }

    /// The cultural action with `effect`, if the data has one.
    pub fn culture_action(&self, effect: CultureEffect) -> Option<CulturalActionType> {
        self.d.cultural_actions.keys().find(|&a| self.d.cultural_actions[a].effect == effect)
    }

    /// The player a channel's target names: a House's liege (Court, Guard), the civilization
    /// (Spread Religion); `None` for a House that is gone.
    pub fn culture_target_player(&self, target: ChannelTarget) -> Option<PlayerId> {
        match target {
            ChannelTarget::House(h) => self.houses.get(h).map(|house| house.owner),
            ChannelTarget::Player(t) => Some(t),
        }
    }

    /// How a target reads: `House Aldric`, `the Sheaim`.
    pub fn culture_target_name(&self, target: ChannelTarget) -> String {
        match target {
            ChannelTarget::House(h) if self.houses.contains(h) => self.house_label(h),
            ChannelTarget::House(_) => "a House that is gone".into(),
            ChannelTarget::Player(t) => format!("the {}", self.player_get_civilization_short_description(t)),
        }
    }

    /// Why player `p` may not set channel `o` now, in the screen's words (`Ok`: it may). See the
    /// module comment and docs/PERSONS.md, "The fourth build", rules 1, 2, 5 and 6.
    pub fn culture_channel_check(&self, p: PlayerId, o: &ChannelOrder) -> Result<(), String> {
        if !self.player_has_houses(p) {
            return Err("Only a civilization that can have Houses channels its culture.".into());
        }
        if self.culture_cities_of(p).next().is_none() {
            return Err("You have no city.".into());
        }
        let defines = &self.d.defines;
        let effect = self.culture_effect(o.action);
        let team = self.players[p].team;
        match (effect, o.target) {
            (CultureEffect::Court | CultureEffect::Guard, ChannelTarget::House(h)) => {
                if o.religion.is_some() {
                    return Err("Court and Guard take no religion.".into());
                }
                let Some(house) = self.houses.get(h) else { return Err("That House is gone.".into()) };
                let label = self.house_label(h);
                if !self.players[house.player].alive || self.house_cities(h).is_empty() {
                    return Err(format!("{label} is gone."));
                }
                let (lt, ht) = (self.players[house.owner].team, self.players[house.player].team);
                if effect == CultureEffect::Court {
                    if team == lt || team == ht {
                        return Err(format!("{label} is of your realm."));
                    }
                    if !self.teams[team].has_met[ht] {
                        return Err(format!("You have not met {label}."));
                    }
                    if let Some(why) = self.team_vassal_refusal(team) {
                        return Err(why.into());
                    }
                } else if house.owner != p {
                    return Err(format!("{label} is not your House."));
                }
                match self.culture_house_reach(p, h) {
                    Some(r) if r.effect > 0 => {}
                    Some(r) => return Err(format!("{label} is out of reach: {} plots from your nearest city, {} at most.", r.distance, defines.culture_channel_range)),
                    None => return Err(format!("{label} is out of reach.")),
                }
            }
            (CultureEffect::SpreadReligion, ChannelTarget::Player(t)) => {
                let Some(r) = o.religion else { return Err("Choose a religion.".into()) };
                if !self.culture_cities_of(p).any(|c| self.cities[c].has_religion[r]) {
                    return Err("None of your cities has that religion.".into());
                }
                if !self.players.get(t).is_some_and(|pl| pl.alive) || self.culture_cities_of(t).next().is_none() {
                    return Err("That civilization is gone.".into());
                }
                let tt = self.players[t].team;
                if tt != team && t != self.barbarian_player() && !self.teams[team].has_met[tt] {
                    return Err("You have not met them.".into());
                }
                // Whether a city could take it, not the chance now: a turn without culture (in
                // disorder) does not end the channel.
                let reach = self.culture_spread_reach(p, t, r, 0);
                if reach.is_empty() {
                    return Err("None of their cities is within reach.".into());
                }
                if !reach.iter().any(|x| self.culture_can_take_religion(x.city, r, p)) {
                    return Err("None of their cities in reach can take it: they have it, or their civics admit only their state religion.".into());
                }
            }
            (CultureEffect::Court | CultureEffect::Guard, ChannelTarget::Player(_)) => return Err("Court and Guard take a House.".into()),
            (CultureEffect::SpreadReligion, ChannelTarget::House(_)) => return Err("Spread Religion takes a civilization.".into()),
        }
        Ok(())
    }

    /// Player `p` channels `order` from now on, or nothing (`None`): always its own decision (a
    /// governor never sets one). Setting the channel it already has keeps it (and its start turn).
    /// Free: favour already built fades on its own. The target is told when the channel starts
    /// and when it stops.
    pub fn player_set_culture_channel(&mut self, p: PlayerId, order: Option<ChannelOrder>) -> Result<(), String> {
        let current = self.players[p].culture_channel.as_ref().map(|ch| ch.order());
        if current == order {
            return Ok(());
        }
        if let Some(o) = &order {
            self.culture_channel_check(p, o)?;
        }
        if current.is_some() {
            self.player_culture_channel_end(p, None);
        }
        if let Some(o) = order {
            let turn = self.game_turn();
            self.players[p].culture_channel = Some(CultureChannel { action: o.action, target: o.target, religion: o.religion, since_turn: turn });
            self.culture_channel_started(p);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // The turn
    // -----------------------------------------------------------------------------------------

    /// Player `p`'s channel at the start of its cities' turns (`player_do_turn`): one that can no
    /// longer go on ends (its caster told why); the inflow counts from 0 (each city adds what it
    /// diverts: `city_do_turn`).
    pub fn player_culture_channel_turn_start(&mut self, p: PlayerId) {
        self.players[p].culture_inflow = 0;
        let Some(ch) = self.players[p].culture_channel.clone() else { return };
        if let Err(why) = self.culture_channel_check(p, &ch.order()) {
            self.player_culture_channel_end(p, Some(why));
        }
    }

    /// City `c` diverts its share to its owner's channel in its turn: added to the owner's
    /// inflow. Returns it (hundredths).
    pub(crate) fn city_divert_culture(&mut self, c: CityId) -> i32 {
        let diverted = self.city_culture_diverted_times100(c);
        let owner = self.cities[c].owner;
        self.players[owner].culture_inflow += diverted;
        diverted
    }

    /// Player `p`'s channel after its cities' turns: Spread Religion rolls (with this turn's
    /// inflow); a Court channel leaves the House's liege a point of `MEMORY_COURTED_HOUSE` every
    /// `CULTURE_COURT_MEMORY_TURNS` turns while that memory's count is below `CULTURE_MEMORY_MAX`
    /// (8). Court and Guard otherwise act in the liege's turn (`persons/favour.rs`).
    pub fn player_do_culture_channel(&mut self, p: PlayerId) {
        let Some(ch) = self.players[p].culture_channel.clone() else { return };
        match (self.culture_effect(ch.action), ch.target) {
            (CultureEffect::Court, ChannelTarget::House(h)) => {
                let Some(liege) = self.houses.get(h).map(|house| house.owner) else { return };
                let every = self.d.defines.culture_court_memory_turns.max(1);
                let turns = self.game_turn() - ch.since_turn;
                let cap = self.d.defines.culture_memory_max.max(1);
                if turns > 0 && turns % every == 0 && self.players[liege].ai_memory_count[p][MemoryType::MEMORY_COURTED_HOUSE] < cap {
                    self.player_add_memory(liege, p, MemoryType::MEMORY_COURTED_HOUSE);
                }
            }
            (CultureEffect::SpreadReligion, ChannelTarget::Player(t)) => {
                let Some(r) = ch.religion else { return };
                let inflow = self.players[p].culture_inflow;
                if inflow <= 0 {
                    return;
                }
                let reach = self.culture_spread_reach(p, t, r, inflow);
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
                    if let Some(x) = chosen {
                        self.culture_spread(p, &ch, x, r, chance);
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether `caster`'s channel `o` is hostile to its target (its target's player).
    pub fn culture_is_hostile(&self, caster: PlayerId, o: &ChannelOrder) -> bool {
        let Some(target) = self.culture_target_player(o.target) else { return false };
        if self.players[target].team == self.players[caster].team {
            return false;
        }
        match self.d.cultural_actions[o.action].hostile {
            CultureHostility::Never => false,
            CultureHostility::Always => true,
            CultureHostility::DifferentStateReligion => match self.player_get_state_religion(target) {
                Some(state) => Some(state) != o.religion,
                None => false,
            },
        }
    }

    // -----------------------------------------------------------------------------------------
    // Spread Religion
    // -----------------------------------------------------------------------------------------

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

    /// Each of `target`'s cities that `caster`'s Spread Religion of `r` reaches with `inflow`
    /// (hundredths a turn), in the target's city order: its distance from the caster's nearest
    /// city, the distance effect and its chance a turn (hundredths of a percent:
    /// `CULTURE_SPREAD_PER_CULTURE` a culture of inflow times the effect, divided by one plus the
    /// religions already there; 0 when it cannot take it). Cities out of reach are left out.
    pub fn culture_spread_reach(&self, caster: PlayerId, target: PlayerId, r: ReligionType, inflow: i32) -> Vec<ChannelReach> {
        let per = self.d.defines.culture_spread_per_culture;
        let mut out = Vec::new();
        for t in self.culture_cities_of(target) {
            let Some(distance) = self.culture_city_distance(caster, t) else { continue };
            let effect = self.culture_distance_effect(distance);
            if effect <= 0 {
                continue;
            }
            let amount = if self.culture_can_take_religion(t, r, caster) {
                let present = self.d.religions.keys().filter(|&o| self.cities[t].has_religion[o]).count() as i32;
                (inflow.max(0) as i64 * per as i64 * effect as i64 / 10000 / (1 + present) as i64) as i32
            } else {
                0
            };
            out.push(ChannelReach { city: t, distance, effect, amount });
        }
        out
    }

    /// Spread Religion's chance a turn (hundredths of a percent) from its reach: the cities'
    /// chances added up, at most `CULTURE_SPREAD_MAX`.
    pub fn culture_spread_chance(&self, reach: &[ChannelReach]) -> i32 {
        reach.iter().map(|r| r.amount).sum::<i32>().min(self.d.defines.culture_spread_max.max(0))
    }

    /// `of` remembers a point of cultural hostility (`slot` 0) or friendship (1) towards `towards`.
    fn culture_add_memory(&mut self, of: PlayerId, towards: PlayerId, slot: usize) {
        let cap = self.d.defines.culture_memory_max.max(1);
        let m = &mut self.players[of].culture_memory[towards][slot];
        *m = (*m + 1).min(cap);
    }

    /// Religion `r` spreads to `x.city` through `caster`'s channel (as a missionary's spread, with
    /// FfH's religion hooks): both sides are told, and the target's leader remembers it when it
    /// has a state religion (friendship for its own, hostility for another).
    fn culture_spread(&mut self, caster: PlayerId, ch: &CultureChannel, x: ChannelReach, r: ReligionType, chance: i32) {
        let d = self.d;
        let turn = self.game_turn();
        let t = x.city;
        let owner = self.cities[t].owner;
        self.city_set_has_religion(t, r, true, true, false);
        if !self.cities.contains(t) {
            return;
        }
        let religion = d.text.name(&d.religions[r].base.description).to_string();
        let target = d.text.name(&self.cities[t].name).to_string();
        let at = (self.cities[t].x, self.cities[t].y);
        let pct = format_chance(chance);
        let civ = self.player_get_civilization_adjective(owner);
        self.log.msg_at(turn, Some(caster), at, || format!("{religion} spread to {target} ({civ}) through our culture ({pct} a turn)."));
        let foreign = self.players[owner].team != self.players[caster].team;
        if foreign {
            let by = self.player_get_civilization_adjective(caster);
            self.log.msg_at(turn, Some(owner), at, || format!("{religion} spread to {target} through {by} culture ({pct} a turn)."));
            if let Some(state) = self.player_get_state_religion(owner) {
                self.culture_add_memory(owner, caster, if state == r { 1 } else { 0 });
            }
        }
        self.tel_culture(caster, "culture_spread", ch.action, owner, 1);
    }

    // -----------------------------------------------------------------------------------------
    // Start and end
    // -----------------------------------------------------------------------------------------

    /// The player who is told of `caster`'s channel `o` starting and stopping: a House's liege
    /// (Court; a Guard's liege is its caster, told nothing), the target civilization (Spread
    /// Religion, when foreign).
    fn culture_told(&self, caster: PlayerId, o: &ChannelOrder) -> Option<PlayerId> {
        let target = self.culture_target_player(o.target)?;
        (self.players[target].alive && self.players[target].team != self.players[caster].team).then_some(target)
    }

    /// Player `p`'s channel ends (`why`: it could no longer go on, its caster told why; `None`: its
    /// owner changed it). Its target is told.
    fn player_culture_channel_end(&mut self, p: PlayerId, why: Option<String>) {
        let Some(ch) = self.players[p].culture_channel.take() else { return };
        let turn = self.game_turn();
        let o = ch.order();
        let what = self.culture_channel_name(&o);
        if let Some(why) = why {
            self.log.msg(turn, Some(p), || format!("Channel ended: {what}. {why}"));
        }
        if let Some(t) = self.culture_told(p, &o) {
            let by = self.player_get_civilization_short_description(p);
            let text = match o.target {
                ChannelTarget::House(_) => format!("The {by} no longer {what_verb}.", what_verb = self.culture_channel_verb(&o)),
                ChannelTarget::Player(_) => format!("The {by} no longer channel {} at us.", self.culture_action_name(&o)),
            };
            self.log.msg(turn, Some(t), || text);
        }
        self.tel_culture(p, "culture_channel_end", ch.action, self.culture_target_tel(ch.target), 1);
    }

    /// Player `p`'s channel was just set: its target is told.
    fn culture_channel_started(&mut self, p: PlayerId) {
        let Some(ch) = self.players[p].culture_channel.clone() else { return };
        let o = ch.order();
        if let Some(t) = self.culture_told(p, &o) {
            let turn = self.game_turn();
            let by = self.player_get_civilization_short_description(p);
            let text = match o.target {
                ChannelTarget::House(_) => format!("The {by} {}.", self.culture_channel_verb(&o)),
                ChannelTarget::Player(_) => format!("The {by} channel their culture into {} at us.", self.culture_action_name(&o)),
            };
            self.log.msg(turn, Some(t), || text);
        }
        self.tel_culture(p, "culture_channel", ch.action, self.culture_target_tel(ch.target), 1);
    }

    /// The player telemetry names as a channel's target (a House's player).
    fn culture_target_tel(&self, target: ChannelTarget) -> PlayerId {
        match target {
            ChannelTarget::House(h) => self.houses.get(h).map_or(self.barbarian_player(), |house| house.player),
            ChannelTarget::Player(t) => t,
        }
    }

    /// `court House Aldric`, `guard House Aldric` (Court and Guard, after a civilization's name).
    fn culture_channel_verb(&self, o: &ChannelOrder) -> String {
        let target = self.culture_target_name(o.target);
        match self.culture_effect(o.action) {
            CultureEffect::Guard => format!("guard {target}"),
            _ => format!("court {target}"),
        }
    }

    /// `Court`, `Spread Religion (Octopus Overlords)`: the action and its religion, without the
    /// target.
    pub fn culture_action_name(&self, o: &ChannelOrder) -> String {
        let d = self.d;
        let action = d.text.name(&d.cultural_actions[o.action].base.description).to_string();
        match o.religion {
            Some(r) => format!("{action} ({})", d.text.name(&d.religions[r].base.description)),
            None => action,
        }
    }

    /// `Court House Aldric`, `Guard House Aldric`, `Spread Religion (Octopus Overlords) to the
    /// Sheaim`.
    pub fn culture_channel_name(&self, o: &ChannelOrder) -> String {
        let d = self.d;
        let action = d.text.name(&d.cultural_actions[o.action].base.description).to_string();
        let target = self.culture_target_name(o.target);
        match o.religion {
            Some(r) => format!("{action} ({}) to {target}", d.text.name(&d.religions[r].base.description)),
            None => format!("{action} {target}"),
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

    // -----------------------------------------------------------------------------------------
    // The read API
    // -----------------------------------------------------------------------------------------

    /// Every channel in the game, in caster order.
    pub fn culture_channels(&self) -> Vec<Channel> {
        self.player_ids().filter_map(|p| self.players[p].culture_channel.clone().filter(|_| self.players[p].alive).map(|channel| Channel { caster: p, channel })).collect()
    }

    /// Whether team `team` sees `caster`'s channels: its own, and those of civilizations it has
    /// met.
    pub fn culture_channel_visible(&self, team: TeamId, caster: PlayerId) -> bool {
        let ct = self.players[caster].team;
        ct == team || self.teams[team].has_met[ct]
    }

    /// The channels team `team` sees between others: visible, neither cast by its own team nor
    /// aimed at it (a House of its realm, or its own civilization), in caster order.
    pub fn culture_channels_between_others(&self, team: TeamId) -> Vec<Channel> {
        self.culture_channels()
            .into_iter()
            .filter(|ch| {
                let target = self.culture_target_player(ch.channel.target);
                self.players[ch.caster].team != team && self.culture_channel_visible(team, ch.caster) && target.is_none_or(|t| self.players[t].team != team)
            })
            .collect()
    }

    /// What order `o` of player `p` does at the inflow now (pure): its refusal, inflow, reach and
    /// effect (Court: the courtship of the House as it would run, `persons::CourtshipView`; Guard:
    /// what it takes from each courtship, `persons::GuardView`; Spread Religion: the chance a turn
    /// and the cities), with its one line.
    pub fn culture_channel_preview(&self, p: PlayerId, o: &ChannelOrder) -> ChannelPreview {
        let standing = self.players[p].culture_channel.as_ref().is_some_and(|ch| ch.order() == *o);
        let refusal = if standing { None } else { self.culture_channel_check(p, o).err() };
        let inflow = self.player_culture_inflow_now(p);
        let mut reach = None;
        let mut line = String::new();
        let effect = match (self.culture_effect(o.action), o.target) {
            (CultureEffect::Court, ChannelTarget::House(h)) if self.houses.contains(h) => {
                reach = self.culture_house_reach(p, h);
                let gain = (inflow as i64 * reach.map_or(0, |r| r.effect) as i64 / 100) as i32;
                let cv = self.house_courtship_at(h, p, gain);
                line = cv.summary.clone();
                ChannelEffect::Court(cv)
            }
            (CultureEffect::Guard, ChannelTarget::House(h)) if self.houses.contains(h) => {
                reach = self.culture_house_reach(p, h);
                let gv = self.house_guard_with(h, inflow);
                line = gv.line.clone();
                ChannelEffect::Guard(gv)
            }
            (CultureEffect::SpreadReligion, ChannelTarget::Player(t)) if self.players.get(t).is_some() => match o.religion {
                Some(r) => {
                    let cities = self.culture_spread_reach(p, t, r, inflow);
                    reach = self.culture_reach_to(p, &self.culture_cities_of(t).collect::<Vec<_>>());
                    let chance = self.culture_spread_chance(&cities);
                    let can = cities.iter().filter(|x| self.culture_can_take_religion(x.city, r, p)).count();
                    line = format!("{} a turn ({can} {} in reach can take it).", format_chance(chance), if can == 1 { "city" } else { "cities" });
                    ChannelEffect::Spread { chance, cities }
                }
                None => ChannelEffect::None,
            },
            _ => ChannelEffect::None,
        };
        ChannelPreview { order: *o, refusal, inflow, reach, effect, line }
    }

    /// The targets player `p`'s `action` may take (with `religion` for Spread Religion), each with
    /// its label, its one line and its refusal: Court, every House of another realm it has met;
    /// Guard, its own Houses; Spread Religion, every civilization it has met with a city, its own
    /// first. In id order.
    pub fn culture_channel_options(&self, p: PlayerId, action: CulturalActionType, religion: Option<ReligionType>) -> Vec<ChannelOption> {
        let team = self.players[p].team;
        let mut targets: Vec<ChannelTarget> = Vec::new();
        match self.culture_effect(action) {
            CultureEffect::Court => {
                for (h, house) in self.houses.iter() {
                    let (lt, ht) = (self.players[house.owner].team, self.players[house.player].team);
                    if lt != team && ht != team && self.teams[team].has_met[ht] && self.players[house.player].alive {
                        targets.push(ChannelTarget::House(h));
                    }
                }
            }
            CultureEffect::Guard => targets.extend(self.player_houses(p).into_iter().map(ChannelTarget::House)),
            CultureEffect::SpreadReligion => {
                targets.push(ChannelTarget::Player(p));
                for t in self.player_ids() {
                    let pl = &self.players[t];
                    if t != p && pl.alive && !pl.cities.is_empty() && !self.player_is_barbarian(t) && (pl.team == team || self.teams[team].has_met[pl.team]) {
                        targets.push(ChannelTarget::Player(t));
                    }
                }
            }
        }
        targets.sort();
        targets.dedup();
        targets
            .into_iter()
            .map(|target| {
                let o = ChannelOrder { action, target, religion };
                let preview = self.culture_channel_preview(p, &o);
                let label = match target {
                    ChannelTarget::House(h) => format!("{} ({})", self.house_label(h), self.player_get_civilization_short_description(self.houses[h].owner)),
                    ChannelTarget::Player(t) => self.player_get_civilization_short_description(t),
                };
                let line = if preview.refusal.is_some() { String::new() } else { preview.line };
                ChannelOption { target, label, line, refusal: preview.refusal }
            })
            .collect()
    }

    /// The names of cities `cities`, as a list: `Ventry, Corona and Astrum`.
    pub(crate) fn culture_city_names(&self, cities: &[CityId]) -> String {
        and_list(&cities.iter().map(|&c| self.d.text.name(&self.cities[c].name).to_string()).collect::<Vec<_>>())
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
