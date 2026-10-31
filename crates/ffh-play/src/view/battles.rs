//! Engagement combat as the human sees it (docs/ENGAGEMENTS.md; phase6_plan 9.2 and 15; the
//! battle UI design's log, DESIGN.md section 10): the battle reports (`battle_reports`, the view's
//! `battles`, the turn report's `engagements`, the reply of the command that launched one), a
//! battle's log as the battle screens read it (`battle_log`; `assets/app/battle_log.js`'s header
//! lists every field), and the plan of an attack (`engage_preview`).
//!
//! **One shape for the screens.** The engine keeps a [`BattleReport`] (the summary, which
//! outlives the log) and its [`BattleLog`]; the screens read DESIGN's `BattleLog`, which is both:
//! the view joins them and words the enums as their names. What the saved report does not hold is
//! derived here, never stored: the attacking plot's terrain (the map's, now), each side's
//! civilization and label, each fighter's display name, target order (its role's) and
//! promotions (the unit's while it lives, else the promotions among its A and Dv parts), and the
//! units that moved in (the log's `MoveIn`).
//!
//! **Per viewer (G5).** A viewer sees the reports of the battles it fought on a side (the
//! engine's `BattleReport::involves`), or every one under debug reveal. Its own side shows its real
//! owners, the other side the owners it saw (`BattleReport::owner_for`: a hidden-nationality
//! attacker stays the barbarian it looked like); the other side's stance and army stay unseen
//! (phase 6 reveals no enemy stance). Every fighter of the log is shown: the engine logs only the
//! units the battle met (the defenders the attacker could see, the attackers that fought).
//!
//! **The plan** (`engage_preview`) is the attack before it is made: the pool on the attacking
//! plot, the force (the one asked for, checked, else the stance's proposal) and the stance's own
//! pick, the slots and why, the defenders the attacker sees and the ones likely to fight
//! (their stance unseen: those every stance would commit, and those some would), and every
//! pair's A and Dv both ways, in the battle log's shapes. Its forecast is the sampled estimate of
//! phase6_plan 15.2, which comes with step 7 (`forecast: null` until then: the screens say
//! "estimate coming"); the What-if plans (`alternatives: true`) likewise.

use ffh_core::engagement::report::*;
use ffh_core::engagement::{EngageResult, Fate, FighterIdx, HitCtx, Phase, Side, WarTest};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;

use crate::command::{plot_json, EngageBy};
use crate::json::Json;
use crate::names;
use crate::query::{invalid, QueryErr};
use crate::view::{city_name, Viewer};

/// The log schema's version (DESIGN 10.2 `v`).
pub const LOG_VERSION: i32 = 1;

// ---------------------------------------------------------------------------------------------
// Names of the enums

fn side_ix(s: Side) -> usize {
    s.index()
}

pub fn result_name(r: EngageResult) -> &'static str {
    match r {
        EngageResult::AttackerWon => "AttackerWon",
        EngageResult::DefenderHeld => "DefenderHeld",
        EngageResult::Draw => "Draw",
        EngageResult::Mutual => "Mutual",
    }
}

fn block_name(b: Block) -> &'static str {
    match b {
        Block::Empty => "Empty",
        Block::Shielded => "Shielded",
        Block::VolleyImmune => "VolleyImmune",
        Block::AtLimit => "AtLimit",
    }
}

fn rule_name(r: TargetRule) -> &'static str {
    match r {
        TargetRule::FrontFirst => "FrontFirst",
        TargetRule::FrontRankDown => "FrontRankDown",
        TargetRule::FlankerScreensFirst => "FlankerScreensFirst",
        TargetRule::FlankerAround => "FlankerAround",
        TargetRule::FlankerFrontLast => "FlankerFrontLast",
        TargetRule::AssassinSupportFirst => "AssassinSupportFirst",
        TargetRule::AssassinBack => "AssassinBack",
        TargetRule::AssassinFrontLast => "AssassinFrontLast",
        TargetRule::BodyguardFirst => "BodyguardFirst",
    }
}

fn class_name(c: ffh_data::TargetClass) -> &'static str {
    match c {
        ffh_data::TargetClass::Front => "FRONT",
        ffh_data::TargetClass::Back => "BACK",
        ffh_data::TargetClass::Screen => "SCREEN",
        ffh_data::TargetClass::HeroOrSupport => "HERO_SUPPORT",
    }
}

fn phase_json(p: Phase) -> Json {
    match p {
        Phase::Opening => crate::jobj! { "phase" => "Opening" },
        Phase::Volley(k) => crate::jobj! { "phase" => "Volley", "n" => k },
        Phase::Round(n) => crate::jobj! { "phase" => "Round", "n" => n },
    }
}

fn fate_json(f: &Fate) -> Json {
    match f {
        Fate::Survived => crate::jobj! { "kind" => "Survived" },
        Fate::Killed { by, phase } => crate::jobj! { "kind" => "Killed", "by" => *by, "in" => phase_json(*phase) },
        Fate::Withdrew { phase, capped } => crate::jobj! { "kind" => "Withdrew", "in" => phase_json(*phase), "capped" => *capped },
        Fate::SatOut => crate::jobj! { "kind" => "SatOut" },
    }
}

fn plot_idx_json(g: &Game, p: PlotIdx) -> Json {
    let (x, y) = g.map.xy(p);
    plot_json(x, y)
}

// ---------------------------------------------------------------------------------------------
// Parts: terms, stats, slots

