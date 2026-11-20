//! Houses (this game's own rules, not FFH2's; docs/PERSONS.md, "The fourth build": courting is a
//! cultural channel, on "The third build": Houses without Persons of Note, on "The second build":
//! Houses as vassal civilizations). The module keeps the name of the system it began as; it holds
//! no person since the third build.
//!
//! **Houses are vassal civilizations** (`houses.rs`, `vassals.rs`). A House is a player of its own
//! (`House::player`, `Player::house`), the vassal of its liege (`House::owner`). Its leader is its
//! player's leader: portrait, name and title ([`leader_name`], [`leader_title`]), and the ambition
//! its first trait gives ([`ambition_of_leader`], [`Game::house_ambition`]). Every vassal is a
//! House: the vassal rules (`team_set_vassal`) make and end the records. A realm without a House
//! founds one with its first grant of a Crown city (a new player born of the liege, its leader drawn
//! from the free minor leaders: `leaders.rs`); a realm with Houses grants to one of them (the city
//! moves to its player). A House pays tribute, can be levied by its liege's armies, and takes
//! gifts.
//!
//! **Requests and loyalty** (`demands.rs`, `loyalty.rs`). Each House makes one request at a time
//! (`House::demand`; the code keeps the word "demand", every text the player reads says
//! "request"), an act on the board with a term; met or refused, it leaves a timed modifier on the
//! House's loyalty. The sum gives one of five states, the attitude scale (Friendly to Furious); a
//! Furious House issues an ultimatum, which its liege accepts (a contract: `ultimatums.rs`) or
//! refuses (it breaks away: war; `secession.rs`; a direct-rule civic is every House breaking away
//! at once). Annoyed or worse takes no levy.
//!
//! **Splits** (`splits.rs`): a House holding 6 great cities or more may split, its farther half
//! going to a cadet House.
//!
//! **Courting** (`favour.rs`; the fourth build): a civilization's Court channel (`culture.rs`)
//! builds a House's favour toward it; at the requirement, which slides with the House's loyalty
//! and its leader's attitude toward the courter, the House splits toward the courter. A liege's
//! Guard channel wears every courter's favour down. **Gifts** (`gifts.rs`): a House takes gold and
//! resources, nothing asked back; they are ordinary gifts (its leader's attitude toward the giver).
//!
//! **A realm shares its knowledge** (`knowledge.rs`): a tech one team of a realm (a liege's team
//! and its Houses' teams) gains, every other team of the realm gains too.
//!
//! **The world units** (`world_unit.rs`): the mean commerce of the world's civilizations (an
//! ultimatum's gold) and their median culture (a courtship's requirement).
//!
//! **Read API** (`read.rs`) and **previews** (`preview.rs`) are what the client and the AI see.

use std::sync::OnceLock;

use ffh_data::*;

use crate::game::Game;
use crate::ids::*;
use crate::log::{LogMark, LogTag};

mod crown;
mod demands;
mod favour;
mod gifts;
mod houses;
mod knowledge;
mod leaders;
mod loyalty;
mod preview;
mod read;
mod secession;
mod splits;
mod turn;
mod ultimatums;
mod vassals;
mod world_unit;

pub use crown::CROWN_OVER_FACTOR;
pub use favour::{attitude_step, CourtshipView, Favour, GuardView, GuardedCourtship, Requirement, SplitView};
pub use houses::CityHold;
pub use leaders::{leader_name, leader_title, LeaderPool};
pub use loyalty::tribute_part;
pub use preview::{GrantCandidate, PersonsAct, PreviewLine};
pub use read::{CourtView, DemandView, GrantMaintenance, HouseMark, HouseView, LevyStatus, LoyaltyPart, TributeRate, TributeView};
pub use ultimatums::{ContractAct, ContractView};

// ---------------------------------------------------------------------------------------------
// Numbers of the build that are not defines (docs/PERSONS.md, "The second build")
// ---------------------------------------------------------------------------------------------

