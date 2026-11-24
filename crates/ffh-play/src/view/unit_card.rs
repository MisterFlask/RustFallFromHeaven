//! The Unit Card's battle sheet (the client's unit card): what a unit, or a fresh unit of a type,
//! does in a formation battle (docs/ENGAGEMENTS.md, "The rules"), as numbers the server computes.
//!
//! The same members go in two places: flat on every view unit that engages on land
//! ([`crate::view::battles::unit_fields`], the unit as it is) and as the `battle` object of the
//! data's `UnitType` entries ([`type_battle`]: a fresh unit of the type holding its
//! `free_promotions`, no civilization, trait or city promotions, standing nowhere: no place,
//! fortification or Homeland terms). Both are built by one [`Sheet`] and one writer ([`write`]),
//! and their values come from the rules' own code (`ffh_core::engagement::stats`: the stats'
//! formula, the rows with a battle spell, a Spell row's spell; `formation`: a hit's mean, the hit
//! and crit chances, an action's pattern and splash), so a type's card and a fresh unit's agree.
//!
//! Members: `engage_row_order`, `engage_attack` and `engage_defense` (x100), `engage_max_hp`,
//! `engage_speed`, `engage_actions` (`[{kind, n}] x3`, the extra action included), `engage_magic`
//! and `engage_magic_defense` (x100), `engage_accuracy`, `engage_evasion`, `engage_crit`,
//! `engage_large`; `engage_rows` (per row `{kind, n, pattern?, splash?, spell?, bolt?, hit?, crit?,
//! dmg?, heal?}`), `engage_keywords` (the rows' kinds, patterns and splash, then Large, each once),
//! `engage_charms` and `engage_heal_spell` (spell indices; absent when none).
//!
//! `dmg` is the expected HP of one hit against each of the reference defences
//! ([`CARD_REF_DEFENSES`], on the stat sheet's scale), before the 80-120% roll, a crit, the
//! ground and the pair's percentages: a physical hit from `engage_attack`, a spell from
//! `engage_magic` times the spell's power, no resistance.

use ffh_core::army::hero_promotion;
use ffh_core::engagement::formation::{self, Params};
use ffh_core::engagement::stats;
use ffh_core::engagement::{HitCtx, Side};
use ffh_core::game::Game;
use ffh_core::ids::UnitId;
use ffh_core::unit::CombatTerms;
use ffh_data::*;

use crate::json::Json;
use crate::view::battles::{actions_json, kind_name};

/// The reference defences of a card's `dmg` (on the stat sheet's scale: a Warrior's defence is
/// 20): weak, middling and strong targets. Sent as the data's `card_ref_defenses`.
pub const CARD_REF_DEFENSES: [i32; 3] = [20, 40, 80];

/// What the battle sheet is made from: a unit's values, or a fresh unit's of a type.
#[derive(Clone, Debug)]
pub struct Sheet {
    pub row_order: [usize; ROWS],
    /// x100.
    pub attack: i32,
    pub defense: i32,
    pub max_hp: i32,
    pub speed: i32,
    /// What it does from each row, its extra actions included.
    pub actions: [RowActions; ROWS],
    /// x100.
    pub magic: i32,
    pub magic_defense: i32,
    pub accuracy: i32,
    pub evasion: i32,
    pub crit: i32,
    pub large: bool,
    /// Its type's attack pattern from each row.
    pub patterns: [AttackPattern; ROWS],
    /// The battle spells it casts with its SPELL actions (none when no row casts).
    pub spells: BattleSpells<SpellType>,
    /// It has collateral damage.
    pub collateral: bool,
    /// Its heal, in percent of a hit (`None`: the define's).
    pub heal_pct: Option<i32>,
    /// It has first strikes or chance first strikes (`KEYWORD_FIRST_STRIKE`).
    pub first_strike: bool,
    /// It has a withdrawal chance, attacking or defending (`KEYWORD_WITHDRAWAL`).
    pub withdrawal: bool,
    /// It has a combat limit below 100% and can attack (`KEYWORD_COMBAT_LIMIT`;
    /// `help::engagement_notes`' test).
    pub combat_limit: bool,
}

