//! Bridges Cataclysm's 3x3 Squads + City Sentinels + Ground + Mana Battle Spells into the shared
//! pure `ffh-battle` formation resolver, producing detailed battle records and attack forecasts.

use ffh_battle::{
    self, ActionKind, Event as BattleEvent, Fate, Fighter, Formation, Ground, GroundRule, Outcome, Pair, Params, RowActions, Side, Spell, ROWS,
};

use crate::city::City;
use crate::data::{CaptainAura, UnitClassId};
use crate::ids::{BattleId, MemberId, PlayerId, PlotIdx, SquadId};
use crate::rng::Rng;
use crate::squad::{Squad, SquadMember};
use crate::supply::supply_combat_pct;
use crate::world::{PlotKind, Terrain, World};

/// Side-level combat modifiers aggregated from Faction, Techs, Secret Techs, Religion, Captain Aura,
/// Mana pool, and Supply status.
#[derive(Clone, Debug, Default)]
pub struct SideMods {
    pub atk_pct: i32,
    pub def_pct: i32,
    pub spell_pct: i32,
    pub heal_pct: i32,
    pub armor_bonus: i32,
    pub accuracy_bonus: i32,
    pub evasion_bonus: i32,
    pub crit_bonus: i32,
    pub speed_bonus: i32,
    pub charge_bonus_pct: i32,
    pub bombard_pct: i32,
    pub sentinel_hp_pct: i32,
    pub pursuit: bool,
    pub forest_ranged_immune: bool,
    pub ranged_pierce: bool,
    pub heal_revive_pct: i32,
    pub has_mana_boost: bool,
    pub hell_ground_bonus: bool,
    pub supply_pct: i32,
    pub front_row_atk_pct: i32,
    pub front_row_def_pct: i32,
}

#[derive(Clone, Debug)]
pub struct FighterSnap {
    pub index: usize,
    pub side: Side,
    pub owner: PlayerId,
    pub member_id: Option<MemberId>,
    pub sentinel_idx: Option<usize>,
    pub class: UnitClassId,
    pub name: String,
    pub level: i32,
    pub is_hero: bool,
    pub large: bool,
    pub lane: u8,
    pub row: u8,
    pub max_hp: i32,
    pub hp_start: i32,
    pub hp_end: i32,
    pub a_x100: i32,
    pub dv_x100: i32,
    pub ma_x100: i32,
    pub md_x100: i32,
    pub speed: i32,
    pub accuracy: i32,
    pub evasion: i32,
    pub crit: i32,
    pub anti_flank: bool,
    pub actions: [RowActions; ROWS],
    pub fate: Fate,
}

#[derive(Clone, Debug)]
pub struct BattleRecord {
    pub id: BattleId,
    pub turn: i32,
    pub from: PlotIdx,
    pub to: PlotIdx,
    pub place_name: Option<String>,
    pub is_city: bool,
    pub attacker: PlayerId,
    pub defender: PlayerId,
    pub attacker_label: String,
    pub defender_label: String,
    pub attacker_squad: SquadId,
    pub defender_squad: Option<SquadId>,
    pub ground: Ground,
    pub ground_rules: Vec<GroundRule>,
    pub fighters: Vec<FighterSnap>,
    pub events: Vec<BattleEvent>,
    pub rounds: i32,
    pub loser: Option<Side>,
    pub decided_by: &'static str,
    pub verdict_calc: Vec<String>,
    pub hp_lost: [i32; 2],
    pub plot_taken: bool,
    pub city_captured: bool,
    pub defenders_left: usize,
}

#[derive(Clone, Debug)]
pub struct Forecast {
    pub runs: i32,
    pub plot_falls_permille: i32,
    pub line_wiped_permille: i32,
    pub attackers_wiped_permille: i32,
    pub expected_lost: [i32; 2], // x100
    pub per_fall_pct: Vec<i32>,
}

pub fn ground_for(world: &World, from: PlotIdx, to: PlotIdx, is_city: bool) -> Ground {
    let pl = world.plot(to);
    let forest = pl.feature.is_woods();
    let hills = pl.kind == PlotKind::Hills;
    let marsh = pl.terrain == Terrain::Marsh;
    let open = !is_city && !forest && !hills && !marsh;
    let crossing = world.plot(from).river_bits != 0 && world.plot(to).river_bits != 0;
    Ground { city: is_city, forest, hills, open, marsh, crossing, trench_pct: 0 }
}

