//! When the AI casts a world spell (`bGlobal`: once a game, by any unit of the civilization).
//! FfH 0.41 wrote this timing into the rules, as AI-only branches of the spells' Python
//! requirements (`if pPlayer.isHuman() == False:` in `CvSpellInterface.py` `req*`); the port
//! dropped them from the rules (docs/PORTING.md, "Changes from FFH2"), so a world spell has the
//! same requirements for every player and the AI decides when. [`ready`] is FfH's condition, the
//! AI's default timing, except for three spells that condition almost never let the AI cast
//! ([`expert_moment`]; `FFH_AI_WS_TIMING_OFF` restores FfH's): `casting::choose_spell` (the
//! end-of-turn spell choice) and the civilizations' held casts (`civs/world_spell.rs`) cast a
//! world spell only once it holds. A civilization can wait longer through its `spell_value`
//! (`civs/`).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum World {
    ArcaneLacuna,
    Ardor,
    DivineRetribution,
    ForTheHorde,
    GiftsOfNantosuelta,
    IntoTheMist,
    Legends,
    MarchOfTheTrees,
    MotherLode,
    RagingSeas,
    Rally,
    ReligiousFervor,
    Revelry,
    RiverOfBlood,
    Sanctuary,
    Stasis,
    VeilOfNight,
    Warcry,
    WildHunt,
    Worldbreak,
}

const NAMES: [(&str, World); 20] = [
    ("SPELL_ARCANE_LACUNA", World::ArcaneLacuna),
    ("SPELL_ARDOR", World::Ardor),
    ("SPELL_DIVINE_RETRIBUTION", World::DivineRetribution),
    ("SPELL_FOR_THE_HORDE", World::ForTheHorde),
    ("SPELL_GIFTS_OF_NANTOSUELTA", World::GiftsOfNantosuelta),
    ("SPELL_INTO_THE_MIST", World::IntoTheMist),
    ("SPELL_LEGENDS", World::Legends),
    ("SPELL_MARCH_OF_THE_TREES", World::MarchOfTheTrees),
    ("SPELL_MOTHER_LODE", World::MotherLode),
    ("SPELL_RAGING_SEAS", World::RagingSeas),
    ("SPELL_RALLY", World::Rally),
    ("SPELL_RELIGIOUS_FERVOR", World::ReligiousFervor),
    ("SPELL_REVELRY", World::Revelry),
    ("SPELL_RIVER_OF_BLOOD", World::RiverOfBlood),
    ("SPELL_SANCTUARY", World::Sanctuary),
    ("SPELL_STASIS", World::Stasis),
    ("SPELL_VEIL_OF_NIGHT", World::VeilOfNight),
    ("SPELL_WARCRY", World::Warcry),
    ("SPELL_WILD_HUNT", World::WildHunt),
    ("SPELL_WORLDBREAK", World::Worldbreak),
];

/// The typed mana bonuses `reqArcaneLacuna` counts on the map (raw mana excluded).
const MANA_TYPES: [&str; 17] = [
    "BONUS_MANA_AIR",
    "BONUS_MANA_BODY",
    "BONUS_MANA_CHAOS",
    "BONUS_MANA_DEATH",
    "BONUS_MANA_EARTH",
    "BONUS_MANA_ENCHANTMENT",
    "BONUS_MANA_ENTROPY",
    "BONUS_MANA_FIRE",
    "BONUS_MANA_LAW",
    "BONUS_MANA_LIFE",
    "BONUS_MANA_METAMAGIC",
    "BONUS_MANA_MIND",
    "BONUS_MANA_NATURE",
    "BONUS_MANA_SHADOW",
    "BONUS_MANA_SPIRIT",
    "BONUS_MANA_SUN",
    "BONUS_MANA_WATER",
];

/// Per spell type: the world spell whose timing it has (`None` for the other spells, and for
/// Hyborem's Whisper, which FfH never timed for the AI).
fn table(d: &GameData) -> &'static [Option<World>] {
    static TABLE: std::sync::OnceLock<Vec<Option<World>>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| d.spells.keys().map(|s| NAMES.iter().find(|(n, _)| *n == d.spells[s].base.type_).map(|&(_, w)| w)).collect())
}