impl Sheet {
    /// Unit `u` as it is: its A before the bonuses of a plot it attacks and its Dv where it
    /// stands (`at`), as the view's units always sent them.
    pub fn of_unit(g: &Game, u: UnitId, at: ffh_core::ids::PlotIdx) -> Sheet {
        let ctx = HitCtx::at(at);
        let t = g.units[u].unit_type;
        Sheet {
            row_order: g.unit_engage_row_order(u),
            attack: g.engage_stat(u, None, Side::Defender, true, &ctx).total_x100,
            defense: g.engage_job_defense(u, &ctx),
            max_hp: g.unit_engage_max_hp(u),
            speed: g.unit_engage_speed(u),
            actions: g.unit_engage_actions(u),
            magic: g.engage_magic_x100(u, true),
            magic_defense: g.engage_magic_x100(u, false),
            accuracy: g.unit_engage_accuracy(u),
            evasion: g.unit_engage_evasion(u),
            crit: g.unit_engage_crit(u),
            large: g.unit_engage_large(u),
            patterns: g.d.engage_patterns(t),
            spells: g.unit_engage_battle_spells(u),
            collateral: g.unit_collateral_damage(u) > 0,
            heal_pct: g.d.units[t].heal_percent,
            first_strike: g.unit_first_strikes(u) + g.unit_chance_first_strikes(u) > 0,
            withdrawal: g.unit_withdrawal_probability(u) > 0 || g.unit_get_withdrawl_prob_defensive(u) > 0,
            combat_limit: g.units[u].combat_limit < g.d.defines.max_hit_points && g.unit_attack_rating(u) > 0 && !g.d.units[t].only_defensive,
        }
    }

    /// A fresh unit of type `t` holding its `free_promotions` (their `stat_changes` and typed
    /// attack points, battle spells, the Hero's extra action), standing nowhere; `None` for a type
    /// that takes no part in land battles (a ship, an air unit, a type without a stat sheet or
    /// that does not fight).
    pub fn of_type(d: &GameData, t: UnitType) -> Option<Sheet> {
        let info = &d.units[t];
        info.stats?;
        if !d.type_engages_on_land(t) || !info.fights() {
            return None;
        }
        let free = &info.free_promotions;
        let has = |p: PromotionType| free[p];
        // What a unit gains with its promotions as it is made (`unit_apply_stat_changes`,
        // `unit_change_damage_type_attack`): their points and percentages, summed.
        let mut flat: StatSheet = [0; StatType::NUM];
        let mut pct: StatSheet = [0; StatType::NUM];
        let mut typed = 0;
        for p in d.promotions.keys().filter(|&p| free[p]) {
            let pi = &d.promotions[p];
            for s in StatType::ALL {
                flat[s.idx()] += pi.stat_changes[s.idx()].flat;
                pct[s.idx()] += pi.stat_changes[s.idx()].percent;
            }
            typed += pi.damage_type_attack.iter().sum::<i32>();
        }
        let stat = |s: StatType| stats::stat_value(d, t, s, flat[s.idx()], pct[s.idx()]);
        let hero = hero_promotion(d).is_some_and(|h| free[h]);
        let rows = stats::engage_rows_of(d, t, has, info.religion_type, || stat(StatType::Magic));
        let actions = rows_with_extra_actions(rows, d.engage_extra_actions(t, hero));
        let spells = if actions.iter().any(|r| r.kind == ActionKind::Spell) { d.engage_battle_spells(t, has, info.religion_type) } else { BattleSpells::default() };
        // A and Dv against nobody, standing nowhere: the rating (sheet and typed points) plus the
        // promotions' points, x10, under the own percentage (`Game::engage_combat_terms` with no
        // place term).
        // First strikes, withdrawal and the combat limit, as a unit has them once made: its type's
        // and its promotions' changes (`unit_first_strikes`, `unit_withdrawal_probability`,
        // `unit_get_withdrawl_prob_defensive`, `unit_calc_combat_limit`: the lowest limit).
        let promos = || d.promotions.keys().filter(|&p| free[p]).map(|p| &d.promotions[p]);
        let first_strikes = info.first_strikes + info.chance_first_strikes + promos().map(|p| p.first_strikes_change + p.chance_first_strikes_change).sum::<i32>();
        let extra_withdrawal: i32 = promos().map(|p| p.withdrawal_change).sum();
        let limit = promos().map(|p| p.combat_limit).filter(|&v| v != 0).fold(info.combat_limit, i32::min);
        let rated = |rating: i32, s: StatType| CombatTerms { base: (rating + flat[s.idx()] + typed) * 10, modifier: pct[s.idx()] }.strength();
        Some(Sheet {
            row_order: d.engage_row_order(t),
            attack: rated(info.attack_rating(), StatType::Attack),
            defense: rated(info.defense_rating(), StatType::Defense),
            max_hp: stat(StatType::Hp),
            speed: stat(StatType::Speed),
            actions,
            magic: stats::magic_terms_of(d, t, has, &flat, &pct, true).0.strength(),
            magic_defense: stats::magic_terms_of(d, t, has, &flat, &pct, false).0.strength(),
            accuracy: stat(StatType::Accuracy),
            evasion: stat(StatType::Evasion),
            crit: stat(StatType::Crit),
            large: info.large,
            patterns: info.engage_patterns,
            spells,
            collateral: info.collateral_damage > 0,
            heal_pct: info.heal_percent,
            first_strike: first_strikes > 0,
            withdrawal: info.withdrawal_probability + extra_withdrawal > 0 || info.withdrawl_prob_defensive + extra_withdrawal > 0,
            combat_limit: limit < d.defines.max_hit_points && info.attack_rating() + flat[StatType::Attack.idx()] + typed > 0 && !info.only_defensive,
        })
    }
}

