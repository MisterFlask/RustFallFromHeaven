//! The combat simulator the AI decides its attacks on (moved out of `campaign.rs`): an attack
//! played out in expected values with the rules' combat odds (fortification, city defense, first
//! strikes, the attacker's health) as it will be fought -- every attack by the attacker the order
//! picks next ([`Order::Strike`] for a siege operation's strike, [`Order::Group`] for a stack's
//! group attack), against the defender the rules put up against it ([`simulate`]). The siege
//! operations judge their force with it ([`covers`], `campaign.rs`), every field attack too
//! (`military::field_sim`).

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::unit::CombatDetails;
use ffh_core::utils::{combat_odds, CombatOddsInput};
use ffh_data::*;

use crate::campaign::{bold_striker, fights, precious_bar};

/// Enemy units able to defend city plot `tp` that we know of (the garrison where
/// [`crate::view::sees_units`]: fog included with `FOG_ATTACK`, never stealthed units; the city
/// owner's units count before the war is declared too).
pub(crate) fn visible_defenders(g: &Game, team: TeamId, tp: PlotIdx) -> Vec<UnitId> {
    if !crate::view::sees_units(g, team, tp) {
        return Vec::new();
    }
    let owner = g.map.plots[tp].plot_city.map(|c| g.players[g.cities[c].owner].team);
    g.map.plots[tp]
        .units
        .iter()
        .copied()
        .filter(|&e| {
            let et = g.unit_get_team(e);
            et != team && (g.at_war(team, et) || Some(et) == owner) && g.unit_can_defend(e, Some(tp)) && !g.unit_is_invisible(e, team, false)
        })
        .collect()
}

/// `apply_combat_modifier` of the rules, inverted and reapplied: a strength computed with
/// modifier `from` recomputed with modifier `to`.
fn remodify(strength: i32, from: i32, to: i32) -> i32 {
    let base = if from > 0 { strength as i64 * 100 / (from as i64 + 100) } else { strength as i64 * (100 - from as i64) / 100 };
    let s = if to > 0 { base * (to as i64 + 100) / 100 } else { base * 100 / (100 - to as i64) };
    (s as i32).max(1)
}

/// One fight of the simulation: attacker `a` against defender `d` (on `dp`) at `hp` hit points
/// with `wall_cut` points of city defense bombarded away. Returns (odds per mille, expected
/// damage `a` deals when it loses). The odds are the rules' `getCombatOdds` with those two
/// changes.
pub(crate) fn fight(g: &Game, a: UnitId, d: UnitId, dp: PlotIdx, hp: i32, wall_cut: i32) -> (i32, i32) {
    pair(g, a, d, dp, wall_cut).fight(g, hp)
}

/// One attacker against one defender: what [`fight`] needs that does not change as the defender
/// loses health (the simulation computes it once per pair, then fights at any health).
#[derive(Copy, Clone, Debug)]
struct Pair {
    /// The defender's strength against this attacker at full health, walls cut.
    d_max: i32,
    d_max_hp: i32,
    a_str: i32,
    a_fp: i32,
    a_hp: i32,
    a_limit: i32,
    a_fs: i32,
    a_cfs: i32,
    a_immune: bool,
    d_fs: i32,
    d_cfs: i32,
    d_immune: bool,
    /// The defender is of a world unit class (the rules halve it when choosing a defender).
    d_world: bool,
    /// The attacker's strength and firepower without regard to the defender, and its withdrawal
    /// chance: what the rules' own estimate uses ([`Pair::ai_odds`]).
    ai_str: i32,
    ai_fp: i32,
    withdraw: i32,
}

fn pair(g: &Game, a: UnitId, d: UnitId, dp: PlotIdx, wall_cut: i32) -> Pair {
    let mut det = CombatDetails::default();
    let mut d_max = g.unit_max_combat_str(d, Some(dp), Some(a), Some(&mut det));
    if wall_cut > 0 {
        d_max = remodify(d_max, det.modifier_total, det.modifier_total - wall_cut);
    }
    Pair {
        d_max,
        d_max_hp: g.unit_max_hit_points(d).max(1),
        a_str: g.unit_curr_combat_str(a, None, Some(d), None),
        a_fp: g.unit_curr_firepower(a, None, Some(d)),
        a_hp: g.unit_curr_hit_points(a),
        a_limit: g.units[a].combat_limit,
        a_fs: g.unit_first_strikes(a),
        a_cfs: g.unit_chance_first_strikes(a),
        a_immune: g.unit_immune_to_first_strikes(a),
        d_fs: g.unit_first_strikes(d),
        d_cfs: g.unit_chance_first_strikes(d),
        d_immune: g.unit_immune_to_first_strikes(d),
        d_world: ffh_core::utils::is_world_unit_class(g.d, g.unit_get_unit_class_type(d)),
        ai_str: g.unit_curr_combat_str(a, None, None, None),
        ai_fp: g.unit_curr_firepower(a, None, None),
        withdraw: g.unit_withdrawal_probability(a),
    }
}