/// `Request met: +3 (60 turns)`.
pub const DEMAND_MET: (i32, i32) = (3, 60);
/// `Request refused: -3 (60 turns)`.
pub const DEMAND_REFUSED: (i32, i32) = (-3, 60);
/// `Granted <city>: +1 (30 turns)`.
pub const GRANTED: (i32, i32) = (1, 30);
/// `<city> granted to <House>`: a claimant House.
pub const SNUB_HOUSE: (i32, i32) = (-1, 30);
/// `Levied: -1 (20 turns)`: each levy order placed in a House's city.
pub const LEVIED: (i32, i32) = (-1, 20);
/// `Capitulated: -4 (100 turns)`: a House that became a vassal by capitulating.
pub const CAPITULATED: (i32, i32) = (-4, 100);
/// `Sought protection: +2 (60 turns)`: a House that became a vassal by a deal.
pub const SOUGHT_PROTECTION: (i32, i32) = (2, 60);
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
/// A House's rate in force moves one [`TRIBUTE_STEP`] toward the rate its liege set every this
/// many turns (rule 4: 0% to 50% takes 25 turns, and so does 50% back to 0%).
pub const TRIBUTE_LAG_TURNS: i32 = 5;
/// The Land list's foreign cities: within this many plots of a city of the House.
pub const LAND_RANGE: i32 = 12;
/// A House keeps at most this many lines of record (the oldest go).
pub const DEEDS_KEPT: usize = 40;
/// A request with this many turns left is told to its House's liege once.
pub const DEMAND_WARNING_TURNS: i32 = 10;
/// The colours a House's shield is drawn with (`House::color` is an index below this).
pub const HOUSE_COLORS: u8 = 8;

// ---- The third build, part 2A: splits, ultimatums and contracts (rules 4 to 6) ----

/// While a contract runs, a House's loyalty counts as at least this (Annoyed: rule 6).
pub const CONTRACT_FLOOR: i32 = FURIOUS_AT + 1;
/// Rule 5 (b): an ultimatum asks a lower tribute when the rate set is this or more, percent.
pub const ULTIMATUM_TRIBUTE_FROM: i32 = 20;
/// Rule 5 (b): the rate asked is the highest whose loyalty once in force is this or more.
pub const ULTIMATUM_TRIBUTE_LOYALTY: i32 = -4;
/// Rule 5 (f): the gold a turn per House city is the world unit plus this, divided by
/// [`ULTIMATUM_GOLD_DIVISOR`] (2 a city at turn 150's unit of 47, 6 at turn 300's 153).
pub const ULTIMATUM_GOLD_ROUND: i32 = 12;
/// Rule 5 (f): see [`ULTIMATUM_GOLD_ROUND`].
pub const ULTIMATUM_GOLD_DIVISOR: i32 = 24;

// ---------------------------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------------------------

/// The kind of request a House makes: its leader's ambition ([`ambition_of_leader`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ambition {
    Land,
    War,
    Faith,
    Works,
}

impl Ambition {
    pub fn name(self) -> &'static str {
        match self {
            Ambition::Land => "Land",
            Ambition::War => "War",
            Ambition::Faith => "Faith",
            Ambition::Works => "Works",
        }
    }
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

    /// Annoyed or Furious: a House takes no levy.
    pub fn withholds(self) -> bool {
        self >= LoyaltyState::Annoyed
    }
}

/// Why a loyalty modifier is there: what its label needs, as plain data (city and House names
/// as text, since a city changes id with its owner).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoyaltyCause {
    /// A request met.
    DemandMet,
    /// A request lapsed or made impossible by an act of the liege.
    DemandRefused,
    /// A city granted to the House (`city`: its name).
    Granted { city: String },
    /// A city the House claimed granted to another House (`house`: its label).
    GrantedElsewhere { city: String, house: String },
    /// A levy order placed in a city of the House.
    Levied,
    /// The House became a vassal by capitulating.
    Capitulated,
    /// The House became a vassal by a deal.
    SoughtProtection,
}

impl LoyaltyCause {
    /// The modifier's label, bare: `Request met`, `Granted Torrin`, `Torrin granted to House Arne`.
    pub fn label(&self) -> String {
        match self {
            LoyaltyCause::DemandMet => "Request met".into(),
            LoyaltyCause::DemandRefused => "Request refused".into(),
            LoyaltyCause::Granted { city } => format!("Granted {city}"),
            LoyaltyCause::GrantedElsewhere { city, house } => format!("{city} granted to {house}"),
            LoyaltyCause::Levied => "Levied".into(),
            LoyaltyCause::Capitulated => "Capitulated".into(),
            LoyaltyCause::SoughtProtection => "Sought protection".into(),
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

/// The loyalty of a House: its modifiers and the state last told to its liege.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    pub mods: Vec<LoyaltyMod>,
    /// The state last told to the liege (a change is told once, in the liege's turn processing).
    pub state: LoyaltyState,
}

/// What a request asks for. A city is named by its plot (a city changes id with its owner).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemandKind {
    /// The city at this plot granted to the House (taken first when it is another owner's).
    GrantCity(PlotIdx),
    /// The city at this plot taken (the War list, while at war with its owner).
    TakeCity(PlotIdx),
    /// War with this civilization.
    War(PlayerId),
    /// A state religion adopted.
    AdoptReligion,
    /// The state religion in the city at this plot.
    SpreadReligion(PlotIdx),
}

/// A House's standing request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Demand {
    pub kind: DemandKind,
    /// The game turn it was made.
    pub issued: i32,
    /// The game turn it lapses.
    pub deadline: i32,
}

