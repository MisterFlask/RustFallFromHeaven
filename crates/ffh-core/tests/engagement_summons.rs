//! A caster's summons in its battles (docs/ENGAGEMENTS.md, "Summons"): a temporary summon spell is
//! never cast on the map; its unit fights in each of its caster's battles, on either side, beside
//! the caster, within the side's extra summon slots, and is gone once the battle is over; Twincast
//! calls two; the preview shows the summons and changes nothing.
//!
//! Every test asserts what holds in every battle (no seed's story).

use ffh_core::defs::MOVE_DIRECT_ATTACK;
use ffh_core::engagement::report::BattleReport;
use ffh_core::engagement::{PreviewInput, Side};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::setup::GameSetup;
use ffh_data::*;

/// A duel map, players 0 and 1 at war, `from` and `to` two unowned flat grassland plots side by
/// side with no feature, improvement or river crossing.
struct World {
    g: Game,
    p0: PlayerId,
    p1: PlayerId,
    from: PlotIdx,
    to: PlotIdx,
}

fn world() -> World {
    let d = ffh_data::global();
    let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
    g.game_init();
    let flat = |g: &Game, p: PlotIdx| {
        let plot = &g.map.plots[p];
        !g.plot_is_water(p) && !g.plot_is_peak(p) && !g.plot_is_hills(p) && plot.units.is_empty() && plot.owner.is_none() && plot.feature_type.is_none() && plot.improvement_type.is_none()
    };
    let (from, to) = g
        .map
        .plot_ids()
        .filter(|&p| flat(&g, p))
        .find_map(|p| {
            let (x, y) = g.map.xy(p);
            let q = g.map.plot_xy(x + 1, y)?;
            (flat(&g, q) && !g.plot_is_river_crossing(p, Some(DirectionType::DIRECTION_EAST))).then_some((p, q))
        })
        .expect("two flat plots side by side");
    let grass = d.t::<TerrainType>("TERRAIN_GRASS");
    g.map.plots[from].terrain_type = Some(grass);
    g.map.plots[to].terrain_type = Some(grass);
    let (p0, p1) = (PlayerId(0), PlayerId(1));
    let (t0, t1) = (g.players[p0].team, g.players[p1].team);
    g.team_declare_war(t0, t1, false, None);
    // The battle log is written for a human's battle.
    g.players[p1].human = true;
    World { g, p0, p1, from, to }
}

impl World {
    fn unit(&mut self, owner: PlayerId, name: &str, at: PlotIdx) -> UnitId {
        let (x, y) = self.g.map.xy(at);
        let ty = self.g.d.t::<UnitType>(name);
        let u = self.g.player_init_unit(owner, ty, x, y, None, None);
        self.g.player_army_sync(owner);
        u
    }

    fn promote(&mut self, u: UnitId, names: &[&str]) {
        for name in names {
            let p = self.g.d.t::<PromotionType>(name);
            self.g.unit_set_has_promotion(u, p, true);
        }
    }

    /// `u`'s group attacks `to`; the reports it added.
    fn attack(&mut self, u: UnitId) -> Vec<BattleReport> {
        let before = self.g.battle_report_next;
        let grp = self.g.units[u].group.expect("a group");
        let (x, y) = self.g.map.xy(self.to);
        let (attacked, _) = self.g.group_attack(grp, x, y, MOVE_DIRECT_ATTACK);
        assert!(attacked);
        self.g.battle_reports.iter().filter(|r| r.id >= before).cloned().collect()
    }

    /// Units of type `name` in the game.
    fn count(&self, name: &str) -> usize {
        let ty = self.g.d.t::<UnitType>(name);
        self.g.units.iter().filter(|(_, u)| u.unit_type == ty).count()
    }
}

/// The fighters of `r`'s log of type `name` on side `side`.
fn fighters(g: &Game, r: &BattleReport, name: &str, side: Side) -> usize {
    let ty = g.d.t::<UnitType>(name);
    r.log.as_ref().expect("a logged battle").fighters.iter().filter(|f| f.unit_type == ty && f.side == side && f.place.is_some()).count()
}

