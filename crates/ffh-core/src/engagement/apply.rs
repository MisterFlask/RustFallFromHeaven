//! After the battle (spec 1.9, G2), in this order:
//!
//! 1. The survivors' damage is written back (HP to the damage field, plan 4.1), and the
//!    Sentinels' to their city (docs/SIEGES.md: a fallen Sentinel is breached); the victims die
//!    in kill order (`setDamage(max)`, delayed death: the unit-killed event fires here, as for
//!    today's loser).
//! 2. **Battle XP**, before any per-kill effect (docs/SIEGES.md, "XP"): each kill of a unit is
//!    worth `clamp(xp_value(v) x Dv(v <- k) / A(k -> v), MIN, MAX)` (`k` its killer), shared
//!    equally, in hundredths, by the killing side's units that fought (not sat out) and are alive
//!    after the battle (not the summons called into it), withdrawn ones included (`Unit::xp_carry` keeps what is below a point; the
//!    remainder of the division goes to the killer, else the first sharer). `xp_value` is the
//!    victim's defense XP value if it attacked, its attack XP value if it defended (open point 4).
//!    A Sentinel gives and takes no XP. Withdrawn units get the withdrawal XP.
//! 3. **Per kill**, in kill order: the barbarian win counter, war weariness and war success
//!    ([`Game::engage_kill_bookkeeping`]), then `combatWon`'s per-kill part
//!    ([`Game::unit_combat_won_as`]: free XP, the global counter, capture and enslavement, gold,
//!    duration, created units, the promotion from combat, the heal on a kill, the Python won and
//!    lost callbacks). A killer that died in the battle gets only the loser's callback. The
//!    conversion (`UnitConvertFromCombat`: Wolves, Lions, Baby Spiders, werewolves) is rolled per
//!    kill until it succeeds and applied once, after the killer's last kill.
//! 4. **Per engagement**: the promotions removed by combat leave every committed unit that fought
//!    and lives (standing or withdrawn: Hidden's quirk may move or kill it, so every later step
//!    re-checks); `PromotionCombatApply` once per holder that killed (on its victims' plot) or died
//!    (on its killer); poison: each survivor that took damage from a poison-typed enemy rolls once.
//!    Then the summons called into the battle that did not fall leave the game (summons.rs).
//! 5. The victims are killed (immortals are reborn); a defending victim's capturing player is set
//!    first when the attackers will move in. (Phase 7: the writes of the battle's `Cast` records
//!    go here.)
//! 6. **Medics** (Q12): each committed survivor heals the best same-tile heal of its team's units
//!    on its plot or the best adjacent heal next to it, at most once per unit per game turn
//!    (`Unit::battle_heal_turn`). Then phase 7's post-battle hook
//!    ([`Game::engage_post_battle_spells`], empty).
//! 7. Withdrawn defenders (outside cities only) flee to the nearest valid plot (or die with none);
//!    a marching one keeps its march. (The round model's Fear rout after a won battle is gone:
//!    formation battles are not won by damage.)
//! 8. **Attack cost**: every committed attacker (sat-out ones included) has made its attack
//!    (`made_attack`, `attacks_made`) and pays today's attack move cost unless it moves in.
//!    Withdrawn attackers stay in their group and army; the survivors stay on their plot.
//! 9. **The result** (spec 1.7): the plot falls only when no defender is left on it (in the line
//!    or in reserve, a city's Sentinels standing included): then the launcher's group moves in (a
//!    city is captured as today), and only it; its mission queue is cleared. Otherwise nobody
//!    moves. A kill a Sentinel makes or suffers has no per-kill effect (no unit made it).
//! 10. The armies of every player involved are settled (`player_army_sync`).

use ffh_data::*;

use super::engage::{damage_from_hp, Eng, Fought};
use super::formation::{FighterIdx, Side};
use super::report::{EngageResult, HealSource, KillKind, LogEvent, Stage, XpSource};
use super::sink::Sink;
use crate::game::Game;
use crate::ids::*;
use crate::unit::{KillCreate, WonEffect, WonMode};

