//! The read API of the Houses: everything the Court screen, the map's name plates, the diplomacy
//! and Armies screens and the AI read (pure, `&self`). The client's view code is written against
//! it.
//!
//! - [`Game::house_view`]: one House: its player, leader, civilization, cities, attitude and its
//!   parts, request, record, tribute, levy, the outcome of its ultimatum when Furious, its contract,
//!   its split line, freeing, its courtships and its guard.
//! - [`Game::court_view`]: a player's Houses, grantable cities, the founding of a first House and
//!   the default tribute.
//! - [`Game::city_house_mark`]: a city's House colour and state for the map and the lists (cheap).
//! - [`Game::grant_maintenance`]: what a grant does to the liege's maintenance (the Crown's limit
//!   included).
//! - The Crown's limit (rule 14): [`Game::player_crown_limit`], [`Game::player_crown_cities`],
//!   [`Game::player_crown_over`], [`Game::player_crown_over_cost_times100`] (`crown.rs`).
//! - Tribute: [`Game::house_tribute_at`], [`Game::house_loyalty_at_tribute`],
//!   [`Game::house_tribute_turns`], [`Game::house_tribute_in_force_after`] (`vassals.rs`).
//! - Levy: [`Game::house_levy_refusal`], [`Game::city_levy_refusal`], [`Game::player_levy_cities`].
//! - Courting (the fourth build, `favour.rs`): [`Game::house_courtships`],
//!   [`Game::house_courtship_at`], [`Game::house_guard_view`], [`Game::house_guard_with`],
//!   [`Game::house_requirement`], [`Game::house_split_toward`], [`Game::house_courting_line`];
//!   gifts: [`Game::house_gift_refusal`] (`gifts.rs`); the channels themselves: `culture.rs`.
//! - Previews: [`Game::grant_candidates`], [`Game::persons_preview`] (`preview.rs`).

use ffh_data::*;

use super::loyalty::tribute_part;
use super::*;

/// One part of a loyalty sum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoyaltyPart {
    /// `Request refused`, `Granted Torrin`, `State religion opposed`, `Tribute 30%`.
    pub label: String,
    pub amount: i32,
    /// Turns left (`None`: standing, as long as its cause lasts).
    pub turns_left: Option<i32>,
}

/// A House's standing request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemandView {
    pub kind: DemandKind,
    /// `Torrin (Sheaim) taken and granted to House Ashledger.`
    pub text: String,
    pub turns_left: i32,
}

/// A House's tribute, for its card and the selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TributeView {
    /// The rate its liege set, percent.
    pub pct: i32,
    /// The rate in force, percent (what it pays and its loyalty part).
    pub now: i32,
    /// The turns until the rate set is in force (0: it is).
    pub turns: i32,
    /// What it gave at its last payment (gold only; the turn: -1, none yet).
    pub last_gold: i32,
    pub last_turn: i32,
    /// Each tribute it may be set to (0 to 50 by 10).
    pub at: Vec<TributeRate>,
}

