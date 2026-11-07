//! The great-person deck (docs/DECKS.md): what the AI does with the card a great person holds.
//! The card is worth a great person by design, so the AI discovers it as soon as its needs are
//! known; research is pulled toward missing needs (`research::goal_bonus`), and a young great
//! person waits for them while they are near. A card whose needs are far is given up: the great
//! person is used the old way and the card goes back into the deck.

use std::sync::OnceLock;

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

/// The AI's use of the cards' content, once a turn after the empire decisions: each batch of
/// cards adds its turn function to [`TURNS`] (world spells to cast, rituals to start, ...).
/// Content the generic AI already handles (units and buildings it values by their stats) needs
/// nothing here.
pub fn turn(g: &mut Game, v: &crate::view::View) {
    for f in TURNS {
        f(g, v);
    }
}

/// Each card batch's AI turn function.
const TURNS: &[fn(&mut Game, &crate::view::View)] = &[];

/// How many turns a great person waits for its card's needs.
const WAIT_TURNS: i32 = 30;

/// How many of its card's needs may be missing for a great person to wait.
const WAIT_MISSING: usize = 2;

fn discover_spell(d: &GameData) -> SpellType {
    static S: OnceLock<SpellType> = OnceLock::new();
    *S.get_or_init(|| d.t::<SpellType>("SPELL_DISCOVER_CARD"))
}

/// The needs of `card` that `p`'s team does not know (the and-prerequisites, or the cheapest
/// or-prerequisite when none is known).
pub fn missing_needs(g: &Game, p: PlayerId, card: TechType) -> Vec<TechType> {
    let team = g.players[p].team;
    let info = &g.d.techs[card];
    let mut out: Vec<TechType> = info.prereq_and_techs.iter().copied().filter(|&t| !g.team_is_has_tech(team, Some(t))).collect();
    if !info.prereq_or_techs.is_empty() && !info.prereq_or_techs.iter().any(|&t| g.team_is_has_tech(team, Some(t))) {
        if let Some(&t) = info.prereq_or_techs.iter().min_by_key(|&&t| (g.d.techs[t].research_cost, t)) {
            out.push(t);
        }
    }
    out
}

/// The techs `p` lacks for the cards its great people hold, in unit order, without repeats.
pub fn needed_techs(g: &Game, p: PlayerId) -> Vec<TechType> {
    let mut out = Vec::new();
    for &u in &g.players[p].units {
        if let Some(card) = g.units[u].card {
            for t in missing_needs(g, p, card) {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
    }
    out
}

/// What a great person does with its card.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CardPlan {
    /// Discovered now: the great person is used up.
    Discovered,
    /// Its needs are near and the great person young: keep it safe and wait.
    Wait,
    /// No card, or its needs are far: use the great person the old way.
    Ignore,
}

/// A great person's card: discovered if it can be; kept while its needs are near and the great
/// person is young.
pub fn great_person_card(g: &mut Game, u: UnitId) -> CardPlan {
    let Some(card) = g.units[u].card else { return CardPlan::Ignore };
    let spell = discover_spell(g.d);
    if g.unit_can_cast(u, spell, false) {
        crate::magic::cast(g, u, spell, "card", 0);
        return CardPlan::Discovered;
    }
    let p = g.units[u].owner;
    let team = g.players[p].team;
    let missing = missing_needs(g, p, card);
    let young = g.game_turn() - g.units[u].game_turn_created <= WAIT_TURNS;
    let near = missing.len() <= WAIT_MISSING
        && missing.iter().all(|&t| {
            g.player_can_research(p, t, false)
                || g.d.techs[t]
                    .prereq_and_techs
                    .iter()
                    .all(|&q| g.player_can_research(p, q, false) || g.team_is_has_tech(team, Some(q)))
        });
    if young && near {
        CardPlan::Wait
    } else {
        CardPlan::Ignore
    }
}
