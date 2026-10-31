//! Commands: what the human does, decoded from the protocol, validated, then applied as ports of
//! what the C++ exe does for human input (design §4.5): the `CvNet*::Execute` net messages
//! (CvMessageData.cpp) and the exe-side helpers that send them (`CvGame::selectionListMove`,
//! `selectionListGameNetMessage`, `cityPushOrder`, the `handleAction` confirmations, the
//! DECLAREWARMOVE / LOADUNIT / DIPLOVOTE / EVENT popups).
//!
//! Validation runs before any rules call and is pure (`&Game`): ids exist
//! and belong to the human, type indices are in range, and the rules' own `can*` gates pass, as
//! the exe only offers legal actions. A command that would need a C++ confirmation popup returns
//! [`Confirm`] (nothing changes); the client resends it with the confirmation flag.

use std::fmt::Write as _;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::log::LogEntry;
use ffh_core::structs::*;
use ffh_data::*;

use crate::decision;
use crate::json::{FieldError, Json};
use crate::names;
use crate::selection;

/// A command (design §3). Type-valued fields are typed; data fields whose type depends on
/// another field (a mission's `data1`, an order's `data`) are raw C++ ints.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Right-click move (`selectionListMove`). `declare_war`: `None` asks (DECLAREWARMOVE),
    /// `Some(true)` declares and moves, `Some(false)` is the popup's "no".
    Move { units: Vec<UnitId>, x: i32, y: i32, append: bool, declare_war: Option<bool> },
    Mission { units: Vec<UnitId>, mission: MissionType, data1: i32, data2: i32, flags: i32, append: bool },
    DoCommand { units: Vec<UnitId>, command: CommandType, data1: i32, data2: i32, alt: bool, confirmed: bool },
    JoinGroup { unit: UnitId, head: Option<UnitId> },
    AutoMission { unit: UnitId },
    PushOrder { city: CityId, order: OrderType, data: i32, alt: bool, shift: bool, ctrl: bool },
    PopOrder { city: CityId, index: i32 },
    DoTask { city: CityId, task: TaskType, data1: i32, data2: i32, option: bool, alt: bool, shift: bool, ctrl: bool },
    /// This game's cultural channel (docs/CULTURE.md): city `city` channels `action` (with
    /// `religion` for Spread Religion) into civilization `target` from now on; no `action`: it
    /// channels nothing.
    CultureChannel { city: CityId, action: Option<CulturalActionType>, target: Option<PlayerId>, religion: Option<ReligionType> },
    /// This game's armies (docs/ARMIES.md): form a new army of the units (on one plot).
    ArmyForm { units: Vec<UnitId> },
    /// Split the units off their army into a new one.
    ArmySplit { units: Vec<UnitId> },
    /// The units (army units on one plot, from armies or a garrison) join army `army` there; a
    /// whole army joining another is a merge.
    ArmyJoin { units: Vec<UnitId>, army: ArmyId },
    /// The units (in one of the human's cities) join its garrison.
    ArmyGarrison { units: Vec<UnitId> },
    ArmyRename { army: ArmyId, name: String },
    /// The army's units join the garrison of the nearest city (they march there).
    ArmyDisband { army: ArmyId },
    /// Upgrade the army's units, the most experienced first, while the gold lasts.
    ArmyUpgrade { army: ArmyId },
    /// The size the army is to be kept at (0: none); while it is short of it, it requisitions.
    ArmyTarget { army: ArmyId, size: i32 },
    /// Army `army` requisitions from city `city` (`on`: last in its list) or no longer does (its
    /// requisition order there is withdrawn).
    ArmyRequisition { army: ArmyId, city: CityId, on: bool },
    /// Army `army` requisitions from `cities` (in that order) and from no other.
    ArmyRequisitionSet { army: ArmyId, cities: Vec<CityId> },
    /// Where city `city`'s trained land combat units go: production order `index`'s, or the
    /// city's default (`index` -1); `dest` `None` = the city's default (for an order) or its own
    /// garrison (for the city). A unit bound elsewhere marches there.
    RecruitDest { city: CityId, index: i32, dest: Option<ffh_core::army::RecruitDest> },
    /// This game's governors (docs/GOVERNORS.md): rule governed city `city` directly, handing
    /// `instead_of` (directly ruled) to a governor when no slot is free.
    GovernDirect { city: CityId, instead_of: Option<CityId> },
    /// Hand directly ruled city `city` to a governor.
    GovernHand { city: CityId },
    /// Governor `governor` governs city `city` (from the bench or another city).
    GovernAssign { city: CityId, governor: GovernorId },
    Research { tech: Option<TechType>, shift: bool },
    /// Merged over the current civics.
    UpdateCivics { civics: Vec<CivicType> },
    PercentChange { commerce: CommerceType, change: i32 },
    Convert { religion: Option<ReligionType> },
    DeclareWar { team: TeamId },
    /// A deal offered to an AI from the diplomacy screen: `give` is what the human gives,
    /// `get` what it asks (items of the two trade tables, `view::diplo`).
    ProposeDeal { to: PlayerId, give: Vec<TradeData>, get: Vec<TradeData> },
    Event { triggered: TriggeredId, event: EventType },
    Vote { vote: VoteId, choice: PlayerVote },
    /// The secretary general's election (BUTTONPOPUP_CHOOSEELECTION): `option` indexes the
    /// selection's resolutions; `None` is the popup's "none" button (no resolution).
    Election { selection: VoteId, option: Option<usize> },
    SetOption { option: PlayerOptionType, value: bool },
    AutoMoves,
    EndTurn { force: bool },
    /// FfH `setAIAutoPlay` (a testing aid).
    AutoPlay { turns: i32 },
}

/// A confirmation the C++ asks before acting (a popup); the command is resent with its flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// DECLAREWARMOVE: resend `move` with `declare_war: true` (or `false`; then the units still
    /// move when `move_anyway`, the popup's `getOption2`: the target plot is not that team's).
    DeclareWar { team: TeamId, move_anyway: bool },
    /// CONFIRMCOMMAND (`bConfirmCommand`: delete): resend with `confirmed: true`.
    Command { command: CommandType },
    /// CONFIRMCASTGLOBAL: resend with `confirmed: true`.
    CastGlobal { spell: SpellType },
    /// CONFIRMCASTWAR: the spell hits a team we are not at war with.
    CastWar { spell: SpellType, team: TeamId },
    /// LOADUNIT: pick a transport, then send `do_command COMMAND_LOAD_UNIT` with `data1` = its
    /// owner and `data2` = its id.
    LoadUnit { transports: Vec<UnitId> },
    /// The end-turn gate (§2.3): resend with `force: true`.
    EndTurn { events: usize, votes: usize, elections: usize, research: bool, ready_units: usize },
}

impl Confirm {
    pub fn to_json(&self) -> Json {
        match self {
            Confirm::DeclareWar { team, move_anyway } => crate::jobj! { "kind" => "declare_war", "team" => *team, "move_anyway" => *move_anyway },
            Confirm::Command { command } => crate::jobj! { "kind" => "command", "command" => Json::idx(*command) },
            Confirm::CastGlobal { spell } => crate::jobj! { "kind" => "cast_global", "spell" => Json::idx(*spell) },
            Confirm::CastWar { spell, team } => crate::jobj! { "kind" => "cast_war", "spell" => Json::idx(*spell), "team" => *team },
            Confirm::LoadUnit { transports } => crate::jobj! { "kind" => "load_unit", "transports" => transports.clone() },
            Confirm::EndTurn { events, votes, elections, research, ready_units } => crate::jobj! {
                "kind" => "end_turn", "events" => *events, "votes" => *votes, "elections" => *elections, "research" => *research, "ready_units" => *ready_units,
            },
        }
    }
}

/// A command's result.
#[derive(Clone, Debug, PartialEq)]
pub enum CmdOk {
    /// Applied; `log` holds the messages it produced for the human.
    Applied { log: Vec<LogEntry>, accepted: Option<bool> },
    /// Nothing changed: the C++ would ask first.
    NeedsConfirm(Confirm),
}

/// Why a command was refused (nothing changed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CmdErr {
    /// The human does not have control (the game runs, or is over).
    NotYourTurn,
    /// A blocking decision is open.
    DecisionPending,
    /// Malformed: unknown command, missing field, type or index out of range.
    Invalid(String),
    /// An id of someone else's unit or city.
    NotOwner,
    /// An id that does not exist (any more).
    Gone,
    /// Well-formed but not possible now (the rules' gate refuses).
    Cannot(String),
}

impl CmdErr {
    /// The protocol error code.
    pub fn code(&self) -> &'static str {
        match self {
            CmdErr::NotYourTurn => "not_your_turn",
            CmdErr::DecisionPending => "decision_pending",
            CmdErr::Invalid(_) => "invalid",
            CmdErr::NotOwner => "not_owner",
            CmdErr::Gone => "gone",
            CmdErr::Cannot(_) => "cannot",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            CmdErr::Invalid(s) | CmdErr::Cannot(s) => s.clone(),
            CmdErr::NotYourTurn => "the human does not have control".into(),
            CmdErr::DecisionPending => "a decision must be answered first".into(),
            CmdErr::NotOwner => "not yours".into(),
            CmdErr::Gone => "no such object".into(),
        }
    }
}

impl std::fmt::Display for CmdErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.detail())
    }
}

impl From<FieldError> for CmdErr {
    fn from(e: FieldError) -> CmdErr {
        CmdErr::Invalid(e.to_string())
    }
}

fn invalid(s: impl Into<String>) -> CmdErr {
    CmdErr::Invalid(s.into())
}

fn cannot(s: impl Into<String>) -> CmdErr {
    CmdErr::Cannot(s.into())
}

/// Every command name (`c`), including the not yet available `propose_deal` (P3).
pub const COMMAND_NAMES: &[&str] = &[
    "move",
    "mission",
    "do_command",
    "join_group",
    "auto_mission",
    "push_order",
    "pop_order",
    "do_task",
    "culture_channel",
    "army_form",
    "army_split",
    "army_join",
    "army_garrison",
    "army_rename",
    "army_disband",
    "army_upgrade",
    "army_target",
    "army_requisition",
    "army_requisition_set",
    "recruit_dest",
    "govern_direct",
    "govern_hand",
    "govern_assign",
    "research",
    "update_civics",
    "percent_change",
    "convert",
    "declare_war",
    "propose_deal",
    "event",
    "vote",
    "election",
    "set_option",
    "auto_moves",
    "end_turn",
    "autoplay",
];

