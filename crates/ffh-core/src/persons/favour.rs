//! Courting by culture (docs/PERSONS.md, "The fourth build", rules 2 to 5 and 7): a House's
//! favour toward the civilizations that court it, the requirement, the split toward a courter and
//! the guard.
//!
//! - **Favour** (`House::favour`, in courter order, hundredths of culture). In its liege's turn,
//!   with the other House rules ([`Game::player_persons_favour`], last), each courter's favour gains
//!   the courter's inflow (what its cities diverted in their last turn, `Player::culture_inflow`)
//!   times its reach (from the courter's nearest city to the House's nearest city) while its Court
//!   channel runs at the House, loses the guard's (the liege's Guard channel at the House: its
//!   inflow times its reach), then fades `CULTURE_FAVOUR_FADE_PERCENT` (at least a hundredth), at
//!   least 0 ([`Game::favour_step`]). A courter whose channel stopped keeps its favour while it
//!   fades; favour goes when its courter dies or joins the House's team or its liege's, and with
//!   the House.
//! - **The requirement** ([`Game::house_requirement`]): `(L + 6 − 2A, at least 1) × W`: L the
//!   House's loyalty to its liege, A its leader's attitude toward the courter (Furious −2 to
//!   Friendly +2), W the world culture unit (`world_unit.rs`).
//! - **The split** ([`Game::house_courted_split`]): when a courter's favour reaches the requirement
//!   (the first in courter order, one a turn), the half of the House's cities nearest the courter
//!   (never its capital; a House of one city whole: [`Game::house_split_toward`]) go to the
//!   courter, in peace: a new House of the courter's realm of the House's civilization, its leader
//!   drawn from the free minor leaders (none free: no split this turn), or the House itself when it
//!   goes whole. The hold contest keeps a city where the liege's army units outnumber the House's
//!   own; when it keeps every city, the favour is spent all the same. The liege remembers it
//!   (`MEMORY_SPLIT_HOUSE`); the courter's favour there is 0 after.
//! - **Notices**: when a courter's favour first reaches two thirds of the requirement the liege and
//!   the courter are told (`warned`: told again only after it fell below half).
//!
//! The read API: [`Game::house_courtships`] (every courtship of a House: favour, requirement and
//! its parts, where it settles, the turns to the split, the cities that would go and those the
//! hold contest keeps), [`Game::house_courtship_at`] (a courtship at a given gain: the previews),
//! [`Game::house_guard_view`] and [`Game::house_guard_with`] (a guard's effect on each courter),
//! [`Game::house_courting_line`] (the courter's line in its trade table with the House).

use ffh_data::*;

use super::*;
use crate::culture::{ChannelTarget, Reach};
use crate::log::Audience;

/// A House's favour toward one courter (`House::favour`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Favour {
    pub courter: PlayerId,
    /// Hundredths of culture.
    pub amount: i32,
    /// The liege and the courter were told it passed two thirds of the requirement (cleared when
    /// it falls below half).
    pub warned: bool,
}

/// The requirement of a courtship ([`Game::house_requirement`]): `(L + 6 − 2A, at least 1) × W`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    /// Culture (favour reaches it at `100 × total` hundredths).
    pub total: i32,
    /// L: the House's loyalty to its liege (a contract's floor included).
    pub loyalty: i32,
    /// A: its leader's attitude toward the courter, −2 (Furious) to +2 (Friendly).
    pub attitude: i32,
    /// W: the world culture unit.
    pub unit: i32,
}

/// The cities a split toward a courter takes ([`Game::house_split_toward`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitView {
    /// The House has one city: it goes whole (it becomes the courter's House).
    pub whole: bool,
    /// The cities that go (the half nearest the courter, those the hold contest keeps left out), in
    /// the order of their distance to the courter.
    pub going: Vec<CityId>,
    /// The cities of that half the liege's armies hold (they stay with the House).
    pub held: Vec<CityId>,
}

