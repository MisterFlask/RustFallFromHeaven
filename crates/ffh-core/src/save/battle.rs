//! [`Persist`](super::Persist) for engagement combat's saved state: the battle reports and their
//! logs (`engagement/report.rs`) and the formation resolver's types they hold (`Side`, `Ground`,
//! `GroundRule`, the action `Event` with its hits). Structs are listed field by field in
//! declaration order and enums variant by variant with fixed tags (`persist_struct!`,
//! `persist_enum!`), so a new field or variant fails to compile until it is listed here (and
//! `FORMAT_VERSION` is bumped).

use super::io::{persist_enum, persist_newtype, persist_struct};

use crate::engagement::formation::{Entry, Event, Ground, GroundRule, Heal, Hit, Knock, Note, Side};
use crate::engagement::report::*;

persist_enum!(Side { 0 => Attacker, 1 => Defender });
persist_enum!(EngageResult { 0 => Taken, 1 => Cleared, 2 => Repulsed, 3 => Held });

persist_struct!(BattleReport { id, turn, plot, from, place, sides, outcome, log });
persist_struct!(ReportSide { player, shown_player, players, slots, lanes, used, committed, sat_out, fallen, escaped, standing, lost, summons, shore });
persist_struct!(PlotSnap { x, y, terrain, feature, hills, river_crossing, city });
persist_struct!(CitySnap { id, name, owner, size, defense_pct });
persist_struct!(Outcome { result, rounds, plot_taken, city_captured, moved_in, defenders_left, sentinels, sentinels_breached });

persist_struct!(BattleLog { slots, lanes, ground, rules, ordered, tactics, fighters, pairs, events, forecast });
persist_struct!(Ground { city, forest, hills });
persist_enum!(GroundRule {
    0 => CityShots { pct },
    1 => ForestRanged { pct },
    2 => HillsAttacker { pct },
    3 => HillsDefenderRanged { pct },
    4 => Charge { pct },
    5 => NoFlanking,
});
persist_struct!(SideSlots { base, changes, total, river_only });
persist_struct!(SlotChange { src, delta });
persist_enum!(SlotSource {
    0 => Terrain(t),
    1 => Feature(f),
    2 => Hills,
    3 => River,
    4 => Sentinels(n),
    6 => Minimum,
    7 => Summons(n),
    20 => Improvement(i),
});

persist_struct!(LogFighter {
    unit, sentinel, unit_type, name, owner, shown_owner, side, place, large, level, xp, hero, max_hp,
    hp_start, hp_battle, speed, first_strikes, chance_first_strikes, rolled_first_strikes, actions,
    spell, accuracy, evasion, crit, anti_flank, flanker, withdraw_pct, combat_limit, collateral_limit,
    def_strike, immune, fear, a, dv, mag, mdef, fate, dealt, taken, healed, summoner, on_kill,
});
persist_enum!(LogFate { 0 => Standing, 1 => Fell { by, round }, 2 => Withdrew { round }, 3 => SatOut });
persist_newtype!(Immunities);
persist_struct!(Stat { base_x100, terms, total_x100 });
persist_struct!(Term { src, pct });
persist_enum!(TermSource {
    0 => Own,
    1 => Promotion(p),
    2 => Spell(s),
    3 => BattleEffect(s),
    4 => Hills,
    5 => Terrain(t),
    6 => Feature(f),
    7 => Fortify,
    8 => CityDefense,
    9 => UnitCityDefense,
    10 => UnitHillsDefense,
    11 => UnitTerrainDefense(t),
    12 => UnitFeatureDefense(f),
    13 => CityAttack,
    14 => UnitTerrainAttack(t),
    15 => UnitFeatureAttack(f),
    16 => Amphibious,
    17 => VsUnitCombat(c),
    18 => VsUnitClass(c),
    19 => VsPromotion(p),
    20 => VsAnimal,
    21 => VsBarbarian,
    22 => VsDomain(d),
    23 => InBorders,
    24 => GlobalCounter,
    25 => Handicap,
    26 => Aura(u),
    27 => Typed(t),
    28 => UnitHillsAttack,
    29 => Improvement(i),
    30 => RangeDefense,
    31 => SeaWear,
});
persist_struct!(LogPair { h, t, a_x100, dv_x100, mean, ma_x100, md_x100, spell_pct, magic_mean, a_extra, dv_extra });

