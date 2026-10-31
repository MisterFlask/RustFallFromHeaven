//! The engagement's working value (plan section 6.1): the fighters of both sides in slot order,
//! their role flags, HP, volleys, withdrawal chance and combat limit, the pair table of A and Dv,
//! and the battle-level effects of phase 7 (6.2). It is built for one call and never saved.
//!
//! Plain values only: the rules map units onto [`Fighter`]s (role, `engage_hit`, slots and picks
//! live in the rules), so this module depends on nothing in `Game`.

/// Which side a fighter is on.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    Attacker,
    Defender,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Attacker => Side::Defender,
            Side::Defender => Side::Attacker,
        }
    }

    /// 0 for the attacker, 1 for the defender.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A target class in a role's priority order (plan 2.1; the rules map ffh-data's role table onto
/// it). A class is the set of the enemy's engaged fighters of that kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetClass {
    /// Front-position fighters (`!Fighter::back`), screens included.
    Front,
    /// Back-position fighters (`Fighter::back`): shielded while an enemy front fighter is engaged,
    /// unless the hitter ignores the shield.
    Back,
    /// Screens (`Fighter::screen`).
    Screen,
    /// Heroes and support units (`Fighter::hero_or_support`): phase 7's assassin profile.
    HeroOrSupport,
}

/// A fighter's index in [`Battle::fighters`].
pub type FighterIdx = u16;

/// One committed unit, as the round model sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fighter {
    pub side: Side,
    /// The caller's handle (the rules: the unit id), carried to the log untouched.
    pub tag: u32,
    /// Back position: shielded by its own side's front (Backline, Support).
    pub back: bool,
    /// A screen (in the enemy's `Screen` class; flankers hit screens first).
    pub screen: bool,
    /// May hit back-position enemies while enemy front fighters are engaged (Flanker, Assassin).
    pub ignores_shield: bool,
    /// In the enemy's `HeroOrSupport` class (phase 7: no role order lists it in phase 6).
    pub hero_or_support: bool,
    /// The role's target-class order: the first class with an available target is hit.
    pub order: Vec<TargetClass>,
    /// Maximum HP (the role's, or the unit's `iHP`): 100% of the damage field.
    pub max_hp: i32,
    /// HP at the start of the battle, in centi-HP (`(100 - damage) * max_hp`), after the opening's
    /// defensive strikes and collateral; 1..=`max_hp * 100`.
    pub hp: i32,
    /// Opening volleys: `first_strikes` plus a roll of 0..=`chance_first_strikes`.
    pub first_strikes: i32,
    pub chance_first_strikes: i32,
    /// Never the target of an opening volley.
    pub first_strike_immune: bool,
    /// Withdrawal chance in percent, rolled once, the first round the fighter ends below
    /// `Params::withdraw_hp_pct` (or dead). The rules pass 0 for defenders in a city, held or
    /// immobile units.
    pub withdraw_pct: i32,
    /// Combat limit in percent of max HP (100 = none). Read for attackers only: its hits never
    /// take a target past it, and it leaves when every target is at it. Defenders have none.
    pub combat_limit: i32,
    /// Failed the morale check (the rules rolled it, plan 6.3): in the battle, never fights, never
    /// hit. Read by [`super::resolve_sampled`]; the estimator reads `fear_fail` instead.
    pub sat_out: bool,
    /// The chance (in 1/4096) that the fighter fails the morale check: the estimator's weight of
    /// it sitting out. 0 when it cannot fail (or when `sat_out` is already rolled).
    pub fear_fail: i32,
    /// Phase 7: a summon that joined beyond the slots (`ENGAGEMENT_SUMMON_EXTRA_SLOTS`).
    pub extra: bool,
    /// Battle modifiers (phase 7 `BattleEffect::Mod`; 0 in phase 6): a combat percentage on the
    /// fighter's A when it hits and on its Dv when it is hit, on top of the pair table.
    pub a_pct: i32,
    pub dv_pct: i32,
}

impl Fighter {
    /// A front-line fighter at full health with no volleys, withdrawal or limit, targeting
    /// `Front` then `Back` (the frontline, screen, backline and support order).
    pub fn new(side: Side, max_hp: i32) -> Fighter {
        Fighter {
            side,
            tag: 0,
            back: false,
            screen: false,
            ignores_shield: false,
            hero_or_support: false,
            order: vec![TargetClass::Front, TargetClass::Back],
            max_hp,
            hp: max_hp * 100,
            first_strikes: 0,
            chance_first_strikes: 0,
            first_strike_immune: false,
            withdraw_pct: 0,
            combat_limit: 100,
            sat_out: false,
            fear_fail: 0,
            extra: false,
            a_pct: 0,
            dv_pct: 0,
        }
    }