/// A line of a House's record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deed {
    pub turn: i32,
    pub text: String,
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
    /// A cadet House, born of a House's split (`House <name>`, its parent's civilization; the third
    /// build, rule 4).
    Split,
    /// A House a rival's courtship took (the fourth build, rule 4): born of the split toward the
    /// courter (`House <name>`, the courted House's civilization), or a House of one city that went
    /// whole (its name kept).
    Courted,
}

impl HouseOrigin {
    /// A House a realm made (a founding, a split, a courted House): named `House <name>`; the
    /// others are vassal civilizations that keep their names. (A House is labelled by its
    /// player's name: [`Game::house_label`].)
    pub fn founded(self) -> bool {
        matches!(self, HouseOrigin::Founded | HouseOrigin::Split | HouseOrigin::Courted)
    }
}

/// A House (`Game::houses`): a vassal civilization (`player`) of its liege (`owner`). Its leader is
/// its player's leader.
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
    pub standing: Standing,
    /// Its standing request (rule 2).
    pub demand: Option<Demand>,
    /// No request is made before this game turn (after one was met or refused).
    pub demand_rest_until: i32,
    /// Its record, oldest first (at most [`DEEDS_KEPT`]).
    pub deeds: Vec<Deed>,
    pub founded_turn: i32,
    /// The colour index of its shield (below [`HOUSE_COLORS`]).
    pub color: u8,
    /// The tribute its liege set, percent (0 to [`TRIBUTE_MAX`] by [`TRIBUTE_STEP`]): the rate
    /// in force moves toward it.
    pub tribute: i32,
    /// The tribute in force, percent: what the House pays and its standing loyalty part. One
    /// [`TRIBUTE_STEP`] toward [`House::tribute`] every [`TRIBUTE_LAG_TURNS`] turns (rule 4).
    pub tribute_now: i32,
    /// The game turn the count toward the rate's next step runs from: the turn the rate in force
    /// took effect, or the turn the liege set a new rate while the rate in force was at rest. The
    /// next step takes effect [`TRIBUTE_LAG_TURNS`] turns after it.
    pub tribute_since: i32,
    /// The gold its last tribute gave (tribute is gold only: the third build, rule 10).
    pub last_gold: i32,
    /// The game turn its last tribute was paid (-1: none yet).
    pub last_turn: i32,
    // ---- The third build, part 2A (rules 5 and 6) ----
    /// Its last refused request (rule 5 (a): an ultimatum asks it again while its `Request
    /// refused` runs).
    pub refused: Option<DemandKind>,
    /// Its contract with its liege, while one runs (rule 6).
    pub contract: Option<Contract>,
    /// The ultimatums it has issued (rule 5 (f): each raises the gold of the next).
    pub ultimatums: i32,
    // ---- The fourth build, rule 2 (courting by culture; `favour.rs`) ----
    /// Its favour toward each civilization that courts it, in courter order.
    pub favour: Vec<Favour>,
}

// ---- The third build, part 2A: ultimatums and contracts (rules 5 and 6) ----

/// What an ultimatum demands (rule 5), and its contract's bound (rule 6). A city is named by its
/// plot (a city changes id with its owner).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UltimatumKind {
    /// (a) The city at this plot (a Crown city) granted to the House. No bound.
    GrantCity(PlotIdx),
    /// (a), (d) War on this civilization. Bound: no peace with it.
    War(PlayerId),
    /// (a) This state religion adopted. Bound: no change of state religion.
    AdoptReligion(ReligionType),
    /// (b) The House's tribute set to this rate, percent. Bound: no rate set above it.
    Tribute(i32),
    /// (c) Peace with this civilization. Bound: no war on it.
    Peace(PlayerId),
    /// (e) This resource given to the House by a trade of its liege's. Bound: the trade is not
    /// cancelled.
    Resource(BonusType),
    /// (f) This much gold a turn from the liege's treasury to the House's. Bound: it runs its
    /// turns.
    Gold(i32),
}

/// An ultimatum (rule 5): House `house`'s demand, answered by its liege at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ultimatum {
    pub house: HouseId,
    pub kind: UltimatumKind,
    /// It is the House's refused request again (rule 5 (a)).
    pub request: bool,
    /// The game turn its contract would end, accepted (`PERSON_ULTIMATUM_TURNS` from now).
    pub until: i32,
}

