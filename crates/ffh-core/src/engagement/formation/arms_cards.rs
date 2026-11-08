//! The great-person cards that act inside a formation battle (docs/CARDS.md; the batch "battle
//! arms"), on the pure resolver. A fighter carries what its cards give it in [`Arms`] (none by
//! default); the resolver asks these functions at a few points of a battle:
//!
//! - **The action's kind** ([`Formation::arms_kind`]): a fighter with a first-round action (the
//!   Siege Colossus's BOMBARD) takes it in round 1 in place of its row's, with the row's count.
//! - **Held** ([`Formation::arms_held`]): a fighter whose melee is held (the Moat: the attacker in
//!   the first battle at the city each turn) spends its STRIKE, CHARGE and STAB actions in the
//!   first rounds: the action does nothing and draws nothing ([`super::Note::Held`]).
//! - **The pattern** ([`Formation::arms_pattern`]): a first-round action's own pattern; else the
//!   pattern of a fighter's first STRIKE or CHARGE each round (Wardance: CLEAVE, in pass 1, the
//!   pass where every unit acts first); else its row's.
//! - **Damage taken** ([`Formation::arms_taken_pct`]): a fighter that resists shots and bombards
//!   (the Arcane Fortress's Sentinels) takes that much less from SHOOT and BOMBARD.
//! - **Damage dealt** ([`Run::arms_dealt_pct`]): a fighter of a last stand (the Last Stand Hymn's
//!   defenders) deals more once half of its side's committed fighters (not Sentinels) have fallen.
//! - **The end of a round** ([`Run::arms_round_end`]): a fighter that heals behind it (Templar)
//!   heals the nearest ally behind it in its lanes, in fighter order, after the round's last pass;
//!   a heal event with pass 0. Nothing is drawn.
//!
//! None of these draws a random number: the RNG order of docs/ENGAGEMENTS.md is unchanged (a held
//! action draws nothing, as an action with no target).

use ffh_data::{ActionKind, AttackPattern};

use super::{Event, FighterIdx, Formation, Heal, Pcts, Run, State};

/// A fighter's card effects (module comment). The default does nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arms {
    /// Its STRIKE, CHARGE and STAB actions in rounds `1..=hold_melee_rounds` are spent (a Moat).
    pub hold_melee_rounds: i32,
    /// What it does in round 1 in place of its row's action (the row's count), and its pattern
    /// then (the Siege Colossus).
    pub first_round: Option<(ActionKind, AttackPattern)>,
    /// The pattern of its first STRIKE or CHARGE each round (Wardance).
    pub first_melee_pattern: Option<AttackPattern>,
    /// At the end of each round it heals the nearest ally behind it in its lanes this many
    /// centi-HP (Templar).
    pub heal_behind: i32,
    /// Once half of its side's committed fighters (not Sentinels) have fallen, its hits deal this
    /// percent more (the Last Stand Hymn).
    pub last_stand_pct: i32,
    /// Percent less damage it takes from SHOOT and BOMBARD (the Arcane Fortress's Sentinels).
    pub ranged_resist: i32,
}

impl Formation {
    /// The kind `h` acts with in round `round` from a row whose action is `row_kind`: its
    /// first-round action in round 1 (when the row acts at all), else the row's.
    pub fn arms_kind(&self, h: usize, row_kind: ActionKind, round: i32) -> ActionKind {
        match self.fighters[h].arms.first_round {
            Some((k, _)) if round == 1 && row_kind != ActionKind::None => k,
            _ => row_kind,
        }
    }

    /// Whether `h`'s action of `kind` (as fought) in round `round` is spent: a STRIKE, CHARGE or
    /// STAB while its melee is held.
    pub fn arms_held(&self, h: usize, kind: ActionKind, round: i32) -> bool {
        round <= self.fighters[h].arms.hold_melee_rounds && matches!(kind, ActionKind::Strike | ActionKind::Charge | ActionKind::Stab)
    }

