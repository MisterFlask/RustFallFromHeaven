//! The read API of Persons of Note: everything the Court screen, the map's name plates, the
//! diplomacy and Armies screens and the AI read (pure, `&self`). The client's view code is written
//! against it.
//!
//! - [`Game::person_view`]: one person (the Court screen's row and its details).
//! - [`Game::house_view`]: one House: its player, leader, civilization, cities, attitude and its
//!   parts, tribute, levy, countdown and outcome, freeing, its price to secede.
//! - [`Game::court_view`]: a player's cast, Houses, grantable cities and default tribute.
//! - [`Game::city_house_mark`]: a city's House colour and state for the map and the lists (cheap).
//! - [`Game::grant_maintenance`]: what a grant does to the liege's maintenance (the Crown's limit
//!   included).
//! - The Crown's limit (rule 14): [`Game::player_crown_limit`], [`Game::player_crown_cities`],
//!   [`Game::player_crown_over`], [`Game::player_crown_over_cost_times100`] (`crown.rs`); a House's
//!   size part [`Game::house_size_part`].
//! - Tribute: [`Game::house_tribute_at`], [`Game::house_loyalty_at_tribute`] (`vassals.rs`).
//! - Levy: [`Game::house_levy_refusal`], [`Game::city_levy_refusal`], [`Game::player_levy_cities`].
//! - Side deals: [`Game::house_bribe_price`], [`Game::house_bribe_refusal`] (`bribes.rs`).
//! - Previews: [`Game::grant_candidates`], [`Game::persons_preview`] (`preview.rs`).

use ffh_data::*;

use super::loyalty::{tribute_part, Holder};
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
    /// Off the map (a House's head, a great person's founder; a unit carried and not placed).
    Away,
}

/// One part of a loyalty sum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoyaltyPart {
    /// `Demand refused`, `Granted Torrin`, `State religion opposed`, `Tribute 30%`.
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
    /// Its term does not run this turn (a first build's Works wonder in the seat's queue).
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
    /// The turns left of their (or their House's) Furious countdown.
    pub countdown_turns: Option<i32>,
    /// The countdown's stated outcome (`House Ashledger will break away ...`).
    pub outcome: Option<String>,
    pub demand: Option<DemandView>,
    /// No demand before this many turns (after one was met or refused); 0 when none is due.
    pub demand_rest_turns: i32,
    pub rival: Option<PersonId>,
    pub rival_name: Option<String>,
    pub deeds: Vec<Deed>,
    pub entered_turn: i32,
}

/// A House's tribute, for its card and the selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TributeView {
    /// The tribute, percent.
    pub pct: i32,
    /// What it gave at its last payment (the turn: -1, none yet).
    pub last_gold: i32,
    pub last_research: i32,
    pub last_turn: i32,
    /// For each tribute it may be set to (0 to 50 by 10): (percent, gold, research it would pay
    /// this turn, the House's loyalty and state at it).
    pub at: Vec<(i32, i32, i32, i32, LoyaltyState)>,
}

/// Whether a House takes levies, and why not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevyStatus {
    pub takes: bool,
    /// `House Arne is Annoyed. It takes no levy.`
    pub reason: Option<String>,
}