/// One courtship of a House ([`Game::house_courtships`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourtshipView {
    pub house: HouseId,
    pub courter: PlayerId,
    /// Its favour now, hundredths of culture.
    pub favour: i32,
    /// What its Court channel adds a turn at the inflow now (hundredths; 0 without one at the
    /// House), and its reach.
    pub gain: i32,
    pub reach: Option<Reach>,
    /// What a guard takes from it a turn (hundredths).
    pub guard: i32,
    pub requirement: Requirement,
    /// Where its favour settles at these inflows (hundredths).
    pub settles: i32,
    /// The liege turns until the split at these inflows (`None`: never).
    pub turns: Option<i32>,
    /// What the split would take now (`None`: the House has no city).
    pub split: Option<SplitView>,
    /// Its favour is past two thirds of the requirement (a line that needs attention).
    pub attention: bool,
    /// `Favour 80 of 188, a split in ~12 turns (Ventry, Corona).`
    pub summary: String,
    /// `Courted by the Sheaim: favour 80 of 188, a split in ~12 turns (Ventry, Corona).`
    pub line: String,
}

/// A guard's effect on one courtship ([`GuardView::courtships`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuardedCourtship {
    pub courter: PlayerId,
    /// Hundredths.
    pub favour: i32,
    pub gain: i32,
    /// Where it settles with the guard and without it (hundredths), and the turns to the split
    /// each way (`None`: never).
    pub settles_with: i32,
    pub settles_without: i32,
    pub turns_with: Option<i32>,
    pub turns_without: Option<i32>,
}

/// A guard of a House ([`Game::house_guard_view`], [`Game::house_guard_with`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardView {
    pub house: HouseId,
    /// The House's liege, whose channel guards it.
    pub guard: PlayerId,
    pub reach: Option<Reach>,
    /// The guard's inflow and what it takes from each courtship a turn (hundredths).
    pub inflow: i32,
    pub loss: i32,
    pub courtships: Vec<GuardedCourtship>,
    /// `Guarded: −6 a turn to each courtship.`
    pub line: String,
}

/// The step of an attitude on the requirement's scale: Furious −2, Annoyed −1, Cautious 0,
/// Pleased +1, Friendly +2.
pub fn attitude_step(a: AttitudeType) -> i32 {
    match a {
        AttitudeType::ATTITUDE_FURIOUS => -2,
        AttitudeType::ATTITUDE_ANNOYED => -1,
        AttitudeType::ATTITUDE_PLEASED => 1,
        AttitudeType::ATTITUDE_FRIENDLY => 2,
        _ => 0,
    }
}

/// The most turns a prediction runs (favour that has not reached the requirement by then never
/// does at these inflows).
const PREDICTION_TURNS: i32 = 1000;

impl Game {
    // -----------------------------------------------------------------------------------------
    // The numbers
    // -----------------------------------------------------------------------------------------

    /// The requirement of `courter`'s courtship of House `h` now (pure): `(L + 6 − 2A, at least
    /// 1) × W` ([`Requirement`]).
    pub fn house_requirement(&self, h: HouseId, courter: PlayerId) -> Requirement {
        let loyalty = self.house_loyalty(h);
        let attitude = attitude_step(self.player_ai_get_attitude(self.houses[h].player, courter, true));
        let unit = self.world_culture_unit();
        Requirement { total: (loyalty + 6 - 2 * attitude).max(1) * unit, loyalty, attitude, unit }
    }

    /// A turn of favour `amount` (hundredths) under `gain` and a guard's `guard` (hundredths a
    /// turn): `amount + gain − guard`, at least 0, then less `CULTURE_FAVOUR_FADE_PERCENT` of it (at
    /// least a hundredth), at least 0. The rules and the predictions share it.
    pub fn favour_step(&self, amount: i32, gain: i32, guard: i32) -> i32 {
        let f = (amount + gain - guard).max(0);
        if f == 0 {
            return 0;
        }
        let fade = (f as i64 * self.d.defines.culture_favour_fade_percent.max(0) as i64 / 100).max(1) as i32;
        (f - fade).max(0)
    }

    /// The liege turns from favour `amount` until it reaches `requirement` (culture) under a steady
    /// `gain` and `guard` (`None`: it never does).
    pub fn favour_turns(&self, amount: i32, gain: i32, guard: i32, requirement: i32) -> Option<i32> {
        let need = 100 * requirement;
        let mut f = amount;
        for t in 1..=PREDICTION_TURNS {
            let next = self.favour_step(f, gain, guard);
            if next >= need {
                return Some(t);
            }
            if next <= f && t > 1 {
                return None;
            }
            f = next;
        }
        None
    }