    /// The attack pattern of `h`'s action of `kind` (as fought) from row `row` in round `round`,
    /// pass `pass` (module comment).
    pub fn arms_pattern(&self, h: usize, row: u8, kind: ActionKind, round: i32, pass: u8) -> AttackPattern {
        let x = &self.fighters[h];
        match (x.arms.first_round, x.arms.first_melee_pattern) {
            (Some((_, p)), _) if round == 1 => p,
            (_, Some(p)) if pass == 1 && kind.melee() => p,
            _ => x.patterns[(row as usize).min(x.patterns.len() - 1)],
        }
    }

    /// The percentage of a hit's damage of `kind` that `t` takes (100 less its resistance to
    /// shots and bombards).
    pub fn arms_taken_pct(&self, t: usize, kind: ActionKind) -> i32 {
        let resist = self.fighters[t].arms.ranged_resist;
        if resist != 0 && matches!(kind, ActionKind::Shoot | ActionKind::Bombard) {
            (100 - resist).max(0)
        } else {
            100
        }
    }

    /// [`Formation::arms_taken_pct`] multiplied into `x`.
    pub(super) fn arms_taken(&self, t: usize, kind: ActionKind, x: &mut Pcts) {
        let pct = self.arms_taken_pct(t, kind);
        if pct != 100 {
            x.mul(pct);
        }
    }

    /// The ally `h` heals at the end of a round: the nearest unit of its side behind it in its
    /// lanes, on the field, not a Sentinel (ties by fighter order).
    pub fn arms_behind(&self, s: &State, h: usize) -> Option<usize> {
        let fh = &self.fighters[h];
        (0..self.len())
            .filter(|&t| t != h && s.on_field(t) && self.fighters[t].side == fh.side && !self.fighters[t].fixed && s.row[t] > s.row[h])
            .filter(|&t| self.fighters[t].lanes().any(|l| fh.lanes().contains(&l)))
            .min_by_key(|&t| (s.row[t], t))
    }

    /// Whether half of side `side`'s committed fighters (not Sentinels) have fallen.
    pub fn arms_half_fallen(&self, s: &State, side: super::Side) -> bool {
        let (mut all, mut fallen) = (0, 0);
        for (i, x) in self.fighters.iter().enumerate() {
            if x.side == side && !x.fixed {
                all += 1;
                if !s.alive[i] {
                    fallen += 1;
                }
            }
        }
        all > 0 && fallen * 2 >= all
    }
}

