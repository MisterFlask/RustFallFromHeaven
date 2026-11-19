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