    /// Where favour settles under a steady `gain` and `guard` (hundredths), from 0.
    pub fn favour_settles(&self, gain: i32, guard: i32) -> i32 {
        let mut f = 0;
        for _ in 0..PREDICTION_TURNS {
            let next = self.favour_step(f, gain, guard);
            if next == f {
                break;
            }
            f = next;
        }
        f
    }

    /// House `h`'s favour toward `courter` now (hundredths).
    pub fn house_favour(&self, h: HouseId, courter: PlayerId) -> i32 {
        self.houses.get(h).and_then(|house| house.favour.iter().find(|f| f.courter == courter)).map_or(0, |f| f.amount)
    }

    /// Whether player `q`'s channel is `effect` at House `h`.
    fn channel_at(&self, q: PlayerId, h: HouseId, effect: CultureEffect) -> bool {
        self.players[q].culture_channel.as_ref().is_some_and(|ch| ch.target == ChannelTarget::House(h) && self.culture_effect(ch.action) == effect)
    }

    /// What `inflow` (hundredths a turn) of player `q` comes to at House `h`: times the reach from
    /// `q`'s nearest city to the House's nearest city.
    fn inflow_at(&self, q: PlayerId, h: HouseId, inflow: i32) -> i32 {
        let effect = self.culture_house_reach(q, h).map_or(0, |r| r.effect);
        (inflow as i64 * effect as i64 / 100) as i32
    }

    /// What `courter`'s Court channel adds to House `h`'s favour a turn: in the rules (`live`
    /// false) the inflow of its cities' last turn, in the predictions (`live`) the inflow at its
    /// cities' culture now; 0 without a Court channel of its at the House.
    pub fn house_court_gain(&self, h: HouseId, courter: PlayerId, live: bool) -> i32 {
        if !self.channel_at(courter, h, CultureEffect::Court) {
            return 0;
        }
        let inflow = if live { self.player_culture_inflow_now(courter) } else { self.players[courter].culture_inflow };
        self.inflow_at(courter, h, inflow)
    }

    /// What the guard of House `h` (its liege's Guard channel at it) takes from each courtship a
    /// turn (hundredths; `live` as [`Game::house_court_gain`]); 0 without one.
    pub fn house_guard_loss(&self, h: HouseId, live: bool) -> i32 {
        let Some(house) = self.houses.get(h) else { return 0 };
        let liege = house.owner;
        if !self.channel_at(liege, h, CultureEffect::Guard) {
            return 0;
        }
        let inflow = if live { self.player_culture_inflow_now(liege) } else { self.players[liege].culture_inflow };
        self.inflow_at(liege, h, inflow)
    }

    // -----------------------------------------------------------------------------------------
    // The split's cities
    // -----------------------------------------------------------------------------------------

    /// What a split of House `h` toward `courter` would take now (pure): the half of its cities
    /// nearest the courter (plot distance to the courter's nearest city; ties: the lower city id),
    /// rounded down, at least one, never its capital; a House of one city whole. The hold contest
    /// sorts them into those that go and those its liege's armies hold (`Game::city_held_by_army`).
    /// `None` when it has no city.
    pub fn house_split_toward(&self, h: HouseId, courter: PlayerId) -> Option<SplitView> {
        let cities = self.house_cities(h);
        if cities.is_empty() {
            return None;
        }
        let whole = cities.len() == 1;
        let half: Vec<CityId> = if whole {
            cities
        } else {
            let seat = self.house_seat(h);
            let n = (cities.len() / 2).max(1);
            let mut rest: Vec<(i32, CityId)> = cities.into_iter().filter(|&c| Some(c) != seat).map(|c| (self.culture_city_distance(courter, c).unwrap_or(i32::MAX), c)).collect();
            rest.sort();
            rest.into_iter().take(n).map(|(_, c)| c).collect()
        };
        let (held, going): (Vec<CityId>, Vec<CityId>) = half.into_iter().partition(|&c| self.city_held_by_army(c));
        Some(SplitView { whole, going, held })
    }

    // -----------------------------------------------------------------------------------------
    // The read
    // -----------------------------------------------------------------------------------------