/// True once player `p` may cast `spell` by the AI's timing (always, for a spell that is not a
/// timed world spell). These are FfH 0.41's AI-only requirements, condition for condition; the
/// rules check the rest (`unit_can_cast`).
pub fn ready(g: &Game, p: PlayerId, spell: SpellType) -> bool {
    let Some(w) = table(g.d)[spell.idx()] else { return true };
    let d = g.d;
    let pl = &g.players[p];
    let wars = || g.team_get_at_war_count(pl.team, true);
    if matches!(w, World::Sanctuary | World::MarchOfTheTrees) && crate::switches::WS_LOSING_WAR.on(g, p) {
        return losing_war(g, p) && city_in_peril(g, p);
    }
    if crate::switches::WS_TIMING.on(g, p) {
        if let Some(ready) = expert_moment(g, p, w) {
            return ready;
        }
    }
    match w {
        World::ArcaneLacuna => MANA_TYPES.iter().map(|name| g.map.num_bonus[d.t::<BonusType>(name)]).sum::<i32>() >= 7,
        World::Ardor => pl.great_people_created >= 5,
        World::DivineRetribution | World::MarchOfTheTrees | World::Sanctuary => wars() >= 2,
        World::ForTheHorde => g.players[g.barbarian_player()].num_units() >= 60,
        World::GiftsOfNantosuelta | World::Legends | World::Rally | World::RiverOfBlood | World::Stasis => pl.num_cities() >= 5,
        World::IntoTheMist => pl.num_units() >= 40,
        World::MotherLode => pl.improvement_count[d.t::<ImprovementType>("IMPROVEMENT_MINE")] >= 10,
        World::RagingSeas => enemy_fleet(g, p),
        World::ReligiousFervor => g.player_get_state_religion(p).is_some_and(|r| pl.cities.iter().filter(|&&c| g.cities[c].has_religion[r]).count() >= 5),
        World::Revelry => pl.total_population >= 25,
        World::VeilOfNight => pl.num_units() >= 25 && wars() == 0,
        World::Warcry => pl.num_units() >= 25 && wars() > 0,
        World::WildHunt => wars() > 0 && pl.num_units() >= 20,
        World::Worldbreak => g.get_global_counter() >= 50,
    }
}

/// The moments an expert would pick for the world spells FfH's conditions almost never allowed
/// (the owner's rule that civilizations use their own mechanics; docs/AI.md, "World spells"):
/// `None` keeps FfH's condition.
/// - Arcane Lacuna (no other player can cast for 20 turns, our adepts gain as much experience as
///   the map has typed mana): at war with a civilization. FfH waited for 7 typed mana nodes on
///   the map, which no measured game reached.
/// - Worldbreak (every unit in a foreign city takes the Armageddon counter as damage, and cities
///   burn): at war with a civilization once the counter is 20. FfH waited for 50, never reached.
/// - Religious Fervor (a priest in every city of our state religion, with experience by their
///   number): FfH's five cities of the religion, or three that are two thirds of ours (a small
///   Malakim empire never had five).
///
/// Sanctuary and March of the Trees wait for a war we are losing with a city in peril
/// ([`losing_war`], [`city_in_peril`]; `FFH_AI_WS_LOSING_WAR_OFF` restores FfH's two wars),
/// checked in [`ready`].
fn expert_moment(g: &Game, p: PlayerId, w: World) -> Option<bool> {
    let pl = &g.players[p];
    let team = pl.team;
    let barb = g.barbarian_team();
    let civ_war = || g.teams.keys().any(|t| t != team && t != barb && g.teams[t].alive_count > 0 && g.at_war(team, t));
    Some(match w {
        World::ArcaneLacuna => civ_war(),
        World::Worldbreak => g.get_global_counter() >= 20 && civ_war(),
        World::ReligiousFervor => g.player_get_state_religion(p).is_some_and(|r| {
            let with = pl.cities.iter().filter(|&&c| g.cities[c].has_religion[r]).count();
            with >= 5 || (with >= 3 && with * 3 >= pl.cities.len() * 2)
        }),
        _ => return None,
    })
}