pub fn default_params(ac: i32) -> Params {
    let mut p = Params::default();
    if ac >= 80 {
        // Crisis "The Sky Bleeds": all battles deal +15% damage.
        p.base_damage = 46;
    }
    p
}

fn member_stats(
    m: &SquadMember,
    mods: &SideMods,
    aura: CaptainAura,
    is_defender_in_city: bool,
    city_def_pct: i32,
    on_hell: bool,
) -> (i32, i32, i32, i32, Fighter) {
    let info = m.class.info();
    let lvl_pct = (m.level - 1).max(0) * 8;
    let hero_pct = if m.is_hero() { 20 } else { 0 };
    let wound_pen = m.wounds * 15;

    let mut atk_pct = 100 + mods.atk_pct + lvl_pct + hero_pct - wound_pen;
    let mut def_pct = 100 + mods.def_pct + lvl_pct + hero_pct - wound_pen;
    let mut mag_pct = 100 + mods.spell_pct + lvl_pct + hero_pct - wound_pen;
    let mut md_pct = 100 + mods.def_pct + lvl_pct + hero_pct - wound_pen;

    match aura {
        CaptainAura::Vanguard => atk_pct += 12,
        CaptainAura::Bulwark => {
            def_pct += 12;
            md_pct += 12;
        }
        CaptainAura::ArcaneFocus => mag_pct += 15,
        _ => {}
    }
    if m.row == 0 {
        atk_pct += mods.front_row_atk_pct;
        def_pct += mods.front_row_def_pct;
    }
    if on_hell && mods.hell_ground_bonus {
        atk_pct += 15;
        def_pct += 15;
    }
    if is_defender_in_city {
        def_pct += city_def_pct / 2;
        md_pct += city_def_pct / 3;
    }
    if mods.supply_pct > 0 && mods.supply_pct != 100 {
        atk_pct = atk_pct * mods.supply_pct / 100;
        mag_pct = mag_pct * mods.supply_pct / 100;
    }

    let a_x100 = (info.atk * 10 * atk_pct.max(25) / 100).max(50);
    let dv_x100 = (info.def * 10 * def_pct.max(25) / 100).max(50);
    let ma_x100 = if info.magic > 0 { (info.magic * 10 * mag_pct.max(25) / 100).max(50) } else { a_x100 };
    let md_x100 = (info.mdef * 10 * md_pct.max(25) / 100).max(50);

    let mut f = Fighter::new(Side::Attacker, m.max_hp());
    f.lane = m.lane;
    f.row = m.row;
    f.large = info.large;
    f.hp = m.hp_centi.clamp(1, m.max_hp() * 100);
    f.a = a_x100;
    f.speed = info.speed + mods.speed_bonus + if aura == CaptainAura::SwiftMarch { 1 } else { 0 };
    f.accuracy = info.acc + mods.accuracy_bonus + if aura == CaptainAura::Skirmisher { 10 } else { 0 };
    f.evasion = info.eva + mods.evasion_bonus + if aura == CaptainAura::Skirmisher { 8 } else { 0 };
    f.crit = info.crit + mods.crit_bonus;
    f.anti_flank = info.anti_flank;
    f.flanker = info.rows.iter().any(|r| r.kind == ActionKind::Charge);
    f.actions = info.rows;
    f.patterns = info.patterns;
    f.armor = (info.armor + mods.armor_bonus).max(0) * 100;
    f.pierce = info.armor_piercing || (mods.ranged_pierce && info.rows.iter().any(|r| matches!(r.kind, ActionKind::Shoot | ActionKind::Stab)));
    f.pursuit = mods.pursuit && f.flanker;
    f.heal_pct = Some(50 + mods.heal_pct + if aura == CaptainAura::ArcaneFocus { 15 } else { 0 });
    f.magic.revive = mods.heal_revive_pct;
    f.fodder = true;

    // Battle spells powered automatically by empire Mana pool (zero per-turn adept clicking!).
    if info.rows.iter().any(|r| r.kind == ActionKind::Spell) {
        let base_pow = if info.tier >= 4 {
            120
        } else if info.tier >= 2 {
            100
        } else {
            75
        };
        let mana_bonus = if mods.has_mana_boost { 20 } else { 0 };
        f.spell = Spell {
            id: None,
            power: base_pow + mana_bonus + mods.spell_pct / 2,
            splash: info.tier >= 2,
            pattern: None,
        };
    }
    // Round-1 charge bonus & bombard boost.
    if f.flanker && mods.charge_bonus_pct > 0 {
        f.round_pct[0] = 100 + mods.charge_bonus_pct;
    }
    if info.rows.iter().any(|r| r.kind == ActionKind::Bombard) && mods.bombard_pct > 0 {
        f.round_pct = [100 + mods.bombard_pct; ROWS];
    }

    (a_x100, dv_x100, ma_x100, md_x100, f)
}

