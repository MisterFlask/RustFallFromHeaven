//! The read API of Persons of Note: everything the Court screen, the map's name plates and the AI
//! read (pure, `&self`). Stable: the client's view code is written against it.
//!
//! - [`Game::person_view`]: one person (the Court screen's row and its details).
//! - [`Game::house_view`]: one House.
//! - [`Game::court_view`]: a player's cast, Houses and grantable cities.
//! - [`Game::city_house_mark`]: a city's House colour and state for the map and the lists (cheap).
//! - Previews: [`Game::grant_candidates`], [`Game::persons_preview`] (`preview.rs`).
//! - Quota: [`Game::house_orders_left`], [`Game::house_orders_total`],
//!   [`Game::city_house_takes_order`] (`houses.rs`).

use ffh_data::*;

use super::loyalty::Holder;
use super::*;

/// Where a person is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonWhere {
    /// With (in the group of) this army.
    Army(ArmyId),
    /// In this city (of any owner), with no army.
    City(CityId),
    /// On this plot, with no army and in no city.
    Plot(PlotIdx),
    /// Off the map (a great person's founder; a unit carried and not placed).
    Away,
}

/// One part of a loyalty sum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoyaltyPart {
    /// `Demand refused`, `Granted Torrin`, `State religion opposed`.
    pub label: String,
    pub amount: i32,
    /// Turns left (`None`: standing, as long as its cause lasts).
    pub turns_left: Option<i32>,
}

/// A person's standing demand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemandView {
    pub kind: DemandKind,
    /// `Torrin (Sheaim) taken and granted to House Ashledger.`
    pub text: String,
    pub turns_left: i32,
    /// Its term does not run this turn (a Works wonder in the seat's queue).
    pub paused: bool,
}

/// One person, for the Court screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonView {
    pub id: PersonId,
    pub owner: PlayerId,
    pub name: String,
    /// The minor leader's title (`The Auditor of Oaths`); `None` for a hero and for a leader whose
    /// text has none.
    pub title: Option<String>,
    /// The leader (the portrait); `None` for a hero.
    pub leader: Option<LeaderHeadType>,
    pub origin: PersonOrigin,
    pub unit: Option<UnitId>,
    pub unit_type: Option<UnitType>,
    /// Their unit's level (`None` without a unit).
    pub level: Option<i32>,
    pub location: PersonWhere,
    pub post: Post,
    pub ambition: Ambition,
    /// Their loyalty (a head's: their House's) and its parts.
    pub loyalty: i32,
    pub parts: Vec<LoyaltyPart>,
    pub state: LoyaltyState,
    /// The turns left of their (or their House's) Disloyal countdown.
    pub countdown_turns: Option<i32>,
    /// The countdown's stated outcome (`House Ashledger will secede with ...`).
    pub outcome: Option<String>,
    pub demand: Option<DemandView>,
    /// No demand before this many turns (after one was met or refused); 0 when none is due.
    pub demand_rest_turns: i32,
    pub rival: Option<PersonId>,
    pub rival_name: Option<String>,
    pub deeds: Vec<Deed>,
    /// The trait their House's cities would have ([`Game::person_house_trait`]).
    pub house_trait: Option<TraitType>,
    pub entered_turn: i32,
}

/// One House.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseView {
    pub id: HouseId,
    pub owner: PlayerId,
    /// `House Arne`.
    pub label: String,
    pub name: String,
    pub color: u8,
    pub head: Option<PersonId>,
    pub seat: Option<CityId>,
    pub cities: Vec<CityId>,
    pub founded_turn: i32,
    pub loyalty: i32,
    pub parts: Vec<LoyaltyPart>,
    pub state: LoyaltyState,
    pub countdown_turns: Option<i32>,
    /// The countdown's stated outcome (only while one runs).
    pub outcome: Option<String>,
    /// Orders left (`None`: no limit) and the Loyal total.
    pub orders_left: Option<i32>,
    pub orders_total: i32,
    /// Turns until the oldest order in the window frees (`None`: none placed).
    pub order_frees_in: Option<i32>,
    /// The seat is empty and its owner must name one of `heir_candidates`.
    pub needs_heir: bool,
    pub heir_candidates: Vec<PersonId>,
    /// The trait the House's cities have (its head's [`Game::person_house_trait`]).
    pub house_trait: Option<TraitType>,
}