/// A war we are losing, the moment for the defensive world spells (Sanctuary: no foreign unit
/// may enter our land for 30 turns; March of the Trees: a Treant on every forest of ours, the
/// forests' age spent) once the invasion reaches a city ([`city_in_peril`]): the civilizations at
/// war with us have at least 1.5 times our power (More Naval AI's `reqSanctuary` /
/// `reqMarchOfTheTrees`). FfH's own condition, two wars at once, fired late and during our own
/// winning offensives as often as in a losing defence, where the lockout and the Treants delay an
/// attacker we would have held anyway and March of the Trees spends the elves' Ancient Forests for
/// nothing (docs/AI_LOG.md, "World spells at moments of their own"). The power ratio alone cast
/// Sanctuary at the declaration of a war whose invasion came after it had worn off.
pub fn losing_war(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let ours = g.players[p].power.max(1) as i64;
    let theirs: i64 = g
        .player_ids()
        .filter(|&o| {
            let other = &g.players[o];
            other.alive && !g.player_is_barbarian(o) && other.team != team && g.at_war(team, other.team)
        })
        .map(|o| g.players[o].power as i64)
        .sum();
    theirs * 100 >= ours * 150
}

/// An invasion at the gates: two cities of ours, or the capital, each with at least
/// [`INVASION_UNITS`] units of a civilization at war with us within three plots whose attack
/// exceeds the city's garrison (fortified defenders at 1.5 times their power, as `View::peril`).
/// Barbarians and a raid on one city do not count: the spell is cast once a game, and the
/// Elohim's Sanctuary spent on a raid of four units at one city (seed 21, turn 141; the city held
/// without it) was gone when the real invasion came seventy turns later. docs/AI_LOG.md ("World
/// spells at moments of their own") names this moment: when cities would fall, several in peril
/// or the capital.
pub fn city_in_peril(g: &Game, p: PlayerId) -> bool {
    let team = g.players[p].team;
    let barb = g.barbarian_team();
    let capital = g.player_get_capital_city(p);
    let in_peril = |c: CityId| {
        let (x, y) = (g.cities[c].x, g.cities[c].y);
        let (mut n, mut threat) = (0, 0);
        for dx in -3..=3 {
            for dy in -3..=3 {
                let Some(q) = g.plot_xy(x, y, dx, dy) else { continue };
                for &u in &g.map.plots[q].units {
                    let other = g.players[g.units[u].owner].team;
                    // Fog included (`view::known_threat`).
                    if other == team || other == barb || !g.at_war(team, other) || !crate::view::known_threat(g, p, u) {
                        continue;
                    }
                    let power = crate::view::attack_power(g, u);
                    if power > 0 {
                        n += 1;
                        threat += power;
                    }
                }
            }
        }
        n >= INVASION_UNITS && threat > crate::production::garrison(g, p, g.city_plot(c)).1 * 3 / 2
    };
    let mut perils = 0;
    for &c in &g.players[p].cities {
        if in_peril(c) {
            if Some(c) == capital {
                return true;
            }
            perils += 1;
            if perils >= 2 {
                return true;
            }
        }
    }
    false
}

/// Units an invasion has at least ([`city_in_peril`]).
const INVASION_UNITS: i32 = 4;

/// `reqRagingSeas`'s AI condition: a player we are at war with (the barbarians included) has a
/// fleet worth more than 10 (frigates and galleons 2, men-o'-war and queens of the line 3, other
/// warships and transports 1).
fn enemy_fleet(g: &Game, p: PlayerId) -> bool {
    let d = g.d;
    let team = g.players[p].team;
    let weights = [
        ("UNITCLASS_ARCANE_BARGE", 1),
        ("UNITCLASS_CARAVEL", 1),
        ("UNITCLASS_GALLEY", 1),
        ("UNITCLASS_FRIGATE", 2),
        ("UNITCLASS_GALLEON", 2),
        ("UNITCLASS_MAN_O_WAR", 3),
        ("UNITCLASS_PRIVATEER", 1),
        ("UNITCLASS_QUEEN_OF_THE_LINE", 3),
        ("UNITCLASS_TRIREME", 1),
    ];
    g.player_ids().any(|o| {
        let other = &g.players[o];
        other.alive
            && g.teams[team].at_war[other.team]
            && weights.iter().map(|&(class, w)| other.unit_class_count[d.t::<UnitClassType>(class)] * w).sum::<i32>() > 10
    })
}