    /// Whether the fighter is in the enemy's target class `c`.
    pub fn in_class(&self, c: TargetClass) -> bool {
        match c {
            TargetClass::Front => !self.back,
            TargetClass::Back => self.back,
            TargetClass::Screen => self.screen,
            TargetClass::HeroOrSupport => self.hero_or_support,
        }
    }

    /// The HP below which the fighter rolls its withdrawal, in centi-HP.
    pub fn withdraw_threshold(&self, p: &Params) -> i32 {
        p.withdraw_hp_pct * self.max_hp
    }
}

/// The per-pair strengths: the hitter's A against this target and the target's Dv against this
/// hitter, both x100 (plan 4.2, `engage_hit`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Pair {
    pub a: i32,
    pub dv: i32,
}

/// The rule constants a battle reads (the `ENGAGEMENT_*` defines, plan 2.5).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Params {
    /// B in `B x A / (A + Dv) x r` (`ENGAGEMENT_BASE_DAMAGE`), in HP.
    pub base_damage: i32,
    /// r, in percent: `rand_min..=rand_max` (`ENGAGEMENT_DAMAGE_RAND_MIN` / `_MAX`).
    pub rand_min: i32,
    pub rand_max: i32,
    /// Rounds before both sides disengage (`ENGAGEMENT_MAX_ROUNDS`).
    pub max_rounds: i32,
    /// HP percent below which withdrawal is rolled (`ENGAGEMENT_WITHDRAW_HP_PERCENT`).
    pub withdraw_hp_pct: i32,
}

impl Default for Params {
    /// The spec's values.
    fn default() -> Params {
        Params { base_damage: 20, rand_min: 80, rand_max: 120, max_rounds: 40, withdraw_hp_pct: 40 }
    }
}

/// A battle-level effect of a phase 7 hook (plan 6.2, 14): `engage_pre_battle` and
/// `engage_opening_spells` return them; both integrators apply them the same way, so the
/// estimator and the preview see in-battle spells without touching the RNG or the game. The
/// writes to `Game` are deferred to after the battle through the `Cast` records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BattleEffect {
    /// A combat percentage on the fighter's A (when it hits) and Dv (when it is hit), added to
    /// [`Fighter::a_pct`] / [`Fighter::dv_pct`]; the mean damage table is rebuilt from it.
    Mod { fighter: FighterIdx, a_pct: i32, dv_pct: i32 },
    /// Damage before the volleys, in percent of the fighter's max HP (an opening damage spell);
    /// a kill is credited to `by`.
    Damage { fighter: FighterIdx, pct: i32, by: Option<FighterIdx> },
    /// A summon that joined in the pre-battle phase: it is in `fighters` (with `extra`), so the
    /// integrators read this as a record only.
    Join { fighter: FighterIdx },
    /// A spell cast in the battle (the spell's type index), for the deferred writes and the
    /// report; no effect in the integrators.
    Cast { caster: FighterIdx, spell: u32 },
}

/// One class of a hitter's order, made disjoint from the classes before it (a flanker's `Front`
/// holds no screens: they were its `Screen` class), with its static members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClassEntry {
    pub class: TargetClass,
    pub members: Vec<FighterIdx>,
}

/// The engagement: fighters, pair table, effects, and the tables both integrators read.
#[derive(Clone, Debug)]
pub struct Battle {
    pub params: Params,
    pub fighters: Vec<Fighter>,
    pub effects: Vec<BattleEffect>,
    /// `n * n`, `[h * n + t]`: the pair table as given (cross-side entries only).
    pairs: Vec<Pair>,
    /// `n * n`: the mean damage of a hit, in centi-HP, after the battle modifiers.
    mean: Vec<i32>,
    /// Per hitter: its order as disjoint classes.
    classes: Vec<Vec<ClassEntry>>,
    /// Per side: its front-position fighters (the shield).
    front: [Vec<FighterIdx>; 2],
    /// The hit order: attackers and defenders interleaved in slot order (A0, D0, A1, D1, ...).
    order: Vec<FighterIdx>,
}