/// A House's contract (rule 6): an accepted ultimatum's demand, binding until `until`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contract {
    pub kind: UltimatumKind,
    /// The game turn it ends.
    pub until: i32,
    /// The deal of a resource given (rule 5 (e)).
    pub deal: Option<DealId>,
}

/// Whom a city is granted to ([`Game::player_grant_city`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrantTo {
    /// A House that exists: the city moves to its player.
    House(HouseId),
    /// A new House founded with the city (the third build, rule 3: only while the realm has no
    /// House), its leader drawn from the free minor leaders ([`Game::house_leader_draw`]).
    NewHouse,
}

// ---------------------------------------------------------------------------------------------
// Type names resolved once
// ---------------------------------------------------------------------------------------------

/// Type names the Houses' rules read, resolved once.
pub(crate) struct Names {
    pub war: Vec<TraitType>,
    pub faith: Vec<TraitType>,
    pub works: Vec<TraitType>,
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
pub fn and_list(items: &[String]) -> String {
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
    /// Whether player `p` can have Houses: a living civilization, not the barbarians, a minor
    /// civilization, a crusade Host or a House.
    pub fn player_has_houses(&self, p: PlayerId) -> bool {
        self.players[p].alive && self.players[p].house.is_none() && !self.player_is_barbarian(p) && !self.player_is_minor_civ(p) && !self.player_is_crusade_host(p)
    }

    /// A House's label: `House Arne` (a founded House), `Hippus` (a vassal civilization): its
    /// player's name ([`Game::player_house_name`]: a founded House's player is described `House
    /// <name>`; a House that went whole to a courter keeps it).
    pub fn house_label(&self, h: HouseId) -> String {
        self.player_house_name(self.houses[h].player)
    }

    /// The House player `p` is, if it is one.
    pub fn player_house(&self, p: PlayerId) -> Option<HouseId> {
        self.players.get(p)?.house.filter(|&h| self.houses.contains(h))
    }

    /// House `h`'s leader: its player's.
    pub fn house_leader(&self, h: HouseId) -> Option<LeaderHeadType> {
        self.player_get_leader_type(self.houses[h].player)
    }

    /// House `h`'s ambition: the one its leader's first trait gives (Land without a leader).
    pub fn house_ambition(&self, h: HouseId) -> Ambition {
        self.house_leader(h).map_or(Ambition::Land, |l| ambition_of_leader(self.d, l))
    }

    /// House `h`'s alignment: its leader's.
    pub fn house_alignment(&self, h: HouseId) -> Option<AlignmentType> {
        self.house_leader(h).and_then(|l| self.d.leader_heads[l].alignment)
    }

    /// A city's display name.
    pub(crate) fn persons_city_name(&self, c: CityId) -> String {
        self.d.text.name(&self.cities[c].name).to_string()
    }

    /// The live city on plot `plot`, if any.
    pub(crate) fn persons_city_at(&self, plot: PlotIdx) -> Option<CityId> {
        self.map.plots[plot].plot_city.filter(|&c| self.cities.contains(c))
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

    /// A message to the liege of House `h` about it, marked for the client's alerts (at its seat).
    pub(crate) fn house_msg(&mut self, h: HouseId, f: impl FnOnce() -> String) {
        if !self.log.enabled {
            return;
        }
        let turn = self.game_turn();
        let p = self.houses[h].owner;
        match self.house_seat(h).map(|c| (self.cities[c].x, self.cities[c].y)) {
            Some(at) => self.log.msg_marked(turn, Some(p), at, LogMark::House, LogTag::House(h), f),
            None => self.log.msg(turn, Some(p), f),
        }
    }

    /// Adds a line to House `h`'s record (the oldest goes past [`DEEDS_KEPT`]).
    pub(crate) fn house_add_deed(&mut self, h: HouseId, text: String) {
        let turn = self.game_turn();
        let deeds = &mut self.houses[h].deeds;
        deeds.push(Deed { turn, text });
        if deeds.len() > DEEDS_KEPT {
            let extra = deeds.len() - DEEDS_KEPT;
            deeds.drain(..extra);
        }
    }

    /// Battle report `id` in the record of `house`, the House whose city the battle assaulted:
    /// `Torrin withstood an assault.` when the city held.
    pub(crate) fn house_battle_deed(&mut self, id: u32, house: Option<HouseId>) {
        let Some(h) = house.filter(|&h| self.houses.contains(h)) else { return };
        let Some(r) = self.battle_report(id) else { return };
        let Some(city) = &r.place.city else { return };
        let held = matches!(r.outcome.result, crate::engagement::report::EngageResult::Repulsed | crate::engagement::report::EngageResult::Held);
        if held {
            let place = self.d.text.name(&city.name).to_string();
            self.house_add_deed(h, format!("{place} withstood an assault."));
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