/// One rate of a House's tribute selector ([`TributeView::at`]): what setting it now would do,
/// once it is in force.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TributeRate {
    /// The rate, percent.
    pub pct: i32,
    /// The gold it pays at the House's commerce now.
    pub gold: i32,
    /// The standing part `Tribute N%` at it ([`tribute_part`]).
    pub part: i32,
    /// The turns until it would be in force were it set now ([`Game::house_tribute_turns`]; 0: it
    /// is the rate in force).
    pub turns: i32,
    /// The House's loyalty and state once it is in force ([`Game::house_loyalty_at_tribute`]).
    pub loyalty: i32,
    pub state: LoyaltyState,
    /// A contract's bound refuses it (the third build, rule 6; the reason:
    /// [`Game::player_check_house_tribute`]).
    pub barred: bool,
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
    /// Its player's leader (the portrait), the leader's name and title (`The Auditor of Oaths`;
    /// `None` for a leader whose text has none), and the ambition its first trait gives.
    pub leader: Option<LeaderHeadType>,
    pub leader_name: String,
    pub leader_title: Option<String>,
    pub ambition: Ambition,
    pub color: u8,
    /// Its capital.
    pub seat: Option<CityId>,
    pub cities: Vec<CityId>,
    pub founded_turn: i32,
    pub loyalty: i32,
    pub parts: Vec<LoyaltyPart>,
    /// Its attitude toward its liege.
    pub state: LoyaltyState,
    /// What its being Furious means (no contract running): its ultimatum at its liege's next turn,
    /// and the break-away a refusal is ([`Game::house_breakaway_outcome`]).
    pub outcome: Option<String>,
    // ---- the third build, part 2A ----
    /// Its contract, while one runs (rule 6).
    pub contract: Option<ContractView>,
    /// `Holds 7 cities: 5% a turn to split.` when it may split (rule 4).
    pub split_line: Option<String>,
    /// The ultimatums it has issued.
    pub ultimatums: i32,
    /// Its request (rule 2), and between requests the turns before the next one may be made.
    pub demand: Option<DemandView>,
    pub demand_rest_turns: i32,
    /// Its record, oldest first.
    pub deeds: Vec<Deed>,
    pub tribute: TributeView,
    pub levy: LevyStatus,
    /// Whether its liege may free it (else why not).
    pub free: Result<(), &'static str>,
    /// Every courtship of it (the fourth build, rule 2: the civilizations it keeps a favour toward
    /// and those whose Court channel runs at it), in player order.
    pub courtships: Vec<CourtshipView>,
    /// Its guard, while its liege's Guard channel runs at it (rule 5).
    pub guard: Option<GuardView>,
}

/// A player's court.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourtView {
    pub player: PlayerId,
    pub houses: Vec<HouseView>,
    /// The Crown's cities that can be granted now (empty when grants are refused).
    pub grantable: Vec<CityId>,
    /// Whether it can grant cities (else why not: direct rule, no Houses for this civilization).
    pub grants: Result<(), &'static str>,
    /// Whether its next grant can found a House (rule 3: a realm without a House, a leader free;
    /// else why not), and where its leader would come from ([`Game::house_founding_line`]).
    pub founding: Result<(), &'static str>,
    pub founding_line: Option<String>,
    /// The tribute a new House takes, percent.
    pub tribute_default: i32,
    /// The Crown's limit (rule 14; `None`: no limit), its cities (the capital first), those
    /// beyond the limit, and what they cost beyond the normal rate (hundredths of gold a turn,
    /// inflation included).
    pub crown_limit: Option<i32>,
    pub crown_cities: Vec<CityId>,
    pub crown_over: Vec<CityId>,
    pub crown_over_cost_times100: i32,
}

