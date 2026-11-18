//! Persons of Note (this game's own rules, not FFH2's; docs/PERSONS.md, "The second build":
//! Houses as vassal civilizations).
//!
//! **The cast.** Every living civilization that is no House (not the barbarians, a minor
//! civilization or a crusade Host) has a small cast of named persons (`Game::persons`, its list
//! `Player::persons` in the order they entered): its heroes, its best veterans, the founders it
//! draws from its great people, and the rulers of its Houses. A person other than a hero is a
//! leader: one of the civilization's preferred minor leaders, drawn with the game RNG from those not
//! in play and not drawn before in this game (`Game::persons_drawn`), or the leader of a vassal
//! civilization (origin `Ruler`). Entry (`entry.rs`) runs once a turn in the owner's turn
//! processing.
//!
//! **Houses are vassal civilizations** (`houses.rs`, `vassals.rs`). A House is a player of its own
//! (`House::player`, `Player::house`), the vassal of its liege (`House::owner`), headed by a person
//! of the liege's cast who is its leader. Every vassal is a House: the vassal rules
//! (`team_set_vassal`) make and end the records. A grant of a Crown city founds one (a new player
//! born of the liege, led by the person granted it) or enlarges one (the city moves to its player).
//! A House pays tribute, can be levied by its liege's armies, and answers offers to secede.
//!
//! **Posts.** A person holds at most one post: the command of an army (`command.rs`: the person
//! with the army that the player named, else the one of the highest level) or the head of a House.
//!
//! **Demands and loyalty** (`demands.rs`, `loyalty.rs`). Each person makes one demand at a time,
//! an act on the board with a term; met or refused, it leaves a timed modifier on the loyalty of
//! the House (a head's) or the person (the others). The sum gives one of five states, the attitude
//! scale (Friendly to Furious); a Furious House or commander starts a countdown, at whose end the
//! House breaks away (war) or the commander secedes (`secession.rs`; a direct-rule civic is every
//! House breaking away at once). Annoyed or worse withholds: a House takes no levy, a commander's
//! army no order to attack (`command.rs`).
//!
//! **Side deals** (`bribes.rs`): a House gives Secede or Become your vassal (trade items) for gold.
//!
//! **Read API** (`read.rs`) and **previews** (`preview.rs`) are what the client and the AI see.

use std::sync::OnceLock;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::log::{LogMark, LogTag};

mod bribes;
mod command;
mod crown;
mod demands;
mod entry;
mod houses;
mod loyalty;
mod preview;
mod read;
mod secession;
mod turn;
mod vassals;

pub use bribes::{house_item, house_offer_value, BribeKind};
pub use crown::{CROWN_OVER_FACTOR, HOUSE_CITIES_FREE};
pub use entry::{leader_name, leader_title};
pub use houses::Claimant;
pub use preview::{GrantCandidate, PersonsAct, PreviewLine};
pub use read::{CourtView, DemandView, GrantMaintenance, HouseMark, HouseView, LevyStatus, LoyaltyPart, PersonView, PersonWhere};

// ---------------------------------------------------------------------------------------------
// Numbers of the build that are not defines (docs/PERSONS.md, "The second build")
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
/// `Levied: -1 (20 turns)`: each levy order placed in a House's city.
pub const LEVIED: (i32, i32) = (-1, 20);
/// `Capitulated: -4 (100 turns)`: a House that became a vassal by capitulating.
pub const CAPITULATED: (i32, i32) = (-4, 100);
/// `Sought protection: +2 (60 turns)`: a House that became a vassal by a deal.
pub const SOUGHT_PROTECTION: (i32, i32) = (2, 60);
/// `Offered 300 gold by the Hippus`: an offer to secede, shown (no amount) for this many turns.
pub const OFFER_NOTED_TURNS: i32 = 20;
/// `State religion opposed: -2`, standing (not stored).
pub const RELIGION_OPPOSED: i32 = -2;
/// Friendly at this loyalty or more.
pub const FRIENDLY_AT: i32 = 3;
/// Pleased at this loyalty or more (below Friendly).
pub const PLEASED_AT: i32 = 1;
/// Annoyed at this loyalty or less (above Furious).
pub const ANNOYED_AT: i32 = -3;
/// Furious at this loyalty or less.
pub const FURIOUS_AT: i32 = -6;
/// A new realm's default tribute (`Player::house_tribute`), percent.
pub const TRIBUTE_DEFAULT: i32 = 20;
/// The highest tribute, percent.
pub const TRIBUTE_MAX: i32 = 50;
/// The tribute's step, percent.
pub const TRIBUTE_STEP: i32 = 10;
/// A House's price to secede: `(loyalty + 6) x` this `x` its cities, gold.
pub const BRIBE_PER_CITY: i32 = 25;
/// Gold per turn in an offer counts this many times.
pub const BRIBE_GOLD_PER_TURN_TURNS: i32 = 10;
/// The attitude modifier (`ai_attitude_extra`, FfH's scale: 4 a step, the AI's relation 2) an AI
/// liege takes toward a civilization that offered one of its Houses to secede.
pub const BRIBE_GRUDGE: i32 = -4;
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
    /// The leader of a civilization that became a vassal (a House's head; no unit).
    Ruler,
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