pub struct BuiltFormation {
    pub formation: Formation,
    pub snaps: Vec<FighterSnap>,
}

/// Builds the `ffh_battle::Formation` for an attacking Squad against a defending Squad (optional)
/// and any standing Wall Sentinels of a defended City.
pub fn build_formation(
    world: &World,
    from: PlotIdx,
    to: PlotIdx,
    att_squad: &Squad,
    def_squad: Option<&Squad>,
    city: Option<&City>,
    att_mods: &SideMods,
    def_mods: &SideMods,
    ac: i32,
) -> BuiltFormation {
    let is_city = city.is_some();
    let ground = ground_for(world, from, to, is_city);
    let params = default_params(ac);
    let on_hell = world.plot(to).terrain.is_hell();
    let city_def_pct = city.map_or(0, City::defense_pct);

    let mut fighters = Vec::new();
    let mut stats: Vec<(i32, i32, i32, i32, UnitClassId)> = Vec::new();
    let mut snaps = Vec::new();

    // Attacker squad members
    for m in &att_squad.members {
        let mut att_m = att_mods.clone();
        att_m.supply_pct = supply_combat_pct(att_squad.unsupplied_turns);
        let (a, dv, ma, md, mut f) = member_stats(m, &att_m, att_squad.captain_aura, false, 0, on_hell);
        f.side = Side::Attacker;
        let idx = fighters.len();
        snaps.push(FighterSnap {
            index: idx,
            side: Side::Attacker,
            owner: att_squad.owner,
            member_id: Some(m.id),
            sentinel_idx: None,
            class: m.class,
            name: m.display_name().to_string(),
            level: m.level,
            is_hero: m.is_hero(),
            large: f.large,
            lane: f.lane,
            row: f.row,
            max_hp: f.max_hp,
            hp_start: f.hp,
            hp_end: f.hp,
            a_x100: a,
            dv_x100: dv,
            ma_x100: ma,
            md_x100: md,
            speed: f.speed,
            accuracy: f.accuracy,
            evasion: f.evasion,
            crit: f.crit,
            anti_flank: f.anti_flank,
            actions: f.actions,
            fate: Fate::Standing,
        });
        fighters.push(f);
        stats.push((a, dv, ma, md, m.class));
    }

    // Track occupied front cells on defender side so Sentinels stand in front and push units back cleanly.
    let mut def_occupied = [[false; 3]; 3];
    let sentinel_lanes: [u8; 3] = [1, 0, 2];

    if let Some(c) = city {
        let mut s_placed = 0usize;
        for (si, &dmg) in c.sentinel_damage.iter().enumerate() {
            if dmg >= 100 || s_placed >= 3 {
                continue;
            }
            let lane = sentinel_lanes[s_placed];
            s_placed += 1;
            let base = UnitClassId::Sentinel.info();
            let scale = 100 + city_def_pct + def_mods.sentinel_hp_pct;
            let max_hp = (base.hp * scale / 100).max(50);
            let hp = ((100 - dmg) * max_hp).max(1);
            let a = base.atk * 10;
            let dv = base.def * 10 * scale / 100;
            let md = base.mdef * 10 * scale / 100;
            let mut f = Fighter::new(Side::Defender, max_hp);
            f.lane = lane;
            f.row = 0;
            f.hp = hp;
            f.a = a;
            f.speed = 0;
            f.accuracy = base.acc;
            f.evasion = 0;
            f.crit = base.crit;
            f.anti_flank = true;
            f.fixed = true;
            f.armor = base.armor * 100;
            f.actions = base.rows;
            f.patterns = base.patterns;
            f.fodder = false;
            def_occupied[0][lane as usize] = true;
            let idx = fighters.len();
            snaps.push(FighterSnap {
                index: idx,
                side: Side::Defender,
                owner: c.owner,
                member_id: None,
                sentinel_idx: Some(si),
                class: UnitClassId::Sentinel,
                name: format!("{} Wall", c.name),
                level: 1,
                is_hero: false,
                large: false,
                lane,
                row: 0,
                max_hp,
                hp_start: hp,
                hp_end: hp,
                a_x100: a,
                dv_x100: dv,
                ma_x100: a,
                md_x100: md,
                speed: 0,
                accuracy: base.acc,
                evasion: 0,
                crit: base.crit,
                anti_flank: true,
                actions: f.actions,
                fate: Fate::Standing,
            });
            fighters.push(f);
            stats.push((a, dv, a, md, UnitClassId::Sentinel));
        }
    }

    if let Some(ds) = def_squad {
        for m in &ds.members {
            let mut def_m = def_mods.clone();
            def_m.supply_pct = supply_combat_pct(ds.unsupplied_turns);
            let (a, dv, ma, md, mut f) = member_stats(m, &def_m, ds.captain_aura, is_city, city_def_pct, on_hell);
            f.side = Side::Defender;
            // If Sentinels occupy this unit's cell, nudge it to the nearest free cell!
            let can_place = |r: u8, l: u8, large: bool, occ: &[[bool; 3]; 3]| -> bool {
                if r >= 3 || l >= 3 || (large && l >= 2) {
                    return false;
                }
                if large {
                    !occ[r as usize][l as usize] && !occ[r as usize][(l + 1) as usize]
                } else {
                    !occ[r as usize][l as usize]
                }
            };
            if !can_place(f.row, f.lane, f.large, &def_occupied) {
                let mut found = None;
                let row_try = [f.row, (f.row + 1) % 3, (f.row + 2) % 3];
                let lane_try: &[u8] = if f.large { &[0, 1] } else { &[1, 0, 2] };
                'find: for &r in &row_try {
                    for &l in lane_try {
                        if can_place(r, l, f.large, &def_occupied) {
                            found = Some((r, l));
                            break 'find;
                        }
                    }
                }
                if let Some((r, l)) = found {
                    f.row = r;
                    f.lane = l;
                }
            }
            if f.row < 3 && f.lane < 3 {
                def_occupied[f.row as usize][f.lane as usize] = true;
                if f.large && f.lane + 1 < 3 {
                    def_occupied[f.row as usize][(f.lane + 1) as usize] = true;
                }
            }
            let idx = fighters.len();
            snaps.push(FighterSnap {
                index: idx,
                side: Side::Defender,
                owner: ds.owner,
                member_id: Some(m.id),
                sentinel_idx: None,
                class: m.class,
                name: m.display_name().to_string(),
                level: m.level,
                is_hero: m.is_hero(),
                large: f.large,
                lane: f.lane,
                row: f.row,
                max_hp: f.max_hp,
                hp_start: f.hp,
                hp_end: f.hp,
                a_x100: a,
                dv_x100: dv,
                ma_x100: ma,
                md_x100: md,
                speed: f.speed,
                accuracy: f.accuracy,
                evasion: f.evasion,
                crit: f.crit,
                anti_flank: f.anti_flank,
                actions: f.actions,
                fate: Fate::Standing,
            });
            fighters.push(f);
            stats.push((a, dv, ma, md, m.class));
        }
    }

    let forest = world.plot(to).feature.is_woods();
    let formation = Formation::new(params, ground, [3, 3], fighters, |h, t| {
        let (a, _, ma, _, hc) = stats[h];
        let (_, dv, _, md, tc) = stats[t];
        let mut pct = 100;
        if tc == UnitClassId::Sentinel {
            pct += hc.info().damage_vs_walls;
        }
        // Ljosalfar Archers ignore forest ranged penalty by compensating its 50% modifier.
        if forest && matches!(hc, UnitClassId::Archer | UnitClassId::Longbowman) {
            let side_m = if snaps[h].side == Side::Attacker { att_mods } else { def_mods };
            if side_m.forest_ranged_immune {
                pct = pct * 2;
            }
        }
        Pair { a, dv, ma, md, spell_pct: 100, pct }
    });

    BuiltFormation { formation, snaps }
}