/// A term of A or Dv: `{src, pct}` and the source's payload (`promotion`, `terrain`, ...).
pub fn term_json(t: &Term) -> Json {
    let j = |src: &str| crate::jobj! { "src" => src, "pct" => t.pct };
    match &t.src {
        TermSource::Own => j("Own"),
        TermSource::Promotion(p) => j("Promotion").with("promotion", Json::idx(*p)),
        TermSource::Spell(s) => j("Spell").with("spell", Json::idx(*s)),
        TermSource::BattleEffect(s) => j("BattleEffect").with("spell", Json::idx(*s)),
        TermSource::Hills => j("Hills"),
        TermSource::Terrain(x) => j("Terrain").with("terrain", Json::idx(*x)),
        TermSource::Feature(x) => j("Feature").with("feature", Json::idx(*x)),
        TermSource::Fortify => j("Fortify"),
        TermSource::CityDefense => j("CityDefense"),
        TermSource::UnitCityDefense => j("UnitCityDefense"),
        TermSource::UnitHillsDefense => j("UnitHillsDefense"),
        TermSource::UnitTerrainDefense(x) => j("UnitTerrainDefense").with("terrain", Json::idx(*x)),
        TermSource::UnitFeatureDefense(x) => j("UnitFeatureDefense").with("feature", Json::idx(*x)),
        TermSource::CityAttack => j("CityAttack"),
        TermSource::UnitTerrainAttack(x) => j("UnitTerrainAttack").with("terrain", Json::idx(*x)),
        TermSource::UnitFeatureAttack(x) => j("UnitFeatureAttack").with("feature", Json::idx(*x)),
        TermSource::Amphibious => j("Amphibious"),
        TermSource::VsUnitCombat(x) => j("VsUnitCombat").with("unit_combat", Json::idx(*x)),
        TermSource::VsUnitClass(x) => j("VsUnitClass").with("unit_class", Json::idx(*x)),
        TermSource::VsPromotion(p) => j("VsPromotion").with("promotion", Json::idx(*p)),
        TermSource::VsAnimal => j("VsAnimal"),
        TermSource::VsBarbarian => j("VsBarbarian"),
        TermSource::VsDomain(x) => j("VsDomain").with("domain", Json::idx(*x)),
        TermSource::InBorders => j("InBorders"),
        TermSource::GlobalCounter => j("GlobalCounter"),
        TermSource::Handicap => j("Handicap"),
        TermSource::Aura(u) => j("Aura").with("unit", *u),
        TermSource::Typed(x) => j("Typed").with("damage", Json::idx(*x)),
        TermSource::UnitHillsAttack => j("UnitHillsAttack"),
        TermSource::Improvement(x) => j("Improvement").with("improvement", Json::idx(*x)),
        TermSource::RangeDefense => j("RangeDefense"),
    }
}

fn terms_json(ts: &[Term]) -> Json {
    Json::Arr(ts.iter().map(term_json).collect())
}

/// A fighter's A (`attack`) or Dv as the screens read it (DESIGN `Stat`): `total = base x role x
/// (1 + terms)`, so `base_x100` is the strength before its role's percent. The engine's
/// [`Stat::base_x100`] has the role in it (`engage_base_x100`), so the view takes it out; a unit
/// type with a fixed strength (`iAttack` / `iDefense`) takes no role percent on it, and shows its
/// role's percent as 100.
pub fn stat_json(d: &GameData, unit_type: UnitType, attack: bool, s: &Stat) -> Json {
    let info = &d.units[unit_type];
    let fixed = if attack { info.attack_override } else { info.defense_override } > 0;
    let (base, role) = if fixed || s.role_pct <= 0 { (s.base_x100, if fixed { 100 } else { s.role_pct }) } else { (s.base_x100 * 100 / s.role_pct, s.role_pct) };
    crate::jobj! { "base_x100" => base, "role_pct" => role, "terms" => terms_json(&s.terms), "total_x100" => s.total_x100 }
}

fn slot_change_json(c: &SlotChange) -> Json {
    let j = |src: &str| crate::jobj! { "src" => src, "delta" => c.delta };
    match &c.src {
        SlotSource::Terrain(t) => j("Terrain").with("terrain", Json::idx(*t)),
        SlotSource::Feature(f) => j("Feature").with("feature", Json::idx(*f)),
        SlotSource::Hills => j("Hills"),
        SlotSource::River => j("River"),
        SlotSource::CityDefense { pct, next_at } => j("CityDefense").with("pct", *pct).with("next_at", *next_at),
        SlotSource::Bombarded { pct } => j("Bombarded").with("pct", *pct),
        SlotSource::Minimum => j("Minimum"),
        SlotSource::Summons(n) => j("Summons").with("n", *n),
    }
}

pub fn side_slots_json(s: &SideSlots) -> Json {
    crate::jobj! {
        "base" => s.base,
        "changes" => Json::Arr(s.changes.iter().map(slot_change_json).collect()),
        "total" => s.total,
        "river_only" => s.river_only,
    }
}

/// The defended city as it was (its name resolved: the engine keeps its text key).
fn city_snap_json(d: &GameData, c: &CitySnap) -> Json {
    crate::jobj! { "id" => c.id, "name" => names::resolve_keys(d, &c.name).into_owned(), "owner" => c.owner, "size" => c.size, "defense_pct" => c.defense_pct }
}

fn plot_snap_json(d: &GameData, p: &PlotSnap) -> Json {
    crate::jobj! {
        "x" => p.x,
        "y" => p.y,
        "terrain" => Json::opt_idx(p.terrain),
        "feature" => Json::opt_idx(p.feature),
        "hills" => p.hills,
        "river_crossing" => p.river_crossing,
        "city" => p.city.as_ref().map(|c| city_snap_json(d, c)),
    }
}

/// A plot as it is now, in the log's `PlotSnap` shape (the attacking plot; the plan's plots).
fn plot_now(v: &Viewer, p: PlotIdx, river_crossing: bool) -> Json {
    let g = v.g;
    let plot = &g.map.plots[p];
    let (x, y) = g.map.xy(p);
    let city = plot.plot_city.filter(|&c| g.cities.contains(c) && v.city_revealed(c)).map(|c| {
        let city = &g.cities[c];
        crate::jobj! { "id" => c, "name" => city_name(g, c), "owner" => city.owner, "size" => city.population, "defense_pct" => g.city_defense_modifier(c, false) }
    });
    crate::jobj! {
        "x" => x,
        "y" => y,
        "terrain" => Json::opt_idx(plot.terrain_type),
        "feature" => Json::opt_idx(v.plot_revealed(p).then_some(plot.feature_type).flatten()),
        "hills" => g.plot_is_hills(p),
        "river_crossing" => river_crossing,
        "city" => city,
    }
}

// ---------------------------------------------------------------------------------------------
// Fighters and pairs

