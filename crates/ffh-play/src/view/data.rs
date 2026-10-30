//! The static data tables (`data`, design §4.7): one array per named kind (every info table,
//! GlobalTypes list and hard-coded C++ enum of [`crate::names::KINDS`]), indexed like the
//! protocol's type indices. Each entry is `{type, name}`; info-table entries add the help,
//! strategy and pedia texts (Civ4 markup kept, `null` when the text is missing) and the
//! gameplay fields the renderer and the screens need for the main tables, and `effects`: what an
//! entry does, as Civ4 markup ([`crate::view::help::effects_of`]; unique improvements:
//! [`crate::view::improvement_help`]). Built once.

use std::sync::OnceLock;

use ffh_data::*;

use crate::json::Json;
use crate::names::{self, KINDS};

/// Text of a key as `gDLL->getText(key)` gives it (`%%` is a percent sign: "a 5% defensive
/// bonus"), or `null` when the key is empty or has no text.
fn text(d: &GameData, key: &str) -> Json {
    if key.is_empty() {
        return Json::Null;
    }
    let t = names::text(d, key);
    if t == key {
        Json::Null
    } else {
        Json::from(crate::view::help::fill(t, &[]))
    }
}

/// A name without its hyperlinks: the builds' texts link their improvement ("Build a
/// [LINK=IMPROVEMENT_FARM]Farm[\LINK]"), which reads as "Build a Farm" where names are plain text.
fn unlinked(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(i) = rest.find('[') {
        out.push_str(&rest[..i]);
        let tag = &rest[i..];
        let end = tag.find(']').map(|j| j + 1);
        match end {
            Some(j) if tag.starts_with("[LINK=") || tag.starts_with("[\\LINK]") => rest = &tag[j..],
            _ => {
                out.push('[');
                rest = &tag[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn yields(y: &[i32; YieldType::NUM]) -> Json {
    Json::from(y.to_vec())
}

/// The gameplay fields of entry `i` of the table `kind`.
fn fields(d: &GameData, kind: &str, i: usize, j: &mut Json) {
    match kind {
        "TerrainType" => {
            let t = &d.terrains[TerrainType::from_usize(i)];
            j.set("yields", yields(&t.yield_));
            j.set("movement_cost", t.movement_cost);
            j.set("defense", t.defense_modifier);
            j.set("water", t.water);
            j.set("impassable", t.impassable);
        }
        "CulturalActionType" => {
            let a = &d.cultural_actions[CulturalActionType::from_usize(i)];
            j.set("tier", a.tier);
            j.set("rate", a.rate);
            j.set("duration", a.duration);
            j.set("target", match a.target {
                CultureTargetKind::OwnOrAllyCity => "own_or_ally",
                CultureTargetKind::ForeignCity => "foreign",
                CultureTargetKind::AnyCity => "any",
            });
            j.set("hostile", match a.hostile {
                CultureHostility::Never => "never",
                CultureHostility::Always => "always",
                CultureHostility::DifferentStateReligion => "different_state_religion",
            });
            j.set("effect", match a.effect {
                CultureEffect::Festival => "festival",
                CultureEffect::TradeFair => "trade_fair",
                CultureEffect::SpreadReligion => "spread_religion",
                CultureEffect::InciteUnrest => "incite_unrest",
                CultureEffect::SiphonKnowledge => "siphon_knowledge",
                CultureEffect::FomentRevolt => "foment_revolt",
                CultureEffect::FlipCity => "flip_city",
            });
        }
        "FeatureType" => {
            let f = &d.features[FeatureType::from_usize(i)];
            j.set("yields", yields(&f.yield_change));
            j.set("movement_cost", f.movement_cost);
            j.set("defense", f.defense_modifier);
        }
        "BonusType" => {
            let b = &d.bonuses[BonusType::from_usize(i)];
            j.set("yields", yields(&b.yield_change));
            j.set("class", Json::opt_idx(b.bonus_class_type));
            j.set("tech_reveal", Json::opt_idx(b.tech_reveal));
            j.set("tech_trade", Json::opt_idx(b.tech_city_trade));
            j.set("health", b.health);
            j.set("happiness", b.happiness);
        }
        "ImprovementType" => {
            let m = &d.improvements[ImprovementType::from_usize(i)];
            j.set("yields", yields(&m.yield_change));
            j.set("defense", m.defense_modifier);
            j.set("upgrade", Json::opt_idx(m.improvement_upgrade));
            j.set("upgrade_time", m.upgrade_time);
            j.set("permanent", m.permanent);
            j.set("unique", m.unique);
            if m.unique {
                j.set("effects", crate::view::improvement_help::effects(d, ImprovementType::from_usize(i)));
            }
        }
        "RouteType" => {
            let r = &d.routes[RouteType::from_usize(i)];
            j.set("yields", yields(&r.yield_change));
            j.set("movement_cost", r.movement_cost);
            j.set("value", r.value);
        }
        "UnitType" => {
            let u = &d.units[UnitType::from_usize(i)];
            j.set("class", Json::opt_idx(u.unit_class_type));
            j.set("combat", u.combat);
            j.set("combat_defense", u.combat_defense);
            j.set("moves", u.moves);
            j.set("cost", u.production_cost);
            j.set("domain", Json::opt_idx(u.domain_type));
            j.set("unit_combat", Json::opt_idx(u.unit_combat_type));
            j.set("tech", Json::opt_idx(u.prereq_and_tech));
            j.set("tier", u.tier);
            j.set("found", u.found);
            j.set("worker", u.builds.iter().any(|&b| b));
        }
        "BuildingType" => {
            let b = &d.buildings[BuildingType::from_usize(i)];
            j.set("class", Json::opt_idx(b.building_class_type));
            j.set("cost", b.production_cost);
            j.set("tech", Json::opt_idx(b.prereq_and_tech));
        }
        "TechType" => {
            let t = &d.techs[TechType::from_usize(i)];
            j.set("cost", t.research_cost);
            j.set("era", Json::opt_idx(t.era));
            j.set("and", Json::Arr(t.prereq_and_techs.iter().map(|&p| Json::idx(p)).collect()));
            j.set("or", Json::Arr(t.prereq_or_techs.iter().map(|&p| Json::idx(p)).collect()));
        }
        "CivicType" => {
            let c = &d.civics[CivicType::from_usize(i)];
            j.set("option", Json::opt_idx(c.civic_option_type));
            j.set("upkeep", Json::opt_idx(c.upkeep));
            j.set("anarchy", c.anarchy_length);
            j.set("tech", Json::opt_idx(c.tech_prereq));
        }
        "ReligionType" => {
            let r = &d.religions[ReligionType::from_usize(i)];
            // The first to discover it founds the religion.
            j.set("tech", Json::opt_idx(r.tech_prereq));
            j.set("alignment", Json::opt_idx(r.alignment));
        }
        "SpecialistType" => {
            let s = &d.specialists[SpecialistType::from_usize(i)];
            j.set("yields", yields(&s.yield_change));
            j.set("commerces", Json::from(s.commerce_change.to_vec()));
            j.set("great_people", s.great_people_rate_change);
            j.set("experience", s.experience);
        }
        "SpellType" => {
            let s = &d.spells[SpellType::from_usize(i)];
            j.set("global", s.global);
            j.set("causes_war", s.causes_war);
            j.set("range", s.range);
        }
        "BuildType" => {
            let b = &d.builds[BuildType::from_usize(i)];
            j.set("improvement", Json::opt_idx(b.improvement));
            j.set("route", Json::opt_idx(b.route));
            j.set("time", b.time);
            j.set("tech", Json::opt_idx(b.tech_prereq));
        }
        "CommandType" => {
            let c = &d.commands[CommandType::from_usize(i)];
            j.set("confirm", c.confirm_command);
            j.set("all", c.all);
        }
        _ => {}
    }
}

fn build(d: &GameData) -> Json {
    let dir = ffh_data::default_data_dir();
    let mut tables = Json::obj();
    for k in KINDS {
        // The action hotkeys and button order (`keys`, `order`) of missions, commands and builds.
        let keys = crate::view::hotkeys::read(&dir, k.kind);
        let mut list = Vec::with_capacity((k.count)(d));
        for i in 0..(k.count)(d) {
            let name = (k.name)(d, i);
            let mut j = crate::jobj! { "type" => name, "name" => unlinked(&k.display(d, i)) };
            if let Some(base) = (k.base)(d, i) {
                j.set("help", text(d, &base.help));
                j.set("strategy", text(d, &base.strategy));
                j.set("pedia", text(d, &base.civilopedia));
            }
            fields(d, k.kind, i, &mut j);
            // What it does (the help texts, `view::help`); the unique improvements set theirs above.
            if let Some(e) = crate::view::help::effects_of(d, k.kind, i) {
                j.set("effects", e);
            }
            if let Some((_, a)) = keys.iter().find(|(t, _)| t == name) {
                j.set("keys", Json::Arr(a.bindings.iter().map(crate::view::hotkeys::Binding::to_json).collect()));
                j.set("order", a.order);
            }
            list.push(j);
        }
        tables.set(k.kind, Json::Arr(list));
    }
    crate::jobj! {
        "kinds" => Json::Arr(KINDS.iter().map(|k| Json::from(k.kind)).collect()),
        "tables" => tables,
        "interface_modes" => crate::view::hotkeys::interface_modes_json(),
        // The global defines the screens use (the sliders' step).
        "defines" => crate::jobj! { "commerce_percent_change_increments" => d.defines.commerce_percent_change_increments },
    }
}

/// The `data` reply (built once per process; the data is global).
pub fn data(d: &'static GameData) -> &'static Json {
    static DATA: OnceLock<Json> = OnceLock::new();
    DATA.get_or_init(|| build(d))
}

#[cfg(test)]
mod tests {
    use super::unlinked;

    #[test]
    fn names_lose_their_links() {
        assert_eq!(unlinked("Build a [LINK=IMPROVEMENT_FARM]Farm[\\LINK]"), "Build a Farm");
        assert_eq!(unlinked("+1 Trade Route ([ICON_TRADE])"), "+1 Trade Route ([ICON_TRADE])");
        assert_eq!(unlinked("Warrior"), "Warrior");
        assert_eq!(unlinked("a [b"), "a [b");
    }
}