#[test]
fn a_summon_spell_is_never_cast_on_the_map() {
    let d = ffh_data::global();
    let summons: Vec<&str> = d.spells.iter().filter(|s| s.battle_summon).map(|s| s.base.type_.as_str()).collect();
    for name in ["SPELL_SUMMON_AIR_ELEMENTAL", "SPELL_SUMMON_EINHERJAR", "SPELL_SUMMON_SPECTRE", "SPELL_CREATE_PUPPET", "SPELL_SUMMON_EARTH_ELEMENTAL"] {
        assert!(summons.contains(&name), "{name} is a battle summon: {summons:?}");
    }
    for name in ["SPELL_RAISE_SKELETON", "SPELL_SUMMON_TREANT", "SPELL_BREATH_FIRE", "SPELL_FLOATING_EYE", "SPELL_SEVER", "SPELL_SUMMON_KRAKEN"] {
        assert!(!summons.contains(&name), "{name} is not a battle summon");
    }
    let mut w = world();
    let (p0, from) = (w.p0, w.from);
    let mage = w.unit(p0, "UNIT_MAGE", from);
    w.promote(mage, &["PROMOTION_AIR1", "PROMOTION_AIR2", "PROMOTION_AIR3"]);
    let spell = d.t::<SpellType>("SPELL_SUMMON_AIR_ELEMENTAL");
    assert!(!w.g.unit_can_cast(mage, spell, false), "a battle summon is not cast on the map");
    assert_eq!(w.g.unit_battle_summons(mage), vec![spell]);
}

#[test]
fn an_attacking_caster_calls_its_summon_and_it_is_gone_after() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let mage = w.unit(p0, "UNIT_MAGE", from);
    w.promote(mage, &["PROMOTION_AIR1", "PROMOTION_AIR2", "PROMOTION_AIR3"]);
    w.unit(p0, "UNIT_AXEMAN", from);
    w.unit(p1, "UNIT_WARRIOR", to);
    w.unit(p1, "UNIT_WARRIOR", to);
    let reports = w.attack(mage);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    assert_eq!(fighters(&w.g, r, "UNIT_AIR_ELEMENTAL", Side::Attacker), 1, "the summon fights beside its caster");
    assert_eq!(r.sides[0].committed, 3, "the mage, the axeman and the summon");
    assert_eq!(w.count("UNIT_AIR_ELEMENTAL"), 0, "the summon is the battle's only");
    assert!(w.g.units.iter().all(|(_, u)| !u.battle_summon), "no summon outlives its battle");
    // A summon shares no XP and is no defender: the plot falls on the real units' result alone.
    if r.outcome.plot_taken {
        assert_eq!(r.outcome.defenders_left, 0);
    }
}

#[test]
fn a_defending_caster_summons_and_its_summon_does_not_hold_the_plot() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let mage = w.unit(p1, "UNIT_MAGE", to);
    w.promote(mage, &["PROMOTION_EARTH1", "PROMOTION_EARTH2", "PROMOTION_EARTH3"]);
    let axemen: Vec<UnitId> = (0..6).map(|_| w.unit(p0, "UNIT_AXEMAN", from)).collect();
    let reports = w.attack(axemen[0]);
    let r = &reports[0];
    assert_eq!(fighters(&w.g, r, "UNIT_EARTH_ELEMENTAL", Side::Defender), 1, "the Earth Elemental, large: the side's two extra slots");
    assert_eq!(w.count("UNIT_EARTH_ELEMENTAL"), 0);
    let mage_alive = w.g.units.contains(mage) && !w.g.unit_is_dead(mage);
    assert_eq!(r.outcome.defenders_left, mage_alive as i32, "only the mage can be left: the summon leaves after the battle");
}

#[test]
fn twincast_calls_two_within_the_extra_slots() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let archmage = w.unit(p0, "UNIT_ARCHMAGE", from);
    w.promote(archmage, &["PROMOTION_FIRE1", "PROMOTION_FIRE2", "PROMOTION_FIRE3", "PROMOTION_AIR1", "PROMOTION_AIR2", "PROMOTION_AIR3", "PROMOTION_TWINCAST"]);
    let fire = w.g.d.t::<SpellType>("SPELL_SUMMON_FIRE_ELEMENTAL");
    assert_eq!(w.g.unit_battle_summons(archmage), vec![fire, fire], "its strongest summon, twice");
    // A second summoner on the side finds the two extra slots taken.
    let mage = w.unit(p0, "UNIT_MAGE", from);
    w.promote(mage, &["PROMOTION_WATER1", "PROMOTION_WATER2", "PROMOTION_WATER3"]);
    w.unit(p1, "UNIT_WARRIOR", to);
    let reports = w.attack(archmage);
    let r = &reports[0];
    let n_fire = fighters(&w.g, r, "UNIT_FIRE_ELEMENTAL", Side::Attacker);
    let n_water = fighters(&w.g, r, "UNIT_WATER_ELEMENTAL", Side::Attacker);
    assert_eq!(n_fire + n_water, 2, "two extra slots a side: {n_fire} fire, {n_water} water");
    assert_eq!(n_water, 1, "the strongest summons first: the Water Elemental, then a Fire Elemental");
    assert_eq!(w.count("UNIT_FIRE_ELEMENTAL") + w.count("UNIT_WATER_ELEMENTAL"), 0);
}