/// The five states of loyalty: the attitude scale (best first).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoyaltyState {
    Friendly,
    Pleased,
    #[default]
    Cautious,
    Annoyed,
    Furious,
}

impl LoyaltyState {
    /// Every state, best first.
    pub const ALL: [LoyaltyState; 5] = [LoyaltyState::Friendly, LoyaltyState::Pleased, LoyaltyState::Cautious, LoyaltyState::Annoyed, LoyaltyState::Furious];

    /// The state at loyalty `total`.
    pub fn of(total: i32) -> LoyaltyState {
        if total <= FURIOUS_AT {
            LoyaltyState::Furious
        } else if total <= ANNOYED_AT {
            LoyaltyState::Annoyed
        } else if total >= FRIENDLY_AT {
            LoyaltyState::Friendly
        } else if total >= PLEASED_AT {
            LoyaltyState::Pleased
        } else {
            LoyaltyState::Cautious
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            LoyaltyState::Friendly => "Friendly",
            LoyaltyState::Pleased => "Pleased",
            LoyaltyState::Cautious => "Cautious",
            LoyaltyState::Annoyed => "Annoyed",
            LoyaltyState::Furious => "Furious",
        }
    }

    /// The matching attitude (`AttitudeType`).
    pub fn attitude(self) -> AttitudeType {
        match self {
            LoyaltyState::Friendly => AttitudeType::ATTITUDE_FRIENDLY,
            LoyaltyState::Pleased => AttitudeType::ATTITUDE_PLEASED,
            LoyaltyState::Cautious => AttitudeType::ATTITUDE_CAUTIOUS,
            LoyaltyState::Annoyed => AttitudeType::ATTITUDE_ANNOYED,
            LoyaltyState::Furious => AttitudeType::ATTITUDE_FURIOUS,
        }
    }