impl Run<'_> {
    /// The percentage of its hits' damage `h` deals now (100 plus its last stand's once half of
    /// its side has fallen).
    pub(super) fn arms_dealt_pct(&self, h: usize) -> i64 {
        let x = &self.f.fighters[h];
        if x.arms.last_stand_pct != 0 && self.f.arms_half_fallen(&self.s, x.side) {
            (100 + x.arms.last_stand_pct).max(0) as i64
        } else {
            100
        }
    }

    /// The end of a round (module comment): each fighter on the field that heals behind it heals
    /// the nearest ally behind it, in fighter order.
    pub(super) fn arms_round_end(&mut self) {
        let f = self.f;
        for h in 0..f.len() {
            let amount = f.fighters[h].arms.heal_behind;
            if amount <= 0 || !self.s.on_field(h) {
                continue;
            }
            let Some(t) = f.arms_behind(&self.s, h) else { continue };
            let got = amount.min(f.fighters[t].max_hp * 100 - self.s.hp[t]).max(0);
            if got == 0 {
                continue;
            }
            self.s.hp[t] += got;
            self.healed[h] += got;
            self.was_healed[t] += got;
            let ev = Event {
                round: self.round,
                pass: 0,
                actor: h as FighterIdx,
                kind: ActionKind::Heal,
                spell: None,
                target: Some(t as FighterIdx),
                hits: Vec::new(),
                heal: Some(Heal { target: t as FighterIdx, amount: got, r: 100 }),
                knocked: None,
                note: None,
                entered: Vec::new(),
            };
            self.push(ev);
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

    fn row(kind: K, count: i32) -> RowActions {
        RowActions { kind, count }
    }

    fn at(side: Side, lane: u8, r: u8) -> Fighter {
        let mut f = Fighter::new(side, 120);
        f.lane = lane;
        f.row = r;
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

    /// Moat: a held attacker's strikes in round 1 do nothing and draw nothing; its shots fly.
    #[test]
    fn a_held_attacker_spends_its_melee_in_round_one() {
        let mut striker = at(Side::Attacker, 0, 0);
        striker.arms.hold_melee_rounds = 1;
        let mut archer = at(Side::Attacker, 1, 1);
        archer.actions = [row(K::Shoot, 1), row(K::Shoot, 1), row(K::Shoot, 1)];
        archer.arms.hold_melee_rounds = 1;
        let def = at(Side::Defender, 0, 0);
        let b = battle(CITY, vec![striker, archer, def]);
        let out = resolve(&b, &mut Mid);
        let r1: Vec<&Event> = out.events.iter().filter(|e| e.round == 1 && e.actor == 0).collect();
        assert!(!r1.is_empty() && r1.iter().all(|e| e.note == Some(Note::Held) && e.hits.is_empty()), "{r1:?}");
        assert!(out.events.iter().any(|e| e.round == 1 && e.actor == 1 && !e.hits.is_empty()), "the archer shoots in round 1");
        assert!(out.events.iter().any(|e| e.round == 2 && e.actor == 0 && !e.hits.is_empty()), "the striker strikes from round 2");
        // Nothing is drawn for a held action: in round 1 only the archer's shot and the
        // defender's two strikes roll to hit.
        let mut one = b.clone();
        one.params.rounds = 1;
        let mut n = Counting(0);
        resolve(&one, &mut n);
        assert_eq!(n.0, 3);
    }

    /// Counts the miss rolls.
    struct Counting(usize);

    impl BattleRng for Counting {
        fn rand(&mut self, n: i32, label: &'static str) -> i32 {
            if label == "Engagement miss" {
                self.0 += 1;
            }
            Mid.rand(n, label)
        }
    }

    /// The Siege Colossus: a first-round BOMBARD in place of its row's STRIKE, then strikes with
    /// its row's TRAMPLE.
    #[test]
    fn a_first_round_action_replaces_the_rows_in_round_one() {
        let mut colossus = at(Side::Attacker, 0, 0);
        colossus.patterns = [AttackPattern::Trample; ROWS];
        colossus.arms.first_round = Some((K::Bombard, AttackPattern::Single));
        let front = at(Side::Defender, 0, 0);
        let back = at(Side::Defender, 0, 1);
        let b = battle(OPEN, vec![colossus, front, back]);
        let out = resolve(&b, &mut Mid);
        let by = |round: u8| out.events.iter().filter(|e| e.actor == 0 && e.round == round).collect::<Vec<_>>();
        assert!(by(1).iter().all(|e| e.kind == K::Bombard && e.hits.len() == 1), "{:?}", by(1));
        // The bombard falls on the backmost unit of the lane.
        assert!(by(1).iter().all(|e| e.target == Some(2)));
        assert!(by(2).iter().any(|e| e.kind == K::Strike && e.hits.len() == 2), "{:?}", by(2));
    }

    /// Wardance: only the first strike each round cleaves.
    #[test]
    fn the_first_melee_pattern_is_the_first_pass_only() {
        let mut dancer = at(Side::Attacker, 1, 0);
        dancer.arms.first_melee_pattern = Some(AttackPattern::Cleave);
        let still = |mut f: Fighter| {
            f.actions = [row(K::None, 0), row(K::None, 0), row(K::None, 0)];
            f
        };
        // The defender in the middle lane is the most hurt: the dancer's target.
        let mut middle = still(at(Side::Defender, 1, 0));
        middle.hp = 8000;
        let b = battle(OPEN, vec![dancer, still(at(Side::Defender, 0, 0)), middle, still(at(Side::Defender, 2, 0))]);
        let out = resolve(&b, &mut Mid);
        let r1: Vec<&Event> = out.events.iter().filter(|e| e.actor == 0 && e.round == 1).collect();
        assert_eq!(r1.len(), 2, "{r1:?}");
        assert_eq!((r1[0].pass, r1[0].target, r1[0].hits.len()), (1, Some(2), 3), "{r1:?}");
        assert!(r1[0].hits[1..].iter().all(|h| h.splash));
        assert_eq!((r1[1].pass, r1[1].hits.len()), (2, 1), "{r1:?}");
        assert_eq!(b.arms_pattern(0, 0, K::Strike, 2, 1), AttackPattern::Cleave);
        assert_eq!(b.arms_pattern(0, 0, K::Strike, 2, 2), AttackPattern::Single);
        assert_eq!(b.arms_pattern(0, 0, K::Shoot, 2, 1), AttackPattern::Single);
    }

    /// The Arcane Fortress: a fighter that resists shots and bombards takes half from them, and
    /// the whole of a strike.
    #[test]
    fn ranged_resist_halves_shots_and_bombards() {
        let mut wall = at(Side::Defender, 0, 0);
        wall.fixed = true;
        wall.arms.ranged_resist = 50;
        let plain = at(Side::Defender, 1, 0);
        let b = battle(OPEN, vec![at(Side::Attacker, 0, 0), wall, plain]);
        for kind in [K::Shoot, K::Bombard] {
            assert_eq!(b.expected(0, 1, kind, 1, false) * 2, b.expected(0, 2, kind, 1, false), "{kind:?}");
        }
        assert_eq!(b.expected(0, 1, K::Strike, 1, false), b.expected(0, 2, K::Strike, 1, false));
    }

    /// The Last Stand Hymn: the rest deal double once half the committed defenders have fallen
    /// (Sentinels not counted).
    #[test]
    fn a_last_stand_doubles_damage_once_half_have_fallen() {
        let mut d0 = at(Side::Defender, 0, 0);
        d0.arms.last_stand_pct = 100;
        let mut d1 = at(Side::Defender, 1, 0);
        d1.arms.last_stand_pct = 100;
        let mut wall = at(Side::Defender, 2, 0);
        wall.fixed = true;
        let b = battle(CITY, vec![at(Side::Attacker, 0, 0), d0, d1, wall]);
        let mut run = Run::new(&b, false);
        assert_eq!(run.arms_dealt_pct(1), 100);
        run.s.alive[2] = false;
        assert_eq!(run.arms_dealt_pct(1), 200, "one of two committed defenders fell");
        assert_eq!(run.arms_dealt_pct(0), 100, "the attacker has no last stand");
        run.s.alive[2] = true;
        run.s.alive[3] = false;
        assert_eq!(run.arms_dealt_pct(1), 100, "a breached Sentinel does not count");
    }

    /// Templar: at the end of each round it heals the nearest ally behind it 20 HP.
    #[test]
    fn a_templar_heals_the_unit_behind_it_at_the_end_of_each_round() {
        let mut templar = at(Side::Defender, 0, 0);
        templar.arms.heal_behind = 2000;
        templar.actions = [row(K::None, 0), row(K::None, 0), row(K::None, 0)];
        let mut behind = at(Side::Defender, 0, 1);
        behind.hp = 5000;
        behind.actions = [row(K::None, 0), row(K::None, 0), row(K::None, 0)];
        let mut beside = at(Side::Defender, 1, 1);
        beside.hp = 5000;
        beside.actions = [row(K::None, 0), row(K::None, 0), row(K::None, 0)];
        let mut idle = at(Side::Attacker, 0, 2);
        idle.actions = [row(K::None, 0), row(K::None, 0), row(K::Heal, 1)];
        let b = battle(OPEN, vec![idle, templar, behind, beside]);
        let out = resolve(&b, &mut Mid);
        let heals: Vec<&Event> = out.events.iter().filter(|e| e.actor == 1).collect();
        assert_eq!(heals.len(), 3, "one heal a round: {heals:?}");
        assert!(heals.iter().all(|e| e.pass == 0 && e.heal.map(|h| (h.target, h.amount)) == Some((2, 2000))));
        assert_eq!(out.fighters[2].hp, 5000 + 3 * 2000);
        assert_eq!(out.fighters[3].hp, 5000, "the unit beside it is not behind it");
        assert_eq!(out.fighters[1].healed, 6000);
    }
}