#[test]
fn the_preview_shows_the_summons_and_changes_nothing() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let mage = w.unit(p0, "UNIT_MAGE", from);
    w.promote(mage, &["PROMOTION_LAW1", "PROMOTION_LAW2"]);
    let defender = w.unit(p1, "UNIT_MAGE", to);
    w.promote(defender, &["PROMOTION_DEATH1", "PROMOTION_DEATH2"]);
    w.unit(p1, "UNIT_WARRIOR", to);
    let before = w.g.save();
    let next = w.g.units.next_id();
    let inp = PreviewInput::new(p0, from, to);
    let a = w.g.engage_preview(&inp).expect("a preview");
    let b = w.g.engage_preview(&inp).expect("a preview");
    assert_eq!(a, b, "the same plan, the same numbers");
    assert_eq!(w.g.save(), before, "the preview changes nothing");
    assert_eq!(w.g.units.next_id(), next);
    let types: Vec<(String, Side)> = a.summons.iter().map(|s| (w.g.d.units[s.unit_type].base.type_.clone(), s.side)).collect();
    assert!(types.contains(&("UNIT_EINHERJAR".to_string(), Side::Attacker)), "{types:?}");
    assert!(types.contains(&("UNIT_SPECTRE".to_string(), Side::Defender)), "{types:?}");
    assert!(a.summons.iter().all(|s| s.max_hp > 0 && s.hp > 0));
    assert!(a.forecast.per.iter().all(|x| w.g.units.contains(x.unit)), "the forecast's fighters are the game's units");
    // The real fight calls the summons the plan showed.
    let reports = w.attack(mage);
    let r = &reports[0];
    assert_eq!(fighters(&w.g, r, "UNIT_EINHERJAR", Side::Attacker), 1);
    assert_eq!(fighters(&w.g, r, "UNIT_SPECTRE", Side::Defender), 1);
}

/// The kills of fodder (alive, capturable) by fighter `by` in `r`'s log.
fn kills_by(r: &BattleReport, by: UnitId) -> usize {
    use ffh_core::engagement::report::LogFate;
    let log = r.log.as_ref().expect("a logged battle");
    let k = log.fighters.iter().position(|f| f.unit == by).expect("the killer fought");
    log.fighters.iter().filter(|f| matches!(f.fate, Some(LogFate::Fell { by, .. }) if by as usize == k)).count()
}

#[test]
fn a_unit_a_kill_creates_fights_on_and_stays_when_a_unit_of_the_map_made_it() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    // Ars makes a Wraith of each living unit he kills (100%).
    let ars = w.unit(p0, "UNIT_ARS", from);
    for _ in 0..3 {
        let v = w.unit(p1, "UNIT_WARRIOR", to);
        w.g.units[v].damage = 90;
    }
    let before = w.count("UNIT_WRAITH");
    let reports = w.attack(ars);
    let r = &reports[0];
    let kills = kills_by(r, ars);
    assert_eq!(w.count("UNIT_WRAITH") - before, kills, "one Wraith a kill, in the battle or (no room) after it");
    let entered = r.log.as_ref().unwrap().events.iter().filter(|e| matches!(e, ffh_core::engagement::report::LogEvent::Action(a) if !a.entered.is_empty())).count();
    assert!(entered <= kills.min(2));
    assert!(w.g.units.iter().all(|(_, u)| !u.battle_summon), "the Wraiths stay as units of the map; the unused ones are gone");
    assert_eq!(r.sides[0].committed as usize, 1 + entered, "only the Wraiths that entered fought");
}

#[test]
fn a_unit_a_summon_creates_is_the_battle_s_only() {
    let mut w = world();
    let (p0, p1, from, to) = (w.p0, w.p1, w.from, w.to);
    let mage = w.unit(p0, "UNIT_MAGE", from);
    w.promote(mage, &["PROMOTION_AIR1", "PROMOTION_AIR2", "PROMOTION_AIR3"]);
    for _ in 0..3 {
        let v = w.unit(p1, "UNIT_WARRIOR", to);
        w.g.units[v].damage = 90;
    }
    w.attack(mage);
    assert_eq!(w.count("UNIT_AIR_ELEMENTAL"), 0);
    assert_eq!(w.count("UNIT_LIGHTNING_ELEMENTAL"), 0, "the Air Elemental's Lightning Elementals leave with it");
    assert!(w.g.units.iter().all(|(_, u)| !u.battle_summon));
}
