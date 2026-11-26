//! Units created on a kill (the owner's decision, 2026-11-05; docs/ENGAGEMENTS.md, "Summons"): a
//! unit made by a kill in a battle (`UnitCreateFromCombat`: the Air Elemental's Lightning
//! Elemental, Ars's Wraiths, the werewolves' and the Wolf Pack's) fights on in that battle.
//!
//! Its creator ([`Fighter::spawn_chance`]) brings the units its kills may create as fighters off
//! the field ([`Fighter::spawner`], each with its cell). When it kills a [`Fighter::fodder`]
//! enemy it rolls its chance (`0..100` at most the chance, "Engagement create"); on a success the
//! first of its units whose cell is free enters the field there, with the action's event
//! ([`Event::entered`]), and acts from the next pass. A success with none left (or no free cell)
//! is recorded too ([`Spawned::unit`] `None`): the rules make that unit after the battle.

use super::*;

/// A kill that created a unit: its killer and victim, and the fighter that entered the field
/// (`None`: none could; the unit is made after the battle).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Spawned {
    pub killer: FighterIdx,
    pub victim: FighterIdx,
    pub unit: Option<FighterIdx>,
}

impl Run<'_> {
    /// `h` killed `t` (module comment).
    pub(super) fn spawn_on_kill(&mut self, h: usize, t: usize, rng: &mut impl BattleRng) {
        let f = self.f;
        let Some(chance) = f.fighters[h].spawn_chance else { return };
        if !f.fighters[t].fodder || f.fighters[t].side == f.fighters[h].side {
            return;
        }
        if rng.rand(100, "Engagement create") > chance {
            return;
        }
        let free = |u: usize, s: &State| {
            let fu = &f.fighters[u];
            !(0..f.len()).any(|x| x != u && s.on_field(x) && f.fighters[x].side == fu.side && s.row[x] == fu.row && f.fighters[x].lanes().any(|l| fu.lanes().contains(&l)))
        };
        let unit = (0..f.len()).find(|&u| f.fighters[u].spawner == Some(h as FighterIdx) && self.s.mx.pending[u] && self.s.alive[u] && free(u, &self.s));
        if let Some(u) = unit {
            self.s.mx.pending[u] = false;
            self.s.row[u] = f.fighters[u].row;
            self.entered_now.push(Entry { unit: u as FighterIdx, lane: f.fighters[u].lane, row: f.fighters[u].row, hp: self.s.hp[u] });
        }
        self.spawned.push(Spawned { killer: h as FighterIdx, victim: t as FighterIdx, unit: unit.map(|u| u as FighterIdx) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every physical action lands, nothing crits, r is 100, every withdrawal fails, coins are 0;
    /// a creation roll draws `create`.
    struct Sure {
        create: i32,
    }

    impl BattleRng for Sure {
        fn rand(&mut self, n: i32, label: &'static str) -> i32 {
            match label {
                "Engagement hit" => 20,
                "Engagement order" | "Engagement first strikes" => 0,
                "Engagement create" => self.create.clamp(0, n - 1),
                _ => n - 1,
            }
        }
    }

    /// A creator in lane 1 whose created unit waits in lane 0, against a defender of 1 HP in lane 1
    /// (fodder or not) and one at full HP in lane 3.
    fn field(chance: i32, fodder: bool) -> Formation {
        let mut creator = Fighter::new(Side::Attacker, 120);
        creator.lane = 1;
        creator.spawn_chance = Some(chance);
        let mut made = Fighter::new(Side::Attacker, 120);
        made.lane = 0;
        made.spawner = Some(0);
        let mut weak = Fighter::new(Side::Defender, 120);
        weak.lane = 1;
        weak.hp = 100;
        weak.fodder = fodder;
        let mut strong = Fighter::new(Side::Defender, 120);
        strong.lane = 3;
        let even = Pair { a: 400, dv: 400, ma: 400, md: 400, spell_pct: 100, pct: 100 };
        let f = Formation::new(Params::default(), Ground::default(), [4, 4], vec![creator, made, weak, strong], |_, _| even);
        assert_eq!(f.validate(), Ok(()));
        f
    }

    #[test]
    fn a_kill_brings_the_created_unit_into_the_battle() {
        let f = field(100, true);
        let out = resolve(&f, &mut Sure { create: 0 });
        assert_eq!(out.spawned, vec![Spawned { killer: 0, victim: 2, unit: Some(1) }]);
        let entered: Vec<&Event> = out.events.iter().filter(|e| e.entered.iter().any(|x| x.unit == 1)).collect();
        assert_eq!(entered.len(), 1, "it enters with the killing action");
        assert_eq!(entered[0].actor, 0);
        assert_eq!((entered[0].entered[0].lane, entered[0].entered[0].row), (0, 0));
        assert!(out.events.iter().any(|e| e.actor == 1), "it fights on");
        // Off the field before: nobody acts from it or hits it before it enters.
        let at = out.events.iter().position(|e| e.entered.iter().any(|x| x.unit == 1)).unwrap();
        assert!(out.events[..at].iter().all(|e| e.actor != 1 && e.hits.iter().all(|h| h.target != 1)));
    }

    #[test]
    fn no_unit_without_the_roll_or_from_a_kill_that_cannot_create() {
        // The roll fails (chance 50, roll 99).
        let out = resolve(&field(50, true), &mut Sure { create: 99 });
        assert!(out.spawned.is_empty());
        assert!(out.events.iter().all(|e| e.actor != 1 && e.entered.is_empty()));
        // A victim that is not fodder (not alive, cannot be captured): no roll.
        let out = resolve(&field(100, false), &mut Sure { create: 0 });
        assert!(out.spawned.is_empty());
        assert_eq!(out.fighters[1].fate, Fate::Standing, "it never was on the field");
    }
}
