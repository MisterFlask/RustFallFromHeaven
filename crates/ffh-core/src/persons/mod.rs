//! Persons of Note (this game's own rules, not FFH2's; docs/PERSONS.md, "The first build").
//!
//! **The cast.** Every living civilization (not the barbarians, a minor civilization or a crusade
//! Host) has a small cast of named persons (`Game::persons`, its list `Player::persons` in the
//! order they entered): its heroes, its best veterans and the founders it draws from its great
//! people. A person other than a hero is one of the civilization's preferred minor leaders, drawn
//! with the game RNG from those not in play and not drawn before in this game
//! (`Game::persons_drawn`). Entry (`entry.rs`) runs once a turn in the owner's turn processing.
//!
//! **Posts.** A person holds at most one post: the command of an army (`command.rs`: the person
//! with the army that the player named, else the one of the highest level) or the head of a House
//! (`houses.rs`). A House holds cities of its owner granted to it, for good; every other city is
//! the Crown's (`City::house` `None`).
//!
//! **Demands and loyalty** (`demands.rs`, `loyalty.rs`). Each person makes one demand at a time,
//! an act on the board with a term; met or refused, it leaves a timed modifier on the loyalty of
//! the House (a head's) or the person (the others). The sum gives one of four states; Disloyal
//! starts a countdown. What happens at the countdown's end (secession) is step 2 of the build:
//! [`Game::persons_countdowns_due`] lists what is due and nothing acts on it yet.
//!
//! **Read API** (`read.rs`) and **previews** (`preview.rs`) are what the client and the AI see.

use std::sync::OnceLock;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::log::{LogMark, LogTag};

mod command;
mod demands;
mod entry;
mod houses;
mod loyalty;
mod preview;
mod read;
mod turn;

pub use entry::{leader_name, leader_title};
pub use houses::Claimant;
pub use preview::{GrantCandidate, PersonsAct, PreviewLine};
pub use read::{CourtView, DemandView, HouseMark, HouseView, LoyaltyPart, PersonView, PersonWhere};

// ---------------------------------------------------------------------------------------------
// Numbers of the first build that are not defines (docs/PERSONS.md, "The first build")
// ---------------------------------------------------------------------------------------------

/// `Demand met: +3 (60 turns)`.
pub const DEMAND_MET: (i32, i32) = (3, 60);
/// `Demand refused: -3 (60 turns)`.
pub const DEMAND_REFUSED: (i32, i32) = (-3, 60);
/// `Granted <city>: +1 (30 turns)`.
pub const GRANTED: (i32, i32) = (1, 30);
/// `<city> granted to <House>`: a claimant House.
pub const SNUB_HOUSE: (i32, i32) = (-1, 30);
/// `<city> granted to <House>`: the claimant commander who took it.
pub const SNUB_COMMANDER: (i32, i32) = (-2, 30);
/// `State religion opposed: -2`, standing (not stored).
pub const RELIGION_OPPOSED: i32 = -2;
/// Devoted at this loyalty or more.
pub const DEVOTED_AT: i32 = 3;
/// Restless at this loyalty or less.
pub const RESTLESS_AT: i32 = -3;
/// Disloyal at this loyalty or less.
pub const DISLOYAL_AT: i32 = -6;
/// The commander whose army took a city is a claimant to it for this many turns.
pub const TAKEN_CLAIM_TURNS: i32 = 20;
/// The Land list's foreign cities: within this many plots of a city of the House.
pub const LAND_RANGE: i32 = 12;
/// A person keeps at most this many deeds (the oldest go).
pub const DEEDS_KEPT: usize = 40;
/// A demand with this many turns left is told to its owner once.
pub const DEMAND_WARNING_TURNS: i32 = 10;
/// The colours a House's shield is drawn with (`House::color` is an index below this).
pub const HOUSE_COLORS: u8 = 8;

// ---------------------------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------------------------

/// How a person entered the cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersonOrigin {
    /// A unit with `PROMOTION_HERO`.
    Hero,
    /// An army unit of a high level.
    Veteran,
    /// A great person unit spent on founding a House.
    GreatPerson,
}

/// The kind of demand a person makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ambition {
    Land,
    War,
    Faith,
    Works,
}

/// A person's one post.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Post {
    None,
    /// The commander of this army.
    Commander(ArmyId),
    /// The head of this House.
    Head(HouseId),
}

/// The four states of loyalty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoyaltyState {
    Devoted,
    #[default]
    Loyal,
    Restless,
    Disloyal,
}