impl Pair {
    /// [`fight`] at `hp` hit points.
    fn fight(&self, g: &Game, hp: i32) -> (i32, i32) {
        self.fight_capped(g, hp, false)
    }

    /// [`fight`] at `hp` hit points; with `cap`, a lost fight deals at most a round short of the
    /// defender's death (had it dealt more, it would have won).
    fn fight_capped(&self, g: &Game, hp: i32, cap: bool) -> (i32, i32) {
        let max_hp = self.d_max_hp;
        let hp = hp.clamp(1, max_hp);
        let d_max = self.d_max;
        let d_str = (d_max * hp / max_hp).max(1);
        let d_fp = (d_max + d_str + 1) / 2;
        let (a_str, a_fp, a_hp) = (self.a_str, self.a_fp, self.a_hp);
        if a_str <= 0 {
            return (0, 0);
        }
        let k = CombatOddsInput {
            attacker_strength: a_str,
            attacker_firepower: a_fp,
            defender_strength: d_str,
            defender_firepower: d_fp,
            attacker_hit_points: a_hp,
            defender_hit_points: hp,
            defender_max_hit_points: max_hp,
            attacker_combat_limit: self.a_limit,
            attacker_first_strikes: self.a_fs,
            attacker_chance_first_strikes: self.a_cfs,
            attacker_immune_to_first_strikes: self.a_immune,
            defender_first_strikes: self.d_fs,
            defender_chance_first_strikes: self.d_cfs,
            defender_immune_to_first_strikes: self.d_immune,
        };
        let defs = &g.d.defines;
        let odds = combat_odds(&k, defs.combat_die_sides, defs.combat_damage);
        // A lost fight: the rounds the attacker wins before it has lost all its hit points.
        let sf = (a_fp + d_fp + 1) / 2;
        let to_att = (defs.combat_damage * (d_fp + sf) / (a_fp + sf)).max(1);
        let to_def = (defs.combat_damage * (a_fp + sf) / (d_fp + sf)).max(1);
        let q = a_str as i64 * 1000 / (a_str + d_str) as i64; // per mille a round goes our way
        // First strikes (`CvUnit::resolveCombat`): while a side has first
        // strikes left it cannot be hurt, and both count down each round, so the side with more
        // has that many rounds in which only the other is hurt (a chance first strike counts
        // half). Left out, a Moroi at strength 7 against Acheron (19, four first strikes) was
        // expected to take 12 hit points off him, and dealt none: it dies in the rounds he
        // cannot be hurt.
        let (mut a_hp_left, mut free_wound) = (a_hp as i64, 0i64);
        let fs_d = if self.a_immune { 0 } else { self.d_fs as i64 * 2 + self.d_cfs as i64 };
        let fs_a = if self.d_immune { 0 } else { self.a_fs as i64 * 2 + self.a_cfs as i64 };
        let net2 = fs_d - fs_a; // twice the defender's free rounds (negative: ours)
        if net2 > 0 {
            a_hp_left -= net2 * (1000 - q) * to_att as i64 / 2000;
        } else {
            free_wound = -net2 * q * to_def as i64 / 2000;
        }
        let losses = if a_hp_left > 0 { (a_hp_left + to_att as i64 - 1) / to_att as i64 } else { 0 };
        let won = losses * q / (1000 - q).max(50);
        let mut wound = (won * to_def as i64 + free_wound).min(hp as i64) as i32;
        if cap {
            wound = wound.min((hp + to_def - 1) / to_def * to_def - to_def).max(0);
        }
        // Combat limit (siege engines stop at a share of the defender's health).
        let limit = self.a_limit;
        if limit < max_hp {
            wound = wound.min((hp - (max_hp - limit)).max(0));
        }
        (odds, wound)
    }