/// A unit as a fighter of an attack from `ctx.from` on `ctx.to` (the plan's): the battle log's
/// fighter as the engagement writes it (`engagement::sink`), at the unit's HP now.
fn plan_fighter(g: &Game, u: UnitId, side: Side, slot: Option<i32>, in_city: bool, viewer_team: TeamId, ctx: &HitCtx) -> LogFighter {
    let d = g.d;
    let unit = &g.units[u];
    let role = g.unit_combat_role(u);
    let info = role.map(|r| &d.combat_roles[r]);
    let max_hp = g.unit_engage_max_hp(u);
    let withdraw_pct = match side {
        Side::Attacker => g.unit_withdrawal_probability(u),
        Side::Defender if in_city => 0,
        Side::Defender => g.unit_get_withdrawl_prob_defensive(u),
    };
    let immune = Immunities::default()
        .with(Immunities::FIRST_STRIKES, g.unit_immune_to_first_strikes(u))
        .with(Immunities::DEF_STRIKE, g.unit_is_immune_to_defensive_strike(u))
        .with(Immunities::FEAR, g.unit_is_immune_to_fear(u))
        .with(Immunities::COLLATERAL, unit.collateral_damage_protection >= 100);
    LogFighter {
        unit: u,
        unit_type: unit.unit_type,
        name: unit.name.clone(),
        owner: unit.owner,
        shown_owner: g.unit_get_visual_owner(u, Some(viewer_team)),
        side,
        slot,
        level: unit.level,
        xp: unit.experience,
        hero: g.unit_is_hero(u),
        role: role.unwrap_or(CombatRoleType::from_usize(0)),
        pos: if info.is_some_and(|r| r.back) { Position::Back } else { Position::Front },
        screen: info.is_some_and(|r| r.screen),
        ignores_shield: info.is_some_and(|r| r.ignores_shield),
        assassin_profile: info.is_some_and(|r| r.ignores_shield && !r.targets.contains(&ffh_data::TargetClass::Screen)),
        guardsman: false,
        max_hp,
        hp_start: ffh_core::engagement::hp_from_damage(unit.damage, max_hp),
        volleys: g.unit_first_strikes(u),
        volley_chance: g.unit_chance_first_strikes(u),
        withdraw_pct,
        def_strike: (unit.defensive_strike_chance > 0).then_some((unit.defensive_strike_chance, unit.defensive_strike_damage)),
        collateral: (g.unit_collateral_damage(u) > 0).then(|| (g.unit_collateral_damage(u), g.unit_collateral_damage_limit(u), g.unit_collateral_damage_max_units(u))),
        combat_limit: if side == Side::Attacker { unit.combat_limit } else { 100 },
        immune,
        fear: g.unit_is_fear(u),
        a: g.engage_stat(u, None, side, true, ctx),
        dv: g.engage_stat(u, None, side, false, ctx),
        fate: None,
    }
}