/// One House.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseView {
    pub id: HouseId,
    /// The liege.
    pub owner: PlayerId,
    /// Its own player.
    pub player: PlayerId,
    pub origin: HouseOrigin,
    /// `House Arne`, `Hippus`.
    pub label: String,
    pub name: String,
    /// Its player's civilization and its name (`House Arne` for a founded House: its player's own
    /// description; a vassal civilization's short description).
    pub civilization: Option<CivilizationType>,
    pub civilization_name: String,
    /// Its player's leader (the portrait).
    pub leader: Option<LeaderHeadType>,
    pub color: u8,
    /// Its head (the person of the liege's cast who is its leader).
    pub head: Option<PersonId>,
    /// Its capital.
    pub seat: Option<CityId>,
    pub cities: Vec<CityId>,
    pub founded_turn: i32,
    pub loyalty: i32,
    pub parts: Vec<LoyaltyPart>,
    /// Its attitude toward its liege.
    pub state: LoyaltyState,
    pub countdown_turns: Option<i32>,
    /// The countdown's stated outcome (only while one runs).
    pub outcome: Option<String>,
    pub tribute: TributeView,
    pub levy: LevyStatus,
    /// Whether its liege may free it (else why not).
    pub free: Result<(), &'static str>,
    /// Its price to secede (`None`: it refuses any offer: Pleased or Friendly).
    pub bribe_price: Option<i32>,
    /// Its great cities (its size, rule 14) and the realm's threshold above which each is -1.
    pub great_cities: i32,
    pub size_threshold: i32,
}

/// A player's court.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourtView {
    pub player: PlayerId,
    /// Its persons, in cast order (the heads of its Houses among them).
    pub persons: Vec<PersonView>,
    pub houses: Vec<HouseView>,
    /// The Crown's cities that can be granted now (empty when grants are refused).
    pub grantable: Vec<CityId>,
    /// Whether it can grant cities (else why not: direct rule, no Houses for this civilization).
    pub grants: Result<(), &'static str>,
    /// The persons it could still draw.
    pub pool_left: usize,
    /// The tribute a new House takes, percent.
    pub tribute_default: i32,
    /// The Crown's limit (rule 14; `None`: no limit), its cities (the capital first), those
    /// beyond the limit, and what they cost beyond the normal rate (hundredths of gold a turn,
    /// inflation included).
    pub crown_limit: Option<i32>,
    pub crown_cities: Vec<CityId>,
    pub crown_over: Vec<CityId>,
    pub crown_over_cost_times100: i32,
    /// The realm's great cities (the player's and its Houses') and the great cities a House holds
    /// without a grievance (rule 14).
    pub realm_cities: i32,
    pub house_size_threshold: i32,
}

/// A city's House for the map and the lists ([`Game::city_house_mark`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseMark {
    pub house: HouseId,
    /// Its liege.
    pub owner: PlayerId,
    pub color: u8,
    pub state: LoyaltyState,
    pub countdown_turns: Option<i32>,
}

/// What a grant of a city does to its owner's maintenance ([`Game::grant_maintenance`]), in
/// hundredths of gold a turn: the rules' maintenance of each city of the owner now and with the
/// granted city a vassal's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantMaintenance {
    /// The owner's maintenance now (its cities' sum, before inflation).
    pub now_times100: i32,
    /// After the grant (the granted city's own maintenance gone, the others' recomputed).
    pub after_times100: i32,
    /// The number-of-cities part alone, now and after (the city-count maintenance).
    pub count_now_times100: i32,
    pub count_after_times100: i32,
    /// The change in gold a turn the owner pays (after less now, inflation applied; negative: a
    /// saving).
    pub gold_change_times100: i32,
}