/// The members of each command besides `c` (in the order of [`COMMAND_NAMES`]). A request
/// object may also carry the envelope's `id` and `k`; any other member is refused, so that a
/// misspelt flag is an error instead of its default.
pub const COMMAND_FIELDS: &[(&str, &[&str])] = &[
    ("move", &["units", "x", "y", "append", "declare_war"]),
    ("mission", &["units", "mission", "data1", "data2", "flags", "append"]),
    ("do_command", &["units", "command", "data1", "data2", "alt", "confirmed"]),
    ("join_group", &["unit", "head"]),
    ("auto_mission", &["unit"]),
    ("push_order", &["city", "order", "data", "alt", "shift", "ctrl"]),
    ("pop_order", &["city", "index"]),
    ("do_task", &["city", "task", "data1", "data2", "option", "alt", "shift", "ctrl"]),
    ("culture_channel", &["city", "action", "target", "religion"]),
    ("army_form", &["units"]),
    ("army_split", &["units"]),
    ("army_join", &["units", "army"]),
    ("army_garrison", &["units"]),
    ("army_rename", &["army", "name"]),
    ("army_disband", &["army"]),
    ("army_upgrade", &["army"]),
    ("army_target", &["army", "size"]),
    ("army_requisition", &["army", "city", "on"]),
    ("army_requisition_set", &["army", "cities"]),
    ("recruit_dest", &["city", "index", "dest"]),
    ("govern_direct", &["city", "instead_of"]),
    ("govern_hand", &["city"]),
    ("govern_assign", &["city", "governor"]),
    ("research", &["tech", "shift"]),
    ("update_civics", &["civics"]),
    ("percent_change", &["commerce", "change"]),
    ("convert", &["religion"]),
    ("declare_war", &["team"]),
    ("propose_deal", &["to", "give", "get"]),
    ("event", &["triggered", "event"]),
    ("vote", &["vote", "choice"]),
    ("election", &["selection", "option"]),
    ("set_option", &["option", "value"]),
    ("auto_moves", &[]),
    ("end_turn", &["force"]),
    ("autoplay", &["turns"]),
];

/// The envelope's members a command or query object may carry (`transport::Request`).
pub const ENVELOPE_FIELDS: &[&str] = &["id", "k"];

// ---------------------------------------------------------------------------------------------
// Decoding and the canonical form
// ---------------------------------------------------------------------------------------------

/// A data field: an int, or (when `T` is given) a type name of that kind.
fn typed_data<T: names::Named>(d: &GameData, j: &Json, key: &str) -> Result<i32, FieldError> {
    match j.opt_value(key)? {
        None => Ok(-1),
        Some(Json::Str(_)) => names::field::<T>(d, j, key).map(|t| t.idx() as i32),
        Some(_) => j.req::<i32>(key),
    }
}

/// `data1` of a mission: a type name is accepted where the mission's data is a type.
fn mission_data1(d: &GameData, j: &Json, m: MissionType) -> Result<i32, FieldError> {
    match m {
        MissionType::MISSION_BUILD => typed_data::<BuildType>(d, j, "data1"),
        MissionType::MISSION_SPREAD => typed_data::<ReligionType>(d, j, "data1"),
        MissionType::MISSION_SPREAD_CORPORATION => typed_data::<CorporationType>(d, j, "data1"),
        MissionType::MISSION_JOIN => typed_data::<SpecialistType>(d, j, "data1"),
        MissionType::MISSION_CONSTRUCT => typed_data::<BuildingType>(d, j, "data1"),
        _ => j.get_or("data1", -1),
    }
}

fn command_data1(d: &GameData, j: &Json, c: CommandType) -> Result<i32, FieldError> {
    match c {
        CommandType::COMMAND_PROMOTION => typed_data::<PromotionType>(d, j, "data1"),
        CommandType::COMMAND_UPGRADE => typed_data::<UnitType>(d, j, "data1"),
        CommandType::COMMAND_CAST | CommandType::COMMAND_AUTOMATE_SPELL => typed_data::<SpellType>(d, j, "data1"),
        CommandType::COMMAND_AUTOMATE => typed_data::<AutomateType>(d, j, "data1"),
        _ => j.get_or("data1", -1),
    }
}

fn order_data(d: &GameData, j: &Json, o: OrderType) -> Result<i32, FieldError> {
    match o {
        OrderType::ORDER_TRAIN => typed_data::<UnitType>(d, j, "data"),
        OrderType::ORDER_CONSTRUCT => typed_data::<BuildingType>(d, j, "data"),
        OrderType::ORDER_CREATE => typed_data::<ProjectType>(d, j, "data"),
        OrderType::ORDER_MAINTAIN => typed_data::<ProcessType>(d, j, "data"),
        _ => j.get_or("data", -1),
    }
}

/// `{"garrison": city}`, `{"army": id}` or null (this game's armies).
pub fn recruit_dest_json(dest: Option<ffh_core::army::RecruitDest>) -> Json {
    use ffh_core::army::RecruitDest;
    match dest {
        None => Json::Null,
        Some(RecruitDest::Garrison(c)) => crate::jobj! { "garrison" => c },
        Some(RecruitDest::Army(a)) => crate::jobj! { "army" => a },
    }
}

/// The `dest` field of `recruit_dest` ([`recruit_dest_json`]'s form).
fn recruit_dest_field(j: &Json) -> Result<Option<ffh_core::army::RecruitDest>, FieldError> {
    use ffh_core::army::RecruitDest;
    match j.opt_value("dest")? {
        None | Some(Json::Null) => Ok(None),
        Some(v) => {
            if let Some(c) = v.opt::<CityId>("garrison")? {
                Ok(Some(RecruitDest::Garrison(c)))
            } else if let Some(a) = v.opt::<ArmyId>("army")? {
                Ok(Some(RecruitDest::Army(a)))
            } else {
                Err(FieldError::wrong("dest", "{\"garrison\": city} or {\"army\": id} or null", v))
            }
        }
    }
}

fn task_data1(d: &GameData, j: &Json, t: TaskType) -> Result<i32, FieldError> {
    match t {
        TaskType::TASK_HURRY => typed_data::<HurryType>(d, j, "data1"),
        TaskType::TASK_SET_EMPHASIZE => typed_data::<EmphasizeType>(d, j, "data1"),
        TaskType::TASK_CHANGE_SPECIALIST => typed_data::<SpecialistType>(d, j, "data1"),
        // The only value the exe sends (`liberate(iData1 != 0)`).
        TaskType::TASK_LIBERATE => j.get_or("data1", 0),
        _ => j.get_or("data1", -1),
    }
}