    /// The civilizations that court House `h`: those it keeps a favour toward and those whose
    /// Court channel runs at it, in player order.
    pub fn house_courters(&self, h: HouseId) -> Vec<PlayerId> {
        let Some(house) = self.houses.get(h) else { return Vec::new() };
        let mut out: Vec<PlayerId> = house.favour.iter().map(|f| f.courter).collect();
        for q in self.player_ids() {
            if self.players[q].alive && self.channel_at(q, h, CultureEffect::Court) && !out.contains(&q) {
                out.push(q);
            }
        }
        out.sort();
        out
    }

    /// `courter`'s courtship of House `h` at a gain of `gain` a turn (hundredths), the guard as it
    /// runs at the inflow now (pure): the previews read it with the gain an order would have.
    pub fn house_courtship_at(&self, h: HouseId, courter: PlayerId, gain: i32) -> CourtshipView {
        let favour = self.house_favour(h, courter);
        let guard = self.house_guard_loss(h, true);
        let requirement = self.house_requirement(h, courter);
        let settles = self.favour_settles(gain, guard);
        let turns = if 100 * requirement.total <= favour { Some(1) } else { self.favour_turns(favour, gain, guard, requirement.total) };
        let split = self.house_split_toward(h, courter);
        let attention = 3 * favour >= 2 * 100 * requirement.total;
        let summary = self.courtship_summary(favour, gain, requirement.total, settles, turns, split.as_ref());
        let by = self.player_get_civilization_short_description(courter);
        let line = format!("Courted by the {by}: {}{}", summary[..1].to_lowercase(), &summary[1..]);
        CourtshipView { house: h, courter, favour, gain, reach: self.culture_house_reach(courter, h), guard, requirement, settles, turns, split, attention, summary, line }
    }

    /// `courter`'s courtship of House `h` as it runs (its Court channel's gain at the inflow now).
    pub fn house_courtship_view(&self, h: HouseId, courter: PlayerId) -> CourtshipView {
        self.house_courtship_at(h, courter, self.house_court_gain(h, courter, true))
    }

    /// Every courtship of House `h` ([`Game::house_courters`]), in player order.
    pub fn house_courtships(&self, h: HouseId) -> Vec<CourtshipView> {
        self.house_courters(h).into_iter().map(|q| self.house_courtship_view(h, q)).collect()
    }

    /// `Favour 80 of 188, a split in ~12 turns (Ventry, Corona).` (`settling near 120`, `fading`;
    /// the cities held named after `held:`).
    fn courtship_summary(&self, favour: i32, gain: i32, requirement: i32, settles: i32, turns: Option<i32>, split: Option<&SplitView>) -> String {
        let mut s = format!("Favour {} of {requirement}", favour / 100);
        match turns {
            Some(1) => s.push_str(", a split next turn"),
            Some(t) => s.push_str(&format!(", a split in ~{t} turns")),
            None if gain > 0 => s.push_str(&format!(", settling near {}", settles / 100)),
            None => s.push_str(", fading"),
        }
        if let Some(sp) = split {
            let going = self.culture_city_names(&sp.going);
            let held = self.culture_city_names(&sp.held);
            match (sp.going.is_empty(), sp.held.is_empty()) {
                (false, true) => s.push_str(&format!(" ({going})")),
                (false, false) => s.push_str(&format!(" ({going}; held: {held})")),
                (true, false) => s.push_str(&format!(" (held: {held})")),
                (true, true) => {}
            }
        }
        s.push('.');
        s
    }

    /// The guard of House `h` at a guard inflow of `inflow` (hundredths a turn): what it takes from
    /// each courtship, and where each settles with it and without it (pure; the previews).
    pub fn house_guard_with(&self, h: HouseId, inflow: i32) -> GuardView {
        let liege = self.houses[h].owner;
        let loss = self.inflow_at(liege, h, inflow);
        let courtships = self
            .house_courters(h)
            .into_iter()
            .map(|q| {
                let favour = self.house_favour(h, q);
                let gain = self.house_court_gain(h, q, true);
                let req = self.house_requirement(h, q).total;
                GuardedCourtship {
                    courter: q,
                    favour,
                    gain,
                    settles_with: self.favour_settles(gain, loss),
                    settles_without: self.favour_settles(gain, 0),
                    turns_with: self.favour_turns(favour, gain, loss, req),
                    turns_without: self.favour_turns(favour, gain, 0, req),
                }
            })
            .collect();
        GuardView { house: h, guard: liege, reach: self.culture_house_reach(liege, h), inflow, loss, courtships, line: format!("Guarded: \u{2212}{} a turn to each courtship.", loss / 100) }
    }