/// HP from centi-HP, to the nearest.
fn round_hp(centi: i64) -> i64 {
    (centi + 50).div_euclid(100)
}

/// The expected hit of `a` against a target of `dv` (both x100), times `power` percent, in
/// centi-HP (`formation::mean_damage`; 0 when both are 0).
fn mean(p: &Params, a: i32, dv: i32, power: i32) -> i64 {
    let (a, dv) = (a.max(0) as i64, dv.max(0) as i64);
    if a + dv == 0 {
        return 0;
    }
    formation::mean_damage(p.base_damage, a, dv) * power.max(0) as i64 / 100
}

/// What one row of the sheet does, as the resolver would.
struct Row {
    kind: ActionKind,
    n: i32,
    pattern: Option<AttackPattern>,
    splash: bool,
    spell: Option<SpellType>,
    bolt: bool,
    hit: Option<i32>,
    crit: Option<i32>,
    dmg: Option<[i64; 3]>,
    heal: Option<i64>,
}

fn rows(d: &GameData, s: &Sheet) -> Vec<Row> {
    let p = Params::from_defines(&d.defines);
    // What its SPELL actions cast (`Game::engage_fighter`'s spell).
    let spell = stats::engage_spell_of(d, s.spells.attack, &p);
    let wonder = s.spells.attack.is_some_and(|x| !d.spells[x].battle_wonder.is_empty());
    let refs = CARD_REF_DEFENSES.map(|r| r * 10);
    (0..ROWS)
        .map(|i| {
            let r = s.actions[i];
            let mut row = Row { kind: r.kind, n: r.count, pattern: None, splash: false, spell: None, bolt: false, hit: None, crit: None, dmg: None, heal: None };
            if r.kind == ActionKind::None || r.count <= 0 {
                row.n = 0;
                return row;
            }
            let single = |x: AttackPattern| (x != AttackPattern::Single).then_some(x);
            match r.kind {
                ActionKind::Heal => {
                    row.pattern = single(s.patterns[i]);
                    let pct = s.heal_pct.unwrap_or(p.heal_pct);
                    row.heal = Some(round_hp(p.base_damage.max(0) as i64 * pct.max(0) as i64));
                }
                ActionKind::Spell if wonder => {
                    // Each cast rolls its outcome: no pattern, splash or damage of its own.
                    row.spell = s.spells.attack;
                }
                kind => {
                    let collateral = formation::collateral_splashes(s.collateral, kind, &spell);
                    let sp = formation::spread(&p, kind, &spell, || s.patterns[i], collateral);
                    row.pattern = sp.pattern_pct.map(|_| sp.pattern);
                    row.splash = sp.splash();
                    if kind == ActionKind::Spell {
                        row.spell = s.spells.attack;
                        row.bolt = s.spells.attack.is_none();
                        row.dmg = Some(refs.map(|dv| round_hp(mean(&p, s.magic, dv, spell.power))));
                    } else {
                        row.hit = Some(formation::hit_chance(&p, kind, s.accuracy));
                        row.dmg = Some(refs.map(|dv| round_hp(mean(&p, s.attack, dv, 100))));
                    }
                    if kind.can_crit() {
                        row.crit = Some(formation::crit_chance_of(s.crit));
                    }
                }
            }
            row
        })
        .collect()
}

fn pattern_name(p: AttackPattern) -> &'static str {
    match p {
        AttackPattern::Single => "Single",
        AttackPattern::Volley => "Volley",
        AttackPattern::Pierce => "Pierce",
        AttackPattern::Sweep => "Sweep",
        AttackPattern::Cleave => "Cleave",
        AttackPattern::Trample => "Trample",
    }
}

