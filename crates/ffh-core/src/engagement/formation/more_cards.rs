//! The great-person cards of the batch "battle more" (docs/CARDS.md) inside a formation battle, on
//! the pure resolver. A fighter carries what its unit's battle kit (`ffh_data::BattleKit`) gives it
//! in [`Kit`] (none by default); the resolver asks these functions at a few points of a battle
//! (each marked "Card:" in `mod.rs` and `magic.rs`):
//!
//! - **Reach** ([`Formation::kit_walls_first`], [`Formation::kit_intercepts`]): a fighter whose
//!   arrows go for the walls (Fire Arrows) shoots the enemy's standing Sentinels first, as a
//!   BOMBARD does; a shield bearer standing in the front row (the Shieldbearer) takes the SHOOT,
//!   BOMBARD and SPELL aimed down its lanes, as a Sentinel takes the shots and spells.
//! - **Flanking** ([`Formation::flanks`]): a fighter whose charge reaches anywhere (the Gryphon
//!   Riders) charges as where flanking works, whatever the ground: any enemy, the highest threat,
//!   the first round's bonus.
//! - **Damage** ([`Formation::kit_taken`], [`Formation::kit_city_shot`], [`Run::kit_landed`]): a
//!   shield bearer takes its shield's percent less from SHOOT, BOMBARD and SPELL; arrows at the
//!   walls fly full at a Sentinel (a stone's damage: no city penalty); spell damage does nothing to a
//!   fighter immune to spells (the Runeguard): its expected damage is 0, so a caster aims at another
//!   when it can, and the hit that lands does nothing.
//! - **Charms and hexes** ([`Formation::kit_unaffected`]): a fighter immune to charms or hexes (the
//!   Runeguard, the Witch-Hunter, the Automatons) is left out of those that land on its side; a
//!   fighter immune to hexes is not maddened nor grounded.
//! - **Rounds** ([`Run::kit_round`]): a side with drums on the field at a round's start (War Drums)
//!   has their speed more in round 1, taken off at round 2's start.
//!
//! Nothing here draws a random number: the RNG order of docs/ENGAGEMENTS.md is unchanged.

use ffh_data::ActionKind;

use super::{Formation, Pcts, Run, Side, State};

/// A fighter's battle kit (module comment). The default does nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kit {
    /// In the front row it takes the SHOOT, BOMBARD and SPELL aimed down its lanes; it takes this
    /// percent less damage from SHOOT, BOMBARD and SPELL hits.
    pub shield: i32,
    /// Its SHOOT takes the enemy's standing Sentinels first, at a stone's damage (no city penalty).
    pub shots_at_walls: bool,
    /// While it is on the field at round 1's start, its side has this much more speed in round 1.
    pub drums: i32,
    /// Its CHARGE is a charge where flanking works, whatever the ground.
    pub charge_anywhere: bool,
    /// Spell damage does nothing to it; its side's charms and the enemy's hexes do not land on it.
    pub immune_spells: bool,
    pub immune_charms: bool,
    pub immune_hexes: bool,
}

impl Formation {
    /// Whether `h`'s charges work as where flanking works: the ground lets them, or its kit
    /// (charge anywhere).
    pub fn flanks(&self, h: usize) -> bool {
        self.ground.flanking() || self.fighters[h].kit.charge_anywhere
    }

    /// Whether an action of `kind` by `h` takes the enemy's standing Sentinels first: BOMBARD, and
    /// SHOOT by a fighter whose arrows go for the walls.
    pub fn kit_walls_first(&self, h: usize, kind: ActionKind) -> bool {
        kind == ActionKind::Bombard || (kind == ActionKind::Shoot && self.fighters[h].kit.shots_at_walls)
    }

    /// Whether fighter `t`, in row `row`, takes an action of `kind` aimed down its lanes: a Sentinel
    /// takes SHOOT and SPELL (BOMBARD takes the Sentinels first instead), a shield bearer in the front
    /// row SHOOT, BOMBARD and SPELL.
    pub fn kit_intercepts(&self, t: usize, row: u8, kind: ActionKind) -> bool {
        let x = &self.fighters[t];
        (x.fixed && kind != ActionKind::Bombard) || (x.kit.shield > 0 && row == 0 && kind.ranged())
    }

