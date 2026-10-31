//! [`Persist`](super::Persist) for engagement combat's saved state: the battle reports and their
//! logs (`engagement/report.rs`) and the round model's result types they hold (`Side`,
//! `EngageResult`, `Phase`, `Fate`). Structs are listed field by field in declaration order and
//! enums variant by variant with fixed tags (`persist_struct!`, `persist_enum!`), so a new field
//! or variant fails to compile until it is listed here (and `FORMAT_VERSION` is bumped).

use super::io::{persist_enum, persist_newtype, persist_struct};

use crate::engagement::report::*;
use crate::engagement::{EngageResult, Fate, Phase, Side};

persist_enum!(Side { 0 => Attacker, 1 => Defender });
persist_enum!(EngageResult { 0 => AttackerWon, 1 => DefenderHeld, 2 => Draw, 3 => Mutual });
persist_enum!(Phase { 0 => Opening, 1 => Volley(k), 2 => Round(n) });
persist_enum!(Fate { 0 => Survived, 1 => Killed { by, phase }, 2 => Withdrew { phase, capped }, 3 => SatOut });

persist_struct!(BattleReport { id, turn, plot, from, place, sides, outcome, log });
persist_struct!(ReportSide { player, shown_player, players, army, garrison_of, stance, committed, withdrew, sat_out, fled, lost });
persist_struct!(PlotSnap { x, y, terrain, feature, hills, river_crossing, city });
persist_struct!(CitySnap { id, name, owner, size, defense_pct });
persist_struct!(Outcome { result, volley_rounds, rounds, plot_taken, city_captured, moved_in, defenders_left });

persist_struct!(BattleLog { slots, picks, fighters, pairs, events, forecast });
persist_struct!(SideSlots { base, changes, total, river_only });
persist_struct!(SlotChange { src, delta });
persist_enum!(SlotSource {
    0 => Terrain(t),
    1 => Feature(f),
    2 => Hills,
    3 => River,
    4 => CityDefense { pct, next_at },
    5 => Bombarded { pct },
    6 => Minimum,
    7 => Summons(n),
});
persist_struct!(SidePick { auto_pick, picked_by });
persist_enum!(Pick { 0 => Auto, 1 => Player { swaps } });

persist_struct!(LogFighter {
    unit, unit_type, name, owner, shown_owner, side, slot, level, xp, hero, role, pos, screen,
    ignores_shield, assassin_profile, guardsman, max_hp, hp_start, volleys, volley_chance,
    withdraw_pct, def_strike, collateral, combat_limit, immune, fear, a, dv, fate,
});
persist_enum!(Position { 0 => Front, 1 => Back });
persist_newtype!(Immunities);
persist_struct!(Stat { base_x100, role_pct, terms, total_x100 });
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
});
persist_struct!(LogPair { h, t, a_x100, dv_x100, mean, a_extra, dv_extra });

persist_enum!(Stage { 0 => PreBattle, 1 => Morale, 2 => Opening, 3 => Volley(k), 4 => Round(n), 5 => Post });
persist_enum!(LogEvent {
    0 => Stage { stage },
    1 => Cast { caster, spell, targets, effect },
    2 => Morale { unit, source, chance, roll, passed },
    3 => SatOut { unit, why },
    4 => VolleyRoll { unit, chance, roll, total },
    5 => DefStrike { striker, target, rolled, chance, roll, pct, dmg },
    6 => Collateral { source, target, pct, limit_pct, protection_pct, dmg },
    7 => Hit { h, t, rule, step, blocked, eligible, r, dmg, capped },
    8 => RoundEnd { deaths },
    9 => Withdraw { unit, hp_pct, chance, roll, ok, fatal, hp_kept },
    10 => ShieldDown { side },
    11 => End { result, volley_rounds, rounds },
    12 => Heal { unit, amount, source },
    13 => Xp { unit, amount, from },
    14 => KillEffect { killer, victim, kind },
    15 => Flee { unit, to },
    16 => Rout { units, to },
    17 => MoveIn { units, city },
});
persist_struct!(LogDeath { b, killer, assists, overkill });
persist_enum!(TargetRule {
    0 => FrontFirst,
    1 => FrontRankDown,
    2 => FlankerScreensFirst,
    3 => FlankerAround,
    4 => FlankerFrontLast,
    5 => AssassinSupportFirst,
    6 => AssassinBack,
    7 => AssassinFrontLast,
    8 => BodyguardFirst,
});
persist_enum!(Block { 0 => Empty, 1 => Shielded, 2 => VolleyImmune, 3 => AtLimit });
persist_enum!(CastEffect { 0 => Mod { a_pct, dv_pct }, 1 => Damage(hits), 2 => Summon(b) });
persist_enum!(SatOutWhy { 0 => Afraid, 1 => EveryTargetCapped });
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