/// A kind's keyword (`KEYWORD_STRIKE`); none for NONE.
fn kind_keyword(k: ActionKind) -> Option<&'static str> {
    Some(match k {
        ActionKind::None => return None,
        ActionKind::Strike => "KEYWORD_STRIKE",
        ActionKind::Charge => "KEYWORD_CHARGE",
        ActionKind::Stab => "KEYWORD_STAB",
        ActionKind::Shoot => "KEYWORD_SHOOT",
        ActionKind::Spell => "KEYWORD_SPELL",
        ActionKind::Bombard => "KEYWORD_BOMBARD",
        ActionKind::Heal => "KEYWORD_HEAL",
    })
}

fn pattern_keyword(p: AttackPattern) -> Option<&'static str> {
    Some(match p {
        AttackPattern::Single => return None,
        AttackPattern::Volley => "KEYWORD_VOLLEY",
        AttackPattern::Pierce => "KEYWORD_PIERCE",
        AttackPattern::Sweep => "KEYWORD_SWEEP",
        AttackPattern::Cleave => "KEYWORD_CLEAVE",
        AttackPattern::Trample => "KEYWORD_TRAMPLE",
    })
}

/// The sheet's keywords, each once: for each row front to rear that acts, its kind's, its
/// pattern's and Splash when it splashes; then First strike, Withdrawal and Combat limit when it
/// has them; last Large.
pub fn keywords(d: &GameData, s: &Sheet) -> Vec<KeywordType> {
    keyword_names(s, &rows(d, s)).into_iter().filter_map(|n| d.ty::<KeywordType>(n)).collect()
}

/// [`keywords`] by their type names, from the sheet's rows `rows`.
fn keyword_names(s: &Sheet, rows: &[Row]) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let mut add = |n: &'static str| {
        if !names.contains(&n) {
            names.push(n);
        }
    };
    for r in rows.iter().filter(|r| r.n > 0) {
        if let Some(n) = kind_keyword(r.kind) {
            add(n);
        }
        if let Some(n) = r.pattern.and_then(pattern_keyword) {
            add(n);
        }
        if r.splash {
            add("KEYWORD_SPLASH");
        }
    }
    // Its properties: first strikes, withdrawal, a combat limit; then its size.
    for (has, name) in [(s.first_strike, "KEYWORD_FIRST_STRIKE"), (s.withdrawal, "KEYWORD_WITHDRAWAL"), (s.combat_limit, "KEYWORD_COMBAT_LIMIT"), (s.large, "KEYWORD_LARGE")] {
        if has {
            add(name);
        }
    }
    names
}

/// The keywords of unit type `t`'s battle sheet (none for a type without one): the data's
/// reminders put them first.
pub fn type_keywords(d: &GameData, t: UnitType) -> Vec<KeywordType> {
    Sheet::of_type(d, t).map_or_else(Vec::new, |s| keywords(d, &s))
}

fn rows_json(rows: &[Row]) -> Json {
    Json::Arr(
        rows.iter()
            .map(|r| {
                let mut j = crate::jobj! { "kind" => kind_name(r.kind), "n" => r.n };
                if let Some(x) = r.pattern {
                    j.set("pattern", pattern_name(x));
                }
                if r.splash {
                    j.set("splash", true);
                }
                if let Some(x) = r.spell {
                    j.set("spell", Json::idx(x));
                }
                if r.bolt {
                    j.set("bolt", true);
                }
                if let Some(x) = r.hit {
                    j.set("hit", x);
                }
                if let Some(x) = r.crit {
                    j.set("crit", x);
                }
                if let Some(x) = r.dmg {
                    j.set("dmg", Json::Arr(x.iter().map(|&v| Json::from(v)).collect()));
                }
                if let Some(x) = r.heal {
                    j.set("heal", x);
                }
                j
            })
            .collect(),
    )
}