impl LoyaltyState {
    /// The state at loyalty `total`.
    pub fn of(total: i32) -> LoyaltyState {
        if total <= DISLOYAL_AT {
            LoyaltyState::Disloyal
        } else if total <= RESTLESS_AT {
            LoyaltyState::Restless
        } else if total >= DEVOTED_AT {
            LoyaltyState::Devoted
        } else {
            LoyaltyState::Loyal
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LoyaltyState::Devoted => "Devoted",
            LoyaltyState::Loyal => "Loyal",
            LoyaltyState::Restless => "Restless",
            LoyaltyState::Disloyal => "Disloyal",
        }
    }
}

/// Why a loyalty modifier is there: what its label needs, as plain data (city and House names
/// as text, since a city changes id with its owner).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoyaltyCause {
    /// A demand met.
    DemandMet,
    /// A demand lapsed or made impossible by an act of the owner.
    DemandRefused,
    /// A city granted to the holder (`city`: its name).
    Granted { city: String },
    /// A city the holder claimed granted to another House (`house`: its label).
    GrantedElsewhere { city: String, house: String },
}

impl LoyaltyCause {
    /// The modifier's label, bare: `Demand met`, `Granted Torrin`, `Torrin granted to House Arne`.
    pub fn label(&self) -> String {
        match self {
            LoyaltyCause::DemandMet => "Demand met".into(),
            LoyaltyCause::DemandRefused => "Demand refused".into(),
            LoyaltyCause::Granted { city } => format!("Granted {city}"),
            LoyaltyCause::GrantedElsewhere { city, house } => format!("{city} granted to {house}"),
        }
    }
}

/// One loyalty modifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoyaltyMod {
    pub cause: LoyaltyCause,
    pub amount: i32,
    /// The game turn it ends (`None`: it lasts).
    pub until: Option<i32>,
}

/// The loyalty of a House or a person: its modifiers, a running Disloyal countdown, the units
/// sworn to a commander when its countdown began, and the state last told to the owner.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    pub mods: Vec<LoyaltyMod>,
    /// The game turn the countdown ends (`None`: none runs).
    pub countdown: Option<i32>,
    /// A commander's sworn units (its army's units and its own when its countdown began).
    pub sworn: Vec<UnitId>,
    /// The state last told to the owner (a change is told once, in the owner's turn processing).
    pub state: LoyaltyState,
}

/// What a demand asks for. A city is named by its plot (a city changes id with its owner).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemandKind {
    /// A post (a command or a House).
    Post,
    /// A House of their own.
    House,
    /// The city at this plot granted to their House (taken first when it is another owner's).
    GrantCity(PlotIdx),
    /// The city at this plot taken (the War list, while at war with its owner).
    TakeCity(PlotIdx),
    /// War with this civilization.
    War(PlayerId),
    /// A state religion adopted.
    AdoptReligion,
    /// The state religion in the city at this plot.
    SpreadReligion(PlotIdx),
    /// This wonder built in the city at this plot (their seat).
    Wonder { city: PlotIdx, building: BuildingType },
}

/// A person's standing demand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Demand {
    pub kind: DemandKind,
    /// The game turn it was made.
    pub issued: i32,
    /// The game turn it lapses (pushed back a turn each turn its term is paused).
    pub deadline: i32,
}

/// A line of a person's record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deed {
    pub turn: i32,
    pub text: String,
}

/// A person of note (`Game::persons`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub owner: PlayerId,
    pub origin: PersonOrigin,
    /// The minor leader they are (`None` for a hero).
    pub leader: Option<LeaderHeadType>,
    /// Their unit on the map (`None` for a great person's founder: no unit, never dies).
    pub unit: Option<UnitId>,
    pub name: String,
    pub ambition: Ambition,
    pub post: Post,
    /// Their own loyalty (a head's is the House's: `House::standing`).
    pub standing: Standing,
    pub demand: Option<Demand>,
    /// No demand is made before this game turn (after one was met or refused).
    pub demand_rest_until: i32,
    /// The head preferred to them at the last grant they claimed.
    pub rival: Option<PersonId>,
    /// Their record, oldest first (at most [`DEEDS_KEPT`]).
    pub deeds: Vec<Deed>,
    pub entered_turn: i32,
}