/// What the post-battle steps did (for the report).
#[derive(Clone, Debug)]
pub(crate) struct Post {
    pub result: EngageResult,
    pub plot_taken: bool,
    pub city_captured: Option<CityId>,
    pub moved_in: i32,
    /// The defender's units still on the plot (the report's `Outcome::defenders_left`).
    pub defenders_left: i32,
    /// Per committed unit: the XP it gained after the battle.
    pub xp: Vec<i32>,
    /// Per committed unit: its kills.
    pub kills: Vec<i32>,
    /// Defenders that fled (withdrawals).
    pub fled: i32,
}

impl Game {
    /// Whether `u` is on the map and alive.
    fn engage_alive(&self, u: UnitId) -> bool {
        self.units.contains(u) && !self.unit_is_dead(u)
    }

    /// The post-battle steps (module comment).
    pub(crate) fn engage_post(&mut self, e: &mut Eng, r: &Fought, sink: &mut Option<Sink>) -> Post {
        let d = self.d;
        let n = e.units.len();
        let to = e.to;
        let mut post = Post { result: EngageResult::Held, plot_taken: false, city_captured: None, moved_in: 0, defenders_left: 0, xp: vec![0; n], kills: vec![0; n], fled: 0 };
        if let Some(s) = sink.as_mut() {
            s.stage(Stage::Post);
        }
        // The players involved (their armies are settled at the end).
        let mut players: Vec<PlayerId> = e.units.iter().map(|&u| self.units[u].owner).chain(self.map.plots[to].units.iter().map(|&u| self.units[u].owner)).collect();
        players.push(e.p);
        players.sort();
        players.dedup();

        // Card: the Font of Rebirth (docs/CARDS.md): a victim a Font saves fights on as a
        // survivor at 10% of its HP, and is carried to the Font's city before the victims die.
        let saved = crate::ffh::cards::economy::font_saves(self, &e.units, to, r);
        let spared;
        let r = if saved.is_empty() {
            r
        } else {
            spared = crate::ffh::cards::economy::spare_saved(r, &saved);
            &spared
        };
        // Card: Rite of the Necropolis (docs/CARDS.md): the living units this battle kills on the
        // Necropolis's land, counted before they die; they rise as its Pyre Zombies after it.
        let risers = crate::ffh::cards::new_civs_2::necropolis_risers(self, to, r.kills.iter().filter(|k| k.victim < n).map(|k| e.units[k.victim]));

        // 1. Damage written back; the victims die (delayed), in kill order.
        for i in 0..n {
            let u = e.units[i];
            if r.killed(i) || e.sat_out[i] || !self.units.contains(u) {
                continue;
            }
            let dmg = damage_from_hp(r.hp[i], r.max_hp[i]);
            if dmg != self.units[u].damage {
                self.unit_set_damage(u, dmg, None);
            }
        }
        if let Some(c) = e.setup.sentinel_city.filter(|&c| self.cities.contains(c)) {
            // Card: the field's Sentinels keep nothing (more_cards.rs).
            for (j, i) in e.setup.city_sentinel_range().enumerate() {
                let dmg = if r.killed(i) { 100 } else { damage_from_hp(r.hp[i], r.max_hp[i]) };
                self.city_set_sentinel_damage(c, e.setup.sentinels[j], dmg);
            }
        }
        // Card: a field Sentinel's kill is its owner's (more_cards.rs).
        let field_owner = match e.setup.field {
            Some(super::more_cards::SentinelSource::Field { owner, .. }) => Some(owner),
            _ => None,
        };
        let city_owner = e.setup.sentinel_city.filter(|&c| self.cities.contains(c)).map(|c| self.cities[c].owner).or(field_owner);
        let max_hp = d.defines.max_hit_points;
        for k in &r.kills {
            if k.victim >= n {
                continue;
            }
            let v = e.units[k.victim];
            let by = match e.units.get(k.killer) {
                Some(&ku) => self.units.contains(ku).then(|| self.units[ku].owner),
                None => city_owner,
            };
            if self.units.contains(v) {
                self.unit_set_damage(v, max_hp, by);
            }
        }

        // 2. Battle XP.
        let (min_xp, max_xp) = (d.defines.min_experience_per_combat, d.defines.max_experience_per_combat);
        let to_owner = self.map.plots[to].owner;
        let mut xp_from: Vec<Vec<XpSource>> = vec![Vec::new(); n];
        let xp_before: Vec<i32> = e.units.iter().map(|&u| if self.units.contains(u) { self.units[u].experience } else { 0 }).collect();
        for k in &r.kills {
            let vi = k.victim;
            if vi >= n {
                continue; // a Sentinel breached: no XP
            }
            let vu = e.units[vi];
            if !self.units.contains(vu) {
                continue;
            }
            let xp_value = if e.side(vi) == Side::Attacker { self.unit_defense_xp_value(vu) } else { self.unit_attack_xp_value(vu) };
            let max = self.unit_max_xp_value(vu);
            let global = !self.unit_is_barbarian(vu);
            let pr = e.setup.pair(k.killer, vi);
            let full = (xp_value as i64 * pr.dv as i64 / pr.a.max(1) as i64).clamp(min_xp as i64, max_xp.max(min_xp) as i64);
            // A summon called into the battle shares none (summons.rs).
            let sharers: Vec<usize> = e.of(e.side(vi).other()).filter(|&h| !e.sat_out[h] && r.survived(h) && self.engage_alive(e.units[h]) && !self.units[e.units[h]].battle_summon).collect();
            if sharers.is_empty() || full <= 0 {
                continue;
            }
            let total = full * 100;
            let m = sharers.len() as i64;
            let (share, rem) = (total / m, total % m);
            let gets_rem = if sharers.contains(&k.killer) { k.killer } else { sharers[0] };
            for &h in &sharers {
                let hu = e.units[h];
                let amount = share + if h == gets_rem { rem } else { 0 };
                let carry = self.units[hu].xp_carry as i64 + amount;
                self.units[hu].xp_carry = (carry % 100) as i32;
                let whole = (carry / 100) as i32;
                if whole > 0 {
                    let in_borders = to_owner == Some(self.units[hu].owner);
                    self.unit_change_experience(hu, whole, max, true, in_borders, global);
                }
                xp_from[h].push(XpSource::Kill { victim: vi as FighterIdx, share_permille: (amount * 1000 / total) as i32 });
            }
        }
        for i in 0..n {
            let u = e.units[i];
            if !r.withdrew(i) || !self.engage_alive(u) {
                continue;
            }
            let other = e.of(e.side(i).other());
            let max = other.clone().map(|j| self.unit_max_xp_value(e.units[j])).max().unwrap_or(-1);
            let global = other.clone().any(|j| !self.unit_is_barbarian(e.units[j]));
            let in_borders = to_owner == Some(self.units[u].owner);
            self.unit_change_experience(u, d.defines.experience_from_withdrawl, max, true, in_borders, global);
            xp_from[i].push(XpSource::Withdrew);
        }
        for i in 0..n {
            let u = e.units[i];
            if self.units.contains(u) {
                post.xp[i] = self.units[u].experience - xp_before[i];
            }
            if let Some(s) = sink.as_mut() {
                if !xp_from[i].is_empty() {
                    s.event(LogEvent::Xp { unit: i as FighterIdx, amount: post.xp[i], from: std::mem::take(&mut xp_from[i]) });
                }
            }
        }

        // Not in C++: crusade loans (docs/CRUSADE.md): the lent units that fought are engaged, and
        // each kill below credits a lent killer of the crusade's enemy, as after a duel.
        if !self.crusades.is_empty() {
            let fought: Vec<UnitId> = e.units.iter().enumerate().filter(|&(i, _)| !e.sat_out[i]).map(|(_, &u)| u).collect();
            self.crusade_on_engagement(&fought);
        }

        // 3. Per kill, in kill order.
        let mut convert_ok = vec![false; n];
        let mut last_kill = vec![usize::MAX; n];
        for (ki, k) in r.kills.iter().enumerate() {
            if k.killer < n {
                last_kill[k.killer] = ki;
            }
        }
        for (ki, k) in r.kills.iter().enumerate() {
            if k.victim >= n || k.killer >= n {
                continue; // a Sentinel's kill or a Sentinel breached: no unit made it
            }
            let vi = k.victim;
            let vu = e.units[vi];
            let kf = k.killer;
            post.kills[kf] += 1;
            let ku = e.units[kf];
            if !self.units.contains(vu) || !self.units.contains(ku) {
                continue;
            }
            self.engage_kill_bookkeeping(ku, vu, e.side(vi) == Side::Attacker, to);
            if !self.crusades.is_empty() {
                self.crusade_on_combat(ku, vu, false, true);
            }
            if self.telemetry.on() {
                self.tel_engage_kill(ku, vu);
            }
            if !r.killed(kf) && !self.unit_is_dead(ku) {
                let mut effects = Vec::new();
                let roll_convert = !convert_ok[kf];
                // The battle rolled the kill's created unit (summons.rs, formation/spawn.rs).
                let create = if e.setup.fighters[kf].spawn_chance.is_none() {
                    KillCreate::Roll
                } else {
                    match r.spawned.iter().find(|s| (s.0, s.1) == (kf, vi)) {
                        Some((_, _, Some(_))) => KillCreate::Made,
                        // No room on the field: made now, unless a summon made it (it would be the
                        // battle's only).
                        Some((_, _, None)) if self.units[ku].battle_summon => KillCreate::Made,
                        Some((_, _, None)) => KillCreate::Due,
                        None => KillCreate::Failed,
                    }
                };
                let ok = self.unit_combat_won_as(ku, vu, WonMode::Kill { roll_convert, create }, sink.is_some().then_some(&mut effects));
                convert_ok[kf] |= ok;
                if let Some(s) = sink.as_mut() {
                    for eff in effects {
                        let kind = match eff {
                            WonEffect::Capture(t) => KillKind::Capture(t),
                            WonEffect::Enslave(t) => KillKind::Enslave(t),
                            WonEffect::Create(t) => KillKind::Create(t),
                            WonEffect::Gold(g) => KillKind::Gold(g),
                            WonEffect::Promotion(p) => KillKind::Promotion(p),
                            WonEffect::Heal(pct) => {
                                s.event(LogEvent::Heal { unit: kf as FighterIdx, amount: pct * r.max_hp[kf], source: HealSource::OnKill });
                                continue;
                            }
                        };
                        s.event(LogEvent::KillEffect { killer: kf as FighterIdx, victim: vi as FighterIdx, kind });
                    }
                }
                if d.defines.use_combat_result_callback != 0 && self.units.contains(ku) && self.units.contains(vu) {
                    self.ev_combat_result(ku, vu);
                }
            } else if self.units.contains(ku) && self.units.contains(vu) {
                self.py_post_combat_lost(vu, ku);
            }
            if last_kill[kf] == ki && convert_ok[kf] && self.units.contains(ku) {
                if let Some(nu) = self.unit_convert_from_combat(ku) {
                    if let Some(s) = sink.as_mut() {
                        s.event(LogEvent::KillEffect { killer: kf as FighterIdx, victim: vi as FighterIdx, kind: KillKind::Convert(self.units[nu].unit_type) });
                    }
                    e.units[kf] = nu;
                }
            }
        }

        // 4. Per engagement: removed by combat, combat-apply promotions, poison.
        for i in 0..n {
            if !r.survived(i) {
                continue;
            }
            let u = e.units[i];
            for prom in d.promotions.keys() {
                if !self.engage_alive(u) {
                    break;
                }
                if self.units[u].has_promotion[prom] && d.promotions[prom].removed_by_combat {
                    self.unit_set_has_promotion(u, prom, false);
                }
            }
        }
        let apply_chance = d.defines.combat_apply_chance;
        for i in 0..n {
            let u = e.units[i];
            if !self.units.contains(u) {
                continue;
            }
            let applies: Vec<PromotionType> = d.promotions.keys().filter(|&pr| self.units[u].has_promotion[pr]).filter_map(|pr| d.promotions[pr].promotion_combat_apply).collect();
            if applies.is_empty() {
                continue;
            }
            if r.killed(i) {
                // On its killer (the duel's loser branch).
                let Some(k) = r.kills.iter().find(|k| k.victim == i) else { continue };
                let Some(&ku) = e.units.get(k.killer) else { continue };
                for apply in applies {
                    if !self.engage_alive(ku) {
                        break;
                    }
                    let kt = self.unit_get_team(ku);
                    if !self.units[ku].has_promotion[apply] && (self.unit_is_alive(ku) || !d.promotions[apply].prereq_alive) && self.unit_is_enemy(u, Some(kt), None)
                        && self.soren_rand_num(100, "Combat Apply") <= apply_chance
                    {
                        self.unit_set_has_promotion(ku, apply, true);
                    }
                }
            } else if post.kills[i] > 0 && self.engage_alive(u) {
                // On its victims' plot (the duel's winner branch).
                let plot = if e.side(i) == Side::Attacker { to } else { e.from };
                for apply in applies {
                    for lu in self.map.plots[plot].units.clone() {
                        if !self.units.contains(lu) || !self.units.contains(u) {
                            continue;
                        }
                        if !self.units[lu].has_promotion[apply] && (self.unit_is_alive(lu) || !d.promotions[apply].prereq_alive) {
                            let lt = self.unit_get_team(lu);
                            if self.unit_is_enemy(u, Some(lt), None) && self.unit_can_acquire_promotion(lu, apply)
                                && self.soren_rand_num(100, "Combat Apply") <= apply_chance
                            {
                                self.unit_set_has_promotion(lu, apply, true);
                            }
                        }
                    }
                }
            }
        }
        if let Some(poisoned) = opt_from_i32::<PromotionType>(d.defines.poisoned_promotion) {
            for t in 0..n {
                let tu = e.units[t];
                if !r.survived(t) || !self.engage_alive(tu) {
                    continue;
                }
                let by = e.of(e.side(t).other()).find(|&h| {
                    let hu = e.units[h];
                    r.dealt(h, t) > 0 && self.units.contains(hu) && self.units[hu].damage_type_combat[DamageType::DAMAGE_POISON] > 0
                });
                let Some(h) = by else { continue };
                if self.unit_is_alive(tu) && self.units[tu].damage > 0 {
                    let resist = self.unit_get_damage_type_resist(tu, DamageType::DAMAGE_POISON);
                    if self.soren_rand_num(100, "Poisoned") >= resist {
                        self.unit_set_has_promotion(tu, poisoned, true);
                        if let Some(s) = sink.as_mut() {
                            s.event(LogEvent::KillEffect { killer: h as FighterIdx, victim: t as FighterIdx, kind: KillKind::Afflict(poisoned) });
                        }
                    }
                }
            }
        }

        // The units kills created that fought stay, unless a summon created them; then the
        // summons called into the battle that did not fall leave the game (summons.rs), and the
        // units kills could have created that none did.
        let created: Vec<(UnitId, UnitId)> = r
            .entered
            .iter()
            .filter_map(|&i| {
                let c = e.setup.fighters[i].spawner? as usize;
                Some((e.units[i], *e.units.get(c)?))
            })
            .collect();
        self.engage_keep_created(&created);
        let summons: Vec<UnitId> = (0..n).filter(|&i| !r.killed(i)).map(|i| e.units[i]).filter(|&u| self.units.contains(u) && self.units[u].battle_summon).collect();
        self.engage_dismiss_summons(&summons);

        // Card: the Font of Rebirth: the saved go to their Font's city.
        for &(i, c, mark) in &saved {
            let u = e.units[i];
            if self.engage_alive(u) && self.cities.contains(c) {
                self.unit_carry_to_font(u, c, mark);
            }
        }

        // 5. The victims are killed; a defending victim is captured when the attackers move in.
        let launcher = e.launcher.filter(|&g| self.groups.contains(g));
        let mover = launcher.and_then(|g| self.groups[g].units.iter().copied().find(|&u| self.engage_alive(u)));
        let leaving = r.kills.iter().filter(|k| k.victim < n && e.side(k.victim) == Side::Defender).count() as i32
            + e.of(Side::Defender).filter(|&i| r.withdrew(i) && !e.in_city()).count() as i32;
        // Card: the field's Sentinels hold no plot (more_cards.rs).
        let walls_left = e.setup.city_sentinel_range().filter(|&i| !r.killed(i)).count() as i32;
        let will_advance = walls_left == 0 && mover.is_some_and(|m| self.plot_get_num_visible_enemy_defenders(to, m) - leaving <= 0);
        if will_advance {
            let capture = mover.is_some_and(|m| !self.unit_is_no_capture(m));
            for k in r.kills.iter().filter(|k| k.victim < n) {
                let v = e.units[k.victim];
                if e.side(k.victim) == Side::Defender && self.units.contains(v) && (capture || d.units[self.units[v].unit_type].equipment_promotion.is_some()) {
                    self.unit_set_capturing_player(v, Some(e.p));
                }
            }
        }
        for k in r.kills.iter().filter(|k| k.victim < n) {
            let v = e.units[k.victim];
            if self.units.contains(v) {
                self.unit_kill(v, false, None);
            }
        }
        // Phase 7: the writes of the battle's `Cast` records (`b.effects`) go here.

        // 6. Medics (Q12).
        let turn = self.game_turn();
        for i in 0..n {
            let u = e.units[i];
            if !r.survived(i) || !self.engage_alive(u) || self.units[u].damage <= 0 || self.units[u].battle_heal_turn == turn {
                continue;
            }
            let Some((heal, medic, adjacent)) = self.engage_medic_heal(u) else { continue };
            let amount = heal.min(self.units[u].damage);
            self.unit_change_damage(u, -amount, None);
            self.units[u].battle_heal_turn = turn;
            if let Some(s) = sink.as_mut() {
                let unit_type = self.units[medic].unit_type;
                let b_medic = s.index_of(medic);
                s.event(LogEvent::Heal { unit: i as FighterIdx, amount: amount * r.max_hp[i], source: HealSource::Medic { b: b_medic, unit_type, adjacent } });
            }
        }
        // Phase 7 hook: the post-battle spells (their `Cast` records' writes would go here).
        let spells = self.engage_post_battle_spells(&e.setup, r);
        debug_assert!(spells.is_empty(), "post-battle spells are phase 7's: {spells:?}");

        // 7. Withdrawn defenders flee (outside cities).
        if !e.in_city() {
            for i in e.of(Side::Defender) {
                let u = e.units[i];
                // Card: a recalled unit goes to its capital instead (battle_magic.rs).
                if !r.withdrew(i) || !self.engage_alive(u) || self.engage_recalled(e, r, i) {
                    continue;
                }
                self.unit_join_group(u, None, true);
                self.unit_withdrawl_to_nearest_valid_plot(u, true);
                post.fled += 1;
                if let Some(s) = sink.as_mut() {
                    if let Some(at) = self.units.contains(u).then(|| self.unit_plot_opt(u)).flatten() {
                        s.event(LogEvent::Flee { unit: i as FighterIdx, to: at });
                    }
                }
            }
        }

        // Card: Contingency (battle_magic.rs): recalled units go to their capital; the opening
        // casts and recalls use their promotion up.
        self.engage_magic_after(e, r);

        // 8. Attack cost; 9. the launcher moves in when no defender is left.
        let launcher = e.launcher.filter(|&g| self.groups.contains(g));
        let mover = launcher.and_then(|g| self.groups[g].units.iter().copied().find(|&u| self.engage_alive(u)));
        let advance = walls_left == 0 && mover.is_some_and(|m| self.plot_get_num_visible_enemy_defenders(to, m) == 0);
        for i in e.of(Side::Attacker) {
            let u = e.units[i];
            if !self.engage_alive(u) {
                continue;
            }
            self.units[u].made_attack = true;
            self.units[u].attacks_made += 1;
            let moves_in = advance && self.units[u].group == launcher && self.unit_can_advance(u, to, 0);
            if !moves_in {
                let cost = crate::unit::attack_move_cost(self, u, to);
                self.unit_change_moves(u, cost);
            }
        }
        if let Some(g) = launcher {
            if advance {
                let combat_unit = e.of(Side::Attacker).map(|i| e.units[i]).find(|&u| self.engage_alive(u) && self.units[u].group == Some(g) && self.unit_can_advance(u, to, 0));
                let members = self.groups[g].units.clone();
                let city_before = self.map.plots[to].plot_city;
                self.group_move(g, to, true, combat_unit, false);
                let moved: Vec<UnitId> = members.into_iter().filter(|&u| self.units.contains(u) && self.unit_plot_opt(u) == Some(to)).collect();
                post.moved_in = moved.len() as i32;
                post.plot_taken = !moved.is_empty();
                if city_before.is_some() {
                    post.city_captured = self.map.plots[to].plot_city.filter(|&c| self.cities.contains(c) && self.cities[c].owner == e.p);
                }
                if let Some(s) = sink.as_mut() {
                    let units: Vec<FighterIdx> = moved.iter().filter_map(|&u| s.index_of(u)).collect();
                    s.event(LogEvent::MoveIn { units, city: post.city_captured });
                }
            }
            if self.groups.contains(g) {
                self.group_clear_mission_queue(g);
            }
        }
        // 9. The result: the plot fell when none of the defender's units is left on it.
        // (A unit a defender's kill created that stays is a defender left too: summons.rs.)
        let mut left: Vec<UnitId> = e.setup.candidates.iter().copied().chain(e.of(Side::Defender).map(|i| e.units[i])).filter(|&u| self.engage_alive(u) && self.unit_plot_opt(u) == Some(to)).collect();
        left.sort();
        left.dedup();
        post.defenders_left = left.len() as i32 + walls_left;
        post.result = if post.defenders_left == 0 {
            EngageResult::Taken
        } else if r.down[1] {
            EngageResult::Cleared
        } else if r.down[0] {
            EngageResult::Repulsed
        } else {
            EngageResult::Held
        };
        // Card: Founding of the Dawn Knights (docs/CARDS.md): the winners that hold the Vow heal
        // fully after a battle won against units of evil civilizations.
        let won = match post.result {
            EngageResult::Taken | EngageResult::Cleared => Some(Side::Attacker),
            EngageResult::Repulsed => Some(Side::Defender),
            EngageResult::Held => None,
        };
        if let Some(w) = won {
            let winners: Vec<UnitId> = e.of(w).filter(|&i| r.survived(i)).map(|i| e.units[i]).filter(|&u| self.engage_alive(u)).collect();
            let losers: Vec<PlayerId> = e.of(w.other()).map(|i| e.owners[i]).collect();
            crate::ffh::cards::new_civs::dawn_vow_heal(self, &winners, &losers);
        }
        // Card: Rite of the Necropolis: the battle's dead rise.
        if let Some((necropolis, count)) = risers {
            crate::ffh::cards::new_civs_2::necropolis_rise(self, necropolis, to, count);
        }

        // 10. Armies.
        for p in players {
            self.player_army_sync(p);
        }
        post
    }