/// Computes the 2-round engagement verdict: wiped -> more fallen -> more HP lost -> draw.
pub fn verdict_of(snaps: &[FighterSnap], outcome: &Outcome) -> (Option<Side>, &'static str, Vec<String>, [i32; 2]) {
    let mut standing = [0usize; 2];
    let mut fallen = [0usize; 2];
    let mut hp_lost = [0i32; 2];
    for (i, s) in snaps.iter().enumerate() {
        let si = s.side.index();
        let end_hp = outcome.fighters[i].hp;
        hp_lost[si] += (s.hp_start - end_hp).max(0);
        if matches!(outcome.fighters[i].fate, Fate::Fell { .. }) {
            fallen[si] += 1;
        } else {
            standing[si] += 1;
        }
    }
    let mut calc = vec![
        format!("Attacker: {} standing, {} fallen, {:.1} HP lost.", standing[0], fallen[0], hp_lost[0] as f64 / 100.0),
        format!("Defender: {} standing, {} fallen, {:.1} HP lost.", standing[1], fallen[1], hp_lost[1] as f64 / 100.0),
    ];
    let (loser, decided_by) = if standing[0] == 0 && standing[1] > 0 {
        calc.push("Verdict: Attacker wiped out.".into());
        (Some(Side::Attacker), "Wiped")
    } else if standing[1] == 0 && standing[0] > 0 {
        calc.push("Verdict: Defender wiped out.".into());
        (Some(Side::Defender), "Wiped")
    } else if fallen[0] > fallen[1] {
        calc.push(format!("Verdict: Attacker lost more units ({} vs {}).", fallen[0], fallen[1]));
        (Some(Side::Attacker), "Deaths")
    } else if fallen[1] > fallen[0] {
        calc.push(format!("Verdict: Defender lost more units ({} vs {}).", fallen[1], fallen[0]));
        (Some(Side::Defender), "Deaths")
    } else if hp_lost[0] > hp_lost[1] {
        calc.push("Verdict: Attacker lost more HP.".into());
        (Some(Side::Attacker), "Damage")
    } else if hp_lost[1] > hp_lost[0] {
        calc.push("Verdict: Defender lost more HP.".into());
        (Some(Side::Defender), "Damage")
    } else {
        calc.push("Verdict: Even stalemate.".into());
        (None, "Even")
    };
    (loser, decided_by, calc, hp_lost)
}