    /// The rules' own odds estimate, out of 100, with the defender at `hp` hit points (C++
    /// `CvUnitAI::AI_attackOdds`, which `AI_getBestGroupAttacker` picks the attacker by: the
    /// attacker's strength without regard to the defender, first strikes as rounds, withdrawal
    /// counted as a win).
    fn ai_odds(&self, g: &Game, hp: i32) -> i32 {
        let mut ours = self.ai_str;
        if ours == 0 {
            return 1;
        }
        let hp = hp.clamp(0, self.d_max_hp);
        let mut theirs = self.d_max * hp / self.d_max_hp;
        let their_fp = (self.d_max + theirs + 1) / 2;
        let base = 100 * ours / (ours + theirs).max(1);
        if base == 0 {
            return 1;
        }
        let sf = (self.ai_fp + their_fp + 1) / 2;
        let damage = g.d.defines.combat_damage;
        let to_us = 1.max(damage * (their_fp + sf) / (self.ai_fp + sf).max(1));
        let to_them = 1.max(damage * (self.ai_fp + sf) / (their_fp + sf).max(1));
        let hit_limit = self.d_max_hp - self.a_limit;
        let mut rounds_us = (0.max(hp - hit_limit) + to_them - 1) / to_them;
        let mut rounds_them = (0.max(self.a_hp) + to_us - 1) / to_us;
        if !self.d_immune {
            rounds_us -= (base * self.a_fs + base * self.a_cfs / 2) / 100;
        }
        if !self.a_immune {
            rounds_them -= ((100 - base) * self.d_fs + (100 - base) * self.d_cfs / 2) / 100;
        }
        let diff = rounds_us.max(1) - rounds_them.max(1);
        if diff > 0 {
            theirs *= 1 + diff;
        } else {
            ours *= 1 - diff;
        }
        let mut odds = ours * 100 / (ours + theirs).max(1);
        odds += (100 - odds) * self.withdraw / 100;
        odds.clamp(1, 99)
    }

    /// How the rules rank this defender against this attacker at `hp` hit points
    /// (`CvUnit::isBetterDefenderThan`: its strength against the attacker at its health, first
    /// strikes weighed, a world unit halved; ties and the rarer tests left out).
    fn rank(&self, g: &Game, hp: i32) -> i64 {
        let fsd = (g.d.defines.combat_damage * 2 / 5) as i64;
        let mut s = self.d_max as i64 * hp.clamp(0, self.d_max_hp) as i64 / self.d_max_hp as i64;
        if !self.a_immune {
            s = s * ((self.d_fs as i64 * 2 + self.d_cfs as i64) * fsd + 100) / 100;
        }
        if self.d_immune {
            s = s * ((self.a_fs as i64 * 2 + self.a_cfs as i64) * fsd + 100) / 100;
        }
        if self.d_world {
            s /= 2;
        }
        s
    }
}

/// Expected outcome of a strike.
#[derive(Copy, Clone, Debug, Default)]
pub struct Sim {
    /// Attackers used until the defenders fell (`None`: they hold).
    pub used: Option<usize>,
    /// Defenders at the start.
    pub n_def: usize,
    /// Expected defender hit points left after all attacks.
    pub hp_left: i32,
    /// Best odds of one of the fights.
    pub best: i32,
    /// Attackers able to attack.
    pub n_att: usize,
    /// Expected defenders killed and attackers lost, per mille.
    pub kills: i32,
    pub losses: i32,
    /// The best the attack can gain: over the attacks in order, the most that what the kills are
    /// worth exceeds what the losses are worth ([`worth`], kills weighted by the caller), from
    /// some first attacks on; 0 when not even the first attack is worth it.
    pub gain: i64,
    /// The odds of the first attack (per mille; 0 when there is none).
    pub first: i32,
}

/// The order in which [`simulate`] sends the attackers: the order the attack itself
/// will follow.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Order {
    /// A siege operation's strike (`campaign::strike`): collateral attackers while several defenders stand, units
    /// summoned for good, the best odds when 68%+, else the unit cheapest to lose for the damage
    /// it is expected to deal (none that would barely dent the defender); heroes and veterans
    /// below their odds last.
    Strike,
    /// A stack's group attack (`military::assault`): the attacker the rules pick
    /// (`AI_getBestGroupAttacker`: the best odds, heroes below 85% and units above level 4 below
    /// 95% after the others); a hero or veteran below its odds there ends it (`PRECIOUS`).
    Group,
}