/// A city's House for the map and the lists ([`Game::city_house_mark`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseMark {
    pub house: HouseId,
    /// Its liege.
    pub owner: PlayerId,
    pub color: u8,
    pub state: LoyaltyState,
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
    /// The parts of House `h`'s loyalty: the running modifiers, then the standing parts (the state
    /// religion opposed; the tribute in force).
    pub fn house_parts(&self, h: HouseId) -> Vec<LoyaltyPart> {
        let turn = self.game_turn();
        let mut out: Vec<LoyaltyPart> = self.houses[h]
            .standing
            .mods
            .iter()
            .filter(|m| m.until.map_or(true, |u| u > turn))
            .map(|m| LoyaltyPart { label: m.cause.label(), amount: m.amount, turns_left: m.until.map(|u| u - turn) })
            .collect();
        let religion = self.house_religion_part(h);
        if religion != 0 {
            out.push(LoyaltyPart { label: "State religion opposed".into(), amount: religion, turns_left: None });
        }
        let pct = self.houses[h].tribute_now;
        out.push(LoyaltyPart { label: format!("Tribute {pct}%"), amount: tribute_part(pct), turns_left: None });
        // A contract's floor (the third build, rule 6): what it lifts the sum by, so the parts add
        // up to the loyalty every rule counts.
        if let Some(cv) = self.house_contract_view(h) {
            let raw = self.house_loyalty_raw(h);
            let label = cv.floor.trim_end_matches('.').to_string();
            out.push(LoyaltyPart { label, amount: (CONTRACT_FLOOR - raw).max(0), turns_left: None });
        }
        out
    }

    /// House `h`'s tribute view.
    pub fn house_tribute_view(&self, h: HouseId) -> TributeView {
        let house = &self.houses[h];
        let at = (0..=TRIBUTE_MAX)
            .step_by(TRIBUTE_STEP as usize)
            .map(|pct| {
                let gold = self.house_tribute_at(h, pct);
                let loyalty = self.house_loyalty_at_tribute(h, pct);
                let barred = pct != house.tribute && self.player_check_house_tribute(house.owner, h, pct).is_err();
                TributeRate { pct, gold, part: tribute_part(pct), turns: self.house_tribute_turns(h, pct), loyalty, state: LoyaltyState::of(loyalty), barred }
            })
            .collect();
        TributeView {
            pct: house.tribute,
            now: house.tribute_now,
            turns: self.house_tribute_turns(h, house.tribute),
            last_gold: house.last_gold,
            last_turn: house.last_turn,
            at,
        }
    }

    /// House `h`'s view (`None` when it does not stand).
    pub fn house_view(&self, h: HouseId) -> Option<HouseView> {
        let house = self.houses.get(h)?;
        let refusal = self.house_levy_refusal(h);
        let leader = self.house_leader(h);
        let demand = house.demand.map(|dm| DemandView { kind: dm.kind, text: self.demand_kind_text(h, dm.kind), turns_left: (dm.deadline - self.game_turn()).max(0) });
        Some(HouseView {
            id: h,
            owner: house.owner,
            player: house.player,
            origin: house.origin,
            label: self.house_label(h),
            name: house.name.clone(),
            civilization: self.player_get_civilization_type(house.player),
            civilization_name: self.player_house_name(house.player),
            leader,
            leader_name: leader.map(|l| leader_name(self.d, l)).unwrap_or_default(),
            leader_title: leader.and_then(|l| leader_title(self.d, l)),
            ambition: self.house_ambition(h),
            color: house.color,
            seat: self.house_seat(h),
            cities: self.house_cities(h),
            founded_turn: house.founded_turn,
            loyalty: self.house_loyalty(h),
            parts: self.house_parts(h),
            state: self.house_state(h),
            outcome: (self.house_state(h) == LoyaltyState::Furious && !self.house_contract_runs(h)).then(|| self.house_breakaway_outcome(h)),
            contract: self.house_contract_view(h),
            split_line: self.house_split_line(h),
            ultimatums: house.ultimatums,
            demand_rest_turns: if demand.is_none() { (house.demand_rest_until - self.game_turn()).max(0) } else { 0 },
            demand,
            deeds: house.deeds.clone(),
            tribute: self.house_tribute_view(h),
            levy: LevyStatus { takes: refusal.is_none(), reason: refusal },
            free: self.player_check_free_house(house.owner, h),
            courtships: self.house_courtships(h),
            guard: self.house_guard_view(h),
        })
    }

    /// Player `p`'s court: its Houses, the Crown's cities it can grant, whether a grant founds a
    /// House.
    pub fn court_view(&self, p: PlayerId) -> CourtView {
        let grants = self.player_check_grants(p);
        let grantable = if grants.is_ok() { self.players[p].cities.iter().copied().filter(|&c| self.player_check_grant_city(p, c).is_ok()).collect() } else { Vec::new() };
        CourtView {
            player: p,
            houses: self.players[p].houses.iter().filter_map(|&h| self.house_view(h)).collect(),
            grantable,
            grants,
            founding: self.player_check_found_house(p),
            founding_line: self.house_founding_line(p),
            tribute_default: self.players[p].house_tribute,
            crown_limit: self.player_crown_limit(p),
            crown_cities: self.player_crown_cities(p),
            crown_over: self.player_crown_over(p),
            crown_over_cost_times100: self.player_crown_over_cost_times100(p),
        }
    }

    /// City `c`'s House for the map and the lists: its colour and state; `None` for a city of a
    /// civilization that is no House. Cheap (no scan).
    pub fn city_house_mark(&self, c: CityId) -> Option<HouseMark> {
        let h = self.city_house(c)?;
        Some(HouseMark { house: h, owner: self.houses[h].owner, color: self.houses[h].color, state: self.house_state(h) })
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