/// A House (`Game::houses`): cities of its owner granted for good, presided over by a person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct House {
    pub id: HouseId,
    pub owner: PlayerId,
    /// Its name without the word "House" ([`Game::house_label`] adds it).
    pub name: String,
    /// `None`: the seat is empty.
    pub head: Option<PersonId>,
    /// Its first city; when lost, its most populous city (ties: the lower id).
    pub seat: Option<CityId>,
    /// Its cities, in the order granted.
    pub cities: Vec<CityId>,
    pub standing: Standing,
    pub founded_turn: i32,
    /// The colour index of its shield (below [`HOUSE_COLORS`]).
    pub color: u8,
    /// The game turns an empire or requisition order was placed in its cities (pruned past
    /// `PERSON_LEVY_TURNS`).
    pub orders: Vec<i32>,
}

/// Whom a city is granted to ([`Game::player_grant_city`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrantTo {
    /// A House that exists.
    House(HouseId),
    /// A person without a post, who founds a House.
    Person(PersonId),
    /// A great person unit: a person is drawn, founds the House, and the unit is consumed.
    GreatPerson(UnitId),
}

/// A countdown that has run out ([`Game::persons_countdowns_due`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CountdownDue {
    House(HouseId),
    Commander(PersonId),
}

// ---------------------------------------------------------------------------------------------
// Type names resolved once
// ---------------------------------------------------------------------------------------------

/// Type names the persons rules read, resolved once.
pub(crate) struct Names {
    pub war: Vec<TraitType>,
    pub faith: Vec<TraitType>,
    pub works: Vec<TraitType>,
    pub disciple: Option<UnitCombatType>,
    pub adept: Option<UnitCombatType>,
    /// The great person classes: the five card suits' and the Great Commander's.
    pub great: Vec<UnitClassType>,
    pub government: Option<CivicOptionType>,
    pub good: Option<AlignmentType>,
    pub evil: Option<AlignmentType>,
}

pub(crate) fn names(d: &GameData) -> &'static Names {
    static NAMES: OnceLock<Names> = OnceLock::new();
    NAMES.get_or_init(|| {
        let traits = |list: &[&str]| list.iter().filter_map(|n| d.ty::<TraitType>(n)).collect::<Vec<_>>();
        Names {
            war: traits(&["TRAIT_AGGRESSIVE", "TRAIT_RAIDERS", "TRAIT_HORSELORD", "TRAIT_DEXTEROUS", "TRAIT_BARBARIAN"]),
            faith: traits(&["TRAIT_SPIRITUAL", "TRAIT_TOLERANT", "TRAIT_SINISTER"]),
            works: traits(&["TRAIT_INDUSTRIOUS", "TRAIT_CREATIVE", "TRAIT_PHILOSOPHICAL", "TRAIT_ARCANE", "TRAIT_INGENUITY", "TRAIT_SUMMONER", "TRAIT_FINANCIAL"]),
            disciple: d.ty::<UnitCombatType>("UNITCOMBAT_DISCIPLE"),
            adept: d.ty::<UnitCombatType>("UNITCOMBAT_ADEPT"),
            great: ["UNITCLASS_SCIENTIST", "UNITCLASS_PROPHET", "UNITCLASS_ENGINEER", "UNITCLASS_MERCHANT", "UNITCLASS_ARTIST", "UNITCLASS_COMMANDER"].iter().filter_map(|n| d.ty::<UnitClassType>(n)).collect(),
            government: d.ty::<CivicOptionType>("CIVICOPTION_GOVERNMENT"),
            good: d.ty::<AlignmentType>("ALIGNMENT_GOOD"),
            evil: d.ty::<AlignmentType>("ALIGNMENT_EVIL"),
        }
    })
}

/// The ambition a leader's first personal trait gives.
pub fn ambition_of_leader(d: &GameData, leader: LeaderHeadType) -> Ambition {
    let n = names(d);
    match d.leader_heads[leader].traits.first() {
        Some(t) if n.war.contains(t) => Ambition::War,
        Some(t) if n.faith.contains(t) => Ambition::Faith,
        Some(t) if n.works.contains(t) => Ambition::Works,
        _ => Ambition::Land,
    }
}

/// English ordinal-free list: "A", "A and B", "A, B and C".
pub(crate) fn and_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

// ---------------------------------------------------------------------------------------------
// Small accessors
// ---------------------------------------------------------------------------------------------

impl Game {
    /// Whether player `p` has persons and Houses: a living civilization, not the barbarians, a
    /// minor civilization or a crusade Host.
    pub fn player_has_persons(&self, p: PlayerId) -> bool {
        self.players[p].alive && !self.player_is_barbarian(p) && !self.player_is_minor_civ(p) && !self.player_is_crusade_host(p)
    }

    /// A House's label: `House Arne`.
    pub fn house_label(&self, h: HouseId) -> String {
        format!("House {}", self.houses[h].name)
    }