/// What a unit is worth, in hammers, to the side that would lose it: its cost, or for units
/// whose cost says little (animals, summons, heroes such as Acheron at 25) fifteen hammers a
/// point of strength, whichever is more; experience adds a tenth per point.
pub(crate) fn worth(g: &Game, u: UnitId) -> i64 {
    let unit = &g.units[u];
    let info = &g.d.units[unit.unit_type];
    let base = (info.production_cost as i64).max(info.combat as i64 * 15).max(10);
    base * (10 + unit.experience.max(0) as i64) / 10
}

/// The attack of `atts` on the defenders `defs` of plot `tp`, played in expected values as it
/// will be fought: every attack goes to the attacker `order` picks next, against the defender the
/// rules put up against that attacker (its strength against it, at its expected health). Each
/// fight lowers the chance that defender is alive by the odds (a siege engine, which stops at its
/// combat limit, never kills) and its expected health to what winning and losing leave (nothing,
/// or the combat limit; the damage of a lost fight, at most a round short of a kill), and the
/// chance the attacker is lost (unless it withdraws). Collateral damage hits the others. A
/// defender has fallen when it is alive with a chance of `sure` per mille or less; `used` counts
/// the attackers until all have. What the kills and losses are worth ([`worth`], kills counting
/// `kill_pct` percent) gives `gain`. `wall_cut`: city defense points bombarded away first.
/// (Before round 7g -- switch `SIM_FAITHFUL` off, removed once kept -- the strike was played
/// strongest first against the defender strongest without regard to the attacker while the strike
/// sent its sacrifices first, a defender at an eighth of its expected health counted as dead, and
/// a siege engine's win as a kill: the simulation saw the city fall where the strike stalled.)
pub fn simulate(g: &Game, tp: PlotIdx, defs: &[UnitId], atts: &[UnitId], wall_cut: i32, order: Order, kill_pct: i64, sure: i64) -> Sim {
    let n_def = defs.len();
    let atts: Vec<UnitId> = atts.iter().copied().filter(|&u| fights(g, u)).collect();
    let n_att = atts.len();
    if defs.is_empty() {
        return Sim { used: Some(0), n_def, hp_left: 0, best: 1000, n_att, ..Sim::default() };
    }
    let Some(&first) = atts.first() else {
        return Sim { used: None, n_def, hp_left: defs.iter().map(|&d| g.unit_curr_hit_points(d)).sum(), best: 0, n_att, ..Sim::default() };
    };
    let owner = g.units[first].owner;
    let precious = crate::switches::PRECIOUS.on(g, owner);
    let max_hp = g.d.defines.max_hit_points.max(1);
    // Per defender: unit, expected hit points, chance alive (per mille), worth. A defender has
    // fallen when the chance it is still alive is `sure` per mille or less. (Its expected health,
    // which blends the fights it lost and the ones it won, is no measure: after a 50% fight it
    // reads low while the defender is alive half the time and still has to be killed.)
    let mut ds: Vec<(UnitId, i32, i64, i64)> = defs.iter().map(|&d| (d, g.unit_curr_hit_points(d), 1000, worth(g, d))).collect();
    let standing = |d: &(UnitId, i32, i64, i64)| d.2 > sure;
    let pairs: Vec<Vec<Option<Pair>>> = atts
        .iter()
        .map(|&a| defs.iter().map(|&d| (g.unit_can_attack_unit(a, d) && g.unit_can_defend(d, Some(tp))).then(|| pair(g, a, d, tp, wall_cut))).collect())
        .collect();
    let raised = |u: UnitId| crate::magic::raised(g.d, g.units[u].unit_type);
    let mut left: Vec<usize> = (0..n_att).collect();
    let (mut best, mut kills, mut losses, mut gain, mut best_gain, mut first_odds) = (0, 0i64, 0i64, 0i64, 0i64, 0);
    let mut used = 0;
    loop {
        let alive: Vec<usize> = (0..ds.len()).filter(|&j| standing(&ds[j])).collect();
        if alive.is_empty() {
            return Sim { used: Some(used), n_def, hp_left: 0, best, n_att, kills: kills as i32, losses: losses as i32, gain: best_gain, first: first_odds };
        }
        // Every attacker left: the defender it would meet, the odds and the damage of a loss.
        let cands: Vec<(usize, usize, i32, i32)> = left
            .iter()
            .filter_map(|&i| {
                let j = alive.iter().copied().filter(|&j| pairs[i][j].is_some()).max_by_key(|&j| (pairs[i][j].unwrap().rank(g, ds[j].1), std::cmp::Reverse(ds[j].0)))?;
                let (odds, wound) = pairs[i][j].unwrap().fight_capped(g, ds[j].1, true);
                Some((i, j, odds, wound))
            })
            .collect();
        let pick = match order {
            Order::Strike => {
                // Heroes and veterans below their odds last (`PRECIOUS`).
                let held = |&&(i, _, o, _): &&(usize, usize, i32, i32)| precious && {
                    let bar = precious_bar(g, atts[i]);
                    bar > 0 && o < bar
                };
                let line: Vec<&(usize, usize, i32, i32)> = cands.iter().filter(|c| !held(c)).collect();
                let pool: Vec<&(usize, usize, i32, i32)> = if line.is_empty() { cands.iter().collect() } else { line };
                let by_odds = |v: &[&(usize, usize, i32, i32)]| v.iter().copied().max_by_key(|&&(i, _, o, _)| (o, std::cmp::Reverse(atts[i]))).copied();
                let collateral: Vec<&(usize, usize, i32, i32)> = pool.iter().copied().filter(|&&(i, ..)| g.unit_collateral_damage(atts[i]) > 0).collect();
                let summoned: Vec<&(usize, usize, i32, i32)> = pool.iter().copied().filter(|&&(i, ..)| raised(atts[i])).collect();
                let best_odds = pool.iter().map(|&&(_, _, o, _)| o).max().unwrap_or(0);
                if alive.len() >= 2 && !collateral.is_empty() {
                    by_odds(&collateral)
                } else if !summoned.is_empty() {
                    by_odds(&summoned)
                } else if best_odds >= 680 {
                    by_odds(&pool)
                } else {
                    let sacrifice = |&(i, _, _, w): &(usize, usize, i32, i32)| {
                        let u = atts[i];
                        let cost = g.d.units[g.units[u].unit_type].production_cost.max(1) as i64;
                        let strength = w as i64 * 100;
                        strength * (100 + g.unit_withdrawal_probability(u) as i64) * 100 / (cost * (10 + g.units[u].experience.max(0) as i64))
                    };
                    pool.iter().copied().max_by_key(|&c| (sacrifice(c), std::cmp::Reverse(atts[c.0]))).copied().filter(|&(_, _, _, w)| w * 10 >= max_hp)
                }
            }
            Order::Group => {
                // `CvSelectionGroupAI::AI_getBestGroupAttacker` for an AI player.
                let mut pick: Option<(usize, usize, i32, i32)> = None;
                let mut best_value = 0;
                for &c in &cands {
                    let u = atts[c.0];
                    let unit = &g.units[u];
                    let mut value = pairs[c.0][c.1].unwrap().ai_odds(g, ds[c.1].1);
                    if unit.ai_type == Some(UnitAIType::UNITAI_WARWIZARD) && value < 95 || unit.ai_type == Some(UnitAIType::UNITAI_HERO) && value < 85 {
                        value = 0;
                    }
                    if unit.level > 4 && value < 95 {
                        value /= 2;
                    }
                    value = value.max(1);
                    let possible = (alive.len() as i32 - 1).min(g.unit_collateral_damage_max_units(u));
                    if g.unit_collateral_damage(u) > 0 && possible > 0 {
                        value = value * (100 + g.unit_collateral_damage(u) * possible / 5) / 100;
                    }
                    if value > best_value || value == best_value && value > 0 {
                        best_value = value;
                        pick = Some(c);
                    }
                }
                // The stack holds when the rules would send a hero or veteran below its odds.
                pick.filter(|&(i, _, o, _)| {
                    let bar = if precious { precious_bar(g, atts[i]) } else { 0 };
                    bar == 0 || o >= bar
                })
            }
        };
        let Some((i, j, odds, wound)) = pick else { break };
        let a = atts[i];
        let pr = pairs[i][j].unwrap();
        best = best.max(odds);
        if used == 0 {
            first_odds = odds;
        }
        // A siege engine that wins leaves the defender at its combat limit; anything else kills.
        let kills_on_win = pr.a_limit >= pr.d_max_hp;
        let hp = ds[j].1;
        let win_hp = if kills_on_win { 0 } else { hp.min(pr.d_max_hp - pr.a_limit) };
        ds[j].1 = ((odds as i64 * win_hp as i64 + (1000 - odds) as i64 * (hp - wound).max(0) as i64) / 1000) as i32;
        let alive_after = ds[j].2 * (1000 - if kills_on_win { odds } else { 0 }) as i64 / 1000;
        let killed = ds[j].2 - alive_after;
        ds[j].2 = alive_after;
        let lost = (1000 - odds) as i64 * (100 - pr.withdraw.clamp(0, 100)) as i64 / 100;
        kills += killed;
        losses += lost;
        gain += killed * ds[j].3 * kill_pct / 100_000 - lost * worth(g, a) / 1000;
        best_gain = best_gain.max(gain);
        // Collateral damage on the others (CvUnit::collateralCombat).
        if g.unit_collateral_damage(a) > 0 && alive.len() > 1 {
            let cs = g.unit_base_combat_str(a) * g.unit_collateral_damage(a) / 100;
            let limit = g.unit_collateral_damage_limit(a);
            let mut n = (alive.len() as i32 - 1).min(g.unit_collateral_damage_max_units(a));
            let mut others: Vec<usize> = alive.iter().copied().filter(|&o| o != j).collect();
            others.sort_by_key(|&o| (std::cmp::Reverse(ds[o].1), o));
            for o in others {
                if n <= 0 {
                    break;
                }
                n -= 1;
                let ts = g.unit_base_combat_str(ds[o].0);
                let sf = (cs + ts + 1) / 2;
                if ts + sf <= 0 {
                    continue;
                }
                let dmg = g.d.defines.collateral_combat_damage * (cs + sf) / (ts + sf);
                let cap = limit.min(limit * (cs + sf) / (ts + sf));
                let damage = max_hp - ds[o].1;
                let new_damage = damage.max((damage + dmg).min(cap));
                ds[o].1 = max_hp - new_damage;
            }
        }
        left.retain(|&x| x != i);
        used += 1;
    }
    let alive: Vec<usize> = (0..ds.len()).filter(|&j| standing(&ds[j])).collect();
    let hp_left = alive.iter().map(|&j| ds[j].1).sum();
    Sim { used: if alive.is_empty() { Some(used) } else { None }, n_def, hp_left, best, n_att, kills: kills as i32, losses: losses as i32, gain: best_gain, first: first_odds }
}