/// A player's court.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourtView {
    pub player: PlayerId,
    /// Its persons, in cast order.
    pub persons: Vec<PersonView>,
    pub houses: Vec<HouseView>,
    /// The Crown's cities that can be granted now (empty when grants are refused).
    pub grantable: Vec<CityId>,
    /// Whether it can grant cities (else why not: direct rule, no Houses for this civilization).
    pub grants: Result<(), &'static str>,
    /// The persons it could still draw.
    pub pool_left: usize,
}

/// A city's House for the map and the lists ([`Game::city_house_mark`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseMark {
    pub house: HouseId,
    pub color: u8,
    pub state: LoyaltyState,
    pub countdown_turns: Option<i32>,
}

impl Game {
    /// The parts of a holder's loyalty, the running modifiers then the standing religion part.
    pub(crate) fn holder_parts(&self, hd: Holder) -> Vec<LoyaltyPart> {
        let turn = self.game_turn();
        let mut out: Vec<LoyaltyPart> = self
            .holder_standing(hd)
            .mods
            .iter()
            .filter(|m| m.until.map_or(true, |u| u > turn))
            .map(|m| LoyaltyPart { label: m.cause.label(), amount: m.amount, turns_left: m.until.map(|u| u - turn) })
            .collect();
        let religion = self.holder_religion_part(hd);
        if religion != 0 {
            out.push(LoyaltyPart { label: "State religion opposed".into(), amount: religion, turns_left: None });
        }
        out
    }

    fn holder_countdown_turns(&self, hd: Holder) -> Option<i32> {
        self.holder_standing(hd).countdown.map(|end| (end - self.game_turn()).max(0))
    }

    /// The trait person `x`'s House's cities would have (the rule only): the first of their
    /// leader's traits in authored order that their owner (its leader and civilization) does not
    /// have. `None` for a hero.
    pub fn person_house_trait(&self, x: PersonId) -> Option<TraitType> {
        let l = self.persons[x].leader?;
        let p = self.persons[x].owner;
        self.d.leader_heads[l].traits.iter().copied().find(|&t| !self.players[p].traits[t])
    }

    /// Where person `x` is.
    pub fn person_where(&self, x: PersonId) -> PersonWhere {
        let Some(u) = self.person_unit(x) else { return PersonWhere::Away };
        if let Some(a) = self.units[u].army.filter(|&a| self.armies.contains(a)) {
            return PersonWhere::Army(a);
        }
        // A hero grouped with an army is with it.
        if let Some(grp) = self.units[u].group {
            let owner = self.units[u].owner;
            if let Some(&a) = self.players[owner].armies.iter().find(|&&a| self.armies.get(a).is_some_and(|army| army.group == Some(grp))) {
                return PersonWhere::Army(a);
            }
        }
        match self.unit_plot_opt(u) {
            Some(plot) => match self.persons_city_at(plot) {
                Some(c) => PersonWhere::City(c),
                None => PersonWhere::Plot(plot),
            },
            None => PersonWhere::Away,
        }
    }

