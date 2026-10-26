//! What a leader says (Civ4's diplomacy screen): the line an AI leader speaks to the human in a
//! diplomacy moment (`AI_DIPLOCOMMENT_*`), chosen from `CIV4DiplomacyInfos.xml` as the exe does,
//! with its `[CT_NAME]`-style placeholders filled in. Presentation only.
//!
//! The moments the client shows (design §5.1 "Diplomacy minimum"; docs/HUMAN.md "Diplomacy"):
//! * the leader opens the screen: first contact (`AI_DIPLOCOMMENT_FIRST_CONTACT`) and a war it
//!   declares on the human (`AI_DIPLOCOMMENT_DECLARE_WAR`), from the game log's diplomacy entries
//!   (`ffh_core::log::DiploEntry`, the `beginDiplomacy` calls of `CvTeam::makeHasMet` and
//!   `CvTeam::declareWar`), queued as the session's moments;
//! * the human opens it: the leader's greeting ([`greeting`], a port of
//!   `CvPlayerAI::AI_getGreeting`), or `AI_DIPLOCOMMENT_REFUSE_TO_TALK` when it will not talk
//!   (`AI_isWillingToTalk`, [`super::diplo::ai_willing_to_talk`]);
//! * its answer to a proposed deal, which the client picks from the deal and the answer (the
//!   `speech` query takes any `AI_DIPLOCOMMENT_*`).
//!
//! **Choosing a response.** The exe's choice is not in the DLL source. A response matches when
//! each of its filters (civilizations, leaders, attitudes towards the human, power relative to
//! the human) is empty or holds the AI's value; among the matching responses those naming the
//! AI's leader win, then those naming its civilization (FfH2 gives every leader its own lines
//! for the main moments, and a few moments keep the stock BtS lines for all, which would
//! otherwise drown the leader's own), and one of their lines is picked. The pick and the
//! greeting's chances (C++ `GC.getASyncRand()`, the interface's own generator) come from a
//! caller's `salt`, so a query stays pure.
//!
//! **Power** (`DiplomacyPowerTypes`, also not in the DLL): the AI is weaker when its power
//! (`CvPlayer::getPower`) is below 4/5 of the human's, stronger above 5/4 of it, else equal.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::json::Json;
use crate::query::{invalid, QueryErr};
use crate::view::Viewer;

/// The interface's random numbers for one line (C++ `GC.getASyncRand()`): splitmix64 from the
/// caller's salt.
pub struct Async(u64);