    /// The best battle heal for unit `u` (Q12): the best same-tile heal among its team's units on
    /// its plot, or the best adjacent heal among its team's units next to it (same area), as
    /// `healRate` takes them; `(heal, medic, adjacent)`.
    fn engage_medic_heal(&self, u: UnitId) -> Option<(i32, UnitId, bool)> {
        let plot = self.unit_plot_opt(u)?;
        // Card: Automatons heal only at a Forge (more_cards.rs).
        if !self.unit_heals_at(u, plot) {
            return None;
        }
        let team = self.unit_get_team(u);
        let mut best: Option<(i32, UnitId, bool)> = None;
        for &lu in &self.map.plots[plot].units {
            let heal = self.units[lu].same_tile_heal;
            if self.unit_get_team(lu) == team && heal > best.map_or(0, |b| b.0) {
                best = Some((heal, lu, false));
            }
        }
        let (px, py) = self.map.xy(plot);
        for i in 0..DirectionType::NUM {
            let Some(lp) = self.plot_direction(px, py, DirectionType::from_usize(i)) else { continue };
            if self.map.plots[lp].area != self.map.plots[plot].area {
                continue;
            }
            for &lu in &self.map.plots[lp].units {
                let heal = self.units[lu].adjacent_tile_heal;
                if self.unit_get_team(lu) == team && heal > best.map_or(0, |b| b.0) {
                    best = Some((heal, lu, true));
                }
            }
        }
        best
    }
}