/// Writes the battle sheet's members into `j` (module comment).
pub fn write(d: &GameData, s: &Sheet, j: &mut Json) {
    j.set("engage_row_order", Json::Arr(s.row_order.iter().map(|&r| Json::from(r)).collect()));
    j.set("engage_attack", s.attack);
    j.set("engage_defense", s.defense);
    j.set("engage_max_hp", s.max_hp);
    j.set("engage_speed", s.speed);
    j.set("engage_actions", actions_json(&s.actions));
    j.set("engage_magic", s.magic);
    j.set("engage_magic_defense", s.magic_defense);
    j.set("engage_accuracy", s.accuracy);
    j.set("engage_evasion", s.evasion);
    j.set("engage_crit", s.crit);
    j.set("engage_large", s.large);
    // (Every visible unit, every sync: its rows are worked out once, its keywords named.)
    let rows = rows(d, s);
    j.set("engage_rows", rows_json(&rows));
    j.set("engage_keywords", Json::Arr(keyword_names(s, &rows).into_iter().map(Json::from).collect()));
    if !s.spells.charms.is_empty() {
        j.set("engage_charms", Json::Arr(s.spells.charms.iter().map(|&c| Json::idx(c)).collect()));
    }
    if let Some(h) = s.spells.heal {
        j.set("engage_heal_spell", Json::idx(h));
    }
}