    /// The guard of House `h`, while its liege's Guard channel runs at it (at the inflow now).
    pub fn house_guard_view(&self, h: HouseId) -> Option<GuardView> {
        let liege = self.houses.get(h)?.owner;
        self.channel_at(liege, h, CultureEffect::Guard).then(|| self.house_guard_with(h, self.player_culture_inflow_now(liege)))
    }

    /// The courter's line in its trade table with House `h`: `Your courtship of House Aldric:
    /// favour 80 of 188, a split in ~12 turns (Ventry, Corona).`; `None` while `giver` does not
    /// court it.
    pub fn house_courting_line(&self, h: HouseId, giver: PlayerId) -> Option<String> {
        if !self.house_courters(h).contains(&giver) {
            return None;
        }
        let cv = self.house_courtship_view(h, giver);
        Some(format!("Your courtship of {}: {}{}", self.house_label(h), cv.summary[..1].to_lowercase(), &cv.summary[1..]))
    }

    // -----------------------------------------------------------------------------------------
    // The liege's turn
    // -----------------------------------------------------------------------------------------

    /// Player `p`'s Houses' favour in its turn processing (last of the House rules): each House in
    /// id order ([`Game::house_favour_turn`]).
    pub(crate) fn player_persons_favour(&mut self, p: PlayerId) {
        let mut houses = self.player_houses(p);
        houses.sort();
        for h in houses {
            if self.houses.contains(h) {
                self.house_favour_turn(h);
            }
        }
    }

    /// House `h`'s favour in its liege's turn: each courter's (in courter order) gains its Court
    /// channel's inflow times its reach, loses the guard's, fades; the notices at two thirds; the
    /// split toward the first courter at the requirement.
    pub(crate) fn house_favour_turn(&mut self, h: HouseId) {
        let (liege, hp) = (self.houses[h].owner, self.houses[h].player);
        let (lt, ht) = (self.players[liege].team, self.players[hp].team);
        let guard = self.house_guard_loss(h, false);
        let mut list: Vec<Favour> = Vec::new();
        let mut before: Vec<i32> = Vec::new();
        for q in self.house_courters(h) {
            let qt = self.players[q].team;
            if !self.players[q].alive || qt == ht || qt == lt {
                continue;
            }
            let old = self.houses[h].favour.iter().find(|f| f.courter == q).copied().unwrap_or(Favour { courter: q, amount: 0, warned: false });
            let gain = self.house_court_gain(h, q, false);
            let amount = self.favour_step(old.amount, gain, guard);
            if amount == 0 && gain == 0 {
                continue;
            }
            list.push(Favour { courter: q, amount, warned: old.warned });
            before.push(old.amount);
        }
        self.houses[h].favour = list;
        let label = self.house_label(h);
        let mut due: Option<(PlayerId, bool)> = None;
        for i in 0..self.houses[h].favour.len() {
            let Favour { courter, amount, warned } = self.houses[h].favour[i];
            let need = 100 * self.house_requirement(h, courter).total;
            if amount >= need {
                if due.is_none() {
                    due = Some((courter, before[i] < need));
                }
                continue;
            }
            if 3 * amount >= 2 * need && !warned {
                self.houses[h].favour[i].warned = true;
                let (by, f, r) = (self.player_get_civilization_short_description(courter), amount / 100, need / 100);
                self.house_msg(h, || format!("{label}: the {by}'s favour is {f} of {r}."));
                self.courter_msg(courter, h, format!("{label}: your favour is {f} of {r}."));
            } else if 2 * amount < need && warned {
                self.houses[h].favour[i].warned = false;
            }
        }
        if let Some((courter, first)) = due {
            self.house_courted_split(h, courter, first);
        }
    }