    /// A city's display name.
    pub(crate) fn persons_city_name(&self, c: CityId) -> String {
        self.d.text.name(&self.cities[c].name).to_string()
    }

    /// The live city on plot `plot`, if any.
    pub(crate) fn persons_city_at(&self, plot: PlotIdx) -> Option<CityId> {
        self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c))
    }

    /// Person `x`'s unit, if it lives.
    pub fn person_unit(&self, x: PersonId) -> Option<UnitId> {
        self.persons[x].unit.filter(|&u| self.units.contains(u))
    }

    /// The House person `x` heads, if any.
    pub fn person_house(&self, x: PersonId) -> Option<HouseId> {
        match self.persons[x].post {
            Post::Head(h) if self.houses.contains(h) => Some(h),
            _ => None,
        }
    }

    /// The army person `x` commands, if any.
    pub fn person_army(&self, x: PersonId) -> Option<ArmyId> {
        match self.persons[x].post {
            Post::Commander(a) if self.armies.contains(a) => Some(a),
            _ => None,
        }
    }

    /// Who a person is in a line about them: `Hrolf`, `Hrolf, 2nd Army` for a commander, `House
    /// Arne` for a head.
    pub fn person_subject(&self, x: PersonId) -> String {
        let name = &self.persons[x].name;
        match self.persons[x].post {
            Post::Commander(a) if self.armies.contains(a) => format!("{name}, {}", self.armies[a].name),
            Post::Head(h) if self.houses.contains(h) => self.house_label(h),
            _ => name.clone(),
        }
    }

    /// The plot a message about person `x` points at: their unit's, else their seat's, else
    /// their owner's capital's.
    pub(crate) fn person_at(&self, x: PersonId) -> Option<(i32, i32)> {
        if let Some(plot) = self.person_unit(x).and_then(|u| self.unit_plot_opt(u)) {
            return Some(self.map.xy(plot));
        }
        if let Some(c) = self.person_house(x).and_then(|h| self.houses[h].seat).filter(|&c| self.cities.contains(c)) {
            return Some((self.cities[c].x, self.cities[c].y));
        }
        let p = self.persons[x].owner;
        self.player_get_capital_city(p).map(|c| (self.cities[c].x, self.cities[c].y))
    }

    /// A message to the owner of person `x` about them, marked for the client's alerts.
    pub(crate) fn person_msg(&mut self, x: PersonId, f: impl FnOnce() -> String) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        let p = self.persons[x].owner;
        match self.person_at(x) {
            Some(at) => self.log.msg_marked(turn, Some(p), at, LogMark::Person, LogTag::Person(x), f),
            None => self.log.msg(turn, Some(p), f),
        }
    }

    /// A message to the owner of House `h` about it, marked for the client's alerts.
    pub(crate) fn house_msg(&mut self, h: HouseId, f: impl FnOnce() -> String) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        let p = self.houses[h].owner;
        let at = self.houses[h].seat.filter(|&c| self.cities.contains(c)).map(|c| (self.cities[c].x, self.cities[c].y));
        let head = self.houses[h].head;
        match (at, head) {
            (Some(at), Some(x)) => self.log.msg_marked(turn, Some(p), at, LogMark::Person, LogTag::Person(x), f),
            (Some(at), None) => self.log.msg_marked(turn, Some(p), at, LogMark::Person, LogTag::None, f),
            _ => self.log.msg(turn, Some(p), f),
        }
    }

    /// Adds a line to person `x`'s record (the oldest goes past [`DEEDS_KEPT`]).
    pub(crate) fn person_add_deed(&mut self, x: PersonId, text: String) {
        let turn = self.game_turn();
        let deeds = &mut self.persons[x].deeds;
        deeds.push(Deed { turn, text });
        if deeds.len() > DEEDS_KEPT {
            let extra = deeds.len() - DEEDS_KEPT;
            deeds.drain(..extra);
        }
    }

    /// Person `x`'s alignment: their leader's, a hero's owner's.
    pub fn person_alignment(&self, x: PersonId) -> Option<AlignmentType> {
        match self.persons[x].leader {
            Some(l) => self.d.leader_heads[l].alignment,
            None => self.players[self.persons[x].owner].alignment,
        }
    }

    /// Whether alignments `a` and `b` are opposite (good and evil).
    pub(crate) fn alignments_opposed(&self, a: Option<AlignmentType>, b: Option<AlignmentType>) -> bool {
        let n = names(self.d);
        match (a, b) {
            (Some(a), Some(b)) => (Some(a) == n.good && Some(b) == n.evil) || (Some(a) == n.evil && Some(b) == n.good),
            _ => false,
        }
    }
}