impl Command {
    /// The protocol name (`c`).
    pub fn name(&self) -> &'static str {
        match self {
            Command::Move { .. } => "move",
            Command::Mission { .. } => "mission",
            Command::DoCommand { .. } => "do_command",
            Command::JoinGroup { .. } => "join_group",
            Command::AutoMission { .. } => "auto_mission",
            Command::PushOrder { .. } => "push_order",
            Command::PopOrder { .. } => "pop_order",
            Command::DoTask { .. } => "do_task",
            Command::CultureChannel { .. } => "culture_channel",
            Command::ArmyForm { .. } => "army_form",
            Command::ArmySplit { .. } => "army_split",
            Command::ArmyJoin { .. } => "army_join",
            Command::ArmyGarrison { .. } => "army_garrison",
            Command::ArmyRename { .. } => "army_rename",
            Command::ArmyDisband { .. } => "army_disband",
            Command::ArmyUpgrade { .. } => "army_upgrade",
            Command::ArmyTarget { .. } => "army_target",
            Command::ArmyRequisition { .. } => "army_requisition",
            Command::ArmyRequisitionSet { .. } => "army_requisition_set",
            Command::RecruitDest { .. } => "recruit_dest",
            Command::GovernDirect { .. } => "govern_direct",
            Command::GovernHand { .. } => "govern_hand",
            Command::GovernAssign { .. } => "govern_assign",
            Command::Research { .. } => "research",
            Command::UpdateCivics { .. } => "update_civics",
            Command::PercentChange { .. } => "percent_change",
            Command::Convert { .. } => "convert",
            Command::DeclareWar { .. } => "declare_war",
            Command::ProposeDeal { .. } => "propose_deal",
            Command::Event { .. } => "event",
            Command::Vote { .. } => "vote",
            Command::Election { .. } => "election",
            Command::SetOption { .. } => "set_option",
            Command::AutoMoves => "auto_moves",
            Command::EndTurn { .. } => "end_turn",
            Command::AutoPlay { .. } => "autoplay",
        }
    }

    /// Decodes a request object (`{"c": "move", ...}`; other members such as `id` and `k` are
    /// ignored). Type references are names or indices.
    pub fn from_json(d: &GameData, j: &Json) -> Result<Command, CmdErr> {
        let c = j.req_str("c")?;
        if let Some((_, fields)) = COMMAND_FIELDS.iter().find(|(name, _)| *name == c) {
            let known: Vec<&str> = ["c"].iter().chain(ENVELOPE_FIELDS).chain(fields.iter()).copied().collect();
            j.only(&known)?;
        }
        let flag = |k: &str| j.get_or(k, false);
        Ok(match c {
            "move" => Command::Move {
                units: j.req("units")?,
                x: j.req("x")?,
                y: j.req("y")?,
                append: flag("append")?,
                declare_war: j.opt("declare_war")?,
            },
            "mission" => {
                let mission = names::field::<MissionType>(d, j, "mission")?;
                Command::Mission {
                    units: j.req("units")?,
                    mission,
                    data1: mission_data1(d, j, mission)?,
                    data2: j.get_or("data2", -1)?,
                    flags: j.get_or("flags", 0)?,
                    append: flag("append")?,
                }
            }
            "do_command" => {
                let command = names::field::<CommandType>(d, j, "command")?;
                Command::DoCommand {
                    units: j.req("units")?,
                    command,
                    data1: command_data1(d, j, command)?,
                    data2: j.get_or("data2", -1)?,
                    alt: flag("alt")?,
                    confirmed: flag("confirmed")?,
                }
            }
            "join_group" => Command::JoinGroup { unit: j.req("unit")?, head: j.opt("head")? },
            "auto_mission" => Command::AutoMission { unit: j.req("unit")? },
            "push_order" => {
                let order = names::field::<OrderType>(d, j, "order")?;
                Command::PushOrder { city: j.req("city")?, order, data: order_data(d, j, order)?, alt: flag("alt")?, shift: flag("shift")?, ctrl: flag("ctrl")? }
            }
            "pop_order" => Command::PopOrder { city: j.req("city")?, index: j.req("index")? },
            "do_task" => {
                let task = names::field::<TaskType>(d, j, "task")?;
                Command::DoTask {
                    city: j.req("city")?,
                    task,
                    data1: task_data1(d, j, task)?,
                    data2: j.get_or("data2", -1)?,
                    option: flag("option")?,
                    alt: flag("alt")?,
                    shift: flag("shift")?,
                    ctrl: flag("ctrl")?,
                }
            }
            "culture_channel" => Command::CultureChannel {
                city: j.req("city")?,
                action: names::opt_field(d, j, "action")?,
                target: j.opt("target")?,
                religion: names::opt_field(d, j, "religion")?,
            },
            "army_form" => Command::ArmyForm { units: j.req("units")? },
            "army_split" => Command::ArmySplit { units: j.req("units")? },
            "army_join" => Command::ArmyJoin { units: j.req("units")?, army: j.req("army")? },
            "army_garrison" => Command::ArmyGarrison { units: j.req("units")? },
            "army_rename" => Command::ArmyRename { army: j.req("army")?, name: j.req("name")? },
            "army_disband" => Command::ArmyDisband { army: j.req("army")? },
            "army_upgrade" => Command::ArmyUpgrade { army: j.req("army")? },
            "army_target" => Command::ArmyTarget { army: j.req("army")?, size: j.req("size")? },
            "army_requisition" => Command::ArmyRequisition { army: j.req("army")?, city: j.req("city")?, on: j.req("on")? },
            "army_requisition_set" => Command::ArmyRequisitionSet { army: j.req("army")?, cities: j.req("cities")? },
            "recruit_dest" => Command::RecruitDest { city: j.req("city")?, index: j.get_or("index", -1)?, dest: recruit_dest_field(j)? },
            "govern_direct" => Command::GovernDirect { city: j.req("city")?, instead_of: j.opt("instead_of")? },
            "govern_hand" => Command::GovernHand { city: j.req("city")? },
            "govern_assign" => Command::GovernAssign { city: j.req("city")?, governor: j.req("governor")? },
            "research" => Command::Research { tech: names::opt_field(d, j, "tech")?, shift: flag("shift")? },
            "update_civics" => Command::UpdateCivics { civics: names::list_field(d, j, "civics")? },
            "percent_change" => Command::PercentChange { commerce: names::field(d, j, "commerce")?, change: j.req("change")? },
            "convert" => Command::Convert { religion: names::opt_field(d, j, "religion")? },
            "declare_war" => Command::DeclareWar { team: j.req("team")? },
            "event" => Command::Event { triggered: j.req("triggered")?, event: names::field(d, j, "event")? },
            "vote" => Command::Vote {
                vote: j.req("vote")?,
                choice: names::decode_player_vote(j.req_value("choice")?).map_err(|e| FieldError::new("choice", e))?,
            },
            "election" => Command::Election { selection: j.req("selection")?, option: j.opt("option")? },
            "set_option" => Command::SetOption { option: names::field(d, j, "option")?, value: j.req("value")? },
            "auto_moves" => Command::AutoMoves,
            "end_turn" => Command::EndTurn { force: flag("force")? },
            "autoplay" => Command::AutoPlay { turns: j.req("turns")? },
            "propose_deal" => {
                let items = |key: &str| -> Result<Vec<TradeData>, FieldError> {
                    j.opt_value(key)?
                        .map_or(Ok(Vec::new()), |v| {
                            let list = v.as_arr().ok_or_else(|| FieldError::wrong(key, "an array of trade items", v))?;
                            list.iter().map(|t| crate::view::diplo::trade_item_from_json(d, t).map_err(|e| FieldError::new(key, e.to_string()))).collect()
                        })
                };
                Command::ProposeDeal { to: j.req("to")?, give: items("give")?, get: items("get")? }
            }
            other => return Err(invalid(format!("unknown command {other:?}"))),
        })
    }

    /// The canonical form: indices for types, every field present. `from_json` of it gives the
    /// same command (journal lines are this form).
    pub fn to_json(&self) -> Json {
        let ids = |v: &[UnitId]| Json::Arr(v.iter().map(|&u| Json::from(u)).collect());
        let j = crate::jobj! { "c" => self.name() };
        match self {
            Command::Move { units, x, y, append, declare_war } => {
                j.with("units", ids(units)).with("x", *x).with("y", *y).with("append", *append).with("declare_war", *declare_war)
            }
            Command::Mission { units, mission, data1, data2, flags, append } => j
                .with("units", ids(units))
                .with("mission", Json::idx(*mission))
                .with("data1", *data1)
                .with("data2", *data2)
                .with("flags", *flags)
                .with("append", *append),
            Command::DoCommand { units, command, data1, data2, alt, confirmed } => j
                .with("units", ids(units))
                .with("command", Json::idx(*command))
                .with("data1", *data1)
                .with("data2", *data2)
                .with("alt", *alt)
                .with("confirmed", *confirmed),
            Command::JoinGroup { unit, head } => j.with("unit", *unit).with("head", *head),
            Command::AutoMission { unit } => j.with("unit", *unit),
            Command::PushOrder { city, order, data, alt, shift, ctrl } => {
                j.with("city", *city).with("order", Json::idx(*order)).with("data", *data).with("alt", *alt).with("shift", *shift).with("ctrl", *ctrl)
            }
            Command::PopOrder { city, index } => j.with("city", *city).with("index", *index),
            Command::DoTask { city, task, data1, data2, option, alt, shift, ctrl } => j
                .with("city", *city)
                .with("task", Json::idx(*task))
                .with("data1", *data1)
                .with("data2", *data2)
                .with("option", *option)
                .with("alt", *alt)
                .with("shift", *shift)
                .with("ctrl", *ctrl),
            Command::CultureChannel { city, action, target, religion } => {
                j.with("city", *city).with("action", Json::opt_idx(*action)).with("target", *target).with("religion", Json::opt_idx(*religion))
            }
            Command::ArmyForm { units } | Command::ArmySplit { units } | Command::ArmyGarrison { units } => j.with("units", ids(units)),
            Command::ArmyJoin { units, army } => j.with("units", ids(units)).with("army", *army),
            Command::ArmyRename { army, name } => j.with("army", *army).with("name", name.as_str()),
            Command::ArmyDisband { army } | Command::ArmyUpgrade { army } => j.with("army", *army),
            Command::ArmyTarget { army, size } => j.with("army", *army).with("size", *size),
            Command::ArmyRequisition { army, city, on } => j.with("army", *army).with("city", *city).with("on", *on),
            Command::ArmyRequisitionSet { army, cities } => j.with("army", *army).with("cities", Json::Arr(cities.iter().map(|&c| Json::from(c)).collect())),
            Command::RecruitDest { city, index, dest } => j.with("city", *city).with("index", *index).with("dest", recruit_dest_json(*dest)),
            Command::GovernDirect { city, instead_of } => j.with("city", *city).with("instead_of", *instead_of),
            Command::GovernHand { city } => j.with("city", *city),
            Command::GovernAssign { city, governor } => j.with("city", *city).with("governor", *governor),
            Command::Research { tech, shift } => j.with("tech", Json::opt_idx(*tech)).with("shift", *shift),
            Command::UpdateCivics { civics } => j.with("civics", Json::Arr(civics.iter().map(|&c| Json::idx(c)).collect())),
            Command::PercentChange { commerce, change } => j.with("commerce", Json::idx(*commerce)).with("change", *change),
            Command::Convert { religion } => j.with("religion", Json::opt_idx(*religion)),
            Command::DeclareWar { team } => j.with("team", *team),
            Command::ProposeDeal { to, give, get } => {
                let items = |v: &[TradeData]| Json::Arr(v.iter().map(crate::view::diplo::trade_item_json).collect());
                j.with("to", *to).with("give", items(give)).with("get", items(get))
            }
            Command::Event { triggered, event } => j.with("triggered", *triggered).with("event", Json::idx(*event)),
            Command::Vote { vote, choice } => j.with("vote", *vote).with("choice", choice.0),
            Command::Election { selection, option } => j.with("selection", *selection).with("option", *option),
            Command::SetOption { option, value } => j.with("option", Json::idx(*option)).with("value", *value),
            Command::AutoMoves => j,
            Command::EndTurn { force } => j.with("force", *force),
            Command::AutoPlay { turns } => j.with("turns", *turns),
        }
    }

    /// One line for people, with type names (`journal print`).
    pub fn describe(&self, d: &GameData) -> String {
        match self {
            Command::Move { units, x, y, append, declare_war } => {
                format!("move {units:?} to ({x}, {y}){}{}", if *append { " (append)" } else { "" }, match declare_war {
                    Some(true) => " declaring war",
                    Some(false) => " without war",
                    None => "",
                })
            }
            Command::Mission { units, mission, data1, data2, .. } => format!("{} {units:?} ({data1}, {data2})", names::name(d, *mission)),
            Command::DoCommand { units, command, data1, data2, alt, .. } => {
                format!("{} {units:?} ({data1}, {data2}){}", names::name(d, *command), if *alt { " for all of that type" } else { "" })
            }
            Command::PushOrder { city, order, data, .. } => format!("city {} {} {}", city.0, names::name(d, *order), data),
            Command::DoTask { city, task, data1, data2, option, .. } => format!("city {} {} ({data1}, {data2}, {option})", city.0, names::name(d, *task)),
            Command::CultureChannel { city, action, target, religion } => match (action, target) {
                (Some(a), Some(t)) => format!(
                    "city {} channels {} into player {}{}",
                    city.0,
                    names::name(d, *a),
                    t.0,
                    religion.map_or(String::new(), |r| format!(" ({})", names::name(d, r)))
                ),
                _ => format!("city {} channels nothing", city.0),
            },
            Command::Research { tech, shift } => {
                format!("research {}{}", tech.map_or("none".to_string(), |t| names::name(d, t).to_string()), if *shift { " (queue)" } else { "" })
            }
            Command::UpdateCivics { civics } => format!("civics {}", civics.iter().map(|&c| names::name(d, c)).collect::<Vec<_>>().join(", ")),
            Command::Event { triggered, event } => format!("event {} for triggered {}", names::name(d, *event), triggered.0),
            Command::Vote { vote, choice } => format!("vote {} on {}", names::player_vote_name(*choice).map_or_else(|| format!("team {}", choice.0), str::to_string), vote.0),
            Command::Election { selection, option } => {
                format!("election {}: {}", selection.0, option.map_or_else(|| "none".to_string(), |i| format!("resolution {i}")))
            }
            Command::SetOption { option, value } => format!("{} = {value}", names::name(d, *option)),
            Command::Convert { religion } => format!("convert to {}", religion.map_or("no religion".to_string(), |r| names::name(d, r).to_string())),
            Command::PercentChange { commerce, change } => format!("{} {change:+}%", names::name(d, *commerce)),
            Command::ProposeDeal { to, give, get } => {
                let items = |v: &[TradeData]| v.iter().map(|t| format!("{} {}", names::name(d, t.item_type), t.data)).collect::<Vec<_>>().join(", ");
                format!("offer player {}: give [{}] for [{}]", to.0, items(give), items(get))
            }
            other => other.to_json().to_string(),
        }
    }
}