impl Game {
    /// The parts of a holder's loyalty: the running modifiers, then the standing parts (the
    /// state religion opposed; a House's tribute).
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
        if let Holder::House(h) = hd {
            let pct = self.houses[h].tribute;
            out.push(LoyaltyPart { label: format!("Tribute {pct}%"), amount: tribute_part(pct), turns_left: None });
            let size = self.house_size_part(h);
            if size != 0 {
                out.push(LoyaltyPart { label: self.house_size_label(h, self.house_great_cities(h)), amount: size, turns_left: None });
            }
        }
        out
    }

    fn holder_countdown_turns(&self, hd: Holder) -> Option<i32> {
        self.holder_standing(hd).countdown.map(|end| (end - self.game_turn()).max(0))
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
            Holder::House(h) => self.house_breakaway_outcome(h),
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
            entered_turn: person.entered_turn,
        })
    }

    /// House `h`'s tribute view.
    pub fn house_tribute_view(&self, h: HouseId) -> TributeView {
        let house = &self.houses[h];
        let at = (0..=TRIBUTE_MAX)
            .step_by(TRIBUTE_STEP as usize)
            .map(|pct| {
                let (gold, research) = self.house_tribute_at(h, pct);
                let loyalty = self.house_loyalty_at_tribute(h, pct);
                (pct, gold, research, loyalty, LoyaltyState::of(loyalty))
            })
            .collect();
        TributeView { pct: house.tribute, last_gold: house.last_gold, last_research: house.last_research, last_turn: house.last_turn, at }
    }

    /// House `h`'s view (`None` when it does not stand).
    pub fn house_view(&self, h: HouseId) -> Option<HouseView> {
        let house = self.houses.get(h)?;
        let hd = Holder::House(h);
        let countdown_turns = self.holder_countdown_turns(hd);
        let refusal = self.house_levy_refusal(h);
        Some(HouseView {
            id: h,
            owner: house.owner,
            player: house.player,
            origin: house.origin,
            label: self.house_label(h),
            name: house.name.clone(),
            civilization: self.player_get_civilization_type(house.player),
            civilization_name: self.player_house_name(house.player),
            leader: self.player_get_leader_type(house.player),
            color: house.color,
            head: house.head.filter(|&x| self.persons.contains(x)),
            seat: self.house_seat(h),
            cities: self.house_cities(h),
            founded_turn: house.founded_turn,
            loyalty: self.holder_loyalty(hd),
            parts: self.holder_parts(hd),
            state: self.holder_state(hd),
            countdown_turns,
            outcome: countdown_turns.map(|_| self.house_breakaway_outcome(h)),
            tribute: self.house_tribute_view(h),
            levy: LevyStatus { takes: refusal.is_none(), reason: refusal },
            free: self.player_check_free_house(house.owner, h),
            bribe_price: self.house_bribe_price(h),
            great_cities: self.house_great_cities(h),
            size_threshold: self.house_size_threshold(h),
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
            tribute_default: self.players[p].house_tribute,
            crown_limit: self.player_crown_limit(p),
            crown_cities: self.player_crown_cities(p),
            crown_over: self.player_crown_over(p),
            crown_over_cost_times100: self.player_crown_over_cost_times100(p),
            realm_cities: self.player_realm_great_cities(p),
            house_size_threshold: self.player_house_size_threshold(p),
        }
    }

    /// City `c`'s House for the map and the lists: its colour, state and countdown; `None` for a
    /// city of a civilization that is no House. Cheap (no scan).
    pub fn city_house_mark(&self, c: CityId) -> Option<HouseMark> {
        let h = self.city_house(c)?;
        let hd = Holder::House(h);
        Some(HouseMark { house: h, owner: self.houses[h].owner, color: self.houses[h].color, state: self.holder_state(hd), countdown_turns: self.holder_countdown_turns(hd) })
    }

    /// What granting city `c` does to its owner's maintenance (pure; the rules' maintenance
    /// functions with the city counted as a vassal's: [`GrantMaintenance`]).
    pub fn grant_maintenance(&self, c: CityId) -> GrantMaintenance {
        let p = self.cities[c].owner;
        let (mut now, mut after, mut count_now, mut count_after) = (0, 0, 0, 0);
        for &o in &self.players[p].cities {
            if !self.cities.contains(o) {
                continue;
            }
            now += self.city_maintenance_without_times100(o, None);
            count_now += self.city_num_cities_part_without_times100(o, None);
            if o != c {
                after += self.city_maintenance_without_times100(o, Some(c));
                count_after += self.city_num_cities_part_without_times100(o, Some(c));
            }
        }
        let inflation = (self.player_calculate_inflation_rate(p) + 100).max(0);
        GrantMaintenance { now_times100: now, after_times100: after, count_now_times100: count_now, count_after_times100: count_after, gold_change_times100: (after - now) * inflation / 100 }
    }
}
