//! The approved minor leaders are usable by breakaway factions, but never new-game choices.

use ffh_data::{jsonc, *};

fn approved() -> jsonc::Value {
    jsonc::parse(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/minor-leaders/concepts.json"))).unwrap()
}

fn string<'a>(value: &'a jsonc::Value, key: &str) -> &'a str {
    value.get(key).unwrap().as_str().unwrap()
}

#[test]
fn all_approved_leaders_load_with_exact_traits_alignment_and_backstory_but_are_not_selectable() {
    let d = global();
    let source = approved();
    let civs = source.as_arr().unwrap();
    assert_eq!(civs.len(), 21);
    let mut count = 0;
    for civ in civs {
        let ct = d.t::<CivilizationType>(string(civ, "civilization"));
        let leaders = civ.get("leaders").unwrap().as_arr().unwrap();
        assert_eq!(leaders.len(), 10, "{}", d.type_name(ct));
        for concept in leaders {
            let name = format!("LEADER_{}", string(concept, "id").to_ascii_uppercase());
            let lt = d.t::<LeaderHeadType>(&name);
            let info = &d.leader_heads[lt];
            assert_eq!(info.minor_leader_of, Some(ct), "{name}");
            assert!(info.preferred_breakaway, "{name}");
            assert!(d.civilizations.iter().all(|c| !c.leaders[lt]), "{name} is not a selectable leader of any civilization");
            assert_eq!(info.alignment, d.ty::<AlignmentType>(&format!("ALIGNMENT_{}", string(concept, "alignment").to_ascii_uppercase())), "{name}");
            assert_eq!(d.text.name(&info.base.description), string(concept, "name"), "{name}");
            let pedia = d.text.get(&info.base.civilopedia);
            assert!(pedia.contains(string(concept, "epithet")), "{name}: epithet");
            assert!(pedia.contains(string(concept, "backstory")), "{name}: backstory");
            let expected: Vec<TraitType> = concept.get("traits").unwrap().as_arr().unwrap().iter().map(|t| d.t(t.as_str().unwrap())).collect();
            let actual: Vec<TraitType> = d.traits.keys().filter(|&t| info.has_trait[t]).collect();
            assert_eq!(actual.len(), expected.len(), "{name}: only the approved personal traits");
            assert!(expected.iter().all(|&t| info.has_trait[t]), "{name}");
            assert!(info.permanent_trait.is_none(), "{name}: no inherited Adaptive identity trait");
            count += 1;
        }
    }
    assert_eq!(count, 210);
    assert_eq!(d.leader_heads.iter().filter(|l| l.preferred_breakaway).count(), 210);
}

/// The owner's rule (2026-11-21): a minor leader's alignment is at most one step
/// (Good - Neutral - Evil) from that of at least one major leader of its civilization, since it
/// heads a House inside that civilization's realm. Major leaders are those the civilization lists
/// that are not minor leaders; Decius, listed under three civilizations, does not count.
#[test]
fn every_minor_leader_is_within_one_alignment_step_of_a_major_leader() {
    let d = global();
    let steps = ["ALIGNMENT_GOOD", "ALIGNMENT_NEUTRAL", "ALIGNMENT_EVIL"].map(|a| d.t::<AlignmentType>(a));
    let step = |a: Option<AlignmentType>| steps.iter().position(|&s| Some(s) == a);
    let decius = d.t::<LeaderHeadType>("LEADER_DECIUS");
    let mut checked = 0;
    for (lt, info) in d.leader_heads.iter_enumerated() {
        let Some(civ) = info.minor_leader_of else { continue };
        let majors: Vec<LeaderHeadType> = d.leader_heads.keys().filter(|&m| d.civilizations[civ].leaders[m] && m != decius && d.leader_heads[m].minor_leader_of.is_none()).collect();
        assert!(!majors.is_empty(), "{} has no major leader", d.type_name(civ));
        let own = step(info.alignment).unwrap_or_else(|| panic!("{} has no alignment", d.type_name(lt)));
        let major_steps: Vec<usize> = majors.iter().map(|&m| step(d.leader_heads[m].alignment).unwrap_or_else(|| panic!("{} has no alignment", d.type_name(m)))).collect();
        let names = |s: &[usize]| s.iter().map(|&i| d.type_name(steps[i])).collect::<Vec<_>>().join(", ");
        assert!(
            major_steps.iter().any(|&m| own.abs_diff(m) <= 1),
            "{} is {}, more than one step from every major leader of {} ({})",
            d.type_name(lt),
            d.type_name(steps[own]),
            d.type_name(civ),
            names(&major_steps)
        );
        checked += 1;
    }
    assert!(checked >= 210, "every minor leader checked: {checked}");
}

#[test]
fn changed_alignments_keep_their_explicit_civilization_commitments() {
    let d = global();
    let veil = d.t::<ReligionType>("RELIGION_THE_ASHEN_VEIL");
    let mercurians = d.t::<CivilizationType>("CIVILIZATION_MERCURIANS");
    let agnostic = d.t::<TraitType>("TRAIT_AGNOSTIC");
    for info in d.leader_heads.iter().filter(|l| l.preferred_breakaway) {
        let civ = info.minor_leader_of.unwrap();
        if civ == mercurians {
            assert_eq!(info.religion_weight_modifier[veil], -100, "{} remains anti-demon", info.base.type_);
        }
        if info.has_trait[agnostic] || d.civilizations[civ].civ_trait == Some(agnostic) {
            assert!(info.religion_weight_modifier.iter().all(|&w| w == -100), "{} remains agnostic", info.base.type_);
        }
        if info.alignment == d.ty::<AlignmentType>("ALIGNMENT_GOOD") {
            assert_ne!(info.favorite_civic, d.ty::<CivicType>("CIVIC_SLAVERY"), "{}", info.base.type_);
        }
    }
}