persist_enum!(Stage { 0 => PreBattle, 1 => Morale, 2 => Opening, 3 => Battle, 4 => Post });
persist_enum!(LogEvent {
    0 => Stage { stage },
    1 => Cast { caster, spell, targets, effect },
    2 => Morale { unit, source, chance, roll, passed },
    3 => SatOut { unit, why },
    4 => DefStrike { striker, target, rolled, chance, roll, pct, dmg },
    5 => Action(ev),
    6 => End { rounds, down },
    7 => Heal { unit, amount, source },
    8 => Xp { unit, amount, from },
    9 => KillEffect { killer, victim, kind },
    10 => Flee { unit, to },
    11 => MoveIn { units, city },
    30 => BuildingStrike { building, hits },
    31 => Starvation { units, dealt_pct },
    32 => ShoreFire { ship, owner, side, hits },
});
persist_struct!(Event { round, pass, actor, kind, spell, target, hits, heal, knocked, note, entered });
persist_struct!(Entry { unit, lane, row, hp });
persist_struct!(Hit { target, dmg, expected, r, crit, miss, killed, withdrew, splash });
persist_struct!(Heal { target, amount, r });
persist_struct!(Knock { unit, from, to });
persist_enum!(Note { 0 => NoTarget, 1 => NoOneHurt, 2 => Skipped, 10 => Held });
persist_enum!(CastEffect { 0 => Mod { a_pct, dv_pct }, 1 => Damage(hits), 2 => Summon(b) });
persist_enum!(SatOutWhy { 0 => Afraid });
persist_enum!(HealSource { 0 => Medic { b, unit_type, adjacent }, 1 => OnKill, 2 => Spell(s) });
persist_enum!(XpSource { 0 => Kill { victim, share_permille }, 1 => Withdrew, 2 => DefStrike });
persist_enum!(KillKind {
    0 => Capture(t),
    1 => Enslave(t),
    2 => Convert(t),
    3 => Create(t),
    4 => Gold(g),
    5 => Afflict(p),
    6 => Promotion(p),
});