/// The `battle` object of unit type `t`'s data entry (module comment); `None` for a type without
/// a battle sheet.
pub fn type_battle(d: &GameData, t: UnitType) -> Option<Json> {
    let s = Sheet::of_type(d, t)?;
    let mut j = Json::obj();
    write(d, &s, &mut j);
    Some(j)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battle(name: &str) -> Json {
        let d = ffh_data::global();
        type_battle(d, d.t::<UnitType>(name)).unwrap_or_else(|| panic!("{name}: no battle sheet"))
    }

    fn row(b: &Json, i: usize) -> &Json {
        b.get("engage_rows").and_then(|r| r.at(i)).unwrap()
    }

    fn int(j: &Json, k: &str) -> i64 {
        j.get(k).and_then(Json::as_i64).unwrap_or_else(|| panic!("no {k} in {j:?}"))
    }

    fn keys(j: &Json) -> Vec<&str> {
        j.as_obj().unwrap().iter().map(|(k, _)| k.as_str()).collect()
    }

    /// The expected hit of attack `a` (the sheet's scale) against defence `dv`, to the nearest HP.
    fn hp(a: f64, dv: f64) -> i64 {
        let base = ffh_data::global().defines.engagement_base_damage as f64;
        (base * a / (a + dv)).round() as i64
    }

    #[test]
    fn an_axeman_strikes_twice_from_the_front() {
        let d = ffh_data::global();
        let b = battle("UNIT_AXEMAN");
        let front = row(&b, 0);
        assert_eq!(front.get("kind").and_then(Json::as_str), Some("Strike"));
        assert_eq!(int(front, "n"), 2);
        assert_eq!(int(front, "hit"), 90);
        assert_eq!(int(front, "crit"), 10);
        let dmg: Vec<i64> = front.get("dmg").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_i64).collect();
        let want: Vec<i64> = CARD_REF_DEFENSES.iter().map(|&r| hp(40.0, r as f64)).collect();
        assert_eq!(dmg, want);
        assert_eq!(keys(front), ["kind", "n", "hit", "crit", "dmg"]);
        assert_eq!(int(&b, "engage_attack"), 400);
        let kw: Vec<&str> = b.get("engage_keywords").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
        assert_eq!(kw, ["KEYWORD_STRIKE"]);
        let _ = d;
    }

    #[test]
    fn a_catapult_has_no_front_action_and_bombards_with_a_splash() {
        let b = battle("UNIT_CATAPULT");
        let front = row(&b, 0);
        assert_eq!(int(front, "n"), 0);
        assert_eq!(keys(front), ["kind", "n"]);
        assert_eq!(front.get("kind").and_then(Json::as_str), Some("None"));
        let rear = row(&b, 2);
        assert_eq!(rear.get("kind").and_then(Json::as_str), Some("Bombard"));
        // 85 accuracy less the bombard's miss; no crit.
        let d = ffh_data::global();
        assert_eq!(int(rear, "hit"), 85 - d.defines.engagement_bombard_miss_percent as i64);
        assert!(rear.get("crit").is_none());
        // Collateral damage: a splash.
        assert_eq!(rear.get("splash").and_then(Json::as_bool), Some(true), "{rear:?}");
        let kw: Vec<&str> = b.get("engage_keywords").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_str).collect();
        assert_eq!(kw, ["KEYWORD_BOMBARD", "KEYWORD_SPLASH", "KEYWORD_WITHDRAWAL", "KEYWORD_COMBAT_LIMIT"]);
    }

    #[test]
    fn a_casters_spell_rows_cast_a_spell_or_the_bolt() {
        let d = ffh_data::global();
        let mut casters = 0;
        for t in d.units.keys() {
            let Some(b) = type_battle(d, t) else { continue };
            for i in 0..ROWS {
                let r = row(&b, i);
                if r.get("kind").and_then(Json::as_str) == Some("Spell") && int(r, "n") > 0 {
                    casters += 1;
                    assert!(r.get("spell").is_some() != r.get("bolt").is_some(), "{}: {r:?}", d.units[t].base.type_);
                    assert!(r.get("hit").is_none() && r.get("crit").is_none(), "{r:?}");
                    let wonder = r.get("spell").and_then(Json::as_i64).is_some_and(|s| !d.spells[SpellType::from_usize(s as usize)].battle_wonder.is_empty());
                    assert_eq!(r.get("dmg").is_some(), !wonder, "{r:?}");
                }
            }
        }
        assert!(casters > 10, "{casters}");
        // An Adept's bolt: its magic at the bolt's power, the row's Pierce.
        let adept = battle("UNIT_ADEPT");
        let rear = row(&adept, 2);
        if rear.get("bolt").is_some() {
            assert_eq!(rear.get("pattern").and_then(Json::as_str), Some("Pierce"));
            let m = int(&adept, "engage_magic") as f64 / 10.0;
            let bolt = d.defines.engagement_bolt_percent as f64 / 100.0;
            let dmg: Vec<i64> = rear.get("dmg").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_i64).collect();
            for (k, &r) in CARD_REF_DEFENSES.iter().enumerate() {
                assert!((dmg[k] as f64 - hp(m, r as f64) as f64 * bolt).abs() <= 1.0, "{dmg:?}");
            }
        }
    }

    /// Every number of the engagement keywords' reminders is its define's, so a define change
    /// fails here rather than leaving a stale reminder.
    #[test]
    fn engagement_reminders_say_the_defines() {
        let d = ffh_data::global();
        let x = &d.defines;
        let cases: &[(&str, &[i32])] = &[
            ("KEYWORD_STRIKE", &[]),
            ("KEYWORD_CHARGE", &[x.engagement_charge_bonus, 1]),
            ("KEYWORD_STAB", &[]),
            ("KEYWORD_SHOOT", &[x.engagement_shoot_miss_percent]),
            ("KEYWORD_SPELL", &[]),
            ("KEYWORD_BOMBARD", &[x.engagement_bombard_miss_percent]),
            ("KEYWORD_HEAL", &[]),
            ("KEYWORD_VOLLEY", &[x.engagement_volley_percent]),
            ("KEYWORD_PIERCE", &[x.engagement_pierce_percent]),
            ("KEYWORD_SWEEP", &[x.engagement_sweep_percent, x.engagement_sweep_percent]),
            ("KEYWORD_CLEAVE", &[x.engagement_cleave_percent]),
            ("KEYWORD_TRAMPLE", &[x.engagement_trample_percent]),
            ("KEYWORD_SPLASH", &[x.engagement_splash_percent]),
            ("KEYWORD_LARGE", &[2]),
            ("KEYWORD_FIRST_STRIKE", &[x.engagement_first_strike_speed]),
            ("KEYWORD_WITHDRAWAL", &[]),
            // An illustration, not a define: a 75% limit leaves 25%.
            ("KEYWORD_COMBAT_LIMIT", &[75, 25]),
        ];
        for &(name, want) in cases {
            let k = d.ty::<KeywordType>(name).unwrap_or_else(|| panic!("{name}"));
            let text = crate::view::keywords::reminder(d, k);
            let nums: Vec<i32> = text.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse().unwrap()).collect();
            assert_eq!(nums, want, "{name}: {text}");
        }
    }

    #[test]
    #[ignore]
    fn print_samples() {
        let d = ffh_data::global();
        let data = crate::view::data::data(d);
        let units = data.get("tables").and_then(|t| t.get("UnitType")).and_then(Json::as_arr).unwrap();
        for n in ["UNIT_BEASTMAN", "UNIT_ARCHER", "UNIT_MAGE", "UNIT_CATAPULT", "UNIT_GHOST", "UNIT_KNIGHT"] {
            let e = units.iter().find(|e| e.get("type").and_then(Json::as_str) == Some(n)).unwrap();
            let mut j = Json::obj();
            for k in ["type", "battle", "free_promotions", "sections", "abilities", "upgrades_to", "reminders"] {
                if let Some(v) = e.get(k) {
                    j.set(k, v.clone());
                }
            }
            println!("{}", j.to_pretty());
        }
        println!("defines: {}", data.get("defines").unwrap().to_pretty());
    }
}