/// A fighter's promotions: the unit's while it lives (with the type it fought as), else those
/// among its A and Dv parts (a fallen unit's are not kept).
fn fighter_promotions(g: &Game, f: &LogFighter) -> Vec<PromotionType> {
    match g.units.get(f.unit).filter(|u| u.unit_type == f.unit_type && u.owner == f.owner) {
        Some(u) => g.d.promotions.keys().filter(|&p| u.has_promotion[p]).collect(),
        None => {
            let mut out: Vec<PromotionType> = Vec::new();
            for t in f.a.terms.iter().chain(&f.dv.terms) {
                if let TermSource::Promotion(p) = t.src {
                    if !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
            out
        }
    }
}

/// A fighter as the screens read it (`b` its index); `owner` the owner to show the viewer.
fn fighter_json(g: &Game, b: usize, f: &LogFighter, owner: PlayerId) -> Json {
    let d = g.d;
    let name = if f.name.is_empty() { names::display(d, f.unit_type).into_owned() } else { names::text(d, &f.name).to_string() };
    let targets: Vec<Json> = d.combat_roles.get(f.role).map_or(Vec::new(), |r| r.targets.iter().map(|&c| Json::from(class_name(c))).collect());
    let immune: Vec<Json> = [(Immunities::FIRST_STRIKES, "FirstStrikes"), (Immunities::DEF_STRIKE, "DefStrike"), (Immunities::FEAR, "Fear"), (Immunities::COLLATERAL, "Collateral")]
        .into_iter()
        .filter(|&(bit, _)| f.immune.has(bit))
        .map(|(_, n)| Json::from(n))
        .collect();
    crate::jobj! {
        "b" => b,
        "unit" => f.unit,
        "unit_type" => Json::idx(f.unit_type),
        "name" => name,
        "owner" => owner,
        "side" => side_ix(f.side),
        "slot" => f.slot,
        "level" => f.level,
        "xp" => f.xp,
        "hero" => f.hero,
        "role" => Json::idx(f.role),
        "pos" => match f.pos { Position::Front => "Front", Position::Back => "Back" },
        "screen" => f.screen,
        "ignores_shield" => f.ignores_shield,
        "assassin_profile" => f.assassin_profile,
        "guardsman" => f.guardsman,
        "targets" => Json::Arr(targets),
        "promotions" => Json::Arr(fighter_promotions(g, f).into_iter().map(Json::idx).collect()),
        "max_hp" => f.max_hp,
        "hp_start" => f.hp_start,
        "volleys" => f.volleys,
        "volley_chance" => f.volley_chance,
        "withdraw_pct" => f.withdraw_pct,
        "def_strike" => f.def_strike.map(|(c, dmg)| crate::jarr![c, dmg]),
        "collateral" => f.collateral.map(|(p, l, n)| crate::jarr![p, l, n]),
        "combat_limit" => f.combat_limit,
        "immune" => Json::Arr(immune),
        "fear" => f.fear,
        "a" => stat_json(d, f.unit_type, true, &f.a),
        "dv" => stat_json(d, f.unit_type, false, &f.dv),
        "fate" => f.fate.as_ref().map(fate_json),
    }
}

fn pair_json(p: &LogPair) -> Json {
    crate::jobj! {
        "h" => p.h,
        "t" => p.t,
        "a_x100" => p.a_x100,
        "dv_x100" => p.dv_x100,
        "mean" => p.mean,
        "a_extra" => terms_json(&p.a_extra),
        "dv_extra" => terms_json(&p.dv_extra),
    }
}

/// `terms` less the terms of `headline` (a multiset difference): the pair-only terms (as the
/// engine's log has them).
fn extra_terms(terms: Vec<Term>, headline: &[Term]) -> Vec<Term> {
    let mut left: Vec<&Term> = headline.iter().collect();
    terms
        .into_iter()
        .filter(|t| match left.iter().position(|h| *h == t) {
            Some(i) => {
                left.swap_remove(i);
                false
            }
            None => t.pct != 0,
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Events

fn heal_source_json(s: &HealSource) -> Json {
    match s {
        HealSource::Medic { b, unit_type, adjacent } => crate::jobj! { "kind" => "Medic", "b" => *b, "unit_type" => Json::idx(*unit_type), "adjacent" => *adjacent },
        HealSource::OnKill => crate::jobj! { "kind" => "OnKill" },
        HealSource::Spell(s) => crate::jobj! { "kind" => "Spell", "spell" => Json::idx(*s) },
    }
}

fn xp_source_json(s: &XpSource) -> Json {
    match s {
        XpSource::Kill { victim, share_permille } => crate::jobj! { "kind" => "Kill", "victim" => *victim, "share_permille" => *share_permille },
        XpSource::Withdrew => crate::jobj! { "kind" => "Withdrew" },
        XpSource::DefStrike => crate::jobj! { "kind" => "DefStrike" },
    }
}

fn kill_kind_json(k: &KillKind) -> Json {
    match k {
        KillKind::Capture(t) => crate::jobj! { "kind" => "Capture", "unit_type" => Json::idx(*t) },
        KillKind::Enslave(t) => crate::jobj! { "kind" => "Enslave", "unit_type" => Json::idx(*t) },
        KillKind::Convert(t) => crate::jobj! { "kind" => "Convert", "unit_type" => Json::idx(*t) },
        KillKind::Create(t) => crate::jobj! { "kind" => "Create", "unit_type" => Json::idx(*t) },
        KillKind::Gold(n) => crate::jobj! { "kind" => "Gold", "amount" => *n },
        KillKind::Afflict(p) => crate::jobj! { "kind" => "Afflict", "promotion" => Json::idx(*p) },
        KillKind::Promotion(p) => crate::jobj! { "kind" => "Promotion", "promotion" => Json::idx(*p) },
    }
}

fn cast_effect_json(e: &CastEffect) -> Json {
    match e {
        CastEffect::Mod { a_pct, dv_pct } => crate::jobj! { "kind" => "Mod", "a_pct" => *a_pct, "dv_pct" => *dv_pct },
        CastEffect::Damage(hits) => crate::jobj! { "kind" => "Damage", "hits" => Json::Arr(hits.iter().map(|&(b, dmg)| crate::jarr![b, dmg]).collect()) },
        CastEffect::Summon(b) => crate::jobj! { "kind" => "Summon", "b" => *b },
    }
}

fn idxs(v: &[FighterIdx]) -> Json {
    Json::Arr(v.iter().map(|&b| Json::from(b)).collect())
}

/// An event of the log, `{kind, ...}` (battle_log.js's header).
fn event_json(g: &Game, e: &LogEvent) -> Json {
    let k = |kind: &str| crate::jobj! { "kind" => kind };
    match e {
        LogEvent::Stage { stage } => match stage {
            Stage::PreBattle => k("Phase").with("phase", "PreBattle"),
            Stage::Morale => k("Phase").with("phase", "Morale"),
            Stage::Opening => k("Phase").with("phase", "Opening"),
            Stage::Volley(n) => k("Phase").with("phase", "Volley").with("n", *n),
            Stage::Round(n) => k("Phase").with("phase", "Round").with("n", *n),
            Stage::Post => k("Phase").with("phase", "Post"),
        },
        LogEvent::Cast { caster, spell, targets, effect } => k("Cast").with("caster", *caster).with("spell", Json::idx(*spell)).with("targets", idxs(targets)).with("effect", cast_effect_json(effect)),
        LogEvent::Morale { unit, source, chance, roll, passed } => k("Morale").with("unit", *unit).with("source", *source).with("chance", *chance).with("roll", *roll).with("passed", *passed),
        LogEvent::SatOut { unit, why } => k("SatOut").with("unit", *unit).with(
            "why",
            match why {
                SatOutWhy::Afraid => "Afraid",
                SatOutWhy::EveryTargetCapped => "EveryTargetCapped",
            },
        ),
        LogEvent::VolleyRoll { unit, chance, roll, total } => k("VolleyRoll").with("unit", *unit).with("chance", *chance).with("roll", *roll).with("total", *total),
        LogEvent::DefStrike { striker, target, rolled, chance, roll, pct, dmg } => {
            k("DefStrike").with("striker", *striker).with("target", *target).with("rolled", *rolled).with("chance", *chance).with("roll", *roll).with("pct", *pct).with("dmg", *dmg)
        }
        LogEvent::Collateral { source, target, pct, limit_pct, protection_pct, dmg } => k("Collateral")
            .with("source", *source)
            .with("target", *target)
            .with("pct", *pct)
            .with("limit_pct", *limit_pct)
            .with("protection_pct", *protection_pct)
            .with("dmg", *dmg),
        LogEvent::Hit { h, t, rule, step, blocked, eligible, r, dmg, capped } => k("Hit")
            .with("h", *h)
            .with("t", *t)
            .with("rule", rule_name(*rule))
            .with("step", *step)
            .with("blocked", Json::Arr(blocked.iter().map(|&(s, b)| crate::jarr![s, block_name(b)]).collect()))
            .with("eligible", idxs(eligible))
            .with("r", *r)
            .with("dmg", *dmg)
            .with("capped", *capped),
        LogEvent::RoundEnd { deaths } => k("RoundEnd").with(
            "deaths",
            Json::Arr(deaths.iter().map(|d| crate::jobj! { "b" => d.b, "killer" => d.killer, "assists" => idxs(&d.assists), "overkill" => d.overkill }).collect()),
        ),
        LogEvent::Withdraw { unit, hp_pct, chance, roll, ok, fatal, hp_kept } => {
            k("Withdraw").with("unit", *unit).with("hp_pct", *hp_pct).with("chance", *chance).with("roll", *roll).with("ok", *ok).with("fatal", *fatal).with("hp_kept", *hp_kept)
        }
        LogEvent::ShieldDown { side } => k("ShieldDown").with("side", side_ix(*side)),
        LogEvent::End { result, volley_rounds, rounds } => k("End").with("result", result_name(*result)).with("volley_rounds", *volley_rounds).with("rounds", *rounds),
        LogEvent::Heal { unit, amount, source } => k("Heal").with("unit", *unit).with("amount", *amount).with("source", heal_source_json(source)),
        LogEvent::Xp { unit, amount, from } => k("Xp").with("unit", *unit).with("amount", *amount).with("from", Json::Arr(from.iter().map(xp_source_json).collect())),
        LogEvent::KillEffect { killer, victim, kind } => k("KillEffect").with("killer", *killer).with("victim", *victim).with("effect", kill_kind_json(kind)),
        LogEvent::Flee { unit, to } => k("Flee").with("unit", *unit).with("to", plot_idx_json(g, *to)),
        LogEvent::Rout { units, to } => k("Rout").with("units", idxs(units)).with("to", Json::Arr(to.iter().map(|&p| plot_idx_json(g, p)).collect())),
        LogEvent::MoveIn { units, city } => k("MoveIn").with("units", idxs(units)).with("city", *city),
    }
}

fn forecast_json(f: &FrozenForecast) -> Json {
    let (source, runs) = match f.source {
        ForecastSource::Sampled { runs } => ("Sampled", Some(runs)),
        ForecastSource::Expected => ("Expected", None),
    };
    let alts = f.alternatives.iter().map(|(k, win)| {
        let j = match k {
            AltKind::ThisPlan => crate::jobj! { "kind" => "ThisPlan" },
            AltKind::StancePick => crate::jobj! { "kind" => "StancePick" },
            AltKind::Swap { out, add } => crate::jobj! { "kind" => "Swap", "out" => *out, "add" => *add },
            AltKind::Stance(s) => crate::jobj! { "kind" => "OtherStance", "stance" => Json::idx(*s) },
        };
        j.with("win_permille", *win)
    });
    crate::jobj! {
        "source" => source,
        "runs" => runs,
        "win_permille" => f.win_permille,
        "draw_permille" => f.draw_permille,
        "rounds" => crate::jobj! { "p10" => f.rounds.0, "p50" => f.rounds.1, "p90" => f.rounds.2 },
        "losses_if_win_x10" => (f.losses_if_win_x10 >= 0).then_some(f.losses_if_win_x10),
        "per" => Json::Arr(f.per.iter().map(|p| crate::jobj! { "b" => p.b, "fall_pct" => p.fall_pct, "withdraw_pct" => p.withdraw_pct, "death_round" => p.death_round }).collect()),
        "curve" => Json::Arr(f.curve.iter().map(|c| Json::from(c.clone())).collect()),
        "alternatives" => Json::Arr(alts.collect()),
    }
}

// ---------------------------------------------------------------------------------------------
// Sides and summaries

/// Whether `v` may see report `r`: it fought on a side (or debug reveal).
fn visible_to(v: &Viewer, r: &BattleReport) -> bool {
    v.reveal || r.involves(v.h)
}

/// The side the viewer fought on (`None`: neither, a reveal view).
fn viewer_side(v: &Viewer, r: &BattleReport) -> Option<Side> {
    r.side_of(v.h)
}

/// Side `s`'s player as the viewer may see it: the real one on its own side (or under reveal),
/// the one it saw on the other.
fn shown_player(v: &Viewer, r: &BattleReport, s: Side) -> PlayerId {
    let side = &r.sides[side_ix(s)];
    if v.reveal || viewer_side(v, r) == Some(s) {
        side.player
    } else {
        side.shown_player
    }
}

/// A side's name for the viewer: the garrison's city, its own army's name, else "Calabim force".
fn side_label(v: &Viewer, r: &BattleReport, s: Side, player: PlayerId) -> String {
    let g = v.g;
    let side = &r.sides[side_ix(s)];
    let own = v.reveal || viewer_side(v, r) == Some(s);
    if let Some(c) = side.garrison_of {
        if let Some(name) = r.place.city.as_ref().filter(|pc| pc.id == c).map(|pc| pc.name.clone()) {
            return names::resolve_keys(g.d, &name).into_owned();
        }
        if g.cities.contains(c) {
            return city_name(g, c);
        }
    }
    if let (true, Some(a)) = (own, side.army) {
        if let Some(army) = g.armies.get(a) {
            return army.name.clone();
        }
    }
    if own && player == v.h {
        "Your force".into()
    } else {
        format!("{} force", g.player_get_civilization_adjective(player))
    }
}

fn lost_count(s: &ReportSide) -> i32 {
    s.lost.iter().map(|(_, n)| n).sum()
}

/// Report `r`'s summary as the viewer may see it (the battle lists, the toasts, the command's
/// reply): where, when, the sides (player, civilization, label, stance when seen, committed,
/// lost by type, withdrawn, sat out, fled), the outcome, and whether its log is kept.
pub fn summary_json(v: &Viewer, r: &BattleReport) -> Json {
    let g = v.g;
    let me = viewer_side(v, r);
    let sides = [Side::Attacker, Side::Defender].map(|s| {
        let side = &r.sides[side_ix(s)];
        let player = shown_player(v, r, s);
        let own = v.reveal || me == Some(s);
        crate::jobj! {
            "player" => player,
            "civ" => Json::opt_idx(g.player_get_civilization_type(player)),
            "label" => side_label(v, r, s, player),
            "stance" => own.then(|| Json::idx(side.stance)),
            "slots" => side.slots,
            "committed" => side.committed,
            "lost" => lost_count(side),
            "lost_types" => Json::Arr(side.lost.iter().map(|&(t, n)| crate::jarr![Json::idx(t), n]).collect()),
            "withdrew" => side.withdrew,
            "sat_out" => side.sat_out,
            "fled" => side.fled,
        }
    });
    let (fx, fy) = g.map.xy(r.from);
    crate::jobj! {
        "id" => r.id,
        "turn" => r.turn,
        "x" => r.place.x,
        "y" => r.place.y,
        "from" => plot_json(fx, fy),
        "place" => r.place.city.as_ref().map(|c| names::resolve_keys(g.d, &c.name).into_owned()),
        "city" => r.place.city.as_ref().map(|c| c.id),
        "viewer_side" => me.map(side_ix),
        "result" => result_name(r.outcome.result),
        "volley_rounds" => r.outcome.volley_rounds,
        "rounds" => r.outcome.rounds,
        "plot_taken" => r.outcome.plot_taken,
        "city_captured" => r.outcome.city_captured,
        "defenders_left" => r.outcome.defenders_left,
        "has_log" => r.log.is_some(),
        "sides" => Json::Arr(sides.into_iter().collect()),
    }
}

/// [`summary_json`] of report `id`, or null when it is not kept or not the viewer's.
pub fn summary_by_id(v: &Viewer, id: u32) -> Json {
    v.g.battle_report(id).filter(|r| visible_to(v, r)).map_or(Json::Null, |r| summary_json(v, r))
}

/// The viewer's kept reports from game turn `from` on, newest first.
pub fn reports_since<'g>(v: &Viewer<'g>, from: i32) -> Vec<&'g BattleReport> {
    let g: &'g Game = v.g;
    g.battle_reports.iter().rev().filter(|r| r.turn >= from && visible_to(v, r)).collect()
}

/// The viewer's kept reports from game turn `from` on, newest first (summaries).
pub fn summaries_since(v: &Viewer, from: i32) -> Vec<Json> {
    reports_since(v, from).into_iter().map(|r| summary_json(v, r)).collect()
}

/// One line for people (the turn report's, `text`): "battle #12 at Strages: held; 2 lost, 5 of
/// theirs fell".
pub fn report_line(v: &Viewer, r: &BattleReport) -> String {
    let g = v.g;
    let place = r.place.city.as_ref().map_or_else(|| format!("({}, {})", r.place.x, r.place.y), |c| names::resolve_keys(g.d, &c.name).into_owned());
    let me = viewer_side(v, r).unwrap_or(Side::Attacker);
    let outcome = match (me, r.outcome.result) {
        (Side::Attacker, EngageResult::AttackerWon) if r.outcome.city_captured.is_some() => "won, the city taken",
        (Side::Attacker, EngageResult::AttackerWon) => "won",
        (Side::Attacker, EngageResult::DefenderHeld) => "repelled",
        (Side::Defender, EngageResult::AttackerWon) => "lost",
        (Side::Defender, EngageResult::DefenderHeld) => "held",
        (_, EngageResult::Draw) => "a draw",
        (_, EngageResult::Mutual) => "both sides spent",
    };
    let (mine, theirs) = (&r.sides[side_ix(me)], &r.sides[side_ix(me.other())]);
    format!("battle #{} at {place}: {outcome}; {} lost, {} of theirs fell", r.id, lost_count(mine), lost_count(theirs))
}

/// `battle_reports`: the viewer's kept reports, newest first (`[]` with engagements off).
pub fn battle_reports(v: &Viewer) -> Json {
    crate::jobj! { "on" => v.g.engagements_on(), "reports" => Json::Arr(summaries_since(v, i32::MIN)) }
}

/// The view's `battles`: the viewer's reports of this game turn and the previous one (null with
/// engagements off): what the map's markers, the toasts and the combat flash read.
pub fn recent(v: &Viewer) -> Json {
    if !v.g.engagements_on() {
        return Json::Null;
    }
    Json::Arr(summaries_since(v, v.g.game_turn() - 1))
}

// ---------------------------------------------------------------------------------------------
// The battle log

/// `battle_log {id}`: report `id` with its log as the battle screens read it (module comment).
/// `gone` when the report is not kept, is not the viewer's, or has lost its log (older reports
/// keep their summary only: `battle_reports` says `has_log`).
pub fn battle_log(v: &Viewer, id: u32) -> Result<Json, QueryErr> {
    let g = v.g;
    let r = g.battle_report(id).filter(|r| visible_to(v, r)).ok_or(QueryErr::Gone)?;
    let log = r.log.as_deref().ok_or(QueryErr::Gone)?;
    let me = viewer_side(v, r);
    let sides = [Side::Attacker, Side::Defender].map(|s| {
        let side = &r.sides[side_ix(s)];
        let player = shown_player(v, r, s);
        let known = v.reveal || me == Some(s);
        let pick = &log.picks[side_ix(s)];
        crate::jobj! {
            "player" => player,
            "shown_player" => side.shown_player,
            "civ" => Json::opt_idx(g.player_get_civilization_type(player)),
            "label" => side_label(v, r, s, player),
            "army" => if known { Json::from(side.army) } else { Json::Null },
            "garrison_of" => side.garrison_of,
            "stance" => known.then(|| Json::idx(side.stance)),
            "stance_known" => known,
            "auto_pick" => idxs(&pick.auto_pick),
            "picked_by" => match &pick.picked_by {
                Pick::Auto => crate::jobj! { "kind" => "Auto" },
                Pick::Player { swaps } => crate::jobj! { "kind" => "Player", "swaps" => Json::Arr(swaps.iter().map(|&(o, i)| crate::jarr![o, i]).collect()) },
            },
        }
    });
    let fighters: Vec<Json> = log.fighters.iter().enumerate().map(|(b, f)| fighter_json(g, b, f, if v.reveal { f.owner } else { r.owner_for(v.h, f.side, f.owner, f.shown_owner) })).collect();
    let moved_in = log.events.iter().find_map(|e| match e {
        LogEvent::MoveIn { units, .. } => Some(idxs(units)),
        _ => None,
    });
    let o = &r.outcome;
    Ok(crate::jobj! {
        "v" => LOG_VERSION,
        "id" => r.id,
        "turn" => r.turn,
        "viewer_side" => me.map(side_ix),
        "plot" => plot_snap_json(g.d, &r.place),
        "from" => plot_now(v, r.from, false),
        "slots" => Json::Arr(log.slots.iter().map(side_slots_json).collect()),
        "sides" => Json::Arr(sides.into_iter().collect()),
        "fighters" => Json::Arr(fighters),
        "pairs" => Json::Arr(log.pairs.iter().map(pair_json).collect()),
        "events" => Json::Arr(log.events.iter().map(|e| event_json(g, e)).collect()),
        "forecast" => log.forecast.as_ref().map(forecast_json),
        "outcome" => crate::jobj! {
            "result" => result_name(o.result),
            "volley_rounds" => o.volley_rounds,
            "rounds" => o.rounds,
            "plot_taken" => o.plot_taken,
            "city_captured" => o.city_captured,
            "moved_in" => moved_in.unwrap_or_else(Json::arr),
            "defenders_left" => o.defenders_left,
        },
        "summary" => summary_json(v, r),
    })
}

// ---------------------------------------------------------------------------------------------
// The plan of an attack

/// The attacking plot and the launcher of `by` (the group whose army's stance the attack takes).
fn attack_origin(v: &Viewer, by: &EngageBy) -> Result<(PlotIdx, Option<GroupId>), QueryErr> {
    let g = v.g;
    match by {
        EngageBy::Army(a) => match g.armies.get(*a) {
            None => Err(QueryErr::Gone),
            Some(army) if army.owner != v.h => Err(QueryErr::NotOwner),
            Some(army) => Ok((g.army_plot(*a).ok_or_else(|| invalid("the army is not on the map"))?, army.group)),
        },
        EngageBy::Units(units) => {
            crate::view::units::check_selection(v, units)?;
            let grp = g.units[units[0]].group.filter(|&gr| g.groups.contains(gr) && units.iter().all(|&u| g.units[u].group == Some(gr)));
            Ok((g.unit_plot(units[0]), grp))
        }
        EngageBy::From(x, y) => Ok((crate::view::map::plot_at(v, *x, *y)?, None)),
    }
}

/// The defenders a viewer expects to fight among `candidates` (their stance unseen, unless the
/// viewer is on their team or reveal is on): all of them when the slots hold them; else those
/// every stance would commit (`sure`) and those only some would (`maybe`), each in the default
/// stance's order.
fn defender_pick(g: &Game, candidates: &[UnitId], known: Option<StanceType>, slots: &ffh_core::engagement::Slots, ctx: &HitCtx) -> (Vec<UnitId>, Vec<UnitId>, bool) {
    if let Some(st) = known {
        return (g.engage_auto_pick(candidates, st, slots, Side::Defender, ctx), Vec::new(), true);
    }
    let default = g.engage_auto_pick(candidates, g.d.default_stance, slots, Side::Defender, ctx);
    if candidates.len() as i32 <= slots.max(Side::Defender) {
        return (default, Vec::new(), true);
    }
    let picks: Vec<Vec<UnitId>> = g.d.stances.keys().map(|st| g.engage_auto_pick(candidates, st, slots, Side::Defender, ctx)).collect();
    let mut order: Vec<UnitId> = default.clone();
    for p in &picks {
        for &u in p {
            if !order.contains(&u) {
                order.push(u);
            }
        }
    }
    let (sure, maybe): (Vec<UnitId>, Vec<UnitId>) = order.into_iter().partition(|u| picks.iter().all(|p| p.contains(u)));
    (sure, maybe, false)
}

/// `engage_preview`: the plan of an attack on `to` by `by` (module comment). `force`: the units
/// to commit (checked: `errors` says why not); else `stance`'s pick; else the proposal of the
/// attack's own stance (the launcher's army's, ...). Pure.
pub fn engage_preview(v: &Viewer, by: &EngageBy, to: (i32, i32), force: Option<&[UnitId]>, stance: Option<StanceType>, alternatives: bool) -> Result<Json, QueryErr> {
    let g = v.g;
    let d = g.d;
    let h = v.h;
    if !g.engagements_on() {
        return Err(invalid("engagements are off in this game"));
    }
    let (from, launcher) = attack_origin(v, by)?;
    let dest = crate::view::map::plot_at(v, to.0, to.1)?;
    if alternatives {
        // The What-if plans are the sampled forecast's (step 7).
        return Ok(crate::jobj! { "alternatives" => Json::arr(), "pending" => true });
    }
    let mut errors: Vec<String> = Vec::new();
    let ((fx, fy), (tx, ty)) = (g.map.xy(from), g.map.xy(dest));
    if g.step_distance(fx, fy, tx, ty) != 1 {
        errors.push("an attack is on a plot next to the attackers".into());
    }
    if !v.plot_visible(dest) {
        errors.push("you cannot see that plot".into());
    }
    let rival = g.map.plots[from].units.iter().filter(|&&u| g.units[u].owner == h).find_map(|&u| g.unit_get_declare_war_move(u, dest));
    let war = rival.map_or(WarTest::AtWar, WarTest::DeclareWar);
    let ctx = HitCtx::new(from, dest);
    let slots = g.engage_slots(&ctx);
    let pool = g.engage_attackers(h, from, dest, war);
    let choice = g.engage_attack_stance(h, from, launcher);
    let stance = stance.filter(|s| s.idx() < d.stances.len()).unwrap_or(choice.stance);
    let auto = g.engage_auto_pick(&pool, stance, &slots, Side::Attacker, &ctx);
    let force: Vec<UnitId> = match force {
        Some(f) => {
            if let Err(e) = g.engage_validate_force(&pool, f, &slots) {
                errors.push(e.into());
            }
            let mut out: Vec<UnitId> = Vec::new();
            for &u in f {
                if pool.contains(&u) && !out.contains(&u) {
                    out.push(u);
                }
            }
            out
        }
        None => g.engage_propose_force(&pool, stance, &slots, &ctx),
    };
    if pool.is_empty() {
        errors.push("no unit here can attack that plot now".into());
    }
    let candidates = if v.plot_visible(dest) { g.engage_defenders(dest, h, if force.is_empty() { &pool } else { &force }, war) } else { Vec::new() };
    if candidates.is_empty() && v.plot_visible(dest) {
        errors.push("there is nobody to fight there".into());
    }
    let in_city = g.plot_is_city(dest, false, None);
    let def_choice = g.engage_defense_stance(dest, &candidates);
    let def_known = (v.reveal || candidates.first().is_some_and(|&u| g.unit_get_team(u) == v.team)).then_some(def_choice.stance);
    let (sure, maybe, exact) = defender_pick(g, &candidates, def_known, &slots, &ctx);

    // The fighters: the pool (side 0), then the defenders seen (side 1).
    let mut units: Vec<(UnitId, Side)> = pool.iter().map(|&u| (u, Side::Attacker)).collect();
    units.extend(candidates.iter().map(|&u| (u, Side::Defender)));
    let fighters: Vec<LogFighter> = units
        .iter()
        .map(|&(u, side)| {
            let slot = if side == Side::Attacker { force.iter().position(|&x| x == u).map(|i| i as i32) } else { None };
            plan_fighter(g, u, side, slot, in_city, v.team, &ctx)
        })
        .collect();
    let b_of = |u: UnitId, side: Side| units.iter().position(|&x| x == (u, side));
    let bs = |list: &[UnitId], side: Side| Json::Arr(list.iter().filter_map(|&u| b_of(u, side)).map(Json::from).collect());
    // Every pair both ways (the cards' damage per hit).
    let params = g.engage_params();
    let mut pairs = Vec::new();
    for (ai, &(a, _)) in units.iter().enumerate().filter(|(_, x)| x.1 == Side::Attacker) {
        for (di, &(dfd, _)) in units.iter().enumerate().filter(|(_, x)| x.1 == Side::Defender) {
            for (h_i, hu, t_i, tu, side) in [(ai, a, di, dfd, Side::Attacker), (di, dfd, ai, a, Side::Defender)] {
                let pr = g.engage_hit(hu, tu, side, &ctx);
                let sa = g.engage_stat(hu, Some(tu), side, true, &ctx);
                let sd = g.engage_stat(tu, Some(hu), side.other(), false, &ctx);
                let mean = (params.base_damage.max(0) as i64 * 100 * pr.a as i64 / (pr.a as i64 + pr.dv as i64).max(1)) as i32;
                pairs.push(pair_json(&LogPair {
                    h: h_i as FighterIdx,
                    t: t_i as FighterIdx,
                    a_x100: pr.a,
                    dv_x100: pr.dv,
                    mean,
                    a_extra: extra_terms(sa.terms, &fighters[h_i].a.terms),
                    dv_extra: extra_terms(sd.terms, &fighters[t_i].dv.terms),
                }));
            }
        }
    }
    let fighters_json: Vec<Json> = fighters
        .iter()
        .enumerate()
        .map(|(b, f)| fighter_json(g, b, f, if f.side == Side::Attacker || v.reveal { f.owner } else { v.unit_owner(f.unit) }))
        .collect();
    // The sides: the attacker's (the viewer's) and the defender's as the viewer sees it.
    let army_name = choice.army.filter(|&a| g.armies.contains(a)).map(|a| g.armies[a].name.clone());
    let def_player = match g.map.plots[dest].plot_city.filter(|&c| g.cities.contains(c) && v.city_revealed(c)) {
        Some(c) => g.cities[c].owner,
        None => candidates.first().map_or(h, |&u| v.unit_owner(u)),
    };
    let def_label = match def_choice.garrison_of {
        Some(c) => city_name(g, c),
        None => format!("{} force", g.player_get_civilization_adjective(def_player)),
    };
    let sides = crate::jarr![
        crate::jobj! {
            "player" => h, "shown_player" => h, "civ" => Json::opt_idx(g.player_get_civilization_type(h)),
            "label" => army_name.unwrap_or_else(|| "Your force".into()), "army" => choice.army, "garrison_of" => choice.garrison_of,
            "stance" => Json::idx(stance), "stance_known" => true,
        },
        crate::jobj! {
            "player" => def_player, "shown_player" => def_player, "civ" => Json::opt_idx(g.player_get_civilization_type(def_player)),
            "label" => def_label, "army" => Json::Null, "garrison_of" => def_choice.garrison_of,
            "stance" => def_known.map(Json::idx), "stance_known" => def_known.is_some(),
        }
    ];
    let defender_pick = if exact {
        crate::jobj! { "kind" => "Known", "units" => bs(&sure, Side::Defender) }
    } else {
        crate::jobj! { "kind" => "Likely", "sure" => bs(&sure, Side::Defender), "maybe" => bs(&maybe, Side::Defender) }
    };
    let reserve: Vec<UnitId> = pool.iter().copied().filter(|u| !force.contains(u)).collect();
    let crossing = g.plot_is_river_crossing(from, g.direction_xy_plots(from, dest));
    Ok(crate::jobj! {
        "plot" => plot_now(v, dest, crossing),
        "from" => plot_now(v, from, false),
        "slots" => Json::Arr(slots.sides.iter().map(side_slots_json).collect()),
        "sides" => sides,
        "fighters" => Json::Arr(fighters_json),
        "pairs" => Json::Arr(pairs),
        "force" => bs(&force, Side::Attacker),
        "auto_pick" => bs(&auto, Side::Attacker),
        "reserve" => bs(&reserve, Side::Attacker),
        "stance" => Json::idx(stance),
        "army_stance" => Json::idx(choice.stance),
        "defender_pick" => defender_pick,
        // The sampled forecast comes with step 7 (phase6_plan 15.2): the screens say so.
        "forecast" => Json::Null,
        "estimate_pending" => true,
        "declare_war" => rival,
        "errors" => Json::Arr(errors.into_iter().map(Json::from).collect()),
    })
}

// ---------------------------------------------------------------------------------------------
// The `combat` query and the view's fields under engagements

/// The `combat` query under engagements, for an attack of `units` (land units) on `plot` (land),
/// given the duel's reply `j`: when the plot holds a defender the attack would meet (a
/// declare-war attack tests the teams it would fight), `engagement: true`, `valid` (the pool on
/// the attacking plot is not empty: some unit there can attack the plot now), `pool` and
/// `defenders` (their sizes) and `declare_war_team`; the duel's odds and strengths stay beside
/// them until the engagement forecast lands (phase 6 step 9 part 2). With no defender there the
/// attack is today's (it captures non-combatants): the duel's reply unchanged.
pub fn combat_fields(v: &Viewer, units: &[UnitId], plot: PlotIdx, mut j: Json) -> Json {
    let g = v.g;
    if !v.plot_visible(plot) {
        return j;
    }
    let from = g.unit_plot(units[0]);
    let rival = units.iter().find_map(|&u| g.unit_get_declare_war_move(u, plot));
    let war = rival.map_or(WarTest::AtWar, WarTest::DeclareWar);
    let pool = g.engage_attackers(v.h, from, plot, war);
    let defenders = g.engage_defenders(plot, v.h, if pool.is_empty() { units } else { &pool }, war);
    if defenders.is_empty() {
        return j;
    }
    j.set("engagement", true);
    j.set("valid", !pool.is_empty());
    j.set("pool", pool.len());
    j.set("defenders", defenders.len());
    j.set("declare_war_team", rival);
    j
}

/// A unit's engagement fields (the view's units, under engagements): its role, its A before the
/// bonuses of a plot it attacks (its role and own percentages: as it hits back), its Dv where it
/// stands (the defense job value of phase6_plan 5.4), both x100, and its max HP.
pub fn unit_fields(g: &Game, u: UnitId, j: &mut Json) {
    let Some(role) = g.unit_combat_role(u) else { return };
    let Some(at) = g.unit_plot_opt(u) else { return };
    let ctx = HitCtx { to: at, from: None, city_defense: None };
    j.set("engage_role", Json::idx(role));
    j.set("engage_attack", g.engage_stat(u, None, Side::Defender, true, &ctx).total_x100);
    j.set("engage_defense", g.engage_job_defense(u, &ctx));
    j.set("engage_max_hp", g.unit_engage_max_hp(u));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_term_source_has_a_name() {
        let srcs = [
            TermSource::Own,
            TermSource::Promotion(PromotionType::from_usize(0)),
            TermSource::Hills,
            TermSource::VsUnitCombat(UnitCombatType::from_usize(0)),
            TermSource::Improvement(ImprovementType::from_usize(0)),
            TermSource::RangeDefense,
        ];
        for s in srcs {
            let j = term_json(&Term { src: s, pct: 5 });
            assert!(j.get("src").and_then(Json::as_str).is_some_and(|n| !n.is_empty()));
            assert_eq!(j.get("pct").and_then(Json::as_i64), Some(5));
        }
    }

    /// The screens' `base_x100` is the strength before the role's percent (the engine's has it in).
    #[test]
    fn a_stat_shows_the_strength_before_its_role() {
        let d = ffh_data::global();
        let horseman = d.t::<UnitType>("UNIT_HORSEMAN");
        let j = stat_json(d, horseman, true, &Stat { base_x100: 440, role_pct: 110, terms: Vec::new(), total_x100: 440 });
        assert_eq!(j.get("base_x100").and_then(Json::as_i64), Some(400));
        assert_eq!(j.get("role_pct").and_then(Json::as_i64), Some(110));
        assert_eq!(j.get("total_x100").and_then(Json::as_i64), Some(440));
    }

    /// The help's engagement wording goes to the units and promotions with the abilities it
    /// explains, and only to them.
    #[test]
    fn the_combat_abilities_get_their_engagement_wording() {
        let d = ffh_data::global();
        let unit = |n: &str| crate::view::help::engagement_notes(d, "UnitType", d.t::<UnitType>(n).idx());
        let promo = |n: &str| crate::view::help::engagement_notes(d, "PromotionType", d.t::<PromotionType>(n).idx());
        assert!(unit("UNIT_HORSEMAN").is_some_and(|t| t.contains("Withdrawal is one roll")), "{:?}", unit("UNIT_HORSEMAN"));
        assert_eq!(unit("UNIT_WARRIOR"), None);
        assert!(promo("PROMOTION_DRILL1").is_some_and(|t| t.contains("opening volleys")));
        assert!(promo("PROMOTION_BLITZ").is_some_and(|t| t.contains("one more battle")));
        assert_eq!(crate::view::help::engagement_notes(d, "TechType", 0), None);
    }
}