/// Today's `apply_combat_modifier` (`unit/combat.rs`): a positive percentage multiplies, a
/// negative one divides.
pub(crate) fn apply_pct(x100: i32, pct: i32) -> i32 {
    let x = x100 as i64;
    let v = if pct > 0 { x * (pct as i64 + 100) / 100 } else { x * 100 / (100 - pct as i64) };
    v.clamp(1, i32::MAX as i64) as i32
}

/// Centi-HP from the damage field (plan 4.1): `(100 - damage) * max_hp`.
pub fn hp_from_damage(damage: i32, max_hp: i32) -> i32 {
    (100 - damage.clamp(0, 100)) * max_hp
}

/// The damage field from centi-HP (plan 4.1): `100 - ceil(hp / max_hp)`, so a fighter with HP
/// left never shows 100; 100 when `hp <= 0`.
pub fn damage_from_hp(hp: i32, max_hp: i32) -> i32 {
    if hp <= 0 || max_hp <= 0 {
        return 100;
    }
    let left = (hp + max_hp - 1) / max_hp;
    (100 - left).clamp(0, 99)
}

impl Battle {
    /// A battle of `fighters` (slot order; attackers usually first) with `pair(h, t)` giving the
    /// hitter's A and the target's Dv for every cross-side pair. Fighters' HP is clamped to
    /// 1..=`max_hp * 100`, `max_hp` to at least 1.
    pub fn new(params: Params, mut fighters: Vec<Fighter>, mut pair: impl FnMut(usize, usize) -> Pair) -> Battle {
        assert!(fighters.len() <= FighterIdx::MAX as usize, "too many fighters");
        for f in fighters.iter_mut() {
            f.max_hp = f.max_hp.max(1);
            f.hp = f.hp.clamp(1, f.max_hp * 100);
        }
        let n = fighters.len();
        let mut pairs = vec![Pair::default(); n * n];
        for h in 0..n {
            for t in 0..n {
                if fighters[h].side != fighters[t].side {
                    pairs[h * n + t] = pair(h, t);
                }
            }
        }
        let classes = (0..n)
            .map(|h| {
                let mut seen = vec![false; n];
                let mut out = Vec::new();
                for &c in &fighters[h].order {
                    let members: Vec<FighterIdx> = (0..n)
                        .filter(|&t| fighters[t].side != fighters[h].side && !seen[t] && fighters[t].in_class(c))
                        .map(|t| t as FighterIdx)
                        .collect();
                    for &t in &members {
                        seen[t as usize] = true;
                    }
                    out.push(ClassEntry { class: c, members });
                }
                out
            })
            .collect();
        let side_list = |s: Side| -> Vec<FighterIdx> { (0..n).filter(|&i| fighters[i].side == s).map(|i| i as FighterIdx).collect() };
        let (atts, defs) = (side_list(Side::Attacker), side_list(Side::Defender));
        let mut order = Vec::with_capacity(n);
        for k in 0..atts.len().max(defs.len()) {
            order.extend(atts.get(k));
            order.extend(defs.get(k));
        }
        let front_of = |s: Side| -> Vec<FighterIdx> { (0..n).filter(|&i| fighters[i].side == s && !fighters[i].back).map(|i| i as FighterIdx).collect() };
        let front = [front_of(Side::Attacker), front_of(Side::Defender)];
        let mut b = Battle { params, fighters, effects: Vec::new(), pairs, mean: vec![0; n * n], classes, front, order };
        b.rebuild_mean();
        b
    }

    /// Adds battle effects: `Mod`s change the fighters' battle modifiers and the mean damage
    /// table; the others are applied by the integrators (`Damage`) or are records.
    pub fn add_effects(&mut self, effects: impl IntoIterator<Item = BattleEffect>) {
        for e in effects {
            if let BattleEffect::Mod { fighter, a_pct, dv_pct } = e {
                let f = &mut self.fighters[fighter as usize];
                f.a_pct += a_pct;
                f.dv_pct += dv_pct;
            }
            self.effects.push(e);
        }
        self.rebuild_mean();
    }

