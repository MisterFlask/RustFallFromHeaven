//! Why the rules refuse a unit order, as sentences: one function per refusal enum of
//! ffh-core's order gates (`unit/order_refusal.rs`), each an exhaustive `match` naming the check
//! that failed, bare (CLAUDE.md, "Rules text and tooltips"). Called from `reasons.rs`
//! (`build_refusal`, `mission_refusal`, `command_refusal`).

use std::borrow::Cow;

use crate::names::display;
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::unit::{
    AcquireRefusal, AutomateRefusal, BuildRefusal, GiftRefusal, ImprovementRefusal, LoadRefusal, MissionRefusal, PlotBuildRefusal, PromoteRefusal, ScrapRefusal,
    UnloadRefusal, UpgradeRefusal,
};
use ffh_data::*;

/// `A, B or C`.
fn or_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {}", rest.join(", "), last),
    }
}

/// `Gold: 40 of 120.`
fn gold_line(have: i32, need: i32) -> String {
    format!("Gold: {} of {}.", have.max(0), need)
}

/// A unit's type name.
fn unit_label(g: &Game, u: UnitId) -> Cow<'static, str> {
    display(g.d, g.units[u].unit_type)
}

// ------------------------------------------------------------------------------------ builds

/// A build's name without its markup (`Build a [LINK=..]Road[\LINK]` -> `Build a Road`).
fn build_label(d: &GameData, b: BuildType) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in display(d, b).chars() {
        match c {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// What makes improvement `imp` valid on a plot (its hills, fresh water, river side, terrains,
/// features and bonuses), as names.
fn improvement_sites(d: &GameData, imp: ImprovementType) -> Vec<String> {
    let info = &d.improvements[imp];
    let mut out: Vec<String> = Vec::new();
    out.extend(d.terrains.keys().filter(|&t| info.terrain_makes_valid[t]).map(|t| display(d, t).into_owned()));
    out.extend(d.features.keys().filter(|&f| info.feature_makes_valid[f]).map(|f| display(d, f).into_owned()));
    if info.hills_makes_valid {
        out.push("hills".to_string());
    }
    if info.fresh_water_makes_valid {
        out.push("fresh water".to_string());
    }
    if info.river_side_makes_valid {
        out.push("a river".to_string());
    }
    out.extend(d.bonuses.keys().filter(|&b| info.improvement_bonus[b].bonus_makes_valid).map(|b| display(d, b).into_owned()));
    out
}

/// Why improvement `imp` cannot be on `plot`, as a sentence.
pub fn improvement_refusal_text(g: &Game, plot: PlotIdx, imp: ImprovementType, r: ImprovementRefusal) -> String {
    let d = g.d;
    let i = display(d, imp);
    match r {
        ImprovementRefusal::City => "No improvement in a city.".to_string(),
        ImprovementRefusal::Unimprovable => match g.map.plots[plot].terrain_type.filter(|_| !g.map.plots[plot].sea_ice) {
            Some(t) => format!("{} takes no improvement.", display(d, t)),
            None => "Nothing is built on Sea Ice.".to_string(),
        },
        ImprovementRefusal::Impassable => "No improvement on an impassable plot.".to_string(),
        ImprovementRefusal::Peak => format!("{i} goes on a peak."),
        ImprovementRefusal::OnWater => format!("{i} goes on water."),
        ImprovementRefusal::OnLand => format!("{i} goes on land."),
        ImprovementRefusal::Feature(f) => format!("{} takes no improvement.", display(d, f)),
        ImprovementRefusal::FreshWater => format!("{i} is not built next to fresh water."),
        ImprovementRefusal::Flatlands => format!("{i} requires flatlands."),
        ImprovementRefusal::NeedsFeature => format!("{i} requires a feature on the plot."),
        ImprovementRefusal::Site => {
            let sites = improvement_sites(d, imp);
            if sites.is_empty() { format!("{i} is built on no plot of this kind.") } else { format!("{i} requires {}.", or_list(&sites)) }
        }
        ImprovementRefusal::RiverSide => format!("{i} requires a river side."),
        ImprovementRefusal::NatureYield(y, n) => format!("{i} requires {n} {} on the plot.", display(d, y)),
        ImprovementRefusal::Irrigation => format!("{i} requires irrigation."),
    }
}

/// Why `build` cannot be done on `plot`, as a sentence.
pub fn plot_build_refusal_text(g: &Game, plot: PlotIdx, build: BuildType, r: PlotBuildRefusal) -> String {
    let d = g.d;
    let info = &d.builds[build];
    let imp = || info.improvement.map_or_else(|| build_label(d, build), |i| display(d, i).into_owned());
    match r {
        PlotBuildRefusal::Disabled => "Building is disabled on this plot.".to_string(),
        PlotBuildRefusal::SeaIce => "Nothing is built on Sea Ice.".to_string(),
        PlotBuildRefusal::Script => "Not allowed on this plot.".to_string(),
        PlotBuildRefusal::Improvement(r) => match info.improvement {
            Some(i) => improvement_refusal_text(g, plot, i, r),
            None => "Not allowed on this plot.".to_string(),
        },
        PlotBuildRefusal::EnemyCityAdjacent => format!("{} goes next to a city of a civilization at war with you.", imp()),
        PlotBuildRefusal::NeedsRoute => format!("{} goes on a route.", imp()),
        PlotBuildRefusal::CanalNotConnected => format!("{} goes next to water or another canal.", imp()),
        PlotBuildRefusal::CausewayNeedsLand => format!("{} goes next to land.", imp()),
        PlotBuildRefusal::PirateCoveSite => format!("{0} goes on an empty coast plot in your borders, 3 plots or more from another {0}.", imp()),
        PlotBuildRefusal::ClaimOwned => format!("{} goes outside every border.", imp()),
        PlotBuildRefusal::NearCity(n) => format!("{} goes {n} plots or more from every city.", imp()),
        PlotBuildRefusal::Permanent(i) => format!("{} on the plot is permanent.", display(d, i)),
        PlotBuildRefusal::Already(i) => format!("{} is here already.", display(d, i)),
        PlotBuildRefusal::SameUpgrade(i) => format!("{} is here: it upgrades to the same improvement.", display(d, i)),
        PlotBuildRefusal::ForeignBorders => "Not in the borders of another civilization.".to_string(),
        PlotBuildRefusal::OwnBorders => "In your borders only.".to_string(),
        PlotBuildRefusal::Route(r) => format!("{} is here already.", display(d, r)),
        PlotBuildRefusal::RouteBonus(b) => format!("Requires {} connected next to the plot.", display(d, b)),
        PlotBuildRefusal::RouteOrBonus(route) => {
            let bonuses: Vec<String> = d.routes[route].prereq_or_bonus.iter().flatten().map(|&b| display(d, b).into_owned()).collect();
            format!("Requires {} connected next to the plot.", or_list(&bonuses))
        }
        PlotBuildRefusal::FeatureOwner(f) => format!("{} is in the borders of a civilization at peace with you.", display(d, f)),
        PlotBuildRefusal::Nothing => {
            let features: Vec<String> = d.features.keys().filter(|&f| info.feature_remove[f]).map(|f| display(d, f).into_owned()).collect();
            if info.improvement.is_none() && info.route.is_none() && !features.is_empty() {
                format!("Requires {} on the plot.", or_list(&features))
            } else {
                format!("{} does nothing on this plot.", build_label(d, build))
            }
        }
    }
}

/// Why unit `u` cannot do `build` on `plot`, as a sentence.
pub fn build_refusal_text(g: &Game, u: UnitId, plot: PlotIdx, build: BuildType, r: BuildRefusal) -> String {
    let d = g.d;
    let name = unit_label(g, u);
    match r {
        BuildRefusal::NotBuilder => format!("{name} does not build {}.", build_label(d, build)),
        BuildRefusal::Plot(r) => plot_build_refusal_text(g, plot, build, r),
        BuildRefusal::Civilization(c) => format!("Only for the {}.", display(d, c)),
        BuildRefusal::Tech(t) => format!("Requires {}.", display(d, t)),
        BuildRefusal::FeatureTech(f, t) => format!("Removing {} requires {}.", display(d, f), display(d, t)),
        BuildRefusal::Gold { have, need } => gold_line(have, need),
        BuildRefusal::Domain => format!("{name} cannot build from this plot."),
        BuildRefusal::AutomatedFort => "Automated units build no forts.".to_string(),
    }
}

// ---------------------------------------------------------------------------------- missions

/// Why unit `u` cannot do a mission at `plot`, as a sentence (`target`: the unit to lead, for
/// `Lead`).
pub fn mission_refusal_text(g: &Game, u: UnitId, plot: PlotIdx, target: Option<UnitId>, r: MissionRefusal) -> String {
    let d = g.d;
    let name = unit_label(g, u);
    match r {
        MissionRefusal::Waiting => format!("{name} is fortified or asleep already."),
        MissionRefusal::Dying => format!("{name} is dying."),
        MissionRefusal::HasCast => format!("{name} has cast this turn."),
        MissionRefusal::NoCity => "Requires a city on the plot.".to_string(),
        MissionRefusal::ForeignCity => "Requires a city of your team.".to_string(),
        MissionRefusal::NotOwnCity => "Requires a city of yours.".to_string(),
        MissionRefusal::CannotEnter => format!("{name} cannot enter this territory."),
        MissionRefusal::NoMoves => format!("{name} has no moves left."),
        MissionRefusal::Fortifies => format!("{name} fortifies instead of sleeping."),
        MissionRefusal::NotFortifyable => format!("{name} does not fortify."),
        MissionRefusal::InBoats => "Not in boats.".to_string(),
        MissionRefusal::NotHurt => format!("{name} is not hurt."),
        MissionRefusal::NoHeal => format!("{name} heals nothing here."),
        MissionRefusal::CannotDefend => format!("{name} cannot defend."),
        MissionRefusal::NoBombard => format!("{name} does not bombard."),
        MissionRefusal::NoAttackLeft => format!("{name} has no attack left this turn."),
        MissionRefusal::Attacked => format!("{name} has attacked this turn."),
        MissionRefusal::Cargo => format!("{name} is loaded on a transport."),
        MissionRefusal::NoBombardTarget => "No city with defenses in reach.".to_string(),
        MissionRefusal::Air => "Air units make no range attack.".to_string(),
        MissionRefusal::NoRange => format!("{name} has no range."),
        MissionRefusal::NoStrength => format!("{name} has no ranged strength."),
        MissionRefusal::CannotFight => format!("{name} cannot fight."),
        MissionRefusal::OffMap => "Off the map.".to_string(),
        MissionRefusal::NotVisible => "The plot is not visible.".to_string(),
        MissionRefusal::OutOfRange(n) => format!("The target is beyond the range of {n} plots."),
        MissionRefusal::NoTarget => "No unit to strike on the target plot.".to_string(),
        MissionRefusal::NoLineOfSight => "The target plot is out of sight.".to_string(),
        MissionRefusal::NotPillager => format!("{name} does not pillage."),
        MissionRefusal::PillageCity => "No pillaging in a city.".to_string(),
        MissionRefusal::NothingToPillage => "No improvement or route to pillage.".to_string(),
        MissionRefusal::Permanent(i) => format!("{} is permanent.", display(d, i)),
        MissionRefusal::PillageNotAtWar => "Pillage in your borders or in the borders of a civilization at war with you.".to_string(),
        MissionRefusal::Domain => format!("{name} cannot act from this plot."),
        MissionRefusal::Lair(i) => format!("Barbarians do not pillage {}.", display(d, i)),
        MissionRefusal::Fields => "Fields are raided, not pillaged.".to_string(),
        MissionRefusal::NotWarship => format!("{name} is not a warship."),
        MissionRefusal::Barbarian => "Barbarians do not patrol.".to_string(),
        MissionRefusal::NotSea => "Patrol on a sea.".to_string(),
        MissionRefusal::PatrolCity => "No Patrol in a city.".to_string(),
        MissionRefusal::NoReligion => "No religion given.".to_string(),
        MissionRefusal::NoSpread(r) => format!("{name} does not spread {}.", display(d, r)),
        MissionRefusal::HasReligion(r) => format!("{} is in the city already.", display(d, r)),
        MissionRefusal::StateReligionOnly => "The city's owner lets in its state religion only.".to_string(),
        MissionRefusal::SpreadScript => "Not allowed in this city.".to_string(),
        MissionRefusal::NoCorporation => "No corporation given.".to_string(),
        MissionRefusal::InactiveCorporation(c) => format!("{} is not active for you.", display(d, c)),
        MissionRefusal::NoCorporationSpread(c) => format!("{name} does not spread {}.", display(d, c)),
        MissionRefusal::HasCorporation(c) => format!("{} is in the city already.", display(d, c)),
        MissionRefusal::ForeignInactiveCorporation(c) => format!("{} is not active for the city's owner.", display(d, c)),
        MissionRefusal::CompetingHeadquarters(c) => format!("The city is the headquarters of {}, a competitor.", display(d, c)),
        MissionRefusal::CorporationBonus(c) => {
            let bonuses: Vec<String> = d.corporations[c].prereq_bonus.iter().map(|&b| display(d, b).into_owned()).collect();
            format!("The city must have {}.", or_list(&bonuses))
        }
        MissionRefusal::Gold { have, need } => gold_line(have, need),
        MissionRefusal::NoSpecialist => "No specialist given.".to_string(),
        MissionRefusal::NotGreatPerson(s) => format!("{name} does not join a city as {}.", display(d, s)),
        MissionRefusal::CityCannotJoin => "The city takes no one.".to_string(),
        MissionRefusal::NoBuilding => "No building given.".to_string(),
        MissionRefusal::HasBuilding(b) => format!("{} is in the city already.", display(d, b)),
        MissionRefusal::NotConstructor(b) => format!("{name} does not build {}.", display(d, b)),
        MissionRefusal::Construct(b, r) => match g.map.plots[plot].plot_city.filter(|&c| g.cities.contains(c)) {
            Some(c) => crate::view::refusal::construct_refusal_text(g, c, b, r),
            None => "Requires a city on the plot.".to_string(),
        },
        MissionRefusal::NoDiscoveryTech => format!("No tech for {name} to discover."),
        MissionRefusal::NoResearch => format!("{name} adds no research."),
        MissionRefusal::NoHurry => format!("{name} adds no production here."),
        MissionRefusal::OneTurnLeft => "The build finishes next turn.".to_string(),
        MissionRefusal::HurryOnly => "Only a building or a ritual is hurried.".to_string(),
        MissionRefusal::NoTradeGold => "The trade mission pays no gold here.".to_string(),
        MissionRefusal::OwnTeamCity => "Trade missions go to a city of another team.".to_string(),
        MissionRefusal::NoCulture => format!("{name} makes no culture."),
        MissionRefusal::NotGoldenAge => format!("{name} starts no golden age."),
        MissionRefusal::GoldenAgeUnits { need, have } => format!("Great people of different kinds ready: {have} of {need}."),
        MissionRefusal::NoneToLead => "No unit of yours on the plot can be led.".to_string(),
        MissionRefusal::Lead(r) => match target {
            Some(t) if g.units.contains(t) => promote_refusal_text(g, t, r),
            _ => "No unit of yours on the plot can be led.".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------------- commands

/// Why unit `u` cannot take promotion `p`, as a sentence.
pub fn acquire_refusal_text(g: &Game, u: UnitId, p: PromotionType, r: AcquireRefusal) -> String {
    let d = g.d;
    let name = unit_label(g, u);
    let pn = display(d, p);
    let promo = |q: PromotionType| format!("the {} promotion", display(d, q));
    match r {
        AcquireRefusal::Has => format!("{name} has {pn} already."),
        AcquireRefusal::Prereq(q) | AcquireRefusal::PrereqAnd(q) => format!("Requires {}.", promo(q)),
        AcquireRefusal::NotPickable => format!("{pn} is not picked: the rules give it."),
        AcquireRefusal::Race => format!("{pn} is a race: it is not picked."),
        AcquireRefusal::Equipment => format!("{pn} is equipment: it is not picked."),
        AcquireRefusal::Level(n) => format!("Requires level {n}: {name} is level {}.", g.units[u].level),
        AcquireRefusal::PrereqOr(a, b, c, e) => {
            let names: Vec<String> = [Some(a), b, c, e].into_iter().flatten().map(|q| display(d, q).into_owned()).collect();
            format!("Requires the {} promotion.", or_list(&names))
        }
        AcquireRefusal::Bonus(b) => format!("Requires {}.", display(d, b)),
        AcquireRefusal::Immune(q) => format!("{} makes {name} immune to {pn}.", display(d, q)),
        AcquireRefusal::Alive => format!("{name} must be alive."),
        AcquireRefusal::Tech(t) => format!("Requires {}.", display(d, t)),
        AcquireRefusal::StateReligion(r) => format!("Requires {} as state religion.", display(d, r)),
        AcquireRefusal::NotForUnit => format!("{pn} is not for a {name}."),
        AcquireRefusal::Withdrawal(n) => format!("The withdrawal chance would pass {n}%."),
        AcquireRefusal::Interception(n) => format!("The interception chance would pass {n}%."),
        AcquireRefusal::Evasion(n) => format!("The evasion chance would pass {n}%."),
        AcquireRefusal::SpellClash => format!("{name} has learnt a spell by a promotion already."),
    }
}

/// Why unit `u` cannot take a promotion now, as a sentence.
pub fn promote_refusal_text(g: &Game, u: UnitId, r: PromoteRefusal) -> String {
    let d = g.d;
    let name = unit_label(g, u);
    match r {
        PromoteRefusal::LeaderSelf => "A leader does not lead itself.".to_string(),
        PromoteRefusal::NoPromotion => "No promotion given.".to_string(),
        PromoteRefusal::Acquire(p, r) => acquire_refusal_text(g, u, p, r),
        PromoteRefusal::Leader(p) => format!("{} comes from a great commander.", display(d, p)),
        PromoteRefusal::NotReady => format!("{name} has not the experience for a promotion."),
    }
}

/// Why unit `u` cannot upgrade to `t`, as a sentence.
pub fn upgrade_refusal_text(g: &Game, u: UnitId, t: UnitType, r: UpgradeRefusal) -> String {
    let d = g.d;
    let name = unit_label(g, u);
    let tn = display(d, t);
    match r {
        UpgradeRefusal::NoUnit => "No unit type given.".to_string(),
        UpgradeRefusal::NoMoves => format!("{name} has no moves left."),
        UpgradeRefusal::Territory => "Units upgrade in your own territory.".to_string(),
        UpgradeRefusal::Gold { have, need } => gold_line(have, need),
        UpgradeRefusal::Level(n) => format!("{tn} requires level {n}: {name} is level {}.", g.units[u].level),
        UpgradeRefusal::Disabled => format!("No unit upgrades to {tn}."),
        UpgradeRefusal::ClassMaxed => format!("{tn}: the limit of its class is reached."),
        UpgradeRefusal::AiMissionary => "AI players keep missionaries below level 2.".to_string(),
        UpgradeRefusal::AiDefender => format!("AI players keep city defenders from {tn}."),
        // `unit_has_upgrade_to`'s checks: the upgrade path, the cargo, the nearest city.
        UpgradeRefusal::NoUpgrade => {
            let class = d.units[t].unit_class_type;
            if !class.is_some_and(|k| g.unit_upgrade_available(u, g.units[u].unit_type, k, 0)) {
                format!("{name} does not upgrade to {tn}.")
            } else if d.units[t].cargo_space < g.units[u].cargo {
                format!("{tn} has no room for the units {name} carries.")
            } else {
                format!("The city of yours nearest to {name} cannot build {tn}.")
            }
        }
    }
}

/// Why unit `u` cannot be automated, as a sentence.
pub fn automate_refusal_text(g: &Game, u: UnitId, r: AutomateRefusal) -> String {
    let name = unit_label(g, u);
    match r {
        AutomateRefusal::NoAutomate => "No automation given.".to_string(),
        AutomateRefusal::NotGroupHead => "Only the head of a group is automated.".to_string(),
        AutomateRefusal::Disabled => "Units do not improve the land by themselves.".to_string(),
        AutomateRefusal::Explore => format!("{name} does not explore."),
        AutomateRefusal::AirRecon => format!("{name} cannot recon."),
        AutomateRefusal::NotMissionary => "Only a missionary spreads its religion by itself.".to_string(),
        AutomateRefusal::NotTerraformer => format!("{name} is not a terraformer."),
    }
}

/// Why unit `u` cannot be gifted, as a sentence.
pub fn gift_refusal_text(g: &Game, u: UnitId, r: GiftRefusal) -> String {
    let name = unit_label(g, u);
    match r {
        GiftRefusal::Disabled => "Units are not gifted in this game.".to_string(),
        GiftRefusal::NotOwned => "Gift in the borders of another civilization.".to_string(),
        GiftRefusal::OwnPlot => "The plot is in your borders.".to_string(),
        GiftRefusal::Enemy => "An enemy unit is in view.".to_string(),
        GiftRefusal::Domain => format!("{name} cannot stand on this plot."),
        GiftRefusal::Corporation => "A unit that spreads a corporation is not gifted.".to_string(),
        GiftRefusal::Transport => "The transport is not of the receiver's team.".to_string(),
        GiftRefusal::ClassMaxed => "The receiver is at the limit of the unit's class.".to_string(),
        GiftRefusal::NotAccepted => "The receiver does not accept it.".to_string(),
        GiftRefusal::TempPlayer => "A temporary player gifts nothing.".to_string(),
        GiftRefusal::War => "At war with the receiver.".to_string(),
    }
}

/// Why unit `u` cannot load onto a transport, as a sentence.
pub fn load_refusal_text(g: &Game, u: UnitId, r: LoadRefusal) -> String {
    let name = unit_label(g, u);
    match r {
        LoadRefusal::Itself => "A unit does not load onto itself.".to_string(),
        LoadRefusal::InRoute => "A caravan of a trade route is not loaded.".to_string(),
        LoadRefusal::OtherTeam => "The transport is of another team.".to_string(),
        LoadRefusal::Cargo => format!("{name} is loaded already."),
        LoadRefusal::HasCargo => format!("{name} carries units."),
        LoadRefusal::TransportCargo => "The transport is itself loaded.".to_string(),
        LoadRefusal::NoRoom => format!("The transport has no room for {name}."),
        LoadRefusal::NotHere => "The transport is not on this plot.".to_string(),
        LoadRefusal::HiddenNationality => "A unit with hidden nationality and a unit without it do not load together.".to_string(),
        LoadRefusal::CityLoad => format!("{name} loads in a city of your team."),
    }
}

/// Why unit `u` cannot unload, as a sentence.
pub fn unload_refusal_text(g: &Game, u: UnitId, r: UnloadRefusal) -> String {
    let name = unit_label(g, u);
    match r {
        UnloadRefusal::NotLoaded => format!("{name} is not loaded."),
        UnloadRefusal::Domain => format!("{name} cannot stand on this plot."),
        UnloadRefusal::AirCapacity => "The city's air capacity is full.".to_string(),
    }
}

/// Why a unit cannot be disbanded, as a sentence.
pub fn scrap_refusal_text(r: ScrapRefusal) -> String {
    match r {
        ScrapRefusal::Fighting => "A battle is under way on the plot.".to_string(),
        ScrapRefusal::TempPlayer => "A temporary player disbands nothing.".to_string(),
        ScrapRefusal::Equipment => "Equipment is not disbanded.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::reasons::tests::plain;
    use ffh_core::setup::GameSetup;

    fn game() -> Game {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_SMALL", 3));
        g.game_init();
        g
    }

    fn check(s: String) {
        assert!(plain(&s) && s.ends_with('.'), "{s}");
    }

    /// Every variant's sentence passes the client's `plainDetail` and ends with a full stop.
    #[test]
    fn every_order_refusal_reads_as_a_sentence() {
        let g = game();
        let d = g.d;
        let p = g.player_ids().next().unwrap();
        let u = g.players[p].units[0];
        let plot = g.unit_plot(u);
        let imp = d.improvements.keys().next().unwrap();
        let build = d.builds.keys().find(|&b| d.builds[b].improvement.is_some()).unwrap();
        let route = d.routes.keys().next().unwrap();
        let feature = d.features.keys().next().unwrap();
        let bonus = d.bonuses.keys().next().unwrap();
        let tech = d.techs.keys().next().unwrap();
        let civ = d.civilizations.keys().next().unwrap();
        let religion = d.religions.keys().next().unwrap();
        let promotion = d.promotions.keys().next().unwrap();
        let unit_type = d.units.keys().next().unwrap();
        let building = d.buildings.keys().next().unwrap();
        let specialist = d.specialists.keys().next().unwrap();
        let yield_ = d.yields.keys().next().unwrap();
        let improvements = [
            ImprovementRefusal::City,
            ImprovementRefusal::Unimprovable,
            ImprovementRefusal::Impassable,
            ImprovementRefusal::Peak,
            ImprovementRefusal::OnWater,
            ImprovementRefusal::OnLand,
            ImprovementRefusal::Feature(feature),
            ImprovementRefusal::FreshWater,
            ImprovementRefusal::Flatlands,
            ImprovementRefusal::NeedsFeature,
            ImprovementRefusal::Site,
            ImprovementRefusal::RiverSide,
            ImprovementRefusal::NatureYield(yield_, 2),
            ImprovementRefusal::Irrigation,
        ];
        for i in d.improvements.keys() {
            for r in improvements {
                check(improvement_refusal_text(&g, plot, i, r));
            }
        }
        let mut plots = vec![
            PlotBuildRefusal::Disabled,
            PlotBuildRefusal::SeaIce,
            PlotBuildRefusal::Script,
            PlotBuildRefusal::EnemyCityAdjacent,
            PlotBuildRefusal::NeedsRoute,
            PlotBuildRefusal::CanalNotConnected,
            PlotBuildRefusal::CausewayNeedsLand,
            PlotBuildRefusal::PirateCoveSite,
            PlotBuildRefusal::ClaimOwned,
            PlotBuildRefusal::NearCity(3),
            PlotBuildRefusal::Permanent(imp),
            PlotBuildRefusal::Already(imp),
            PlotBuildRefusal::SameUpgrade(imp),
            PlotBuildRefusal::ForeignBorders,
            PlotBuildRefusal::OwnBorders,
            PlotBuildRefusal::Route(route),
            PlotBuildRefusal::RouteBonus(bonus),
            PlotBuildRefusal::RouteOrBonus(route),
            PlotBuildRefusal::FeatureOwner(feature),
            PlotBuildRefusal::Nothing,
        ];
        plots.extend(improvements.iter().map(|&r| PlotBuildRefusal::Improvement(r)));
        for b in d.builds.keys() {
            for &r in &plots {
                // (`RouteOrBonus` is only ever the refusal of a route with such bonuses.)
                if matches!(r, PlotBuildRefusal::RouteOrBonus(_)) {
                    continue;
                }
                check(plot_build_refusal_text(&g, plot, b, r));
            }
        }
        for r in d.routes.keys() {
            if d.routes[r].prereq_or_bonus.iter().any(|b| b.is_some()) {
                check(plot_build_refusal_text(&g, plot, build, PlotBuildRefusal::RouteOrBonus(r)));
            }
        }
        let mut builds = vec![
            BuildRefusal::NotBuilder,
            BuildRefusal::Civilization(civ),
            BuildRefusal::Tech(tech),
            BuildRefusal::FeatureTech(feature, tech),
            BuildRefusal::Gold { have: -3, need: 40 },
            BuildRefusal::Domain,
            BuildRefusal::AutomatedFort,
        ];
        builds.extend(plots.iter().filter(|r| !matches!(r, PlotBuildRefusal::RouteOrBonus(_))).map(|&r| BuildRefusal::Plot(r)));
        for r in builds {
            check(build_refusal_text(&g, u, plot, build, r));
        }
        let acquires = [
            AcquireRefusal::Has,
            AcquireRefusal::Prereq(promotion),
            AcquireRefusal::NotPickable,
            AcquireRefusal::Race,
            AcquireRefusal::Equipment,
            AcquireRefusal::Level(4),
            AcquireRefusal::PrereqAnd(promotion),
            AcquireRefusal::PrereqOr(promotion, Some(promotion), None, Some(promotion)),
            AcquireRefusal::Bonus(bonus),
            AcquireRefusal::Immune(promotion),
            AcquireRefusal::Alive,
            AcquireRefusal::Tech(tech),
            AcquireRefusal::StateReligion(religion),
            AcquireRefusal::NotForUnit,
            AcquireRefusal::Withdrawal(90),
            AcquireRefusal::Interception(100),
            AcquireRefusal::Evasion(100),
            AcquireRefusal::SpellClash,
        ];
        let mut promotes = vec![PromoteRefusal::LeaderSelf, PromoteRefusal::NoPromotion, PromoteRefusal::Leader(promotion), PromoteRefusal::NotReady];
        promotes.extend(acquires.iter().map(|&r| PromoteRefusal::Acquire(promotion, r)));
        for &r in &promotes {
            check(promote_refusal_text(&g, u, r));
        }
        let mut missions = vec![
            MissionRefusal::Waiting,
            MissionRefusal::Dying,
            MissionRefusal::HasCast,
            MissionRefusal::NoCity,
            MissionRefusal::ForeignCity,
            MissionRefusal::NotOwnCity,
            MissionRefusal::CannotEnter,
            MissionRefusal::NoMoves,
            MissionRefusal::Fortifies,
            MissionRefusal::NotFortifyable,
            MissionRefusal::InBoats,
            MissionRefusal::NotHurt,
            MissionRefusal::NoHeal,
            MissionRefusal::CannotDefend,
            MissionRefusal::NoBombard,
            MissionRefusal::NoAttackLeft,
            MissionRefusal::Attacked,
            MissionRefusal::Cargo,
            MissionRefusal::NoBombardTarget,
            MissionRefusal::Air,
            MissionRefusal::NoRange,
            MissionRefusal::NoStrength,
            MissionRefusal::CannotFight,
            MissionRefusal::OffMap,
            MissionRefusal::NotVisible,
            MissionRefusal::OutOfRange(2),
            MissionRefusal::NoTarget,
            MissionRefusal::NoLineOfSight,
            MissionRefusal::NotPillager,
            MissionRefusal::PillageCity,
            MissionRefusal::NothingToPillage,
            MissionRefusal::Permanent(imp),
            MissionRefusal::PillageNotAtWar,
            MissionRefusal::Domain,
            MissionRefusal::Lair(imp),
            MissionRefusal::Fields,
            MissionRefusal::NotWarship,
            MissionRefusal::Barbarian,
            MissionRefusal::NotSea,
            MissionRefusal::PatrolCity,
            MissionRefusal::NoReligion,
            MissionRefusal::NoSpread(religion),
            MissionRefusal::HasReligion(religion),
            MissionRefusal::StateReligionOnly,
            MissionRefusal::SpreadScript,
            MissionRefusal::Gold { have: 10, need: 30 },
            MissionRefusal::NoSpecialist,
            MissionRefusal::NotGreatPerson(specialist),
            MissionRefusal::CityCannotJoin,
            MissionRefusal::NoBuilding,
            MissionRefusal::HasBuilding(building),
            MissionRefusal::NotConstructor(building),
            MissionRefusal::NoDiscoveryTech,
            MissionRefusal::NoResearch,
            MissionRefusal::NoHurry,
            MissionRefusal::OneTurnLeft,
            MissionRefusal::HurryOnly,
            MissionRefusal::NoTradeGold,
            MissionRefusal::OwnTeamCity,
            MissionRefusal::NoCulture,
            MissionRefusal::NotGoldenAge,
            MissionRefusal::GoldenAgeUnits { need: 2, have: 1 },
            MissionRefusal::NoneToLead,
        ];
        if let Some(c) = d.corporations.keys().next() {
            missions.extend([
                MissionRefusal::NoCorporation,
                MissionRefusal::InactiveCorporation(c),
                MissionRefusal::NoCorporationSpread(c),
                MissionRefusal::HasCorporation(c),
                MissionRefusal::ForeignInactiveCorporation(c),
                MissionRefusal::CompetingHeadquarters(c),
            ]);
            if !d.corporations[c].prereq_bonus.is_empty() {
                missions.push(MissionRefusal::CorporationBonus(c));
            }
        }
        missions.extend(promotes.iter().map(|&r| MissionRefusal::Lead(r)));
        for r in missions {
            check(mission_refusal_text(&g, u, plot, Some(u), r));
        }
        for r in [
            UpgradeRefusal::NoUnit,
            UpgradeRefusal::NoMoves,
            UpgradeRefusal::Territory,
            UpgradeRefusal::Gold { have: 5, need: 50 },
            UpgradeRefusal::Level(3),
            UpgradeRefusal::Disabled,
            UpgradeRefusal::ClassMaxed,
            UpgradeRefusal::AiMissionary,
            UpgradeRefusal::AiDefender,
            UpgradeRefusal::NoUpgrade,
        ] {
            check(upgrade_refusal_text(&g, u, unit_type, r));
        }
        for r in [
            AutomateRefusal::NoAutomate,
            AutomateRefusal::NotGroupHead,
            AutomateRefusal::Disabled,
            AutomateRefusal::Explore,
            AutomateRefusal::AirRecon,
            AutomateRefusal::NotMissionary,
            AutomateRefusal::NotTerraformer,
        ] {
            check(automate_refusal_text(&g, u, r));
        }
        for r in [
            GiftRefusal::Disabled,
            GiftRefusal::NotOwned,
            GiftRefusal::OwnPlot,
            GiftRefusal::Enemy,
            GiftRefusal::Domain,
            GiftRefusal::Corporation,
            GiftRefusal::Transport,
            GiftRefusal::ClassMaxed,
            GiftRefusal::NotAccepted,
            GiftRefusal::TempPlayer,
            GiftRefusal::War,
        ] {
            check(gift_refusal_text(&g, u, r));
        }
        for r in [
            LoadRefusal::Itself,
            LoadRefusal::InRoute,
            LoadRefusal::OtherTeam,
            LoadRefusal::Cargo,
            LoadRefusal::HasCargo,
            LoadRefusal::TransportCargo,
            LoadRefusal::NoRoom,
            LoadRefusal::NotHere,
            LoadRefusal::HiddenNationality,
            LoadRefusal::CityLoad,
        ] {
            check(load_refusal_text(&g, u, r));
        }
        for r in [UnloadRefusal::NotLoaded, UnloadRefusal::Domain, UnloadRefusal::AirCapacity] {
            check(unload_refusal_text(&g, u, r));
        }
        for r in [ScrapRefusal::Fighting, ScrapRefusal::TempPlayer, ScrapRefusal::Equipment] {
            check(scrap_refusal_text(r));
        }
    }

    /// The orders the starting units are refused name a check, not the generic fallback, and the
    /// refusal twins agree with their gates.
    #[test]
    fn unit_orders_name_the_failing_check() {
        use crate::selection::{can_do_command, can_start_mission};
        use crate::view::reasons::{command_refusal, mission_refusal};
        let g = game();
        let d = g.d;
        let p = g.player_ids().next().unwrap();
        for &u in &g.players[p].units {
            let plot = g.unit_plot(u);
            let name = display(d, g.units[u].unit_type);
            for b in d.builds.keys() {
                let k = b.idx() as i32;
                assert_eq!(g.unit_build_refusal(u, plot, b, false).is_none(), g.unit_can_build(u, plot, b, false));
                assert_eq!(g.plot_build_refusal(plot, b, p, true).is_none(), g.plot_can_build(plot, b, p, true));
                if !can_start_mission(&g, &[u], MissionType::MISSION_BUILD, k, -1, plot, false) {
                    let s = mission_refusal(&g, &[u], MissionType::MISSION_BUILD, k, -1, plot);
                    check(s.clone());
                    assert_ne!(s, format!("{name} cannot {} here.", display(d, MissionType::MISSION_BUILD)));
                }
            }
            for m in [
                MissionType::MISSION_SLEEP,
                MissionType::MISSION_FORTIFY,
                MissionType::MISSION_HEAL,
                MissionType::MISSION_SENTRY,
                MissionType::MISSION_BOMBARD,
                MissionType::MISSION_PILLAGE,
                MissionType::MISSION_PLUNDER,
                MissionType::MISSION_DISCOVER,
                MissionType::MISSION_HURRY,
                MissionType::MISSION_TRADE,
                MissionType::MISSION_GREAT_WORK,
                MissionType::MISSION_GOLDEN_AGE,
                MissionType::MISSION_LEAD,
                MissionType::MISSION_RANGE_ATTACK,
            ] {
                if !can_start_mission(&g, &[u], m, -1, -1, plot, false) {
                    let s = mission_refusal(&g, &[u], m, -1, -1, plot);
                    check(s.clone());
                    assert_ne!(s, format!("{name} cannot {} here.", display(d, m)));
                }
            }
            for c in [CommandType::COMMAND_DELETE, CommandType::COMMAND_GIFT, CommandType::COMMAND_UNLOAD_ALL, CommandType::COMMAND_LOAD, CommandType::COMMAND_UNLOAD] {
                if !can_do_command(&g, &[u], c, -1, -1, false) {
                    let s = command_refusal(&g, &[u], c, -1, -1);
                    check(s.clone());
                    assert_ne!(s, format!("{name} cannot {} now.", display(d, c)));
                }
            }
            for a in d.automates.keys() {
                let k = a.idx() as i32;
                if !can_do_command(&g, &[u], CommandType::COMMAND_AUTOMATE, k, -1, false) {
                    check(command_refusal(&g, &[u], CommandType::COMMAND_AUTOMATE, k, -1));
                }
            }
            for promotion in d.promotions.keys() {
                assert_eq!(g.unit_acquire_promotion_refusal(u, promotion).is_none(), g.unit_can_acquire_promotion(u, promotion));
                let k = promotion.idx() as i32;
                if !can_do_command(&g, &[u], CommandType::COMMAND_PROMOTION, k, -1, false) {
                    check(command_refusal(&g, &[u], CommandType::COMMAND_PROMOTION, k, -1));
                }
            }
            for t in d.units.keys() {
                let k = t.idx() as i32;
                if !can_do_command(&g, &[u], CommandType::COMMAND_UPGRADE, k, -1, false) {
                    check(command_refusal(&g, &[u], CommandType::COMMAND_UPGRADE, k, -1));
                }
            }
        }
    }
}