/// [`Command::describe`] of a journal command object (its JSON as is when it does not decode).
pub fn describe(d: &GameData, j: &Json) -> String {
    match Command::from_json(d, j) {
        Ok(c) => c.describe(d),
        Err(_) => j.to_string(),
    }
}

/// A setup object with type names instead of indices (journal header for people).
pub fn names_in(d: &GameData, setup: &Json) -> String {
    match crate::setup::NewGame::from_json(d, setup) {
        Ok(ng) => {
            let mut s = format!("seed {} with {} opponents, {}, {}, {}", ng.seed, ng.opponents, names::name(d, ng.world), names::name(d, ng.speed), names::name(d, ng.handicap));
            if let Some(c) = ng.civ {
                let _ = write!(s, ", {}", names::name(d, c));
            }
            s
        }
        Err(_) => setup.to_string(),
    }
}

// ---------------------------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------------------------

/// What `validate` decided.
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    /// Change nothing and report success (the DECLAREWARMOVE popup's "no" when the units would
    /// not move anyway).
    Noop,
    /// A deal the AI declines (`propose_deal`): nothing changes, the reply says `accepted:
    /// false` (C++ sends no message).
    Declined,
    Confirm(Confirm),
    Apply(Action),
}

/// A validated command, with what validation resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `selectionListMove` after the DECLAREWARMOVE popup: declare war on `war` first.
    Move { units: Vec<UnitId>, x: i32, y: i32, append: bool, war: Option<TeamId> },
    Mission { units: Vec<UnitId>, mission: MissionType, data1: i32, data2: i32, flags: i32, append: bool },
    /// `CvNetDoCommand` per selected unit (`alt` already checked against `bAll`).
    DoCommand { units: Vec<UnitId>, command: CommandType, data1: i32, data2: i32, alt: bool },
    JoinGroup { unit: UnitId, head: Option<UnitId> },
    AutoMission { unit: UnitId },
    /// `pushOrder` with the flags `cityPushOrder` + `CvNetPushOrder` give.
    PushOrder { city: CityId, order: OrderType, data: i32, save: bool, pop: bool, append: bool },
    PopOrder { city: CityId, index: i32 },
    DoTask { city: CityId, task: TaskType, data1: i32, data2: i32, option: bool, alt: bool, shift: bool, ctrl: bool },
    CultureChannel { city: CityId, order: Option<(CulturalActionType, PlayerId, Option<ReligionType>)> },
    /// An army or governor command, checked by the rules call itself (they change nothing when
    /// they refuse).
    Army(Command),
    Research { tech: Option<TechType>, shift: bool },
    Revolution { civics: IdxVec<CivicOptionType, Option<CivicType>> },
    PercentChange { commerce: CommerceType, change: i32 },
    Convert { religion: Option<ReligionType> },
    DeclareWar { team: TeamId },
    /// The AI accepted: `implementDeal(h, to, give, get)`.
    ImplementDeal { to: PlayerId, give: Vec<TradeData>, get: Vec<TradeData> },
    Event { triggered: TriggeredId, event: EventType },
    Vote { vote: VoteId, choice: PlayerVote },
    /// CvNetChooseElection: `setVoteChosen(option, selection)`.
    Election { selection: VoteId, option: Option<usize> },
    SetOption { option: PlayerOptionType, value: bool },
    AutoMoves,
    /// `drop_events`: the forced end turn closes the open event popups; `close_elections`: and
    /// the open election popups (no resolution, as the popup's "none").
    EndTurn { drop_events: bool, close_elections: Vec<VoteId> },
    AutoPlay { turns: i32 },
}

/// What validation needs from the session.
pub struct Ctx<'a> {
    /// The human's deferred events (session order).
    pub events: &'a [TriggeredId],
}

/// A unit of the human's that takes orders: a marching recruit (this game's armies,
/// docs/ARMIES.md) takes none until it arrives, so every unit command, the automations and the
/// army commands refuse it, naming where it marches.
fn unit_of(g: &Game, h: PlayerId, u: UnitId) -> Result<(), CmdErr> {
    match g.units.get(u) {
        None => Err(CmdErr::Gone),
        Some(unit) if unit.owner != h => Err(CmdErr::NotOwner),
        Some(unit) => match g.unit_march_refusal(u) {
            Some(why) => Err(cannot(format!("the {} is {why}: it takes no orders until it arrives", names::display(g.d, unit.unit_type)))),
            None => Ok(()),
        },
    }
}

fn city_of(g: &Game, h: PlayerId, c: CityId) -> Result<(), CmdErr> {
    match g.cities.get(c) {
        None => Err(CmdErr::Gone),
        Some(city) if city.owner != h => Err(CmdErr::NotOwner),
        Some(_) => Ok(()),
    }
}

/// This game's governors are on.
fn governors_on(g: &Game) -> Result<(), CmdErr> {
    if g.governors_on() {
        Ok(())
    } else {
        Err(cannot("governors are off in this game"))
    }
}

/// A governed city's production is its governor's (docs/GOVERNORS.md).
fn not_governed(g: &Game, c: CityId) -> Result<(), CmdErr> {
    if g.city_is_governed(c) {
        Err(cannot("a governed city's production is its governor's"))
    } else {
        Ok(())
    }
}

/// An army of the human's.
fn army_of(g: &Game, h: PlayerId, a: ArmyId) -> Result<(), CmdErr> {
    match g.armies.get(a) {
        None => Err(CmdErr::Gone),
        Some(army) if army.owner != h => Err(CmdErr::NotOwner),
        Some(_) => Ok(()),
    }
}

/// Applies an army command (the rules check it too; a refusal changes nothing).
fn apply_army(g: &mut Game, h: PlayerId, c: &Command) -> Result<(), CmdErr> {
    let r: Result<(), &'static str> = match c {
        Command::ArmyForm { units } => g.army_form(h, units).map(|_| ()),
        Command::ArmySplit { units } => g.army_split(h, units).map(|_| ()),
        Command::ArmyJoin { units, army } => g.army_join(h, units, *army),
        Command::ArmyGarrison { units } => g.army_to_garrison(h, units),
        Command::ArmyRename { army, name } => g.army_rename(h, *army, name),
        Command::ArmyDisband { army } => g.army_disband(h, *army),
        Command::ArmyUpgrade { army } => g.army_upgrade_all(h, *army).map(|_| ()),
        Command::ArmyTarget { army, size } => g.army_set_target_size(h, *army, *size),
        Command::ArmyRequisition { army, city, on: true } => g.army_add_requisition(h, *army, *city),
        Command::ArmyRequisition { army, city, on: false } => g.army_remove_requisition(h, *army, *city),
        Command::ArmyRequisitionSet { army, cities } => g.army_set_requisition(h, *army, cities),
        Command::GovernDirect { city, instead_of } => g.player_set_direct_rule(h, *city, *instead_of),
        Command::GovernHand { city } => g.city_hand_to_governor(h, *city),
        Command::GovernAssign { city, governor } => g.city_assign_governor(h, *city, *governor),
        Command::RecruitDest { city, index, dest } => {
            if *index < 0 {
                g.cities[*city].recruit_default = *dest;
            } else if let Some(o) = g.cities[*city].order_queue.get_mut(*index as usize) {
                o.dest = *dest;
            }
            Ok(())
        }
        _ => Ok(()),
    };
    r.map_err(cannot)
}

/// The selection: non-empty, distinct, the human's, on one plot (the exe selects on one plot).
fn selection_of(g: &Game, h: PlayerId, units: &[UnitId]) -> Result<(), CmdErr> {
    if units.is_empty() {
        return Err(invalid("field `units`: empty selection"));
    }
    for (i, &u) in units.iter().enumerate() {
        unit_of(g, h, u)?;
        if units[..i].contains(&u) {
            return Err(invalid(format!("field `units`: unit {} twice", u.0)));
        }
    }
    let plot = g.unit_plot(units[0]);
    if units.iter().any(|&u| g.unit_plot(u) != plot) {
        return Err(invalid("field `units`: the selected units are on different plots"));
    }
    Ok(())
}

fn on_map(g: &Game, x: i32, y: i32) -> Result<PlotIdx, CmdErr> {
    let (w, ht) = (g.map.grid_width, g.map.grid_height);
    if x < 0 || y < 0 || x >= w || y >= ht {
        return Err(invalid(format!("plot ({x}, {y}) is off the map ({w} x {ht})")));
    }
    g.map.plot_xy(x, y).ok_or_else(|| invalid(format!("plot ({x}, {y}) is off the map")))
}

fn in_range(what: &str, v: i32, n: usize) -> Result<usize, CmdErr> {
    if v >= 0 && (v as usize) < n {
        Ok(v as usize)
    } else {
        Err(invalid(format!("{what} {v} out of range 0..{n}")))
    }
}

