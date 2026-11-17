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
const TURNS: &[fn(&mut Game, &crate::view::View)] = &[
    crate::cards_map_rules::turn, // batch B9: the map and movement cards
    crate::cards_world_spells::turn, // batch B3: the card world spells
    crate::cards_deck_ui::turn, // batch B0: the deck UI's three targeted cards
    crate::cards_battle_magic::turn, // batch B1: the battle magic cards
    crate::cards_new_civs::turn, // batch B4: the new civilizations
    crate::cards_economy::turn, // batch B10: the economy cards
    crate::cards_world_spells_2::turn, // batch B7: the second card world spells and the rituals
    crate::cards_new_civs_2::turn, // batch B8: the new civilizations, second batch
    crate::cards_battle_more::turn, // batch B6: the battle cards, second batch
];

/// The cards' content that acts once the player's units have moved this turn (a unit's own action
/// that must not hold its stack back): each batch's function in [`END_TURNS`].
pub fn end_turn(g: &mut Game, p: PlayerId) {
    for f in END_TURNS {
        f(g, p);
    }
}

/// Each card batch's end-of-turn function.
const END_TURNS: &[fn(&mut Game, PlayerId)] = &[
    crate::cards_battle_arms::end_turn,
    crate::cards_economy::end_turn, // the Court of Mammon's Buy Out (batch B8's Infernal Gate)
];

/// How many turns a great person waits for its card's needs.
const WAIT_TURNS: i32 = 30;

/// How many of its card's needs may be missing for a great person to wait.
const WAIT_MISSING: usize = 2;

/// `SPELL_DISCOVER_CARD`.
pub fn discover_spell(d: &GameData) -> SpellType {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The sources whose card content is wired by type name: the batches' tables and the inline
    /// lookups (`d.ty`, an `Option`) that skip a missing name without a word.
    const SOURCES: &[(&str, &str)] = &[
        ("cards.rs", include_str!("cards.rs")),
        ("cards_battle_arms.rs", include_str!("cards_battle_arms.rs")),
        ("cards_battle_magic.rs", include_str!("cards_battle_magic.rs")),
        ("cards_battle_more.rs", include_str!("cards_battle_more.rs")),
        ("cards_deck_ui.rs", include_str!("cards_deck_ui.rs")),
        ("cards_economy.rs", include_str!("cards_economy.rs")),
        ("cards_map_rules.rs", include_str!("cards_map_rules.rs")),
        ("cards_new_civs.rs", include_str!("cards_new_civs.rs")),
        ("cards_new_civs_2.rs", include_str!("cards_new_civs_2.rs")),
        ("cards_world_spells.rs", include_str!("cards_world_spells.rs")),
        ("cards_world_spells_2.rs", include_str!("cards_world_spells_2.rs")),
        ("world_spells.rs", include_str!("world_spells.rs")),
        ("world_rituals.rs", include_str!("world_rituals.rs")),
        ("lanun_khazad.rs", include_str!("lanun_khazad.rs")),
        ("civs/mod.rs", include_str!("civs/mod.rs")),
    ];

    /// Prefixes of quoted capitals that are not type names: `FFH_` environment variables, and
    /// the civilization names of `civs/mod.rs` written without `CIVILIZATION_` (checked there,
    /// `every_civ_name_resolves`).
    const NOT_TYPES: &[&str] = &["FFH", "CLAN", "COURT"];

    /// Whether `name` resolves in the data as the kind its prefix names (`None`: a prefix this
    /// test does not know).
    fn resolves(d: &GameData, name: &str) -> Option<bool> {
        let prefix = name.split('_').next().unwrap();
        Some(match prefix {
            "UNIT" => d.ty::<UnitType>(name).is_some(),
            "UNITCLASS" => d.ty::<UnitClassType>(name).is_some(),
            "UNITCOMBAT" => d.ty::<UnitCombatType>(name).is_some(),
            "SPELL" => d.ty::<SpellType>(name).is_some(),
            "TECH" => d.ty::<TechType>(name).is_some(),
            "FLAVOR" => d.ty::<FlavorType>(name).is_some(),
            "TRAIT" => d.ty::<TraitType>(name).is_some(),
            "BUILDING" => d.ty::<BuildingType>(name).is_some(),
            "BUILDINGCLASS" => d.ty::<BuildingClassType>(name).is_some(),
            "BUILD" => d.ty::<BuildType>(name).is_some(),
            "PROJECT" => d.ty::<ProjectType>(name).is_some(),
            "CIVILIZATION" => d.ty::<CivilizationType>(name).is_some(),
            "PROMOTION" => d.ty::<PromotionType>(name).is_some(),
            "IMPROVEMENT" => d.ty::<ImprovementType>(name).is_some(),
            "BONUS" => d.ty::<BonusType>(name).is_some(),
            "ALIGNMENT" => d.ty::<AlignmentType>(name).is_some(),
            "WORLDSIZE" => d.ty::<WorldSizeType>(name).is_some(),
            _ => return None,
        })
    }

    /// The quoted type-like names of `src` (`"PREFIX_REST"`, capitals, digits and underscores),
    /// outside `//` comments.
    fn quoted_names(src: &str) -> Vec<&str> {
        let mut out = Vec::new();
        for line in src.lines() {
            let code = line.find("//").map_or(line, |i| &line[..i]);
            let mut parts = code.split('"');
            parts.next();
            // Every other part is inside quotes.
            while let Some(inside) = parts.next() {
                // A bare prefix (`"UNIT_"`, for `trim_start_matches`) is not a name.
                let ok = inside.contains('_') && !inside.ends_with('_') && inside.starts_with(|c: char| c.is_ascii_uppercase()) && inside.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
                if ok {
                    out.push(inside);
                }
                parts.next();
            }
        }
        out
    }

    /// Every hard-coded type name in the card content's AI is in the game data: a renamed or
    /// misspelt name would make the AI skip its card silently (the tables look names up with
    /// `d.ty`). New kinds of names need a line in [`resolves`].
    #[test]
    fn every_card_type_name_resolves() {
        let d = ffh_data::global();
        let mut checked = 0;
        let mut missing = Vec::new();
        for &(file, src) in SOURCES {
            for name in quoted_names(src) {
                match resolves(d, name) {
                    Some(true) => checked += 1,
                    Some(false) => missing.push(format!("{file}: {name}")),
                    None => assert!(NOT_TYPES.contains(&name.split('_').next().unwrap()), "{file}: {name}: a kind of name this test does not know"),
                }
            }
        }
        eprintln!("{checked} type names resolve");
        assert!(missing.is_empty(), "names not in the data:\n{}", missing.join("\n"));
        assert!(checked > 200, "only {checked} names found: has the way the tables are written changed?");
    }
}