persist_struct!(FrozenForecast { runs, wiped_permille, falls_permille, repulsed_permille, lost_x100, per });
persist_struct!(ForecastFighter { b, fall_pct, escape_pct });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::*;
    use crate::save::{Persist, Reader, Writer};
    use ffh_data::*;

    fn round_trip<T: Persist + PartialEq + std::fmt::Debug>(v: &T) -> usize {
        let mut w = Writer::new();
        v.write(&mut w);
        let bytes = w.into_bytes();
        let mut r = Reader::new(&bytes);
        assert_eq!(&T::read(&mut r).unwrap(), v);
        assert_eq!(r.remaining(), 0);
        bytes.len()
    }

    fn stat(base: i32) -> Stat {
        Stat {
            base_x100: base,
            terms: vec![Term { src: TermSource::Promotion(PromotionType(3)), pct: 20 }, Term { src: TermSource::Fortify, pct: 25 }],
            total_x100: base * 145 / 100,
        }
    }

    fn fighter(side: Side, place: Option<(u8, u8)>, fate: Option<LogFate>) -> LogFighter {
        let row = |kind, count| RowActions { kind, count };
        LogFighter {
            unit: UnitId(17),
            sentinel: place.is_some_and(|p| p.0 == 2),
            unit_type: UnitType(5),
            name: String::new(),
            owner: PlayerId(1),
            shown_owner: PlayerId(8),
            side,
            place,
            large: place.is_some_and(|p| p.0 == 1),
            level: 2,
            xp: 7,
            hero: false,
            max_hp: 120,
            hp_start: 9000,
            hp_battle: 8000,
            speed: 6,
            first_strikes: 1,
            chance_first_strikes: 2,
            rolled_first_strikes: 1,
            actions: [row(ActionKind::Spell, 1), row(ActionKind::Heal, 2), row(ActionKind::None, 0)],
            spell: Some(SpellType(4)),
            accuracy: 95,
            evasion: 15,
            crit: 20,
            anti_flank: true,
            flanker: false,
            withdraw_pct: 30,
            combat_limit: 100,
            collateral_limit: Some(50),
            def_strike: Some((20, 10)),
            immune: Immunities::default().with(Immunities::FEAR, true).with(Immunities::DEF_STRIKE, true),
            fear: true,
            a: stat(400),
            dv: stat(300),
            mag: stat(250),
            mdef: stat(350),
            fate,
            dealt: 2400,
            taken: 1000,
            healed: 300,
            summoner: None,
            on_kill: false,
        }
    }

    /// A report whose log holds every event, slot source, term source, ground rule, action kind,
    /// fate and kill kind.
    fn every_variant() -> BattleReport {
        let terms: Vec<Term> = [
            TermSource::Own,
            TermSource::Promotion(PromotionType(1)),
            TermSource::Spell(SpellType(2)),
            TermSource::BattleEffect(SpellType(3)),
            TermSource::Hills,
            TermSource::Terrain(TerrainType(1)),
            TermSource::Feature(FeatureType(2)),
            TermSource::Fortify,
            TermSource::CityDefense,
            TermSource::UnitCityDefense,
            TermSource::UnitHillsDefense,
            TermSource::UnitTerrainDefense(TerrainType(3)),
            TermSource::UnitFeatureDefense(FeatureType(4)),
            TermSource::CityAttack,
            TermSource::UnitTerrainAttack(TerrainType(5)),
            TermSource::UnitFeatureAttack(FeatureType(6)),
            TermSource::Amphibious,
            TermSource::VsUnitCombat(UnitCombatType(7)),
            TermSource::VsUnitClass(UnitClassType(8)),
            TermSource::VsPromotion(PromotionType(9)),
            TermSource::VsAnimal,
            TermSource::VsBarbarian,
            TermSource::VsDomain(DomainType(0)),
            TermSource::InBorders,
            TermSource::GlobalCounter,
            TermSource::Handicap,
            TermSource::Aura(UnitId(99)),
            TermSource::Typed(DamageType(1)),
            TermSource::UnitHillsAttack,
            TermSource::Improvement(ImprovementType(2)),
            TermSource::RangeDefense,
            TermSource::SeaWear,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, src)| Term { src, pct: i as i32 * 5 - 30 })
        .collect();
        let slots = SideSlots {
            base: 8,
            changes: vec![
                SlotChange { src: SlotSource::Terrain(TerrainType(1)), delta: 2 },
                SlotChange { src: SlotSource::Feature(FeatureType(2)), delta: -2 },
                SlotChange { src: SlotSource::Hills, delta: -2 },
                SlotChange { src: SlotSource::River, delta: -2 },
                SlotChange { src: SlotSource::Sentinels(2), delta: -2 },
                SlotChange { src: SlotSource::Minimum, delta: 1 },
                SlotChange { src: SlotSource::Summons(2), delta: 2 },
            ],
            total: 4,
            river_only: 2,
        };
        let stages = [Stage::PreBattle, Stage::Morale, Stage::Opening, Stage::Battle, Stage::Post];
        let mut events: Vec<LogEvent> = stages.into_iter().map(|stage| LogEvent::Stage { stage }).collect();
        let hit = |target, dmg, crit, miss, killed, withdrew, splash| Hit { target, dmg, expected: 2000, r: 113, crit, miss, killed, withdrew, splash };
        events.extend([
            LogEvent::Cast { caster: 0, spell: SpellType(4), targets: vec![1, 2], effect: CastEffect::Mod { a_pct: 10, dv_pct: -5 } },
            LogEvent::Cast { caster: 0, spell: SpellType(4), targets: vec![1], effect: CastEffect::Damage(vec![(1, 1200), (2, 600)]) },
            LogEvent::Cast { caster: 0, spell: SpellType(5), targets: vec![], effect: CastEffect::Summon(3) },
            LogEvent::Starvation { units: vec![0, 2], dealt_pct: -25 },
            LogEvent::Morale { unit: 1, source: 0, chance: 12, roll: 30, passed: true },
            LogEvent::SatOut { unit: 2, why: SatOutWhy::Afraid },
            LogEvent::DefStrike { striker: 1, target: 0, rolled: 2, chance: 20, roll: 7, pct: 10, dmg: 1000 },
            LogEvent::Action(Event {
                round: 1,
                pass: 2,
                actor: 0,
                kind: ActionKind::Charge,
                spell: None,
                target: Some(1),
                hits: vec![hit(1, 3000, true, false, true, false, false), hit(4, 0, false, false, false, true, true)],
                heal: None,
                knocked: Some(Knock { unit: 1, from: 0, to: 1 }),
                note: None,
                entered: vec![],
            }),
            LogEvent::Action(Event {
                round: 2,
                pass: 1,
                actor: 1,
                kind: ActionKind::Spell,
                spell: Some(4),
                target: Some(0),
                hits: vec![hit(0, 0, false, true, false, false, false)],
                heal: None,
                knocked: None,
                note: None,
                entered: vec![],
            }),
            LogEvent::Action(Event { round: 3, pass: 1, actor: 2, kind: ActionKind::Heal, spell: None, target: Some(3), hits: vec![], heal: Some(Heal { target: 3, amount: 1700, r: 85 }), knocked: None, note: None, entered: vec![] }),
            LogEvent::Action(Event { round: 3, pass: 2, actor: 2, kind: ActionKind::Heal, spell: None, target: None, hits: vec![], heal: None, knocked: None, note: Some(Note::NoOneHurt), entered: vec![] }),
            LogEvent::Action(Event { round: 3, pass: 3, actor: 5, kind: ActionKind::Bombard, spell: None, target: None, hits: vec![], heal: None, knocked: None, note: Some(Note::NoTarget), entered: vec![] }),
            LogEvent::End { rounds: 3, down: [false, true] },
            LogEvent::Heal { unit: 0, amount: 1200, source: HealSource::Medic { b: Some(6), unit_type: UnitType(9), adjacent: false } },
            LogEvent::Heal { unit: 0, amount: 2400, source: HealSource::Medic { b: None, unit_type: UnitType(9), adjacent: true } },
            LogEvent::Heal { unit: 0, amount: 1000, source: HealSource::OnKill },
            LogEvent::Heal { unit: 0, amount: 500, source: HealSource::Spell(SpellType(6)) },
            LogEvent::Xp { unit: 0, amount: 4, from: vec![XpSource::Kill { victim: 1, share_permille: 750 }, XpSource::Withdrew, XpSource::DefStrike] },
            LogEvent::Flee { unit: 6, to: PlotIdx(12) },
            LogEvent::MoveIn { units: vec![5], city: Some(CityId(3)) },
        ]);
        for kind in ActionKind::ALL {
            events.push(LogEvent::Action(Event { round: 1, pass: 1, actor: 3, kind, spell: None, target: Some(0), hits: vec![], heal: None, knocked: None, note: None, entered: vec![] }));
        }
        for kind in [
            KillKind::Capture(UnitType(1)),
            KillKind::Enslave(UnitType(2)),
            KillKind::Convert(UnitType(3)),
            KillKind::Create(UnitType(4)),
            KillKind::Gold(-15),
            KillKind::Afflict(PromotionType(5)),
            KillKind::Promotion(PromotionType(6)),
        ] {
            events.push(LogEvent::KillEffect { killer: 0, victim: 1, kind });
        }
        let log = BattleLog {
            slots: [slots, SideSlots { base: 8, changes: vec![], total: 8, river_only: 0 }],
            lanes: [4, 3],
            ground: Ground { city: true, forest: false, hills: true },
            rules: vec![
                GroundRule::CityShots { pct: 50 },
                GroundRule::ForestRanged { pct: 50 },
                GroundRule::HillsAttacker { pct: 75 },
                GroundRule::HillsDefenderRanged { pct: 125 },
                GroundRule::Charge { pct: 200 },
                GroundRule::NoFlanking,
            ],
            ordered: [true, false],
            tactics: [BattleTacticType(1), BattleTacticType(3)],
            fighters: vec![
                fighter(Side::Attacker, Some((0, 0)), Some(LogFate::Withdrew { round: 2 })),
                fighter(Side::Defender, Some((1, 0)), Some(LogFate::Fell { by: 0, round: 1 })),
                fighter(Side::Attacker, Some((1, 1)), Some(LogFate::SatOut)),
                fighter(Side::Attacker, Some((2, 2)), Some(LogFate::Standing)),
                fighter(Side::Defender, None, None),
            ],
            pairs: vec![LogPair { h: 0, t: 1, a_x100: 440, dv_x100: 660, mean: 800, ma_x100: 250, md_x100: 350, spell_pct: 75, magic_mean: 1666, a_extra: terms, dv_extra: vec![] }],
            events,
            forecast: Some(FrozenForecast {
                runs: 200,
                wiped_permille: 910,
                falls_permille: 640,
                repulsed_permille: 3,
                lost_x100: [130, 420],
                per: vec![ForecastFighter { b: 0, fall_pct: 75, escape_pct: 10 }, ForecastFighter { b: 2, fall_pct: 0, escape_pct: 0 }],
            }),
        };
        let side = |player: u8, shown: u8| ReportSide {
            player: PlayerId(player),
            shown_player: PlayerId(shown),
            players: vec![PlayerId(player)],
            slots: 8,
            lanes: 4,
            used: 7,
            committed: 6,
            sat_out: 1,
            fallen: 3,
            escaped: 1,
            standing: 1,
            lost: vec![(UnitType(2), 2), (UnitType(7), 1)],
            summons: 1,
            shore: vec![(UnitType(3), 1250)],
        };
        BattleReport {
            id: 41,
            turn: 120,
            plot: PlotIdx(1234),
            from: PlotIdx(1235),
            place: PlotSnap {
                x: 34,
                y: 20,
                terrain: Some(TerrainType(2)),
                feature: None,
                hills: true,
                river_crossing: true,
                city: Some(CitySnap { id: CityId(3), name: "Strages".to_string(), owner: PlayerId(2), size: 7, defense_pct: 60 }),
            },
            sides: [side(1, 8), side(2, 2)],
            outcome: Outcome { result: EngageResult::Taken, rounds: 3, plot_taken: true, city_captured: Some(CityId(3)), moved_in: 1, defenders_left: 0, sentinels: 2, sentinels_breached: 1 },
            log: Some(Box::new(log)),
        }
    }

    #[test]
    fn every_report_variant_round_trips() {
        let r = every_variant();
        round_trip(&r);
        for result in [EngageResult::Taken, EngageResult::Cleared, EngageResult::Repulsed, EngageResult::Held] {
            round_trip(&result);
        }
        // A summary without its log is a few dozen bytes.
        let summary = BattleReport { log: None, ..r };
        let n = round_trip(&summary);
        assert!(n < 100, "{n} bytes");
    }

    #[test]
    fn bad_tags_fail_cleanly() {
        assert!(Side::read(&mut Reader::new(&[2])).is_err());
        assert!(LogEvent::read(&mut Reader::new(&[200])).is_err());
        let mut w = Writer::new();
        every_variant().write(&mut w);
        let bytes = w.into_bytes();
        for n in (0..bytes.len()).step_by(7) {
            assert!(BattleReport::read(&mut Reader::new(&bytes[..n])).is_err());
        }
    }

    #[test]
    fn report_views_by_side() {
        let r = every_variant();
        assert_eq!(r.side_of(PlayerId(1)), Some(Side::Attacker));
        assert_eq!(r.side_of(PlayerId(2)), Some(Side::Defender));
        assert!(!r.involves(PlayerId(3)));
        // The attacker looked like the barbarians (hidden nationality): so it stays to the defender
        // and to anyone else, and its own side sees its real owner.
        let a = &r.sides[0];
        assert_eq!(r.owner_for(PlayerId(2), Side::Attacker, a.player, a.shown_player), PlayerId(8));
        assert_eq!(r.owner_for(PlayerId(3), Side::Attacker, a.player, a.shown_player), PlayerId(8));
        assert_eq!(r.owner_for(PlayerId(1), Side::Attacker, a.player, a.shown_player), PlayerId(1));
    }
}