/// `data1`/`data2` of a mission, as the exe sends them: the target plot for the interface-mode
/// missions (go to, route to, airlift, nuke, recon, paradrop, air bomb, range attack); the type
/// of a build / religion / corporation / specialist / building action
/// (`CvActionInfo::getMissionData`) with `data2` -1; the LEADUNIT popup's unit id (or -1) for
/// LEAD; the espionage popup's values; and -1, -1 for every other mission (`handleAction` sends
/// `getMissionData()` = -1 and -1).
fn check_mission_data(g: &Game, m: MissionType, data1: i32, data2: i32) -> Result<(), CmdErr> {
    let d = g.d;
    let no_data2 = || if data2 == -1 { Ok(()) } else { Err(invalid(format!("field `data2`: must be -1 for this mission, got {data2}"))) };
    match m {
        MissionType::MISSION_MOVE_TO
        | MissionType::MISSION_ROUTE_TO
        | MissionType::MISSION_AIRLIFT
        | MissionType::MISSION_NUKE
        | MissionType::MISSION_RECON
        | MissionType::MISSION_PARADROP
        | MissionType::MISSION_AIRBOMB
        | MissionType::MISSION_RANGE_ATTACK => on_map(g, data1, data2).map(|_| ()),
        MissionType::MISSION_BUILD => in_range("build", data1, d.builds.len()).and_then(|_| no_data2()),
        MissionType::MISSION_SPREAD => in_range("religion", data1, d.religions.len()).and_then(|_| no_data2()),
        MissionType::MISSION_SPREAD_CORPORATION => in_range("corporation", data1, d.corporations.len()).and_then(|_| no_data2()),
        MissionType::MISSION_JOIN => in_range("specialist", data1, d.specialists.len()).and_then(|_| no_data2()),
        MissionType::MISSION_CONSTRUCT => in_range("building", data1, d.buildings.len()).and_then(|_| no_data2()),
        MissionType::MISSION_LEAD => {
            if data1 < -1 {
                return Err(invalid(format!("field `data1`: a unit id or -1, got {data1}")));
            }
            no_data2()
        }
        MissionType::MISSION_ESPIONAGE => Ok(()),
        _ if data1 != -1 => Err(invalid(format!("field `data1`: must be -1 for this mission, got {data1}"))),
        _ => no_data2(),
    }
}