impl Async {
    pub fn new(salt: u64) -> Async {
        Async(salt ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// A number in `0..n` (C++ `CvRandom::get(n)`).
    pub fn get(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        if n == 0 {
            0
        } else {
            (z % n as u64) as u32
        }
    }
}

/// `ai`'s power relative to `other`'s (see the module docs).
pub fn power(g: &Game, ai: PlayerId, other: PlayerId) -> DiplomacyPowerType {
    let (ours, theirs) = (g.players[ai].power as i64, g.players[other].power as i64);
    if ours * 5 < theirs * 4 {
        DiplomacyPowerType::DIPLOMACYPOWER_WEAKER
    } else if ours * 4 > theirs * 5 {
        DiplomacyPowerType::DIPLOMACYPOWER_STRONGER
    } else {
        DiplomacyPowerType::DIPLOMACYPOWER_EQUAL
    }
}

/// C++ `CvPlayerAI::AI_getGreeting(ePlayer)`: what `ai` opens with when `other` contacts it.
/// Sometimes, with its interface random numbers, a word on its worst enemy (on trading with it),
/// its nukes, or its army; else `AI_DIPLOCOMMENT_GREETINGS`.
pub fn greeting(g: &Game, ai: PlayerId, other: PlayerId, rand: &mut Async) -> DiploCommentType {
    let d = g.d;
    let (team, their) = (g.players[ai].team, g.players[other].team);
    if their != team {
        let worst_enemy = g.teams[team].ai_worst_enemy;
        // As in C++, a random number is drawn only when the tests before it pass.
        if let Some(we) = worst_enemy.filter(|&we| we != their && g.teams[their].has_met[we]).filter(|_| rand.get(4) == 0) {
            let traded = g.player_ids().any(|p| g.players[p].alive && g.players[p].team == we && g.players[other].ai_peacetime_grant_value[p] + g.players[other].ai_peacetime_trade_value[p] > 0);
            return if traded && !g.at_war(their, we) { d.t("AI_DIPLOCOMMENT_WORST_ENEMY_TRADING") } else { d.t("AI_DIPLOCOMMENT_WORST_ENEMY") };
        } else if g.players[ai].num_nuke_units > 0 && rand.get(4) == 0 {
            return d.t("AI_DIPLOCOMMENT_NUKES");
        } else if g.players[other].power < g.players[ai].power && g.player_ai_get_attitude(ai, other, true) < AttitudeType::ATTITUDE_PLEASED && rand.get(4) == 0 {
            return d.t("AI_DIPLOCOMMENT_UNIT_BRAG");
        }
    }
    d.t("AI_DIPLOCOMMENT_GREETINGS")
}

/// The text keys of the lines `ai` may say to `other` for `comment` (see the module docs);
/// empty when no response matches.
pub fn lines(g: &Game, ai: PlayerId, other: PlayerId, comment: DiploCommentType) -> Vec<&'static str> {
    let d = g.d;
    let leader = g.player_get_leader_type(ai);
    let civ = g.player_get_civilization_type(ai);
    let attitude = g.player_ai_get_attitude(ai, other, true);
    let power = power(g, ai, other);
    fn fits<I: Idx>(flags: &IdxVec<I, bool>, v: Option<I>) -> Option<bool> {
        if !flags.iter().any(|&b| b) {
            Some(false)
        } else if v.is_some_and(|v| flags[v]) {
            Some(true)
        } else {
            None
        }
    }
    let mut best: Option<(u8, Vec<&'static str>)> = None;
    for r in &d.diplomacies[comment].responses {
        let (Some(by_leader), Some(by_civ), Some(_), Some(_)) =
            (fits(&r.leader_head_types, leader), fits(&r.civilization_types, civ), fits(&r.attitude_types, Some(attitude)), fits(&r.diplomacy_power_types, Some(power)))
        else {
            continue;
        };
        let rank = 2 * by_leader as u8 + by_civ as u8;
        match &mut best {
            Some((b, _)) if *b > rank => {}
            Some((b, keys)) if *b == rank => keys.extend(r.diplomacy_text.iter().map(String::as_str)),
            _ => best = Some((rank, r.diplomacy_text.iter().map(String::as_str).collect())),
        }
    }
    best.map(|(_, keys)| keys).unwrap_or_default()
}

/// The line `ai` says to `other` for `comment`, its placeholders filled in (`None` when no
/// response has a line for them).
pub fn line(g: &Game, ai: PlayerId, other: PlayerId, comment: DiploCommentType, rand: &mut Async) -> Option<String> {
    let keys = lines(g, ai, other, comment);
    if keys.is_empty() {
        return None;
    }
    let key = keys[rand.get(keys.len() as u32) as usize];
    Some(fill(g, ai, other, g.d.text.get(key)))
}

/// `speech {with, comment, salt}`: `{with, comment, text}`, what the leader of `with` says to the
/// human for `comment`; with no comment, what it says when the human contacts it: its greeting
/// ([`greeting`]), or `AI_DIPLOCOMMENT_REFUSE_TO_TALK` when it will not talk. `text` is `null`
/// when it has no line for it.
pub fn speech(v: &Viewer, with: PlayerId, comment: Option<DiploCommentType>, salt: i64) -> Result<Json, QueryErr> {
    let g = v.g;
    if with.idx() >= g.num_civ_players() || with == v.h || g.player_is_barbarian(with) || !v.player_met(with) || g.players[with].human || !g.players[with].alive {
        return Err(invalid(format!("field `with`: player {} is not a living AI civilization the human knows", with.0)));
    }
    let mut rand = Async::new(salt as u64);
    let comment = match comment {
        Some(c) => c,
        None if !super::diplo::ai_willing_to_talk(g, with, v.h) => g.d.t("AI_DIPLOCOMMENT_REFUSE_TO_TALK"),
        None => greeting(g, with, v.h, &mut rand),
    };
    let text = line(g, with, v.h, comment, &mut rand);
    Ok(crate::jobj! { "with" => with, "comment" => Json::idx(comment), "text" => text })
}

/// The best unit `ai` has (the strongest by combat strength, then the most expensive), for
/// `[OUR_BEST_UNIT]`.
fn best_unit(g: &Game, ai: PlayerId) -> Option<UnitType> {
    let d = g.d;
    g.players[ai].units.iter().filter(|&&u| g.units.contains(u)).map(|&u| g.units[u].unit_type).filter(|&t| d.units[t].combat > 0).max_by_key(|&t| (d.units[t].combat, d.units[t].production_cost, std::cmp::Reverse(t)))
}

/// A line's placeholders: `[OUR_*]` are the speaking AI's, `[CT_*]` (the contact) the human's;
/// `[NEWLINE]` breaks the line; a `%s1` argument (none of the moments shown has one) and a
/// `[NUM1:he:she:..]` gender choice (the first form) are dropped to their plain forms.
pub fn fill(g: &Game, ai: PlayerId, other: PlayerId, text: &str) -> String {
    let d = g.d;
    let name = |p: PlayerId| crate::view::player_name(g, p);
    let civ_short = |p: PlayerId| crate::names::resolve_keys(d, &g.player_get_civilization_short_description(p)).into_owned();
    let civ_adj = |p: PlayerId| crate::names::resolve_keys(d, &g.player_get_civilization_adjective(p)).into_owned();
    let empire = |p: PlayerId| crate::names::resolve_keys(d, &g.player_get_civilization_description(p)).into_owned();
    let mut out = String::with_capacity(text.len() + 32);
    let mut rest = text;
    while let Some(at) = rest.find(['[', '%']) {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        if let Some(arg) = tail.strip_prefix("%s1") {
            rest = arg;
            continue;
        }
        let Some(end) = tail.find(']').filter(|_| tail.starts_with('[')) else {
            out.push_str(&tail[..1]);
            rest = &tail[1..];
            continue;
        };
        let tag = &tail[1..end];
        let value = match tag {
            "CT_NAME" => name(other),
            "OUR_NAME" => name(ai),
            "CT_CIV_SHORT" => civ_short(other),
            "OUR_CIV_SHORT" => civ_short(ai),
            "CT_CIV_ADJ" => civ_adj(other),
            "OUR_CIV_ADJ" => civ_adj(ai),
            "CT_EMPIRE" => empire(other),
            "OUR_EMPIRE" => empire(ai),
            "NEWLINE" => "\n".into(),
            "OUR_STATE_RELIGION" => g.player_get_state_religion(ai).map_or_else(String::new, |r| d.text.name(&d.religions[r].adjective_key).to_string()),
            "OUR_BEST_UNIT" => best_unit(g, ai).map_or_else(|| "army".into(), |u| d.text.name(&d.units[u].base.description).to_string()),
            "OUR_WORST_ENEMY" => g.teams[g.players[ai].team].ai_worst_enemy.and_then(|t| g.team_get_leader_id(t)).map_or_else(String::new, civ_short),
            _ if tag.starts_with("NUM1:") => tag.split(':').nth(1).unwrap_or("").to_string(),
            // Not a placeholder: kept as written.
            _ => tail[..=end].to_string(),
        };
        out.push_str(&value);
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_leader_has_its_own_lines_for_the_moments_shown() {
        let d = ffh_data::global();
        // FfH2's leaders (those its lines name) each have a first contact, a war declaration, a
        // refusal to talk and a greeting for every attitude.
        let named: Vec<LeaderHeadType> = d.leader_heads.keys().filter(|&l| d.diplomacies[d.t::<DiploCommentType>("AI_DIPLOCOMMENT_FIRST_CONTACT")].responses.iter().any(|r| r.leader_head_types[l])).collect();
        assert!(named.len() >= 60, "{} leaders with a first contact line", named.len());
        for c in ["AI_DIPLOCOMMENT_FIRST_CONTACT", "AI_DIPLOCOMMENT_DECLARE_WAR", "AI_DIPLOCOMMENT_REFUSE_TO_TALK", "AI_DIPLOCOMMENT_GREETINGS", "AI_DIPLOCOMMENT_ACCEPT", "AI_DIPLOCOMMENT_NO_DEAL"] {
            let info = &d.diplomacies[d.t::<DiploCommentType>(c)];
            for &l in &named {
                for a in d.attitudes.keys() {
                    let n: usize = info.responses.iter().filter(|r| r.leader_head_types[l] && (!r.attitude_types.iter().any(|&b| b) || r.attitude_types[a])).map(|r| r.diplomacy_text.len()).sum();
                    assert!(n > 0, "{c}: no line of {} at {}", d.reg.type_name(l), d.reg.type_name(a));
                }
            }
            for r in &info.responses {
                for k in &r.diplomacy_text {
                    assert_ne!(d.text.get(k), k.as_str(), "{c}: {k} has no text");
                }
            }
        }
    }

    #[test]
    fn async_numbers_stay_in_range_and_vary() {
        let mut r = Async::new(7);
        let v: Vec<u32> = (0..64).map(|_| r.get(4)).collect();
        assert!(v.iter().all(|&x| x < 4));
        assert!((0..4).all(|x| v.contains(&x)));
        assert_eq!(Async::new(0).get(0), 0);
    }
}