/// City defense our siege engines in `units` can bombard away within three turns (0 without
/// siege engines).
pub(crate) fn wall_cut(g: &Game, c: CityId, units: &[UnitId]) -> i32 {
    let rate: i32 = units.iter().map(|&u| g.unit_bombard_rate(u)).sum();
    if rate <= 0 {
        return 0;
    }
    let ignore = units.iter().any(|&u| g.unit_bombard_rate(u) > 0 && g.unit_ignore_building_defense(u));
    let def = g.city_defense_modifier(c, ignore);
    let per_turn = if ignore { rate } else { rate * (100 - g.cities[c].building_bombard_defense).max(0) / 100 };
    def.min(per_turn * 3).max(0)
}

/// Does `units` take city `c` (plot `tp`, defenders `defs`)? With `spare`, with a reserve: every
/// defender 90% sure to fall rather than 70% ([`covers_sim`]).
pub(crate) fn covers(g: &Game, c: CityId, tp: PlotIdx, defs: &[UnitId], units: &[UnitId], spare: bool, siege: bool) -> bool {
    if !units.iter().any(|&u| fights(g, u)) {
        return false;
    }
    let s = covers_sim(g, c, tp, defs, units, spare, siege);
    s.used.is_some() || s.best >= 850 && defs.len() == 1
}

/// The simulation [`covers`] judges (also what the strike's trace shows: printing another one,
/// at a different sureness, showed a strike going ahead on a simulation where the city held).
pub(crate) fn covers_sim(g: &Game, c: CityId, tp: PlotIdx, defs: &[UnitId], units: &[UnitId], spare: bool, siege: bool) -> Sim {
    let cut = if siege { wall_cut(g, c, units) } else { 0 };
    // The confidence is the simulation's own -- every defender 90% sure to fall with a reserve to
    // spare, 70% sure without -- in place of attackers held in reserve against an estimate that
    // was optimistic. (At 95% and 80%, every civilization took 23% fewer cities, at no gain:
    // docs/AI.md, round 7g.) `BOLD_STRIKE` (conquerors): without a reserve, 60% sure.
    let bold = bold_striker(g, g.units[units[0]].owner);
    simulate(g, tp, defs, units, cut, Order::Strike, 100, if spare { 100 } else if bold { 400 } else { 300 })
}