/// Pure check of `cmd` for the human `h` at a control point.
pub fn validate(g: &Game, h: PlayerId, cmd: &Command, ctx: &Ctx) -> Result<Plan, CmdErr> {
    let d = g.d;
    let team = g.players[h].team;
    Ok(match cmd {
        Command::Move { units, x, y, append, declare_war } => {
            selection_of(g, h, units)?;
            let plot = on_map(g, *x, *y)?;
            // selectionListMove: the first selected unit whose move needs a war asks.
            let rival = units.iter().find_map(|&u| g.unit_get_declare_war_move(u, plot));
            let mut war = None;
            if let Some(rival) = rival {
                let move_anyway = g.plot_get_team(plot) != Some(rival);
                match declare_war {
                    None => return Ok(Plan::Confirm(Confirm::DeclareWar { team: rival, move_anyway })),
                    Some(true) => war = Some(rival),
                    Some(false) if move_anyway => {}
                    Some(false) => return Ok(Plan::Noop),
                }
            }
            if !selection::can_start_mission(g, units, MissionType::MISSION_MOVE_TO, *x, *y, selection::plot(g, units), false) {
                return Err(cannot("the units are already there"));
            }
            Plan::Apply(Action::Move { units: units.clone(), x: *x, y: *y, append: *append, war })
        }

        Command::Mission { units, mission, data1, data2, flags, append } => {
            selection_of(g, h, units)?;
            if !selection::is_player_mission(*mission) {
                return Err(invalid(format!("{} is not a mission a player gives", names::name(d, *mission))));
            }
            // Every exe path sends iFlags = 0 (handleAction, selectionListMove, DECLAREWARMOVE):
            // the movement flags (MOVE_THROUGH_ENEMY, MOVE_DIRECT_ATTACK, ...) are AI-only.
            if *flags != 0 {
                return Err(invalid(format!("field `flags`: must be 0 (the exe sends no movement flags), got {flags}")));
            }
            check_mission_data(g, *mission, *data1, *data2)?;
            if *mission == MissionType::MISSION_MOVE_TO {
                // Every exe path that sends MOVE_TO for a selection that may need a war goes
                // through selectionListMove (the GO_TO mode, the move flyout, a right click),
                // which asks first (DECLAREWARMOVE; air units never need one). The answer is
                // the `move` command with `declare_war` (P3 review: the mission moved without
                // asking).
                let plot = on_map(g, *data1, *data2)?;
                if let Some(rival) = units.iter().find_map(|&u| g.unit_get_declare_war_move(u, plot)) {
                    return Ok(Plan::Confirm(Confirm::DeclareWar { team: rival, move_anyway: g.plot_get_team(plot) != Some(rival) }));
                }
            }
            // canHandleAction's mission plot: with shift (append) the group's last mission plot
            // if visible, else where the selection stands.
            let mut plot = selection::plot(g, units);
            if let (true, Some(grp)) = (*append, selection::mirrors_group(g, units)) {
                if let Some(last) = g.group_last_mission_plot(grp) {
                    if g.plot_is_visible(last, team) {
                        plot = last;
                    }
                }
            }
            if !selection::can_start_mission(g, units, *mission, *data1, *data2, plot, false) {
                return Err(cannot(format!("{} is not possible", names::name(d, *mission))));
            }
            Plan::Apply(Action::Mission { units: units.clone(), mission: *mission, data1: *data1, data2: *data2, flags: *flags, append: *append })
        }

        Command::DoCommand { units, command, data1, data2, alt, confirmed } => {
            selection_of(g, h, units)?;
            match *command {
                CommandType::COMMAND_AUTOMATE => {
                    in_range("automate type", *data1, d.automates.len())?;
                    if AutomateType::from_usize(*data1 as usize) == AutomateType::AUTOMATE_TERRAFORMING {
                        return Err(cannot("automatic terraforming is not available (FfH's Python terraformer AI is not ported)"));
                    }
                }
                CommandType::COMMAND_AUTOMATE_SPELL => return Err(cannot("no FfH2 0.41 spell can be auto-cast")),
                CommandType::COMMAND_HOTKEY => return Err(invalid("COMMAND_HOTKEY is interface-only")),
                CommandType::COMMAND_PROMOTION => {
                    in_range("promotion", *data1, d.promotions.len())?;
                }
                CommandType::COMMAND_UPGRADE => {
                    in_range("unit type", *data1, d.units.len())?;
                }
                CommandType::COMMAND_CAST => {
                    in_range("spell", *data1, d.spells.len())?;
                }
                CommandType::COMMAND_LOAD_UNIT => {
                    in_range("player", *data1, g.num_players())?;
                }
                _ => {}
            }
            if *command == CommandType::COMMAND_LOAD {
                // handleAction: COMMAND_LOAD opens LOADUNIT, which loads into the only transport
                // or asks which one (launchLoadUnitPopup).
                if !selection::can_do_command(g, units, *command, *data1, *data2, false) {
                    return Err(cannot("nothing to load into"));
                }
                let transports = selection::load_targets(g, units);
                return Ok(match transports.len() {
                    0 => return Err(cannot("nothing to load into")),
                    1 => {
                        let t = transports[0];
                        Plan::Apply(Action::DoCommand {
                            units: units.clone(),
                            command: CommandType::COMMAND_LOAD_UNIT,
                            data1: g.units[t].owner.i(),
                            data2: t.0 as i32,
                            alt: false,
                        })
                    }
                    _ => Plan::Confirm(Confirm::LoadUnit { transports }),
                });
            }
            if !selection::can_do_command(g, units, *command, *data1, *data2, false) {
                return Err(cannot(format!("{} is not possible", names::name(d, *command))));
            }
            if !confirmed {
                if d.commands[*command].confirm_command {
                    return Ok(Plan::Confirm(Confirm::Command { command: *command }));
                }
                if *command == CommandType::COMMAND_CAST {
                    let spell = SpellType::from_usize(*data1 as usize);
                    if d.spells[spell].global {
                        return Ok(Plan::Confirm(Confirm::CastGlobal { spell }));
                    }
                    if d.spells[spell].causes_war {
                        if let Some(team) = cast_war_target(g, units[0], spell) {
                            return Ok(Plan::Confirm(Confirm::CastWar { spell, team }));
                        }
                    }
                }
            }
            let alt = *alt && d.commands[*command].all;
            Plan::Apply(Action::DoCommand { units: units.clone(), command: *command, data1: *data1, data2: *data2, alt })
        }

        Command::JoinGroup { unit, head } => {
            unit_of(g, h, *unit)?;
            if let Some(head) = head {
                unit_of(g, h, *head)?;
            }
            Plan::Apply(Action::JoinGroup { unit: *unit, head: *head })
        }

        Command::AutoMission { unit } => {
            unit_of(g, h, *unit)?;
            Plan::Apply(Action::AutoMission { unit: *unit })
        }

        Command::PushOrder { city, order, data, alt, shift, ctrl } => {
            city_of(g, h, *city)?;
            not_governed(g, *city)?;
            let c = *city;
            let ok = match *order {
                OrderType::ORDER_TRAIN => {
                    let u = UnitType::from_usize(in_range("unit type", *data, d.units.len())?);
                    g.city_offers_unit(c, u) && g.city_can_train(c, u, false, false, false, false)
                }
                OrderType::ORDER_CONSTRUCT => g.city_can_construct(c, BuildingType::from_usize(in_range("building", *data, d.buildings.len())?), false, false, false),
                OrderType::ORDER_CREATE => g.city_can_create(c, ProjectType::from_usize(in_range("project", *data, d.projects.len())?), false, false),
                OrderType::ORDER_MAINTAIN => g.city_can_maintain(c, ProcessType::from_usize(in_range("process", *data, d.processes.len())?), false),
                _ => return Err(invalid("field `order`: not a production order")),
            };
            if !ok {
                return Err(cannot("the city cannot build that"));
            }
            // CvGame::cityPushOrder: the flags it sends.
            let (shift, ctrl) = if g.city_production(c) > 0 {
                (*shift, !*shift)
            } else if *order == OrderType::ORDER_TRAIN && g.city_production_unit(c).map(|u| u.idx() as i32) == Some(*data) {
                (!*ctrl, *ctrl)
            } else {
                (*shift, *ctrl)
            };
            // CvNetPushOrder: pushOrder(eOrder, iData, -1, bAlt, !(bShift || bCtrl), bShift).
            Plan::Apply(Action::PushOrder { city: c, order: *order, data: *data, save: *alt, pop: !(shift || ctrl), append: shift })
        }

        Command::PopOrder { city, index } => {
            city_of(g, h, *city)?;
            not_governed(g, *city)?;
            let len = g.cities[*city].order_queue.len();
            in_range("queue index", *index, len)?;
            // selectedCitiesGameNetMessage(GAMEMESSAGE_POP_ORDER): only while the queue has more
            // than one order.
            if len <= 1 {
                return Err(cannot("the only order cannot be removed: push its replacement instead"));
            }
            // This game's armies: a requisition order is the army's, released on the Armies screen.
            g.city_check_remove_order(*city, *index as usize).map_err(cannot)?;
            Plan::Apply(Action::PopOrder { city: *city, index: *index })
        }

        Command::CultureChannel { city, action, target, religion } => {
            city_of(g, h, *city)?;
            if !g.culture_actions_on() {
                return Err(cannot("cultural actions are off in this game"));
            }
            let order = match (action, target) {
                (None, _) => None,
                (Some(_), None) => return Err(invalid("field `target`: a channel needs a civilization")),
                (Some(a), Some(t)) => {
                    in_range("cultural action", a.idx() as i32, d.cultural_actions.len())?;
                    in_range("player", t.0 as i32, g.num_players())?;
                    if let Some(r) = religion {
                        in_range("religion", r.idx() as i32, d.religions.len())?;
                    }
                    g.culture_channel_check(*city, *a, *t, *religion).map_err(|r| cannot(r.describe()))?;
                    Some((*a, *t, *religion))
                }
            };
            Plan::Apply(Action::CultureChannel { city: *city, order })
        }

        Command::ArmyForm { units } | Command::ArmySplit { units } | Command::ArmyGarrison { units } => {
            if !g.armies_on() {
                return Err(cannot("armies are off in this game"));
            }
            if units.is_empty() {
                return Err(invalid("field `units`: none"));
            }
            for &u in units {
                unit_of(g, h, u)?;
            }
            let plot = g.army_units_plot(h, units).map_err(cannot)?;
            if matches!(cmd, Command::ArmyGarrison { .. }) && !g.map.plots[plot].plot_city.is_some_and(|c| g.cities.contains(c) && g.cities[c].owner == h) {
                return Err(cannot("not in one of your cities"));
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::ArmyJoin { units, army } => {
            army_of(g, h, *army)?;
            if units.is_empty() {
                return Err(invalid("field `units`: none"));
            }
            for &u in units {
                unit_of(g, h, u)?;
            }
            let plot = g.army_units_plot(h, units).map_err(cannot)?;
            if g.army_plot(*army) != Some(plot) {
                return Err(cannot("the army is not on their plot"));
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::ArmyRename { army, name } => {
            army_of(g, h, *army)?;
            if name.trim().is_empty() || name.trim().chars().count() > 40 {
                return Err(cannot("a name of 1 to 40 characters"));
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::ArmyDisband { army } | Command::ArmyUpgrade { army } | Command::ArmyTarget { army, .. } => {
            army_of(g, h, *army)?;
            if matches!(cmd, Command::ArmyDisband { .. }) && (g.army_plot(*army).is_none() || g.players[h].cities.is_empty()) {
                return Err(cannot("you have no city for its units"));
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::ArmyRequisition { army, city, on } => {
            army_of(g, h, *army)?;
            city_of(g, h, *city)?;
            let listed = g.armies[*army].requisition.contains(city);
            if *on {
                g.army_check_requisition_city(h, *army, *city).map_err(cannot)?;
                if listed {
                    return Err(cannot("the army already requisitions from this city"));
                }
            } else if !listed {
                return Err(cannot("the army does not requisition from this city"));
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::ArmyRequisitionSet { army, cities } => {
            army_of(g, h, *army)?;
            g.army_check_requisition(h, *army).map_err(cannot)?;
            for (i, c) in cities.iter().enumerate() {
                city_of(g, h, *c)?;
                if cities[..i].contains(c) {
                    return Err(invalid(format!("field `cities`: city {} twice", c.0)));
                }
                g.army_check_requisition_city(h, *army, *c).map_err(cannot)?;
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::RecruitDest { city, index, dest } => {
            city_of(g, h, *city)?;
            not_governed(g, *city)?;
            if !g.armies_on() {
                return Err(cannot("armies are off in this game"));
            }
            if *index >= 0 {
                let i = in_range("queue index", *index, g.cities[*city].order_queue.len())?;
                let o = &g.cities[*city].order_queue[i];
                let is_army_unit = o.order_type == OrderType::ORDER_TRAIN && (o.data1 as usize) < d.units.len() && g.unit_type_is_army_unit(UnitType::from_usize(o.data1 as usize));
                if !is_army_unit {
                    return Err(cannot("only land combat units are recruited to a destination"));
                }
                if o.requisition {
                    return Err(cannot("a requisition order goes to its army"));
                }
            } else if *index != -1 {
                return Err(invalid("field `index`: an order's index, or -1 for the city's default"));
            }
            if let Some(dst) = dest {
                if !g.recruit_dest_valid(h, *dst) {
                    return Err(cannot("no such army or city of yours"));
                }
            }
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::GovernDirect { city, instead_of } => {
            city_of(g, h, *city)?;
            if let Some(o) = instead_of {
                city_of(g, h, *o)?;
            }
            governors_on(g)?;
            g.player_check_direct_rule(h, *city, *instead_of).map_err(cannot)?;
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::GovernHand { city } => {
            city_of(g, h, *city)?;
            governors_on(g)?;
            g.city_check_hand_to_governor(h, *city).map_err(cannot)?;
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::GovernAssign { city, governor } => {
            city_of(g, h, *city)?;
            governors_on(g)?;
            match g.governors.get(*governor) {
                None => return Err(CmdErr::Gone),
                Some(gov) if gov.owner != h => return Err(CmdErr::NotOwner),
                Some(_) => {}
            }
            if !g.city_is_governed(*city) {
                return Err(cannot("the city is ruled directly"));
            }
            g.city_check_assign_governor(h, *city, *governor).map_err(cannot)?;
            Plan::Apply(Action::Army(cmd.clone()))
        }

        Command::DoTask { city, task, data1, data2, option, alt, shift, ctrl } => {
            city_of(g, h, *city)?;
            let c = *city;
            match *task {
                TaskType::TASK_RAZE | TaskType::TASK_DISBAND | TaskType::TASK_GIFT => {
                    return Err(invalid(format!("{} is only sent by its popup", names::name(d, *task))));
                }
                TaskType::TASK_HURRY => {
                    let hurry = HurryType::from_usize(in_range("hurry", *data1, d.hurries.len())?);
                    if !g.city_can_hurry(c, hurry, false) {
                        return Err(cannot("cannot hurry"));
                    }
                }
                TaskType::TASK_CONSCRIPT => {
                    if !g.city_can_conscript(c) {
                        return Err(cannot("cannot draft"));
                    }
                }
                TaskType::TASK_SET_EMPHASIZE => {
                    in_range("emphasize", *data1, d.emphasizes.len())?;
                    if g.city_is_governed(c) {
                        return Err(cannot("a governed city's emphasis is its governor's"));
                    }
                }
                TaskType::TASK_CHANGE_SPECIALIST => {
                    // The city screen's specialist +/- buttons (WIDGET_CHANGE_SPECIALIST with
                    // iData2 = 1 or -1, CvMainInterface.py), shown as updateCitizenButtons does.
                    let s = SpecialistType::from_usize(in_range("specialist", *data1, d.specialists.len())?);
                    let (plus, minus) = specialist_buttons(g, c, s);
                    match *data2 {
                        1 if !plus => return Err(cannot(format!("no more {} possible", names::name(d, s)))),
                        -1 if !minus => return Err(cannot(format!("no {} to remove", names::name(d, s)))),
                        1 | -1 => {}
                        other => return Err(invalid(format!("field `data2`: the change is 1 or -1 (the +/- buttons), got {other}"))),
                    }
                }
                TaskType::TASK_CHANGE_WORKING_PLOT | TaskType::TASK_CLEAR_WORKING_OVERRIDE => {
                    // handleCityScreenPlotPicked / RightPicked: a plot of the city that we own.
                    let i = in_range("city plot index", *data1, g.city_num_city_plots(c) as usize)?;
                    let owned = g.city_index_plot(c, i).is_some_and(|p| g.map.plots[p].owner == Some(h));
                    if !owned {
                        return Err(cannot("not a plot of ours"));
                    }
                }
                TaskType::TASK_LIBERATE => {
                    // The liberate button and the FREE_COLONY popup send iData1 = 0
                    // (`liberate(bConquest = false)`); 1 is the conquest path of acquireCity.
                    if *data1 != 0 {
                        return Err(invalid(format!("field `data1`: must be 0 for TASK_LIBERATE, got {data1}")));
                    }
                    if g.city_liberation_player(c, false).is_none() {
                        return Err(cannot("nobody to liberate the city to"));
                    }
                }
                TaskType::TASK_RALLY_PLOT => {
                    on_map(g, *data1, *data2)?;
                }
                TaskType::TASK_SET_AUTOMATED_PRODUCTION => not_governed(g, c)?,
                TaskType::TASK_CLEAR_ORDERS => {
                    not_governed(g, c)?;
                    // Clearing the queue would take its requisition orders off too.
                    for i in 0..g.cities[c].order_queue.len() {
                        g.city_check_remove_order(c, i).map_err(cannot)?;
                    }
                }
                TaskType::TASK_SET_AUTOMATED_CITIZENS | TaskType::TASK_CLEAR_RALLY_PLOT => {}
                _ => return Err(invalid("field `task`: unknown task")),
            }
            Plan::Apply(Action::DoTask { city: c, task: *task, data1: *data1, data2: *data2, option: *option, alt: *alt, shift: *shift, ctrl: *ctrl })
        }

        Command::Research { tech, shift } => {
            if let Some(t) = tech {
                if !g.player_can_ever_research(h, *t) {
                    return Err(cannot(format!("{} cannot be researched", names::name(d, *t))));
                }
            }
            Plan::Apply(Action::Research { tech: *tech, shift: *shift })
        }

        Command::UpdateCivics { civics } => {
            let mut v = g.players[h].civics.clone();
            let mut seen: Vec<CivicOptionType> = Vec::new();
            for &c in civics {
                let o = d.civics[c].civic_option_type.ok_or_else(|| invalid(format!("{} has no civic option", names::name(d, c))))?;
                if seen.contains(&o) {
                    return Err(invalid(format!("two civics for {}", names::name(d, o))));
                }
                seen.push(o);
                if v[o] != Some(c) && !g.player_can_do_civics(h, Some(c)) {
                    return Err(cannot(format!("{} is not available", names::name(d, c))));
                }
                v[o] = Some(c);
            }
            if !g.player_can_revolution(h, Some(&v)) {
                return Err(cannot("no revolution possible (anarchy, the revolution timer, a forced civic, or no change)"));
            }
            Plan::Apply(Action::Revolution { civics: v })
        }

        Command::PercentChange { commerce, change } => {
            // The slider buttons send +/- COMMERCE_PERCENT_CHANGE_INCREMENTS (CvMainInterface.py).
            let step = d.defines.commerce_percent_change_increments;
            if *change != step && *change != -step {
                return Err(invalid(format!("field `change`: must be {step} or -{step} (COMMERCE_PERCENT_CHANGE_INCREMENTS), got {change}")));
            }
            if !g.player_is_commerce_flexible(h, *commerce) {
                return Err(cannot(format!("{} is not adjustable", names::name(d, *commerce))));
            }
            Plan::Apply(Action::PercentChange { commerce: *commerce, change: *change })
        }

        Command::Convert { religion } => {
            if !g.player_can_convert(h, *religion) {
                return Err(cannot("cannot convert"));
            }
            Plan::Apply(Action::Convert { religion: *religion })
        }

        Command::DeclareWar { team: t } => {
            in_range("team", t.0 as i32, g.num_teams())?;
            if !g.team_can_declare_war(team, *t) {
                return Err(cannot(format!("cannot declare war on team {}", t.0)));
            }
            Plan::Apply(Action::DeclareWar { team: *t })
        }

        Command::ProposeDeal { to, give, get } => {
            // The diplomacy screen offers only the trade tables' items (`view::diplo`); the AI
            // answers with `AI_considerOffer(eHuman, pTheirList = give, pOurList = get)`, which
            // the exe asks before it sends the implementDeal message.
            crate::view::diplo::check_deal(g, h, *to, give, get)?;
            if !g.controller(*to).consider_offer(g, *to, h, give, get) {
                return Ok(Plan::Declined);
            }
            Plan::Apply(Action::ImplementDeal { to: *to, give: give.clone(), get: get.clone() })
        }

        Command::Event { triggered, event } => {
            if !ctx.events.contains(triggered) {
                return Err(CmdErr::Gone);
            }
            let Some(data) = g.player_get_event_triggered(h, *triggered) else { return Err(CmdErr::Gone) };
            if !d.event_triggers[data.trigger].events.contains(event) {
                return Err(invalid(format!("{} is not an option of this event", names::name(d, *event))));
            }
            if !g.player_can_do_event(h, *event, data) {
                return Err(cannot(format!("{} is not available", names::name(d, *event))));
            }
            Plan::Apply(Action::Event { triggered: *triggered, event: *event })
        }

        Command::Vote { vote, choice } => {
            let Some(pending) = decision::pending_votes(g, h).into_iter().find(|v| v.vote == *vote) else {
                return Err(if g.vote_triggered(*vote).is_some() { cannot("this vote is not waiting for you") } else { CmdErr::Gone });
            };
            if !pending.choices.contains(choice) {
                return Err(cannot(format!("choice {} is not offered for this vote", choice.0)));
            }
            Plan::Apply(Action::Vote { vote: *vote, choice: *choice })
        }

        Command::Election { selection, option } => {
            // The human's open election popups are the selections ffh-core kept for it.
            let Some(e) = decision::pending_elections(g, h).into_iter().find(|e| e.selection == *selection) else {
                return Err(CmdErr::Gone);
            };
            if let Some(i) = option {
                if *i >= e.options.len() {
                    return Err(invalid(format!("field `option`: the election has {} resolutions (0..{}), or none", e.options.len(), e.options.len())));
                }
            }
            Plan::Apply(Action::Election { selection: *selection, option: *option })
        }

        Command::SetOption { option, value } => {
            const ALLOWED: [PlayerOptionType; 5] = [
                PlayerOptionType::PLAYEROPTION_STACK_ATTACK,
                PlayerOptionType::PLAYEROPTION_QUICK_MOVES,
                PlayerOptionType::PLAYEROPTION_SAFE_AUTOMATION,
                PlayerOptionType::PLAYEROPTION_NO_UNIT_CYCLING,
                PlayerOptionType::PLAYEROPTION_WAIT_END_TURN,
            ];
            if !ALLOWED.contains(option) {
                return Err(cannot(format!("{} cannot be changed here", names::name(d, *option))));
            }
            Plan::Apply(Action::SetOption { option: *option, value: *value })
        }

        Command::AutoMoves => Plan::Apply(Action::AutoMoves),

        Command::EndTurn { force } => {
            let p = decision::pending(g, h, ctx.events);
            let ready = p.ready_groups.len();
            if !force && (p.popups() > 0 || ready > 0) {
                return Ok(Plan::Confirm(Confirm::EndTurn {
                    events: p.events.len(),
                    votes: p.votes.len(),
                    elections: p.elections.len(),
                    research: p.research_needed,
                    ready_units: ready,
                }));
            }
            Plan::Apply(Action::EndTurn { drop_events: !p.events.is_empty(), close_elections: p.elections.iter().map(|e| e.selection).collect() })
        }

        Command::AutoPlay { turns } => {
            if !(1..=10_000).contains(turns) {
                // 0 is the stop, sent while the AI autoplay runs (transport `running_reply`).
                return Err(invalid("field `turns`: expected 1..=10000 (0 stops a running autoplay)"));
            }
            Plan::Apply(Action::AutoPlay { turns: *turns })
        }
    })
}

/// The city screen's specialist buttons for `s` in city `c` (CvMainInterface.py
/// `updateCitizenButtons`): `(plus, minus)`, whether the + and - buttons are shown, which is
/// what `do_task TASK_CHANGE_SPECIALIST` with `data2` 1 / -1 accepts. + needs
/// `isSpecialistValid(s, 1)` and room (automated citizens, or fewer than the population plus
/// the free specialists); - a specialist to remove (with automated citizens the larger of the
/// count and the forced count).
pub fn specialist_buttons(g: &Game, c: CityId, s: SpecialistType) -> (bool, bool) {
    let city = &g.cities[c];
    let count = if city.citizens_automated { city.specialist_count[s].max(city.force_specialist_count[s]) } else { city.specialist_count[s] };
    let room = city.citizens_automated || count < city.population + g.city_total_free_specialists(c);
    (g.city_is_specialist_valid(c, s, 1) && room, count > 0)
}

/// `handleAction`'s war check for a `bCausesWar` spell (FfH, Sephi's area rule): a unit of
/// another team on a plot of that team within range, not our vassal and not at war with us; or,
/// with range 0, our plot is owned by a team we are not at war with. Returns that team.
fn cast_war_target(g: &Game, head: UnitId, spell: SpellType) -> Option<TeamId> {
    let range = g.d.spells[spell].range;
    let team = g.unit_get_team(head);
    let here = g.unit_plot(head);
    let here_team = g.plot_get_team(here);
    let (x, y) = (g.units[head].x, g.units[head].y);
    let mut target = None;
    for dx in -range..=range {
        for dy in -range..=range {
            let Some(p) = g.plot_xy(x, y, dx, dy) else { continue };
            for &u in &g.map.plots[p].units {
                let t = g.unit_get_team(u);
                if t != team && Some(t) == here_team && !g.teams[t].vassal[team] && !g.teams[t].at_war[team] {
                    target.get_or_insert(t);
                }
            }
        }
    }
    if let Some(t) = target {
        return Some(t);
    }
    if range == 0 {
        if let Some(t) = here_team {
            if t != team && !g.teams[t].at_war[team] {
                return Some(t);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// Application: the CvNet*::Execute ports
// ---------------------------------------------------------------------------------------------

/// `selectionListGameNetMessage(GAMEMESSAGE_JOIN_GROUP)` unless the selection mirrors a group:
/// the head leaves its group for a new one and the others join it. Returns the group.
fn join_selection(g: &mut Game, units: &[UnitId]) -> Option<GroupId> {
    if let Some(grp) = selection::mirrors_group(g, units) {
        return Some(grp);
    }
    let head = units[0];
    // CvNetJoinGroup(head, -1): joinGroup(NULL) makes a new group; then joinGroup(head's group)
    // for every selected unit (the head's own message is a no-op).
    g.unit_join_group(head, None, true);
    let grp = g.units.get(head)?.group?;
    for &u in &units[1..] {
        if g.units.contains(u) {
            g.unit_join_group(u, Some(grp), true);
        }
    }
    g.units.get(head)?.group
}

/// Applies a validated action for `h`.
pub fn apply(g: &mut Game, h: PlayerId, action: &Action) {
    match action {
        Action::Move { units, x, y, append, war } => {
            if let Some(t) = war {
                // DECLAREWARMOVE OnOk button 0: CvNetChangeWar -> declareWar(eRival, false, NO_WARPLAN).
                let team = g.players[h].team;
                g.team_declare_war(team, *t, false, None);
            }
            // selectionListGameNetMessage(PUSH_MISSION, MISSION_MOVE_TO, x, y, 0, false, bShift).
            if let Some(grp) = join_selection(g, units) {
                g.group_push_mission(grp, MissionType::MISSION_MOVE_TO, *x, *y, 0, *append, true, None, None, None);
            }
        }
        Action::Mission { units, mission, data1, data2, flags, append } => {
            // CvNetPushMission: pushMission(eMission, iData1, iData2, iFlags, bShift, true).
            if let Some(grp) = join_selection(g, units) {
                g.group_push_mission(grp, *mission, *data1, *data2, *flags, *append, true, None, None, None);
            }
        }
        Action::DoCommand { units, command, data1, data2, alt } => {
            // One CvNetDoCommand per selected unit.
            for &u in units {
                if !g.units.get(u).is_some_and(|unit| unit.owner == h) {
                    continue;
                }
                if *alt {
                    // UNOFFICIAL_PATCH: the type is read before the loop.
                    let unit_type = g.units[u].unit_type;
                    for lu in g.players[h].units.clone() {
                        // (not a marching recruit: it takes no orders, docs/ARMIES.md)
                        if g.units.get(lu).is_some_and(|unit| unit.unit_type == unit_type && unit.march.is_none()) {
                            g.unit_do_command(lu, *command, *data1, *data2);
                        }
                    }
                } else {
                    g.unit_do_command(u, *command, *data1, *data2);
                }
            }
        }
        Action::JoinGroup { unit, head } => {
            // CvNetJoinGroup: joinGroup(pHeadUnit->getGroup()) or joinGroup(NULL).
            let grp = head.and_then(|hd| g.units.get(hd)).and_then(|hu| hu.group);
            g.unit_join_group(*unit, grp, true);
        }
        Action::AutoMission { unit } => {
            if let Some(grp) = g.units[*unit].group {
                g.group_auto_mission(grp);
            }
        }
        Action::PushOrder { city, order, data, save, pop, append } => {
            g.city_push_order(*city, *order, *data, -1, *save, *pop, *append, false);
        }
        Action::PopOrder { city, index } => g.city_pop_order(*city, *index, false, false),
        Action::CultureChannel { city, order } => {
            // Validated just before: the check passed.
            let _ = g.city_set_culture_channel(*city, *order);
        }
        // Validated just before (`validate`); a refusal in the rules would change nothing.
        Action::Army(c) => {
            let _ = apply_army(g, h, c);
        }
        Action::DoTask { city, task, data1, data2, option, alt, shift, ctrl } => {
            g.city_do_task(*city, *task, *data1, *data2, *option, *alt, *shift, *ctrl);
        }
        Action::Research { tech, shift } => {
            // CvNetResearch, iDiscover == 0.
            match tech {
                None => g.player_clear_research_queue(h),
                Some(t) => {
                    let team = g.players[h].team;
                    if (g.team_is_has_tech(team, Some(*t)) || g.players[h].researching_tech[*t]) && !shift {
                        g.player_clear_research_queue(h);
                    }
                    g.player_push_research(h, *t, !shift);
                }
            }
        }
        Action::Revolution { civics } => g.player_revolution(h, civics, false),
        Action::PercentChange { commerce, change } => g.player_change_commerce_percent(h, *commerce, *change),
        Action::Convert { religion } => g.player_convert(h, *religion),
        Action::DeclareWar { team } => {
            let my = g.players[h].team;
            g.team_declare_war(my, *team, false, None);
        }
        // gDLL->sendImplementDealMessage -> CvGame::implementDeal(eWho, eOtherWho, .., bForce
        // = false).
        Action::ImplementDeal { to, give, get } => g.implement_deal(h, *to, give, get, false),
        Action::Event { triggered, event } => g.player_apply_event(h, *event, *triggered, true),
        Action::Vote { vote, choice } => g.cast_vote(h, *vote, *choice),
        // CvNetChooseElection::Execute: setVoteChosen(iSelection, iVoteId). The popup's "none"
        // sends nothing in C++ and leaves the selection unused; deleting it here is the same.
        Action::Election { selection, option } => g.set_vote_chosen(*option, *selection),
        Action::SetOption { option, value } => g.players[h].options[*option] = *value,
        Action::AutoMoves => g.player_set_auto_moves(h, true),
        Action::EndTurn { close_elections, .. } => {
            for &selection in close_elections {
                g.set_vote_chosen(None, selection);
            }
            g.player_set_end_turn(h, true)
        }
        Action::AutoPlay { turns } => g.set_ai_auto_play(*turns),
    }
    // This game's armies follow the groups the command changed (docs/ARMIES.md).
    g.player_army_sync(h);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let d = ffh_data::global();
        let cmds = vec![
            Command::Move { units: vec![UnitId(3), UnitId(9)], x: 4, y: 5, append: true, declare_war: Some(false) },
            Command::Move { units: vec![UnitId(3)], x: 4, y: 5, append: false, declare_war: None },
            Command::Mission { units: vec![UnitId(1)], mission: MissionType::MISSION_BUILD, data1: 2, data2: -1, flags: 0, append: false },
            Command::DoCommand { units: vec![UnitId(1)], command: CommandType::COMMAND_CAST, data1: 7, data2: -1, alt: true, confirmed: false },
            Command::JoinGroup { unit: UnitId(1), head: None },
            Command::AutoMission { unit: UnitId(2) },
            Command::PushOrder { city: CityId(1), order: OrderType::ORDER_TRAIN, data: 4, alt: false, shift: true, ctrl: false },
            Command::PopOrder { city: CityId(1), index: 0 },
            Command::DoTask { city: CityId(1), task: TaskType::TASK_HURRY, data1: 0, data2: -1, option: false, alt: false, shift: false, ctrl: false },
            Command::CultureChannel { city: CityId(1), action: Some(CulturalActionType(1)), target: Some(PlayerId(2)), religion: Some(ReligionType(1)) },
            Command::CultureChannel { city: CityId(1), action: Some(CulturalActionType(0)), target: Some(PlayerId(3)), religion: None },
            Command::CultureChannel { city: CityId(1), action: None, target: None, religion: None },
            Command::ArmyForm { units: vec![UnitId(3), UnitId(4)] },
            Command::ArmySplit { units: vec![UnitId(3)] },
            Command::ArmyJoin { units: vec![UnitId(3)], army: ArmyId(2) },
            Command::ArmyGarrison { units: vec![UnitId(5)] },
            Command::ArmyRename { army: ArmyId(1), name: "The Iron Host".into() },
            Command::ArmyDisband { army: ArmyId(1) },
            Command::ArmyUpgrade { army: ArmyId(1) },
            Command::ArmyTarget { army: ArmyId(1), size: 8 },
            Command::ArmyRequisition { army: ArmyId(1), city: CityId(4), on: true },
            Command::ArmyRequisition { army: ArmyId(1), city: CityId(4), on: false },
            Command::ArmyRequisitionSet { army: ArmyId(1), cities: vec![CityId(4), CityId(2)] },
            Command::ArmyRequisitionSet { army: ArmyId(1), cities: vec![] },
            Command::RecruitDest { city: CityId(1), index: 0, dest: Some(ffh_core::army::RecruitDest::Army(ArmyId(2))) },
            Command::RecruitDest { city: CityId(1), index: -1, dest: Some(ffh_core::army::RecruitDest::Garrison(CityId(3))) },
            Command::RecruitDest { city: CityId(1), index: -1, dest: None },
            Command::GovernDirect { city: CityId(1), instead_of: Some(CityId(2)) },
            Command::GovernDirect { city: CityId(1), instead_of: None },
            Command::GovernHand { city: CityId(3) },
            Command::GovernAssign { city: CityId(3), governor: GovernorId(2) },
            Command::Research { tech: None, shift: false },
            Command::Research { tech: Some(TechType(3)), shift: true },
            Command::UpdateCivics { civics: vec![CivicType(1), CivicType(5)] },
            Command::PercentChange { commerce: CommerceType::COMMERCE_GOLD, change: -10 },
            Command::Convert { religion: Some(ReligionType(2)) },
            Command::DeclareWar { team: TeamId(2) },
            Command::ProposeDeal {
                to: PlayerId(1),
                give: vec![TradeData { item_type: TradeableItem::TRADE_PEACE_TREATY, data: 0, offering: false, hidden: false }],
                get: vec![
                    TradeData { item_type: TradeableItem::TRADE_PEACE_TREATY, data: 0, offering: false, hidden: false },
                    TradeData { item_type: TradeableItem::TRADE_GOLD, data: 30, offering: false, hidden: false },
                ],
            },
            Command::Event { triggered: TriggeredId(4), event: EventType(10) },
            Command::Vote { vote: VoteId(1), choice: PlayerVote::PLAYER_VOTE_NEVER },
            Command::Vote { vote: VoteId(1), choice: PlayerVote(3) },
            Command::Election { selection: VoteId(2), option: Some(1) },
            Command::Election { selection: VoteId(2), option: None },
            Command::SetOption { option: PlayerOptionType::PLAYEROPTION_QUICK_MOVES, value: false },
            Command::AutoMoves,
            Command::EndTurn { force: true },
            Command::AutoPlay { turns: 3 },
        ];
        let mut seen: Vec<&str> = Vec::new();
        for c in &cmds {
            let j = c.to_json();
            assert_eq!(&Command::from_json(d, &j).unwrap(), c, "{j}");
            let j2 = Json::parse(&j.to_string()).unwrap();
            assert_eq!(Command::from_json(d, &j2).unwrap().to_json(), j);
            assert!(!c.describe(d).is_empty());
            if !seen.contains(&c.name()) {
                seen.push(c.name());
            }
        }
        // Every command name is covered.
        assert_eq!(seen, COMMAND_NAMES);
    }

    #[test]
    fn decoding_accepts_names_and_reports_fields() {
        let d = ffh_data::global();
        let j = Json::parse(r#"{"id":1,"c":"mission","units":[5],"mission":"MISSION_BUILD","data1":"BUILD_ROAD"}"#).unwrap();
        let Command::Mission { mission, data1, data2, .. } = Command::from_json(d, &j).unwrap() else { panic!() };
        assert_eq!(mission, MissionType::MISSION_BUILD);
        assert_eq!(data1, d.t::<BuildType>("BUILD_ROAD").idx() as i32);
        assert_eq!(data2, -1);
        let j = Json::parse(r#"{"c":"push_order","city":1,"order":"ORDER_TRAIN","data":"UNIT_WARRIOR"}"#).unwrap();
        let Command::PushOrder { data, .. } = Command::from_json(d, &j).unwrap() else { panic!() };
        assert_eq!(data, d.t::<UnitType>("UNIT_WARRIOR").idx() as i32);
        let j = Json::parse(r#"{"c":"vote","vote":1,"choice":"NO_PLAYER_VOTE"}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("field `choice`")));
        let j = Json::parse(r#"{"c":"move","units":[1],"x":"a","y":2}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("field `x`")));
        let j = Json::parse(r#"{"c":"fly"}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("unknown command")));
        let j = Json::parse(r#"{"c":"propose_deal"}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("field `to`")));
        let j = Json::parse(r#"{"c":"propose_deal","to":1,"give":[{"item":"TRADE_GOLD","data":5}]}"#).unwrap();
        let Command::ProposeDeal { give, get, .. } = Command::from_json(d, &j).unwrap() else { panic!() };
        assert_eq!((give[0].item_type, give[0].data, get.len()), (TradeableItem::TRADE_GOLD, 5, 0));
        let j = Json::parse(r#"{"c":"propose_deal","to":1,"give":[{"item":"TRADE_NOTHING"}]}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("field `give`")));
        let j = Json::parse(r#"{"c":"propose_deal","to":1,"get":{"item":0}}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("field `get`")));
        // A misspelt member is refused, not taken for its default (P4 review); the envelope's
        // `id` and `k` are allowed.
        let j = Json::parse(r#"{"c":"end_turn","forse":true}"#).unwrap();
        assert!(matches!(Command::from_json(d, &j), Err(CmdErr::Invalid(s)) if s.contains("field `forse`: unknown field")));
        let j = Json::parse(r#"{"id":"x","k":"cmd","c":"end_turn","force":true}"#).unwrap();
        assert_eq!(Command::from_json(d, &j), Ok(Command::EndTurn { force: true }));
        let names: Vec<&str> = COMMAND_FIELDS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, COMMAND_NAMES);
    }
}