    /// Annoyed or Furious: a House takes no levy, a commander's army no order to attack.
    pub fn withholds(self) -> bool {
        self >= LoyaltyState::Annoyed
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
    /// A levy order placed in a city of the House.
    Levied,
    /// The House became a vassal by capitulating.
    Capitulated,
    /// The House became a vassal by a deal.
    SoughtProtection,
    /// An offer to secede (`gold`: its worth, gold per turn counted ten times; `by`: the
    /// civilization, `Hippus`).
    Offered { gold: i32, by: String },
}

impl LoyaltyCause {
    /// The modifier's label, bare: `Demand met`, `Granted Torrin`, `Torrin granted to House Arne`.
    pub fn label(&self) -> String {
        match self {
            LoyaltyCause::DemandMet => "Demand met".into(),
            LoyaltyCause::DemandRefused => "Demand refused".into(),
            LoyaltyCause::Granted { city } => format!("Granted {city}"),
            LoyaltyCause::GrantedElsewhere { city, house } => format!("{city} granted to {house}"),
            LoyaltyCause::Levied => "Levied".into(),
            LoyaltyCause::Capitulated => "Capitulated".into(),
            LoyaltyCause::SoughtProtection => "Sought protection".into(),
            LoyaltyCause::Offered { gold, by } => format!("Offered {gold} gold by the {by}"),
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

/// The loyalty of a House or a person: its modifiers, a running Furious countdown, the units
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
    /// This wonder built in the city at this plot (kept for saves of the first build: no demand
    /// names a wonder now).
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
    /// The leader they are (`None` for a hero).
    pub leader: Option<LeaderHeadType>,
    /// Their unit on the map (`None` for a House's head and a great person's founder).
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

/// How a House came to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HouseOrigin {
    /// Founded by a grant (`House <name>`, the liege's civilization).
    Founded,
    /// A civilization that capitulated (it keeps its name).
    Capitulated,
    /// A civilization that became a vassal otherwise: a vassal deal, a card's birth, a colony (it
    /// keeps its name).
    Vassal,
}

/// A House (`Game::houses`): a vassal civilization (`player`) of its liege (`owner`), headed by a
/// person of the liege's cast who is its leader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct House {
    pub id: HouseId,
    /// The liege.
    pub owner: PlayerId,
    /// The House's own player (`Player::house` names this record).
    pub player: PlayerId,
    pub origin: HouseOrigin,
    /// A founded House's name without the word "House"; a vassal civilization's short
    /// description ([`Game::house_label`]).
    pub name: String,
    /// Its leader, in the liege's cast (`None` only when the person left without the House
    /// ending, which the rules do not do).
    pub head: Option<PersonId>,
    pub standing: Standing,
    pub founded_turn: i32,
    /// The colour index of its shield (below [`HOUSE_COLORS`]).
    pub color: u8,
    /// The tribute, percent (0 to [`TRIBUTE_MAX`] by [`TRIBUTE_STEP`]).
    pub tribute: i32,
    /// The gold its last tribute gave.
    pub last_gold: i32,
    /// The research its last tribute gave (0 when the liege researched nothing).
    pub last_research: i32,
    /// The game turn its last tribute was paid (-1: none yet).
    pub last_turn: i32,
}

/// Whom a city is granted to ([`Game::player_grant_city`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrantTo {
    /// A House that exists: the city moves to its player.
    House(HouseId),
    /// A person with a leader (a veteran), who founds a House and leaves the map.
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
    /// minor civilization, a crusade Host or a House.
    pub fn player_has_persons(&self, p: PlayerId) -> bool {
        self.players[p].alive && self.players[p].house.is_none() && !self.player_is_barbarian(p) && !self.player_is_minor_civ(p) && !self.player_is_crusade_host(p)
    }

    /// A House's label: `House Arne` (a founded House), `Hippus` (a vassal civilization).
    pub fn house_label(&self, h: HouseId) -> String {
        let house = &self.houses[h];
        match house.origin {
            HouseOrigin::Founded => format!("House {}", house.name),
            _ => house.name.clone(),
        }
    }

    /// The House player `p` is, if it is one.
    pub fn player_house(&self, p: PlayerId) -> Option<HouseId> {
        self.players.get(p)?.house.filter(|&h| self.houses.contains(h))
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

    /// House `h`'s seat: its player's capital (its most populous city without one).
    pub fn house_seat(&self, h: HouseId) -> Option<CityId> {
        let hp = self.houses.get(h)?.player;
        self.player_get_capital_city(hp).or_else(|| self.players[hp].cities.iter().copied().filter(|&c| self.cities.contains(c)).max_by_key(|&c| (self.cities[c].population, std::cmp::Reverse(c))))
    }

    /// House `h`'s cities (its player's), in its player's order.
    pub fn house_cities(&self, h: HouseId) -> Vec<CityId> {
        let Some(house) = self.houses.get(h) else { return Vec::new() };
        self.players[house.player].cities.iter().copied().filter(|&c| self.cities.contains(c)).collect()
    }

    /// The plot a message about person `x` points at: their unit's, else their House's seat's,
    /// else their owner's capital's.
    pub(crate) fn person_at(&self, x: PersonId) -> Option<(i32, i32)> {
        if let Some(plot) = self.person_unit(x).and_then(|u| self.unit_plot_opt(u)) {
            return Some(self.map.xy(plot));
        }
        if let Some(c) = self.person_house(x).and_then(|h| self.house_seat(h)) {
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

    /// A message to the liege of House `h` about it, marked for the client's alerts.
    pub(crate) fn house_msg(&mut self, h: HouseId, f: impl FnOnce() -> String) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        let p = self.houses[h].owner;
        let at = self.house_seat(h).map(|c| (self.cities[c].x, self.cities[c].y));
        let head = self.houses[h].head.filter(|&x| self.persons.contains(x));
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