/// Runs `runs` Monte Carlo simulations on a private RNG to produce an attack preview forecast.
pub fn forecast_formation(built: &BuiltFormation, runs: i32, seed: u64) -> Forecast {
    let mut rng = Rng::new(seed);
    let n = built.snaps.len();
    let mut falls = vec![0i32; n];
    let mut lost = [0i32; 2];
    let mut plot_falls = 0i32;
    let mut line_wiped = 0i32;
    let mut att_wiped = 0i32;
    let runs = runs.max(1);

    for _ in 0..runs {
        let out = ffh_battle::resolve_quiet(&built.formation, &mut rng);
        let (loser, _, _, _) = verdict_of(&built.snaps, &out);
        let mut def_alive = 0;
        let mut att_alive = 0;
        for (i, s) in built.snaps.iter().enumerate() {
            if matches!(out.fighters[i].fate, Fate::Fell { .. }) {
                falls[i] += 1;
                lost[s.side.index()] += 1;
            } else if s.side == Side::Defender {
                def_alive += 1;
            } else {
                att_alive += 1;
            }
        }
        if def_alive == 0 {
            line_wiped += 1;
            plot_falls += 1;
        } else if !built.formation.ground.city && loser == Some(Side::Defender) {
            plot_falls += 1;
        }
        if att_alive == 0 {
            att_wiped += 1;
        }
    }

    Forecast {
        runs,
        plot_falls_permille: plot_falls * 1000 / runs,
        line_wiped_permille: line_wiped * 1000 / runs,
        attackers_wiped_permille: att_wiped * 1000 / runs,
        expected_lost: [lost[0] * 100 / runs, lost[1] * 100 / runs],
        per_fall_pct: falls.into_iter().map(|f| f * 100 / runs).collect(),
    }
}