    /// Person `x`'s view (`None` when they are not in the cast).
    pub fn person_view(&self, x: PersonId) -> Option<PersonView> {
        let person = self.persons.get(x)?;
        let hd = self.person_holder(x);
        let unit = self.person_unit(x);
        let state = self.holder_state(hd);
        let countdown_turns = self.holder_countdown_turns(hd);
        let outcome = countdown_turns.map(|_| match hd {
            Holder::House(h) => self.house_secession_outcome(h),
            Holder::Person(_) => self.commander_secession_outcome(x),
        });
        let demand = person.demand.map(|dm| DemandView { kind: dm.kind, text: self.demand_kind_text(x, dm.kind), turns_left: (dm.deadline - self.game_turn()).max(0), paused: self.demand_is_paused(dm.kind) });
        let rival = person.rival.filter(|&r| self.persons.contains(r));
        Some(PersonView {
            id: x,
            owner: person.owner,
            name: person.name.clone(),
            title: person.leader.and_then(|l| entry::leader_title(self.d, l)),
            leader: person.leader,
            origin: person.origin,
            unit,
            unit_type: unit.map(|u| self.units[u].unit_type),
            level: unit.map(|u| self.units[u].level),
            location: self.person_where(x),
            post: person.post,
            ambition: person.ambition,
            loyalty: self.holder_loyalty(hd),
            parts: self.holder_parts(hd),
            state,
            countdown_turns,
            outcome,
            demand,
            demand_rest_turns: if person.demand.is_none() { (person.demand_rest_until - self.game_turn()).max(0) } else { 0 },
            rival,
            rival_name: rival.map(|r| self.persons[r].name.clone()),
            deeds: person.deeds.clone(),
            house_trait: self.person_house_trait(x),
            entered_turn: person.entered_turn,
        })
    }

    /// House `h`'s view (`None` when it does not stand).
    pub fn house_view(&self, h: HouseId) -> Option<HouseView> {
        let house = self.houses.get(h)?;
        let hd = Holder::House(h);
        let countdown_turns = self.holder_countdown_turns(hd);
        let head = house.head.filter(|&x| self.persons.contains(x));
        let candidates = if head.is_none() { self.player_heir_candidates(house.owner) } else { Vec::new() };
        Some(HouseView {
            id: h,
            owner: house.owner,
            label: self.house_label(h),
            name: house.name.clone(),
            color: house.color,
            head,
            seat: house.seat,
            cities: house.cities.clone(),
            founded_turn: house.founded_turn,
            loyalty: self.holder_loyalty(hd),
            parts: self.holder_parts(hd),
            state: self.holder_state(hd),
            countdown_turns,
            outcome: countdown_turns.map(|_| self.house_secession_outcome(h)),
            orders_left: self.house_orders_left(h),
            orders_total: self.house_orders_total(h),
            order_frees_in: self.house_order_frees_in(h),
            needs_heir: head.is_none() && candidates.len() > 1,
            heir_candidates: candidates,
            house_trait: head.and_then(|x| self.person_house_trait(x)),
        })
    }

    /// Player `p`'s court: its persons, its Houses, the Crown's cities it can grant.
    pub fn court_view(&self, p: PlayerId) -> CourtView {
        let grants = self.player_check_grants(p);
        let grantable = if grants.is_ok() { self.players[p].cities.iter().copied().filter(|&c| self.player_check_grant_city(p, c).is_ok()).collect() } else { Vec::new() };
        CourtView {
            player: p,
            persons: self.players[p].persons.iter().filter_map(|&x| self.person_view(x)).collect(),
            houses: self.players[p].houses.iter().filter_map(|&h| self.house_view(h)).collect(),
            grantable,
            grants,
            pool_left: if self.player_has_persons(p) { self.persons_pool(p).len() } else { 0 },
        }
    }

    /// City `c`'s House for the map and the lists: its colour, state and countdown; `None` for a
    /// Crown city. Cheap (no scan).
    pub fn city_house_mark(&self, c: CityId) -> Option<HouseMark> {
        let h = self.cities.get(c)?.house.filter(|&h| self.houses.contains(h))?;
        let hd = Holder::House(h);
        Some(HouseMark { house: h, color: self.houses[h].color, state: self.holder_state(hd), countdown_turns: self.holder_countdown_turns(hd) })
    }

    /// The House holding city `c` (`None`: the Crown's).
    pub fn city_house(&self, c: CityId) -> Option<HouseId> {
        self.cities.get(c)?.house.filter(|&h| self.houses.contains(h))
    }
}