    /// What `t` takes of a hit of `kind`, multiplied into `x`: nothing of a spell when it is immune
    /// to spells (so a caster aims elsewhere: [`Formation::choose`] weighs the expected damage), a
    /// shield bearer less of SHOOT, BOMBARD and SPELL.
    pub(super) fn kit_taken(&self, t: usize, kind: ActionKind, x: &mut Pcts) {
        let k = &self.fighters[t].kit;
        if kind == ActionKind::Spell && k.immune_spells {
            x.mul(0);
            return;
        }
        if k.shield > 0 && kind.ranged() {
            x.mul((100 - k.shield).max(0));
        }
    }

    /// Whether a hit of `kind` by `h` on `t` pays the city's penalty on the attacker's shots: not an
    /// arrow at the walls on a Sentinel (it strikes as a stone does).
    pub fn kit_city_shot(&self, h: usize, t: usize, kind: ActionKind) -> bool {
        kind == ActionKind::Shoot && !(self.fighters[h].kit.shots_at_walls && self.fighters[t].fixed)
    }

    /// Whether a charm (`enemy` false) or hex (`enemy` true) does not land on fighter `t`.
    pub fn kit_unaffected(&self, t: usize, enemy: bool) -> bool {
        let k = &self.fighters[t].kit;
        if enemy {
            k.immune_hexes
        } else {
            k.immune_charms
        }
    }

    /// The speed side `side`'s drums on the field give it in round 1: the most any of its fighters
    /// on the field gives.
    pub fn kit_drums(&self, s: &State, side: Side) -> i32 {
        (0..self.len()).filter(|&i| self.fighters[i].side == side && s.on_field(i)).map(|i| self.fighters[i].kit.drums).max().unwrap_or(0).max(0)
    }
}