    fn rebuild_mean(&mut self) {
        let n = self.fighters.len();
        for h in 0..n {
            for t in 0..n {
                if self.fighters[h].side != self.fighters[t].side {
                    let p = self.pair(h, t);
                    let (a, dv) = (p.a as i64, p.dv as i64);
                    self.mean[h * n + t] = (self.params.base_damage.max(0) as i64 * 100 * a / (a + dv)) as i32;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.fighters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fighters.is_empty()
    }

    /// The hitter's A and the target's Dv for this pair, after the battle modifiers (x100, >= 1).
    pub fn pair(&self, h: usize, t: usize) -> Pair {
        let p = self.pairs[h * self.fighters.len() + t];
        Pair { a: apply_pct(p.a.max(1), self.fighters[h].a_pct), dv: apply_pct(p.dv.max(1), self.fighters[t].dv_pct) }
    }

    /// The mean damage of a hit of `h` on `t`, in centi-HP: `B x 100 x A / (A + Dv)` (plan 4.3).
    pub fn mean_damage(&self, h: usize, t: usize) -> i32 {
        self.mean[h * self.fighters.len() + t]
    }

    /// The hit order: attackers and defenders interleaved in slot order, neither side favoured.
    pub fn hit_order(&self) -> &[FighterIdx] {
        &self.order
    }

    pub(crate) fn classes(&self, h: usize) -> &[ClassEntry] {
        &self.classes[h]
    }

    /// The front-position fighters of side `s`: while one of them is engaged, `s`'s back
    /// fighters are shielded from hitters that do not ignore the shield.
    pub(crate) fn front(&self, s: Side) -> &[FighterIdx] {
        &self.front[s.index()]
    }

    /// Whether `t` is behind its side's shield for hitter `h` (a back fighter, and `h` does not
    /// ignore the shield).
    pub fn shielded(&self, h: usize, t: usize) -> bool {
        self.fighters[t].back && !self.fighters[h].ignores_shield
    }

    /// The HP (centi-HP) `h`'s hits never take `t` below: `(100 - limit)% x max_hp` for an
    /// attacker with a combat limit below 100 (plan 6.5), else `None`.
    pub fn cap_floor(&self, h: usize, t: usize) -> Option<i32> {
        let f = &self.fighters[h];
        (f.side == Side::Attacker && f.combat_limit < 100).then(|| (100 - f.combat_limit.clamp(0, 100)) * self.fighters[t].max_hp)
    }

    /// The most opening volleys any fighter can have (`first_strikes + chance_first_strikes`).
    pub fn max_volleys(&self) -> i32 {
        self.fighters.iter().map(|f| f.first_strikes.max(0) + f.chance_first_strikes.max(0)).max().unwrap_or(0)
    }

    /// Fighter counts per side (attacker, defender).
    pub fn side_counts(&self) -> (usize, usize) {
        let a = self.fighters.iter().filter(|f| f.side == Side::Attacker).count();
        (a, self.fighters.len() - a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pct_and_hp_conversions() {
        assert_eq!(apply_pct(400, 50), 600);
        assert_eq!(apply_pct(400, -100), 200);
        assert_eq!(apply_pct(400, 0), 400);
        assert_eq!(hp_from_damage(25, 120), 9000);
        assert_eq!(damage_from_hp(9000, 120), 25);
        // A sliver of HP never shows 100 damage.
        assert_eq!(damage_from_hp(1, 120), 99);
        assert_eq!(damage_from_hp(0, 120), 100);
        assert_eq!(damage_from_hp(8999, 120), 25);
    }

    #[test]
    fn classes_are_disjoint_in_order() {
        let mut fl = Fighter::new(Side::Attacker, 100);
        fl.ignores_shield = true;
        fl.order = vec![TargetClass::Screen, TargetClass::Back, TargetClass::Front];
        let mut screen = Fighter::new(Side::Defender, 120);
        screen.screen = true;
        let front = Fighter::new(Side::Defender, 120);
        let mut back = Fighter::new(Side::Defender, 80);
        back.back = true;
        let b = Battle::new(Params::default(), vec![fl, screen, front, back], |_, _| Pair { a: 400, dv: 400 });
        let c = b.classes(0);
        assert_eq!(c[0].members, vec![1]);
        assert_eq!(c[1].members, vec![3]);
        assert_eq!(c[2].members, vec![2]);
        assert_eq!(b.front(Side::Defender), &[1, 2]);
        assert_eq!(b.hit_order(), &[0, 1, 2, 3]);
        assert_eq!(b.mean_damage(0, 1), 1000);
    }
}