persist_struct!(FrozenForecast { source, win_permille, draw_permille, rounds, losses_if_win_x10, per, curve, alternatives });
persist_enum!(ForecastSource { 0 => Sampled { runs }, 1 => Expected });
persist_struct!(ForecastFighter { b, fall_pct, withdraw_pct, death_round });
persist_enum!(AltKind { 0 => ThisPlan, 1 => StancePick, 2 => Swap { out, add }, 3 => Stance(s) });

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
            role_pct: 100,
            terms: vec![Term { src: TermSource::Promotion(PromotionType(3)), pct: 20 }, Term { src: TermSource::Fortify, pct: 25 }],
            total_x100: base * 145 / 100,
        }
    }

    fn fighter(side: Side, slot: Option<i32>, fate: Option<Fate>) -> LogFighter {
        LogFighter {
            unit: UnitId(17),
            unit_type: UnitType(5),
            name: String::new(),
            owner: PlayerId(1),
            shown_owner: PlayerId(8),
            side,
            slot,
            level: 2,
            xp: 7,
            hero: false,
            role: CombatRoleType(0),
            pos: Position::Back,
            screen: false,
            ignores_shield: true,
            assassin_profile: false,
            guardsman: false,
            max_hp: 120,
            hp_start: 9000,
            volleys: 1,
            volley_chance: 2,
            withdraw_pct: 30,
            def_strike: Some((20, 10)),
            collateral: Some((50, 25, 3)),
            combat_limit: 100,
            immune: Immunities::default().with(Immunities::FEAR, true).with(Immunities::COLLATERAL, true),
            fear: true,
            a: stat(400),
            dv: stat(300),
            fate,
        }
    }

    /// A report whose log holds every event, slot source, term source, rule and kill kind.
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
                SlotChange { src: SlotSource::CityDefense { pct: 60, next_at: 75 }, delta: -2 },
                SlotChange { src: SlotSource::Bombarded { pct: 20 }, delta: 1 },
                SlotChange { src: SlotSource::Minimum, delta: 1 },
                SlotChange { src: SlotSource::Summons(2), delta: 2 },
            ],
            total: 4,
            river_only: 2,
        };
        let stages = [Stage::PreBattle, Stage::Morale, Stage::Opening, Stage::Volley(1), Stage::Round(3), Stage::Post];
        let mut events: Vec<LogEvent> = stages.into_iter().map(|stage| LogEvent::Stage { stage }).collect();
        events.extend([
            LogEvent::Cast { caster: 0, spell: SpellType(4), targets: vec![1, 2], effect: CastEffect::Mod { a_pct: 10, dv_pct: -5 } },
            LogEvent::Cast { caster: 0, spell: SpellType(4), targets: vec![1], effect: CastEffect::Damage(vec![(1, 1200), (2, 600)]) },
            LogEvent::Cast { caster: 0, spell: SpellType(5), targets: vec![], effect: CastEffect::Summon(3) },
            LogEvent::Morale { unit: 1, source: 0, chance: 12, roll: 30, passed: true },
            LogEvent::SatOut { unit: 2, why: SatOutWhy::Afraid },
            LogEvent::SatOut { unit: 3, why: SatOutWhy::EveryTargetCapped },
            LogEvent::VolleyRoll { unit: 0, chance: 2, roll: 1, total: 2 },
            LogEvent::DefStrike { striker: 1, target: 0, rolled: 2, chance: 20, roll: 7, pct: 10, dmg: 1000 },
            LogEvent::Collateral { source: 0, target: 1, pct: 50, limit_pct: 25, protection_pct: 0, dmg: 3000 },
            LogEvent::Hit {
                h: 0,
                t: 1,
                rule: TargetRule::FlankerAround,
                step: 1,
                blocked: vec![(0, Block::Empty), (1, Block::Shielded), (2, Block::VolleyImmune), (3, Block::AtLimit)],
                eligible: vec![1, 4],
                r: 117,
                dmg: 1170,
                capped: false,
            },
            LogEvent::RoundEnd {
                deaths: vec![LogDeath { b: 1, killer: Some(0), assists: vec![4], overkill: 240 }, LogDeath { b: 4, killer: None, assists: vec![], overkill: 0 }],
            },
            LogEvent::Withdraw { unit: 0, hp_pct: 30, chance: 45, roll: 12, ok: true, fatal: true, hp_kept: Some(4000) },
            LogEvent::Withdraw { unit: 5, hp_pct: 20, chance: 25, roll: 77, ok: false, fatal: false, hp_kept: None },
            LogEvent::ShieldDown { side: Side::Defender },
            LogEvent::End { result: EngageResult::Mutual, volley_rounds: 2, rounds: 14 },
            LogEvent::Heal { unit: 0, amount: 1200, source: HealSource::Medic { b: Some(6), unit_type: UnitType(9), adjacent: false } },
            LogEvent::Heal { unit: 0, amount: 2400, source: HealSource::Medic { b: None, unit_type: UnitType(9), adjacent: true } },
            LogEvent::Heal { unit: 0, amount: 1000, source: HealSource::OnKill },
            LogEvent::Heal { unit: 0, amount: 500, source: HealSource::Spell(SpellType(6)) },
            LogEvent::Xp { unit: 0, amount: 4, from: vec![XpSource::Kill { victim: 1, share_permille: 750 }, XpSource::Withdrew, XpSource::DefStrike] },
            LogEvent::Flee { unit: 6, to: PlotIdx(12) },
            LogEvent::Rout { units: vec![6], to: vec![PlotIdx(13)] },
            LogEvent::MoveIn { units: vec![5], city: Some(CityId(3)) },
        ]);
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
        for rule in [
            TargetRule::FrontFirst,
            TargetRule::FrontRankDown,
            TargetRule::FlankerScreensFirst,
            TargetRule::FlankerFrontLast,
            TargetRule::AssassinSupportFirst,
            TargetRule::AssassinBack,
            TargetRule::AssassinFrontLast,
            TargetRule::BodyguardFirst,
        ] {
            events.push(LogEvent::Hit { h: 1, t: 0, rule, step: 0, blocked: vec![], eligible: vec![0], r: 80, dmg: 1, capped: true });
        }
        let log = BattleLog {
            slots: [slots, SideSlots { base: 8, changes: vec![], total: 8, river_only: 0 }],
            picks: [
                SidePick { auto_pick: vec![0, 2], picked_by: Pick::Player { swaps: vec![(2, 3)] } },
                SidePick { auto_pick: vec![1], picked_by: Pick::Auto },
            ],
            fighters: vec![
                fighter(Side::Attacker, Some(0), Some(Fate::Withdrew { phase: Phase::Round(11), capped: false })),
                fighter(Side::Defender, Some(0), Some(Fate::Killed { by: Some(0), phase: Phase::Volley(1) })),
                fighter(Side::Attacker, Some(1), Some(Fate::SatOut)),
                fighter(Side::Attacker, Some(2), Some(Fate::Withdrew { phase: Phase::Opening, capped: true })),
                fighter(Side::Defender, Some(1), Some(Fate::Killed { by: None, phase: Phase::Opening })),
                fighter(Side::Attacker, Some(3), Some(Fate::Survived)),
                fighter(Side::Defender, None, None),
            ],
            pairs: vec![LogPair { h: 0, t: 1, a_x100: 440, dv_x100: 660, mean: 800, a_extra: terms, dv_extra: vec![] }],
            events,
            forecast: Some(FrozenForecast {
                source: ForecastSource::Sampled { runs: 200 },
                win_permille: 910,
                draw_permille: 3,
                rounds: (13, 16, 20),
                losses_if_win_x10: 29,
                per: vec![
                    ForecastFighter { b: 0, fall_pct: 75, withdraw_pct: 10, death_round: Some(8) },
                    ForecastFighter { b: 2, fall_pct: 0, withdraw_pct: 0, death_round: None },
                ],
                curve: [vec![60000, 52000, 41000], vec![48000, 39000, 30000]],
                alternatives: vec![(AltKind::ThisPlan, 910), (AltKind::StancePick, 880), (AltKind::Swap { out: 2, add: 3 }, 870), (AltKind::Stance(StanceType(3)), 700)],
            }),
        };
        let side = |player: u8, shown: u8, garrison: Option<CityId>| ReportSide {
            player: PlayerId(player),
            shown_player: PlayerId(shown),
            players: vec![PlayerId(player)],
            army: garrison.is_none().then_some(ArmyId(4)),
            garrison_of: garrison,
            stance: StanceType(1),
            committed: 6,
            withdrew: 1,
            sat_out: 1,
            fled: 0,
            lost: vec![(UnitType(2), 2), (UnitType(7), 1)],
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
            sides: [side(1, 8, None), side(2, 2, Some(CityId(3)))],
            outcome: Outcome {
                result: EngageResult::AttackerWon,
                volley_rounds: 2,
                rounds: 14,
                plot_taken: true,
                city_captured: Some(CityId(3)),
                moved_in: 1,
                defenders_left: 0,
            },
            log: Some(Box::new(log)),
        }
    }

    #[test]
    fn every_report_variant_round_trips() {
        let r = every_variant();
        round_trip(&r);
        for result in [EngageResult::AttackerWon, EngageResult::DefenderHeld, EngageResult::Draw, EngageResult::Mutual] {
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