impl Run<'_> {
    /// A hit of `kind` by `h` on `t` landed (`dmg` its damage before the target's HP caps it): spell
    /// damage does nothing to a fighter immune to spells.
    pub(super) fn kit_landed(&self, t: usize, kind: ActionKind, dmg: i32) -> i32 {
        if kind == ActionKind::Spell && self.f.fighters[t].kit.immune_spells {
            0
        } else {
            dmg
        }
    }

    /// Round `round`'s start (module comment): the drums' speed for round 1, taken off at round 2's
    /// start (by the amount given, whoever is still on the field).
    pub(super) fn kit_round(&mut self, round: i32) {
        let f = self.f;
        if round == 1 {
            for side in [Side::Attacker, Side::Defender] {
                let drums = f.kit_drums(&self.s, side);
                self.drummed[side.index()] = drums;
                if drums > 0 {
                    for i in (0..f.len()).filter(|&i| f.fighters[i].side == side) {
                        self.s.speed[i] += drums;
                    }
                }
            }
        } else if round == 2 {
            for side in [Side::Attacker, Side::Defender] {
                let drums = std::mem::take(&mut self.drummed[side.index()]);
                if drums > 0 {
                    for i in (0..f.len()).filter(|&i| f.fighters[i].side == side) {
                        self.s.speed[i] -= drums;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use ffh_data::ActionKind as K;

    const EQ: Pair = Pair { a: 400, dv: 400, ma: 400, md: 400, spell_pct: 100, pct: 100 };
    const OPEN: Ground = Ground { city: false, forest: false, hills: false };
    const CITY: Ground = Ground { city: true, forest: false, hills: false };
    const FOREST: Ground = Ground { city: false, forest: true, hills: false };

    fn row(kind: K, count: i32) -> RowActions {
        RowActions { kind, count }
    }

    fn at(side: Side, lane: u8, r: u8) -> Fighter {
        let mut f = Fighter::new(side, 120);
        f.lane = lane;
        f.row = r;
        f
    }

    fn still(mut f: Fighter) -> Fighter {
        f.actions = [row(K::None, 0), row(K::None, 0), row(K::None, 0)];
        f
    }

    fn battle(ground: Ground, fighters: Vec<Fighter>) -> Formation {
        let b = Formation::new(Params::default(), ground, [4, 4], fighters, |_, _| EQ);
        assert_eq!(b.validate(), Ok(()));
        b
    }

    /// Every draw answers the middle of its range (no misses, no crits, r = 100).
    struct Mid;

    impl BattleRng for Mid {
        fn rand(&mut self, n: i32, label: &'static str) -> i32 {
            match label {
                "Engagement miss" | "Engagement crit" | "Engagement withdraw" => n - 1,
                "Engagement hit" | "Engagement heal" => n / 2,
                _ => 0,
            }
        }
    }

    /// The Shieldbearer: in the front row it takes the shots, stones and spells aimed down its lane
    /// (not those of the lane beside it), at half damage; knocked out of the front row it does not.
    #[test]
    fn a_shield_bearer_takes_the_missiles_aimed_down_its_lane() {
        let mut shield = still(at(Side::Defender, 1, 0));
        shield.kit.shield = 50;
        let behind = still(at(Side::Defender, 1, 2));
        let beside = still(at(Side::Defender, 2, 2));
        let archer = at(Side::Attacker, 1, 1);
        let b = battle(OPEN, vec![archer, shield, behind, beside]);
        let s = State::new(&b);
        for kind in [K::Shoot, K::Bombard, K::Spell] {
            assert_eq!(b.reach(&s, 0, kind), vec![1, 3], "{kind:?}");
        }
        assert_eq!(b.reach(&s, 0, K::Strike), vec![1, 3]);
        assert_eq!(b.expected(0, 1, K::Shoot, 2, false) * 2, b.expected(0, 2, K::Shoot, 2, false));
        assert_eq!(b.expected(0, 1, K::Spell, 2, false) * 2, b.expected(0, 2, K::Spell, 2, false));
        assert_eq!(b.expected(0, 1, K::Strike, 2, false), b.expected(0, 2, K::Strike, 2, false), "a strike is not shielded");
        // In the middle row it is a unit like any other.
        let mut s2 = State::new(&b);
        s2.row[1] = 1;
        assert_eq!(b.reach(&s2, 0, K::Shoot), vec![2, 3]);
    }

    /// Fire Arrows: an archer's shots take the standing Sentinels first, at a stone's damage (no
    /// city penalty on a Sentinel), and the city penalty on the others.
    #[test]
    fn arrows_at_the_walls_take_the_sentinels_first() {
        let mut archer = at(Side::Attacker, 0, 1);
        archer.actions = [row(K::Shoot, 1), row(K::Shoot, 2), row(K::Shoot, 2)];
        archer.kit.shots_at_walls = true;
        let plain = {
            let mut a = at(Side::Attacker, 1, 1);
            a.actions = [row(K::Shoot, 1), row(K::Shoot, 2), row(K::Shoot, 2)];
            a
        };
        let mut wall = still(at(Side::Defender, 3, 0));
        wall.fixed = true;
        let soldier = still(at(Side::Defender, 0, 1));
        let b = battle(CITY, vec![archer, plain, wall, soldier]);
        let s = State::new(&b);
        assert_eq!(b.reach(&s, 0, K::Shoot), vec![2], "the Sentinel first, wherever it stands");
        assert_eq!(b.reach(&s, 1, K::Shoot), vec![2, 3], "a plain archer shoots the backmost of each lane, the Sentinel in its own");
        assert_eq!(b.expected(0, 2, K::Shoot, 2, false), 2 * b.expected(1, 2, K::Shoot, 2, false), "no city penalty on the walls");
        assert_eq!(b.expected(0, 3, K::Shoot, 2, false), b.expected(1, 3, K::Shoot, 2, false));
    }

    /// The Gryphon Riders: its charge reaches the enemy's rear in a forest, where other chargers
    /// strike, with the first round's bonus.
    #[test]
    fn a_charge_anywhere_flanks_in_a_forest() {
        let mut gryphon = at(Side::Attacker, 0, 0);
        gryphon.actions = [row(K::Charge, 2), row(K::Strike, 1), row(K::Strike, 1)];
        gryphon.kit.charge_anywhere = true;
        let mut horse = at(Side::Attacker, 1, 0);
        horse.actions = [row(K::Charge, 2), row(K::Strike, 1), row(K::Strike, 1)];
        let front = still(at(Side::Defender, 0, 0));
        let mut caster = still(at(Side::Defender, 3, 2));
        caster.actions = [row(K::Strike, 1), row(K::Spell, 1), row(K::Spell, 2)];
        let b = battle(FOREST, vec![gryphon, horse, front, caster]);
        assert!(b.flanks(0) && !b.flanks(1));
        let s = State::new(&b);
        assert_eq!(b.reach(&s, 0, K::Charge), vec![2, 3]);
        assert_eq!(b.choose(&s, 0, K::Charge, 1, &[2, 3]), Some(3), "the highest threat for its HP");
        assert_eq!(b.expected(0, 3, K::Charge, 1, false), 2 * b.expected(0, 3, K::Charge, 2, false), "the first round's bonus");
        let out = resolve(&b, &mut Mid);
        let first = out.events.iter().find(|e| e.actor == 0).expect("the gryphon acts");
        assert_eq!((first.kind, first.target), (K::Charge, Some(3)));
        let other = out.events.iter().find(|e| e.actor == 1).expect("the horseman acts");
        assert_eq!(other.kind, K::Strike, "a charge in a forest is a strike");
    }

    /// The Runeguard: a spell does it no damage, so a caster aims at another; the Witch-Hunter's
    /// hex immunity keeps a hex off it while its side takes it.
    #[test]
    fn immunities_keep_spells_charms_and_hexes_off() {
        let mut guard = still(at(Side::Defender, 1, 2));
        guard.kit.immune_spells = true;
        guard.kit.immune_charms = true;
        guard.kit.immune_hexes = true;
        let other = still(at(Side::Defender, 2, 2));
        let mut caster = at(Side::Attacker, 1, 2);
        caster.actions = [row(K::Spell, 1), row(K::Spell, 1), row(K::Spell, 1)];
        let b = battle(OPEN, vec![caster, guard, other]);
        let s = State::new(&b);
        assert_eq!(b.choose(&s, 0, K::Spell, 1, &[1, 2]), Some(2), "the spell goes where it hurts");
        let run = Run::new(&b, true);
        assert_eq!(run.kit_landed(1, K::Spell, 900), 0);
        assert_eq!(run.kit_landed(1, K::Shoot, 900), 900);
        assert_eq!(run.kit_landed(2, K::Spell, 900), 900);
        let hex = BattleCharm { enemy: true, taken: 20, ..BattleCharm::default() };
        assert_eq!(run.magic_charm_targets(0, &hex), Vec::<usize>::new(), "the front row is empty");
        let all = BattleCharm { whole_side: true, ..hex };
        assert_eq!(run.magic_charm_targets(0, &all), vec![2], "the hex lands on the side but the immune one");
        let charm = BattleCharm { enemy: false, whole_side: true, damage: 20, ..BattleCharm::default() };
        assert_eq!(run.magic_charm_targets(2, &charm), vec![2], "a charm of its own side does not land on it");
    }

    /// War Drums: the drummer's side is one faster in round 1 only.
    #[test]
    fn drums_quicken_their_side_in_round_one() {
        let mut drummer = still(at(Side::Attacker, 0, 2));
        drummer.kit.drums = 1;
        let a = at(Side::Attacker, 1, 0);
        let d = at(Side::Defender, 1, 0);
        let b = battle(OPEN, vec![drummer, a, d]);
        let mut run = Run::new(&b, false);
        run.kit_round(1);
        assert_eq!((run.s.speed[0], run.s.speed[1], run.s.speed[2]), (5, 5, 4));
        run.kit_round(2);
        assert_eq!((run.s.speed[0], run.s.speed[1], run.s.speed[2]), (4, 4, 4));
        // Equal speeds: in round 1 the drummed attacker always strikes first.
        let out = resolve(&b, &mut Mid);
        let r1: Vec<&Event> = out.events.iter().filter(|e| e.round == 1 && e.pass == 1).collect();
        assert_eq!(r1.first().map(|e| e.actor), Some(1), "{r1:?}");
    }
}