    /// A message to `courter` about House `h` (at the House's seat, marked as the Houses' are).
    fn courter_msg(&mut self, courter: PlayerId, h: HouseId, text: String) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        match self.house_seat(h).map(|c| (self.cities[c].x, self.cities[c].y)) {
            Some(at) => self.log.msg_marked(turn, Some(courter), at, LogMark::House, LogTag::None, || text),
            None => self.log.msg(turn, Some(courter), || text),
        }
    }

    /// The players who met the liege `liege` or the courter `courter`, but them (the third
    /// person's news of a split).
    fn split_audience(&self, liege: PlayerId, courter: PlayerId) -> Audience {
        let (lt, ct) = (self.players[liege].team, self.players[courter].team);
        self.audience_where(|q| q != liege && q != courter && self.players[q].team != lt && self.players[q].team != ct && (self.player_knows_team(q, lt) || self.player_knows_team(q, ct)))
    }

    /// The hold contest for city `c` of the half a split takes, as the liege reads it, when a unit
    /// of an army of the liege counts in it: `Corona stays with House Aldric: your army's 4 units
    /// outnumber its 2.`, or `Corona goes to the Sheaim: its 3 units against your army's 2.`
    fn split_hold_line(&self, c: CityId, house: &str, by: &str) -> Option<String> {
        let hold = self.city_hold(c)?;
        if hold.liege == 0 {
            return None;
        }
        let city = self.persons_city_name(c);
        let units = |n: i32| format!("{n} {}", if n == 1 { "unit" } else { "units" });
        let yours = if hold.armies > 1 { "your armies'" } else { "your army's" };
        Some(if hold.held() {
            let verb = if hold.liege == 1 { "outnumbers" } else { "outnumber" };
            format!("{city} stays with {house}: {yours} {} {verb} its {}.", units(hold.liege), hold.house)
        } else {
            format!("{city} goes to the {by}: its {} against {yours} {}.", units(hold.house), hold.liege)
        })
    }

    /// `courter`'s favour at House `h` reached the requirement in the liege's turn: the split
    /// (docs/PERSONS.md, "The fourth build", rule 4). `first`: the favour reached it this turn (a
    /// split put off for want of a leader is told once). Returns the House the cities went to (the
    /// new House, or the House itself when it went whole).
    pub(crate) fn house_courted_split(&mut self, h: HouseId, courter: PlayerId, first: bool) -> Option<HouseId> {
        let split = self.house_split_toward(h, courter)?;
        let (liege, hp) = (self.houses[h].owner, self.houses[h].player);
        let label = self.house_label(h);
        let by = self.player_get_civilization_short_description(courter);
        let liege_name = self.player_get_civilization_short_description(liege);
        let turn = self.game_turn();
        // The contests the liege's armies stood in, told before the cities change hands.
        let contest: Vec<((i32, i32), String)> =
            split.going.iter().chain(&split.held).filter_map(|&c| self.split_hold_line(c, &label, &by).map(|l| ((self.cities[c].x, self.cities[c].y), l))).collect();
        let tell_contest = |g: &mut Game, contest: Vec<((i32, i32), String)>| {
            for (at, line) in contest {
                g.log.msg_marked(turn, Some(liege), at, LogMark::House, LogTag::None, || line);
            }
        };
        if split.going.is_empty() {
            // Every city of the half held: the favour is spent all the same.
            let held = self.culture_city_names(&split.held);
            self.house_msg(h, || format!("The {by}'s courtship of {label} fails: your armies hold {held}."));
            self.courter_msg(courter, h, format!("Your courtship of {label} fails: the {liege_name}'s armies hold {held}."));
            tell_contest(self, contest);
            self.house_favour_reset(h, courter);
            self.tel_house(liege, "split", "courted_held");
            return None;
        }
        let names = self.culture_city_names(&split.going);
        let verb = if split.going.len() == 1 { "joins" } else { "join" };
        let to = if split.whole {
            if self.team_vassal_refusal(self.players[courter].team).is_some() {
                return None;
            }
            tell_contest(self, contest);
            let nh = self.house_join_whole(h, courter)?;
            let others = self.split_audience(liege, courter);
            self.log.msg(turn, Some(liege), || format!("The {by}'s courtship takes {label}: {names} {verb} them."));
            self.courter_msg(courter, nh, format!("{label} joins you with {names}: a House of your realm."));
            self.log.msg_to(turn, others, || format!("{label} leaves the {liege_name} for the {by} with {names}."));
            self.house_add_deed(nh, format!("Courted away from the {liege_name} by the {by}, with {names}."));
            self.tel_house(liege, "split", "courted_whole");
            nh
        } else {
            let Some(civ) = self.player_get_civilization_type(hp) else { return None };
            let Some(leader) = self.house_leader_draw(civ, courter) else {
                if first {
                    self.house_msg(h, || format!("{label} would split toward the {by}: no leader is free."));
                    self.courter_msg(courter, h, format!("{label} would split toward you: no leader is free."));
                }
                self.tel_house(liege, "split", "courted_no_leader");
                return None;
            };
            tell_contest(self, contest);
            let mut cities = split.going.clone();
            cities.sort_by_key(|&c| (std::cmp::Reverse(self.cities[c].population), c));
            let (nh, _) = self.player_found_house_as(courter, civ, leader, &cities, HouseOrigin::Courted)?;
            let (new, led) = (self.house_label(nh), leader_name(self.d, leader));
            let others = self.split_audience(liege, courter);
            self.house_add_deed(nh, format!("Split from {label} of the {liege_name}, led by {led}, with {names}."));
            if self.houses.contains(h) {
                self.house_add_deed(h, format!("Split: {names} went to the {by} as {new}."));
            }
            let text = format!("The {by}'s courtship splits {label}: {names} {verb} them as {new}, led by {led}.");
            if self.houses.contains(h) {
                self.house_msg(h, || text);
            } else {
                self.log.msg(turn, Some(liege), || text);
            }
            self.courter_msg(courter, nh, format!("{label} splits toward you: {names} {verb} you as {new}, led by {led}."));
            self.log.msg_to(turn, others, || format!("The {by}'s courtship splits {label} of the {liege_name}: {names} {verb} the {by} as {new}, led by {led}."));
            self.tel_house(liege, "split", "courted");
            nh
        };
        self.player_add_memory(liege, courter, MemoryType::MEMORY_SPLIT_HOUSE);
        if self.houses.contains(h) {
            self.house_favour_reset(h, courter);
        }
        // Units of the liege left in a city that changed hands go where they may stand.
        self.map_verify_unit_valid_plot();
        Some(to)
    }

    /// `courter`'s favour at House `h` is 0 (a split took its cities, or the hold contest spent it).
    fn house_favour_reset(&mut self, h: HouseId, courter: PlayerId) {
        if let Some(f) = self.houses.get_mut(h).and_then(|house| house.favour.iter_mut().find(|f| f.courter == courter)) {
            f.amount = 0;
            f.warned = false;
        }
    }

    /// A House of one city goes whole to `courter` (rule 4): its record ends with its liege (the
    /// vassalage ends at peace), it makes peace with the wars it had as its liege's House that the
    /// courter does not share, and it becomes the courter's vassal, a House of its realm (origin
    /// `Courted`, its name kept, no modifier, its tribute the realm's default). The record it
    /// becomes.
    fn house_join_whole(&mut self, h: HouseId, courter: PlayerId) -> Option<HouseId> {
        let (liege, hp) = (self.houses[h].owner, self.houses[h].player);
        let name = self.houses[h].name.clone();
        let (ht, lt, ct) = (self.players[hp].team, self.players[liege].team, self.players[courter].team);
        self.team_end_vassalage(ht, lt);
        if self.houses.contains(h) {
            self.house_record_end(h);
        }
        let barbarians = self.players[self.barbarian_player()].team;
        for o in self.team_ids() {
            if o != ht && o != ct && o != barbarians && self.team_is_alive(o) && self.teams[ht].at_war[o] && !self.teams[ct].at_war[o] {
                self.team_make_peace(ht, o, true);
            }
        }
        if !self.teams[ht].vassal[ct] {
            self.team_assign_vassal(ct, ht, false);
        }
        let nh = self.player_house(hp).filter(|&nh| self.houses[nh].owner == courter)?;
        let house = &mut self.houses[nh];
        house.origin = HouseOrigin::Courted;
        house.name = name;
        house.standing.mods.retain(|m| m.cause != LoyaltyCause::SoughtProtection);
        let state = self.house_state(nh);
        self.houses[nh].standing.state = state;
        Some(nh)
    }
}
