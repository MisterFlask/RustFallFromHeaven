//! Formation battles (phase 6, revision 4: the owner's Ogre Battle-style engagements; the browser
//! prototype's `run()` made integer and exact): the pure resolver, which knows nothing of `Game`.
//! docs/ENGAGEMENTS.md has the rules in full; in short:
//!
//! **The field.** Each side stands on a grid of 3 rows (front 0, middle 1, rear 2) by its lanes
//! ([`lanes_for`] its slots). A [`Fighter`] holds one cell, a large one two cells side by side in
//! one row (lanes `l` and `l + 1`), and is the front or back unit of both its lanes.
//!
//! **Order.** A battle is `rounds` rounds. In each, pass k (from 1 to the most actions any unit
//! has from its row at the round's start) lets every unit on the field with at least k actions
//! from its current row act once. Within a pass the faster act first (speed: the unit's, plus
//! `first_strike_speed` per first strike and per chance first strike rolled at the battle's
//! start); equal speeds by a coin each mover draws at the pass's start. One unit acts at a time
//! and each hit lands at once: a unit killed (or withdrawn, or knocked back to a row with fewer
//! actions) before its turn in a pass does not act in it. The battle ends after the last round,
//! or at once when a side has no unit left on the field.
//!
//! **An action** ([`ActionKind`]): its reach (who it can hit), its target among them
//! ([`Formation::choose`]: a flanker's anti-flank units first, then nearby first, the units within
//! one lane of the hitter, but for a charge where flanking works; then the kill or the most hurt),
//! a miss roll for physical kinds, a crit roll for the single-target
//! physical kinds, the hit (`max(1, expected x r)`, r drawn in `rand_min..=rand_max` percent, a
//! crit `crit_damage_pct` of it), a splash on the target's neighbours for units with collateral
//! damage (stronger with Barrage, weaker on a unit with Drill's protection) and spells that
//! splash, and a knock-back for a melee crit. A heal restores
//! `base_damage x heal_pct x r` to the most hurt ally. The expected damage of a hit
//! ([`Formation::expected`]) is `base_damage x 100 x A / (A + Dv)` centi-HP, then times its
//! percentages (a spell's power, the ground's, the first round's charge, a splash's) over one
//! division.
//!
//! **Withdrawal** replaces a killing hit: a unit with a withdrawal chance that a hit would kill
//! rolls it, and on a success the hit does nothing and the unit leaves the battle alive (never a
//! defender in a city). **Siege**: a unit with a combat limit below 100 never takes a target below
//! `(100 - limit)%` of its max HP, and such targets are not in its reach; a splash never takes a
//! unit below its splasher's collateral limit either.
//!
//! **RNG** ([`BattleRng`], the game's sync RNG in a real fight), in this order and nowhere else:
//! per battle, each fighter with chance first strikes in fighter order ("Engagement first
//! strikes", `0..=chance`); per pass, each mover in fighter order ("Engagement order", the coin,
//! `0..COIN`); per action, a physical one's miss ("Engagement miss", `0..100`), if it lands a crit
//! roll for strike, charge, stab and shoot ("Engagement crit"), then r ("Engagement hit"), the
//! target's withdrawal if the hit would kill it ("Engagement withdraw"), then for each splashed
//! neighbour in (lane, row) order its r and its withdrawal; a heal draws its r ("Engagement heal").
//! An action with no target in reach, a heal with no one hurt and a neighbour not splashed (at its
//! floor, or fully protected from collateral damage) draw nothing.
//!
//! Integers throughout: HP in centi-HP, A and Dv x100, percentages multiplied in `i128` and
//! divided once. No map iteration, every sort a total order (ties by fighter index last).

use ffh_data::{ActionKind, Defines, RowActions, ROWS};

use crate::rng::Rng;

#[cfg(test)]
mod tests;

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

/// The synchronized RNG, as the resolvers draw from it. The rules back it with the game's
/// `soren_rand` (the same draws as `soren_rand_num`, so `n` must fit CvRandom's `u16`); tests
/// and the preview use a seeded [`Rng`].
pub trait BattleRng {
    /// A draw in `0..n` (`1 <= n <= 65535`); `label` names it, as `getSorenRandNum`'s log string.
    fn rand(&mut self, n: i32, label: &'static str) -> i32;
}

impl BattleRng for Rng {
    fn rand(&mut self, n: i32, label: &'static str) -> i32 {
        self.get_i(n, label)
    }
}

/// A fighter's index in [`Formation::fighters`].
pub type FighterIdx = u16;

/// The sides of a coin drawn for the order of equal speeds: CvRandom's largest range (the
/// prototype's `2^20` does not fit its `u16`). Equal coins keep fighter order.
pub const COIN: i32 = 0xFFFF;

/// No chance (miss, crit) goes above this, in percent.
pub const MAX_CHANCE: i32 = 95;

/// The lanes of a side with `slots` slots: `max(2, ceil((slots + 2) / 3))`.
pub fn lanes_for(slots: i32) -> u8 {
    ((slots + 2 + 2) / 3).clamp(2, u8::MAX as i32) as u8
}

/// The per-pair strengths (`Game::engage_hit`): the hitter's A against this target and the
/// target's Dv against this hitter, both x100; for its SPELL actions, its magic and the target's
/// magic defence, x100, and the percentage of the spell's damage the target takes (100 less its
/// resistance to the spell's damage type).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Pair {
    pub a: i32,
    pub dv: i32,
    pub ma: i32,
    pub md: i32,
    pub spell_pct: i32,
}

impl Pair {
    /// A pair whose magic is its A and Dv, the spell's damage whole.
    pub fn physical(a: i32, dv: i32) -> Pair {
        Pair { a, dv, ma: a, md: dv, spell_pct: 100 }
    }
}

/// The defended plot's ground, as the battle reads it (docs/ENGAGEMENTS.md, "The ground").
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ground {
    /// A city: the attacker's arrows fly short; its defenders never withdraw; no flanking.
    pub city: bool,
    /// A forest or jungle: arrows and stones fly short on both sides; no flanking.
    pub forest: bool,
    /// Hills: the attacker's hits are weaker, the defender's arrows and stones stronger.
    pub hills: bool,
}

impl Ground {
    /// Whether flanking works here (no city, no forest or jungle): a charge reaches anyone and
    /// is harder in the first round.
    pub fn flanking(&self) -> bool {
        !self.city && !self.forest
    }

    /// The ground's rules that apply here, in the order [`GroundRule`] lists them (the battle
    /// log's and the plan's words).
    pub fn rules(&self, p: &Params) -> Vec<GroundRule> {
        let mut out = Vec::new();
        if self.city {
            out.push(GroundRule::CityShots { pct: p.city_attacker_shot_pct });
        }
        if self.forest {
            out.push(GroundRule::ForestRanged { pct: p.forest_ranged_pct });
        }
        if self.hills {
            out.push(GroundRule::HillsAttacker { pct: p.hills_attacker_pct });
            out.push(GroundRule::HillsDefenderRanged { pct: p.hills_defender_ranged_pct });
        }
        if self.flanking() {
            out.push(GroundRule::Charge { pct: 100 + p.charge_bonus });
        } else {
            out.push(GroundRule::NoFlanking);
        }
        out
    }
}

/// A rule of the ground that applies to a battle ([`Ground::rules`]; percentages of a hit).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GroundRule {
    /// A city: the attacker's SHOOT deals `pct`.
    CityShots { pct: i32 },
    /// A forest or jungle: SHOOT and BOMBARD deal `pct` on both sides.
    ForestRanged { pct: i32 },
    /// Hills: every hit of the attacker deals `pct`.
    HillsAttacker { pct: i32 },
    /// Hills: the defender's SHOOT and BOMBARD deal `pct`.
    HillsDefenderRanged { pct: i32 },
    /// Flanking works: a charge reaches anyone, and its first-round hits deal `pct`.
    Charge { pct: i32 },
    /// No flanking (a city, a forest or jungle): a charge is a strike.
    NoFlanking,
}

/// The rule constants a battle reads (the `ENGAGEMENT_*` defines; percentages in percent).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Params {
    /// B in the expected damage `B x A / (A + Dv)`, in HP.
    pub base_damage: i32,
    /// r, in percent: `rand_min..=rand_max`.
    pub rand_min: i32,
    pub rand_max: i32,
    pub rounds: i32,
    /// Speed per first strike (and per chance first strike rolled).
    pub first_strike_speed: i32,
    /// The miss chance of a physical action, and what a shot and a bombardment add to it.
    pub shoot_miss_pct: i32,
    pub bombard_miss_pct: i32,
    /// A crit's damage, in percent of the hit.
    pub crit_damage_pct: i32,
    /// What a charge adds in the first round where flanking works.
    pub charge_bonus: i32,
    /// A splash on each neighbour, in percent of the hit on the target.
    pub splash_pct: i32,
    /// The magic bolt's power (a caster that knows no damage spell).
    pub bolt_pct: i32,
    /// A heal: `base_damage x heal_pct x r`.
    pub heal_pct: i32,
    /// A missile's strike ([`missile_hit`]), in percent of a hit.
    pub missile_pct: i32,
    pub city_attacker_shot_pct: i32,
    pub forest_ranged_pct: i32,
    pub hills_attacker_pct: i32,
    pub hills_defender_ranged_pct: i32,
}

impl Default for Params {
    /// The spec's values (revision 4).
    fn default() -> Params {
        Params {
            base_damage: 40,
            rand_min: 80,
            rand_max: 120,
            rounds: 3,
            first_strike_speed: 2,
            shoot_miss_pct: 10,
            bombard_miss_pct: 15,
            crit_damage_pct: 150,
            charge_bonus: 100,
            splash_pct: 50,
            bolt_pct: 60,
            heal_pct: 50,
            missile_pct: 50,
            city_attacker_shot_pct: 50,
            forest_ranged_pct: 50,
            hills_attacker_pct: 75,
            hills_defender_ranged_pct: 125,
        }
    }
}

impl Params {
    /// The `ENGAGEMENT_*` defines.
    pub fn from_defines(d: &Defines) -> Params {
        Params {
            base_damage: d.engagement_base_damage,
            rand_min: d.engagement_damage_rand_min,
            rand_max: d.engagement_damage_rand_max.max(d.engagement_damage_rand_min),
            rounds: d.engagement_rounds,
            first_strike_speed: d.engagement_first_strike_speed,
            shoot_miss_pct: d.engagement_shoot_miss_percent,
            bombard_miss_pct: d.engagement_bombard_miss_percent,
            crit_damage_pct: d.engagement_crit_damage_percent,
            charge_bonus: d.engagement_charge_bonus,
            splash_pct: d.engagement_splash_percent,
            bolt_pct: d.engagement_bolt_percent,
            heal_pct: d.engagement_heal_percent,
            missile_pct: d.engagement_missile_percent,
            city_attacker_shot_pct: d.engagement_city_attacker_shot_percent,
            forest_ranged_pct: d.engagement_forest_ranged_percent,
            hills_attacker_pct: d.engagement_hills_attacker_percent,
            hills_defender_ranged_pct: d.engagement_hills_defender_ranged_percent,
        }
    }
}

/// What a caster casts with its SPELL actions.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Spell {
    /// The damage spell (the caller's id: the rules pass the `SpellType` index), or `None`: the
    /// magic bolt.
    pub id: Option<u32>,
    /// Power, in percent of a hit.
    pub power: i32,
    /// It also hits the target's neighbours for `splash_pct` of a hit on it.
    pub splash: bool,
}

impl Spell {
    /// A damage spell the caster knows: power 100, with a splash.
    pub fn known(id: u32) -> Spell {
        Spell { id: Some(id), power: 100, splash: true }
    }

    /// The magic bolt: `bolt_pct`, one target.
    pub fn bolt(p: &Params) -> Spell {
        Spell { id: None, power: p.bolt_pct, splash: false }
    }
}

/// One unit on its side's grid, as the resolver sees it (the rules map units onto it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fighter {
    pub side: Side,
    /// The caller's handle (the rules: the unit id), carried to the log untouched.
    pub tag: u32,
    pub lane: u8,
    /// 0 front, 1 middle, 2 rear.
    pub row: u8,
    /// Two cells side by side (`lane` and `lane + 1`).
    pub large: bool,
    /// Maximum HP (100% of the damage field).
    pub max_hp: i32,
    /// HP at the start of the battle, in centi-HP; 1..=`max_hp * 100`.
    pub hp: i32,
    /// Its speed before first strikes, which the resolver adds (the rules pass
    /// `Game::unit_engage_speed`: its type's own, else its role's).
    pub speed: i32,
    pub first_strikes: i32,
    /// Rolled at the battle's start: `0..=chance` more first strikes.
    pub chance_first_strikes: i32,
    /// What it does from each row, with a tier 4 unit's or a hero's extra action
    /// (`Game::unit_engage_actions`: its type's own rows, else its role's).
    pub actions: [RowActions; ROWS],
    /// Its A against nobody in particular, x100: the kill choice's "most dangerous", the ties and
    /// a charge's threat.
    pub a: i32,
    /// Its accuracy: its chance to hit with a physical action, before the target's evasion
    /// (`Game::unit_engage_accuracy`).
    pub accuracy: i32,
    /// Its evasion: taken off a physical hitter's chance to hit it (`Game::unit_engage_evasion`).
    pub evasion: i32,
    /// Its crit chance (`Game::unit_engage_crit`).
    pub crit: i32,
    /// Of an anti-flank role: enemy flankers take it first.
    pub anti_flank: bool,
    /// A flanker (its rows charge: `Game::unit_engage_flanks`): it takes anti-flank units in its
    /// reach first.
    pub flanker: bool,
    /// Withdrawal chance in percent (a defender in a city never withdraws, whatever this says).
    pub withdraw_pct: i32,
    /// Combat limit in percent of max HP (100: none).
    pub combat_limit: i32,
    /// Collateral damage: `Some(limit)` splashes its damaging hits on the target's neighbours,
    /// never taking one past `limit`% damage (FfH's `iCollateralDamageLimit`).
    pub collateral_limit: Option<i32>,
    /// What its collateral splash adds, in percent (Barrage's `iCollateralDamageChange`: FfH's
    /// `getExtraCollateralDamage`).
    pub collateral_extra: i32,
    /// How much less collateral splash it takes, in percent (Drill's
    /// `iCollateralDamageProtection`); at 100 or more it is never splashed by collateral damage.
    pub collateral_protection: i32,
    /// What its SPELL actions cast.
    pub spell: Spell,
    /// A Sentinel of the defended city (docs/SIEGES.md): never knocked back, never healed; it
    /// takes the shots and spells aimed past it in its lanes, and BOMBARD takes it first.
    pub fixed: bool,
}

impl Fighter {
    /// A frontline unit at full health in the front row of lane 0: speed 4, two strikes from the
    /// front, one from further back, A 400.
    pub fn new(side: Side, max_hp: i32) -> Fighter {
        let strike = |count| RowActions { kind: ActionKind::Strike, count };
        Fighter {
            side,
            tag: 0,
            lane: 0,
            row: 0,
            large: false,
            max_hp,
            hp: max_hp * 100,
            speed: 4,
            first_strikes: 0,
            chance_first_strikes: 0,
            actions: [strike(2), strike(1), strike(1)],
            a: 400,
            accuracy: 90,
            evasion: 0,
            crit: 10,
            anti_flank: false,
            flanker: false,
            withdraw_pct: 0,
            combat_limit: 100,
            collateral_limit: None,
            collateral_extra: 0,
            collateral_protection: 0,
            spell: Spell { id: None, power: 60, splash: false },
            fixed: false,
        }
    }

    /// The lanes it stands in.
    pub fn lanes(&self) -> std::ops::RangeInclusive<u8> {
        self.lane..=self.lane + self.large as u8
    }

    /// The slots it fills (a large unit two).
    pub fn slots(&self) -> i32 {
        1 + self.large as i32
    }
}

/// A value times percentages, divided once at the end.
struct Pcts {
    num: i128,
    den: i128,
}

impl Pcts {
    fn mul(&mut self, pct: i32) {
        self.num *= pct.max(0) as i128;
        self.den *= 100;
    }
}

/// The battle: its rules, ground, each side's lanes, the fighters and the pair table.
#[derive(Clone, Debug)]
pub struct Formation {
    pub params: Params,
    pub ground: Ground,
    /// Lanes per side (attacker, defender).
    pub lanes: [u8; 2],
    pub fighters: Vec<Fighter>,
    /// `n * n`, `[h * n + t]`: the pair table as given (cross-side entries only).
    pairs: Vec<Pair>,
}

/// A formation the resolver cannot play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormationError {
    /// A fighter outside its side's lanes or rows (or a large one at the last lane).
    OffGrid(FighterIdx),
    /// Two fighters of a side in one cell.
    Overlap(FighterIdx, FighterIdx),
}

impl Formation {
    /// A battle of `fighters` on `ground`, `lanes` per side, `pair(h, t)` giving the hitter's A
    /// and the target's Dv for every cross-side pair. Fighters' HP is clamped to
    /// 1..=`max_hp * 100`, `max_hp` to at least 1.
    pub fn new(params: Params, ground: Ground, lanes: [u8; 2], mut fighters: Vec<Fighter>, mut pair: impl FnMut(usize, usize) -> Pair) -> Formation {
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
        Formation { params, ground, lanes, fighters, pairs }
    }

    /// Every fighter inside its side's grid, one per cell.
    pub fn validate(&self) -> Result<(), FormationError> {
        for (i, f) in self.fighters.iter().enumerate() {
            let lanes = self.lanes[f.side.index()];
            if f.row as usize >= ROWS || *f.lanes().end() >= lanes {
                return Err(FormationError::OffGrid(i as FighterIdx));
            }
            for (j, g) in self.fighters.iter().enumerate().take(i) {
                if g.side == f.side && g.row == f.row && f.lanes().any(|l| g.lanes().contains(&l)) {
                    return Err(FormationError::Overlap(j as FighterIdx, i as FighterIdx));
                }
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.fighters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fighters.is_empty()
    }

    /// The hitter's A and the target's Dv, magic and magic defence for this pair (x100, >= 1).
    pub fn pair(&self, h: usize, t: usize) -> Pair {
        let p = self.pairs[h * self.fighters.len() + t];
        Pair { a: p.a.max(1), dv: p.dv.max(1), ma: p.ma.max(1), md: p.md.max(1), spell_pct: p.spell_pct.max(0) }
    }

    /// The ground's percentages on a hit of `kind` by `h`, multiplied into `x` (several multiply).
    fn ground_pcts(&self, h: usize, kind: ActionKind, x: &mut Pcts) {
        let (p, g) = (&self.params, &self.ground);
        let attacker = self.fighters[h].side == Side::Attacker;
        let shot_or_stone = matches!(kind, ActionKind::Shoot | ActionKind::Bombard);
        if g.city && attacker && kind == ActionKind::Shoot {
            x.mul(p.city_attacker_shot_pct);
        }
        if g.forest && shot_or_stone {
            x.mul(p.forest_ranged_pct);
        }
        if g.hills && attacker {
            x.mul(p.hills_attacker_pct);
        }
        if g.hills && !attacker && shot_or_stone {
            x.mul(p.hills_defender_ranged_pct);
        }
    }

    /// The expected damage of a hit of `kind` (as it is fought: a charge where flanking does not
    /// work is a strike) by `h` on `t` in round `round`, in centi-HP: `base_damage x 100 x A /
    /// (A + Dv)`, then times the spell's power, the ground's percentages, the first round's
    /// charge bonus and (`splash`) the splash percentage, for a collateral splash also times
    /// `100 + h`'s collateral extra and `100 - t`'s collateral protection, divided once.
    pub fn expected(&self, h: usize, t: usize, kind: ActionKind, round: i32, splash: bool) -> i32 {
        let p = self.pair(h, t);
        // A spell is the caster's magic against the target's magic defence (docs/UNIT_STATS.md).
        let (a, dv) = if kind == ActionKind::Spell { (p.ma as i64, p.md as i64) } else { (p.a as i64, p.dv as i64) };
        let mean = self.params.base_damage.max(0) as i64 * 100 * a / (a + dv);
        let mut x = Pcts { num: mean as i128, den: 1 };
        if kind == ActionKind::Spell {
            x.mul(self.fighters[h].spell.power);
            x.mul(p.spell_pct);
        }
        self.ground_pcts(h, kind, &mut x);
        if kind == ActionKind::Charge && round == 1 && self.ground.flanking() {
            x.mul(100 + self.params.charge_bonus);
        }
        if splash {
            x.mul(self.params.splash_pct);
            if self.collateral_splash(h, kind) {
                x.mul(100 + self.fighters[h].collateral_extra);
                x.mul(100 - self.fighters[t].collateral_protection);
            }
        }
        (x.num / x.den).min(i32::MAX as i128) as i32
    }

    /// The expected damage of a missile's strike by `h` on `t` ([`missile_hit`]), in centi-HP:
    /// `base_damage x 100 x A / (A + Dv)` times `missile_pct`. The ground's percentages do not
    /// apply: it strikes from outside the field (the rules put the plot's own defence in its
    /// target's Dv instead, as before formation battles).
    pub fn missile_expected(&self, h: usize, t: usize) -> i32 {
        let p = self.pair(h, t);
        let (a, dv) = (p.a as i64, p.dv as i64);
        let mean = self.params.base_damage.max(0) as i64 * 100 * a / (a + dv);
        let mut x = Pcts { num: mean as i128, den: 1 };
        x.mul(self.params.missile_pct);
        (x.num / x.den).min(i32::MAX as i128) as i32
    }

    /// Whether `h`'s hits of `kind` splash as collateral damage: it has collateral damage, and
    /// the hit is not a known spell's (which splashes as the spell: no collateral limit, Barrage
    /// or protection).
    pub fn collateral_splash(&self, h: usize, kind: ActionKind) -> bool {
        let fh = &self.fighters[h];
        fh.collateral_limit.is_some() && !(kind == ActionKind::Spell && fh.spell.splash)
    }

    /// The HP (centi-HP) `h`'s hits never take `t` below: `(100 - limit)%` of its max HP for a
    /// hitter with a combat limit below 100, else 0.
    pub fn floor(&self, h: usize, t: usize) -> i32 {
        let limit = self.fighters[h].combat_limit;
        if limit < 100 {
            (100 - limit.clamp(0, 100)) * self.fighters[t].max_hp
        } else {
            0
        }
    }

    /// The HP (centi-HP) `h`'s collateral splash never takes `t` below: its combat limit's floor
    /// and its collateral limit's, the higher.
    pub fn splash_floor(&self, h: usize, t: usize) -> i32 {
        let collateral = self.fighters[h].collateral_limit.map_or(0, |limit| (100 - limit.clamp(0, 100)) * self.fighters[t].max_hp);
        self.floor(h, t).max(collateral)
    }

    /// The miss chance of a physical action of `kind` by `h` on `t`, in percent: 100 less its
    /// chance to hit, `h`'s accuracy less `t`'s evasion and what the kind takes off (SHOOT,
    /// BOMBARD), within `100 - MAX_CHANCE..=100` (docs/UNIT_STATS.md).
    pub fn miss_chance(&self, kind: ActionKind, h: usize, t: usize) -> i32 {
        let p = &self.params;
        let extra = match kind {
            ActionKind::Shoot => p.shoot_miss_pct,
            ActionKind::Bombard => p.bombard_miss_pct,
            _ => 0,
        };
        100 - (self.fighters[h].accuracy - self.fighters[t].evasion - extra).clamp(100 - MAX_CHANCE, 100)
    }

    /// `h`'s crit chance on a strike, charge, stab or shoot, in percent (at most `MAX_CHANCE`).
    pub fn crit_chance(&self, h: usize) -> i32 {
        self.fighters[h].crit.clamp(0, MAX_CHANCE)
    }
}

/// A battle's state as the resolver plays it.
#[derive(Clone, Debug)]
pub struct State {
    /// Centi-HP (0 for the dead).
    pub hp: Vec<i32>,
    /// The row each fighter acts from (knock-backs move it).
    pub row: Vec<u8>,
    pub alive: Vec<bool>,
    /// Left the battle alive (withdrew).
    pub gone: Vec<bool>,
    /// Speed after the battle's first strike rolls.
    pub speed: Vec<i32>,
}

impl State {
    /// The battle's start: every fighter on the field in its row with its HP and its speed
    /// before first strikes.
    pub fn new(f: &Formation) -> State {
        State {
            hp: f.fighters.iter().map(|x| x.hp).collect(),
            row: f.fighters.iter().map(|x| x.row).collect(),
            alive: vec![true; f.len()],
            gone: vec![false; f.len()],
            speed: f.fighters.iter().map(|x| x.speed).collect(),
        }
    }

    /// Alive and not withdrawn.
    pub fn on_field(&self, i: usize) -> bool {
        self.alive[i] && !self.gone[i]
    }
}

/// Why an action did nothing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Note {
    /// No enemy in reach (or every one at the hitter's combat limit).
    NoTarget,
    /// A heal, and no ally is hurt.
    NoOneHurt,
}

/// One hit of an action, on its target or (`splash`) a neighbour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub target: FighterIdx,
    /// The damage dealt, centi-HP: the HP it took (a killing hit: the HP the target had); 0 for a
    /// miss or a withdrawal.
    pub dmg: i32,
    /// The hit's expected damage (centi-HP, before r and a crit; 0 for a miss).
    pub expected: i32,
    /// r, in percent (0 for a miss).
    pub r: i32,
    pub crit: bool,
    pub miss: bool,
    pub killed: bool,
    pub withdrew: bool,
    pub splash: bool,
}

/// A heal: `amount` centi-HP restored to `target` (r in percent).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Heal {
    pub target: FighterIdx,
    pub amount: i32,
    pub r: i32,
}

/// A knock-back: `unit` moved from row `from` to `to`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Knock {
    pub unit: FighterIdx,
    pub from: u8,
    pub to: u8,
}

/// One action, in resolution order (the battle log's event; spec section 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// Round and pass, both from 1.
    pub round: u8,
    pub pass: u8,
    pub actor: FighterIdx,
    /// The kind as fought (a charge where flanking does not work is a strike).
    pub kind: ActionKind,
    /// A spell's id ([`Spell::id`]; `None`: the magic bolt or not a spell).
    pub spell: Option<u32>,
    /// The target chosen (the centre of a splash, the ally healed).
    pub target: Option<FighterIdx>,
    /// The hits, the target's first, then the splash's in (lane, row) order.
    pub hits: Vec<Hit>,
    pub heal: Option<Heal>,
    pub knocked: Option<Knock>,
    pub note: Option<Note>,
}

/// What became of a fighter.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fate {
    /// On the field at the end.
    Standing,
    /// Killed by `by` in round `round`.
    Fell { by: FighterIdx, round: u8 },
    /// Left the battle alive in round `round` (its withdrawal took the place of a killing hit).
    Withdrew { round: u8 },
}

/// A kill, in kill order.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Kill {
    pub victim: FighterIdx,
    pub killer: FighterIdx,
    pub round: u8,
}

/// A fighter at the end of the battle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct End {
    /// HP left, centi-HP (0: dead).
    pub hp: i32,
    /// The row it ended in.
    pub row: u8,
    /// Its speed in the battle and the chance first strikes it rolled.
    pub speed: i32,
    pub rolled_first_strikes: i32,
    pub fate: Fate,
    /// Damage it dealt and took (centi-HP; a killing hit counts the HP the victim had).
    pub dealt: i32,
    pub taken: i32,
    /// HP its heals restored, and HP it got back from heals (centi-HP).
    pub healed: i32,
    pub was_healed: i32,
}

/// The outcome of [`resolve`]: each fighter's end, the kills, the damage shares, whether each
/// side was cleared from the field, and (unless resolved quietly) the events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub fighters: Vec<End>,
    pub kills: Vec<Kill>,
    /// Rounds begun (0: a side had nobody at the start).
    pub rounds: u8,
    /// Per side (attacker, defender): no unit left on the field at the end.
    pub down: [bool; 2],
    /// Empty when resolved quietly.
    pub events: Vec<Event>,
    /// `n * n`: damage `h` dealt `t` (centi-HP; the shares for battle XP).
    dealt: Vec<i32>,
}

impl Outcome {
    /// The damage `h` dealt `t` in the battle (centi-HP).
    pub fn dealt(&self, h: usize, t: usize) -> i32 {
        self.dealt[h * self.fighters.len() + t]
    }

    /// Fighters killed on each side (attacker, defender).
    pub fn losses(&self, f: &Formation) -> [usize; 2] {
        let mut l = [0; 2];
        for (x, e) in f.fighters.iter().zip(&self.fighters) {
            if matches!(e.fate, Fate::Fell { .. }) {
                l[x.side.index()] += 1;
            }
        }
        l
    }
}

/// Plays the battle on `rng`, with its events.
pub fn resolve(f: &Formation, rng: &mut impl BattleRng) -> Outcome {
    Run::new(f, true).run(rng)
}

/// [`resolve`] without the events (the same draws and the same outcome otherwise).
pub fn resolve_quiet(f: &Formation, rng: &mut impl BattleRng) -> Outcome {
    Run::new(f, false).run(rng)
}

/// A missile's strike (Q11: Fireball, Meteor): fighter `h` (alone on its side) hits one enemy
/// once, as a STRIKE from outside the field: the front unit of an enemy lane, by the normal
/// choice without nearby first, at its own strength ([`Formation::missile_expected`]: no ground
/// rules); no miss, no crit, no splash, no withdrawal; r drawn ("Engagement hit"), never below its
/// combat limit's floor. `None` (nothing drawn): no target in reach.
pub fn missile_hit(f: &Formation, h: usize, rng: &mut impl BattleRng) -> Option<Hit> {
    let s = State::new(f);
    let list = f.reach(&s, h, ActionKind::Strike);
    let t = f.choose_with(&s, h, ActionKind::Strike, 1, &list, true)?;
    let p = &f.params;
    let expected = f.missile_expected(h, t);
    let r = p.rand_min + rng.rand(p.rand_max - p.rand_min + 1, "Engagement hit");
    let raw = (expected as i64 * r as i64 / 100).clamp(1, i32::MAX as i64) as i32;
    let dmg = raw.min((s.hp[t] - f.floor(h, t)).max(0));
    Some(Hit { target: t as FighterIdx, dmg, expected, r, crit: false, miss: false, killed: dmg >= s.hp[t], withdrew: false, splash: false })
}

struct Run<'a> {
    f: &'a Formation,
    s: State,
    log: bool,
    events: Vec<Event>,
    kills: Vec<Kill>,
    dealt: Vec<i32>,
    fate: Vec<Fate>,
    rolled: Vec<i32>,
    healed: Vec<i32>,
    was_healed: Vec<i32>,
    round: u8,
    pass: u8,
}

impl Formation {
    /// The actions `i` has from row `row`.
    fn actions_at(&self, i: usize, row: u8) -> RowActions {
        self.fighters[i].actions.get(row as usize).copied().unwrap_or_default()
    }

    /// `t`'s threat for a charge: its A times its actions from its current row.
    pub fn threat(&self, s: &State, t: usize) -> i64 {
        let a = self.actions_at(t, s.row[t]);
        if a.kind == ActionKind::None {
            0
        } else {
            self.fighters[t].a.max(0) as i64 * a.count.max(0) as i64
        }
    }

    /// The units on the field of side `side` standing in lane `lane`.
    fn lane_units<'s>(&'s self, s: &'s State, side: Side, lane: u8) -> impl Iterator<Item = usize> + 's {
        (0..self.len()).filter(move |&i| s.on_field(i) && self.fighters[i].side == side && self.fighters[i].lanes().contains(&lane))
    }

    /// Who an action of `kind` (as fought) by `h` can hit: STRIKE and a CHARGE where flanking
    /// does not work the front unit of each enemy lane; SHOOT, SPELL and BOMBARD the backmost,
    /// but a Sentinel standing in a lane takes the SHOOT and SPELL aimed down it, and BOMBARD
    /// takes the Sentinels on the field first (docs/SIEGES.md); STAB and a CHARGE where flanking
    /// works every enemy on the field; less the targets at `h`'s combat limit. In fighter order.
    pub fn reach(&self, s: &State, h: usize, kind: ActionKind) -> Vec<usize> {
        let foe = self.fighters[h].side.other();
        let mut list: Vec<usize> = Vec::new();
        let everyone = kind == ActionKind::Stab || (kind == ActionKind::Charge && self.ground.flanking());
        if kind == ActionKind::Bombard {
            let walls: Vec<usize> = (0..self.len()).filter(|&t| s.on_field(t) && self.fighters[t].side == foe && self.fighters[t].fixed && s.hp[t] > self.floor(h, t)).collect();
            if !walls.is_empty() {
                return walls;
            }
        }
        if everyone {
            list.extend((0..self.len()).filter(|&t| s.on_field(t) && self.fighters[t].side == foe));
        } else if matches!(kind, ActionKind::Strike | ActionKind::Charge) || kind.ranged() {
            for lane in 0..self.lanes[foe.index()] {
                let mut units = self.lane_units(s, foe, lane);
                let pick = if kind.ranged() {
                    let units: Vec<usize> = units.collect();
                    let wall = (kind != ActionKind::Bombard).then(|| units.iter().copied().find(|&t| self.fighters[t].fixed)).flatten();
                    wall.or_else(|| units.iter().copied().max_by_key(|&t| s.row[t]))
                } else {
                    units.by_ref().min_by_key(|&t| s.row[t])
                };
                if let Some(t) = pick {
                    list.push(t);
                }
            }
            list.sort_unstable();
            list.dedup();
        }
        list.retain(|&t| s.hp[t] > self.floor(h, t));
        list
    }

    /// The (lane, row) order of the field: ties go by it, never by list order.
    fn place(&self, s: &State, x: usize, y: usize) -> std::cmp::Ordering {
        (self.fighters[x].lane, s.row[x], x).cmp(&(self.fighters[y].lane, s.row[y], y))
    }

    /// The lane distance of `x` and `y` (of either side: lanes are numbered the same on both): the
    /// least over their lanes (a large unit's are both of its lanes).
    pub fn lane_distance(&self, x: usize, y: usize) -> u8 {
        let (fx, fy) = (&self.fighters[x], &self.fighters[y]);
        fx.lanes().flat_map(|a| fy.lanes().map(move |c| a.abs_diff(c))).min().unwrap_or(0)
    }

    /// `h`'s target among `list` for an action of `kind` (as fought) in round `round` (spec 1.5):
    /// a flanker takes the anti-flank units of the list first; then, but for a charge where
    /// flanking works, the units within one lane of it come first (nearby first: the rest only
    /// when none of them is in the list); then a charge where flanking works takes the highest
    /// threat for its HP left; otherwise, if this hit's expected damage would kill some, the one
    /// with the highest A among them, else the one it hurts most for its HP left. Ties by the
    /// higher A, then lane, then row. The expected damage is capped at what `h`'s combat limit
    /// lets it take (a siege engine kills no one). A splash's centre is chosen so too.
    pub fn choose(&self, s: &State, h: usize, kind: ActionKind, round: i32, list: &[usize]) -> Option<usize> {
        self.choose_with(s, h, kind, round, list, false)
    }

    /// [`Formation::choose`], for a hit on the field or (`missile`) a missile's strike, which
    /// comes from outside the field: no nearby first, its own expected damage
    /// ([`Formation::missile_expected`]).
    pub fn choose_with(&self, s: &State, h: usize, kind: ActionKind, round: i32, list: &[usize], missile: bool) -> Option<usize> {
        use std::cmp::Ordering;
        let mut pool: Vec<usize> = list.to_vec();
        if self.fighters[h].flanker && pool.iter().any(|&t| self.fighters[t].anti_flank) {
            pool.retain(|&t| self.fighters[t].anti_flank);
        }
        if !missile && !(kind == ActionKind::Charge && self.ground.flanking()) && pool.iter().any(|&t| self.lane_distance(h, t) <= 1) {
            pool.retain(|&t| self.lane_distance(h, t) <= 1);
        }
        let a = |t: usize| self.fighters[t].a;
        let hp = |t: usize| s.hp[t].max(1) as i64;
        let tie = |x: usize, y: usize| a(y).cmp(&a(x)).then_with(|| self.place(s, x, y));
        if kind == ActionKind::Charge && self.ground.flanking() {
            // threat_x / hp_x > threat_y / hp_y  <=>  threat_x * hp_y > threat_y * hp_x
            let scored: Vec<(usize, i64)> = pool.iter().map(|&t| (t, self.threat(s, t))).collect();
            return scored.into_iter().min_by(|&(x, tx), &(y, ty)| (ty * hp(x)).cmp(&(tx * hp(y))).then_with(|| tie(x, y))).map(|(t, _)| t);
        }
        // Each candidate's expected damage, capped at what the hitter's limit lets it take, and
        // whether it would kill.
        let scored: Vec<(usize, i64, bool)> = pool
            .iter()
            .map(|&t| {
                let floor = self.floor(h, t);
                let e = if missile { self.missile_expected(h, t) } else { self.expected(h, t, kind, round, false) };
                let e = (e as i64).min((s.hp[t] - floor).max(0) as i64);
                (t, e, floor == 0 && e >= s.hp[t] as i64)
            })
            .collect();
        // share_x = e_x / hp_x, compared by cross-multiplication
        let by_share = |(x, ex): (usize, i64), (y, ey): (usize, i64)| -> Ordering { (ey * hp(x)).cmp(&(ex * hp(y))) };
        if scored.iter().any(|c| c.2) {
            return scored
                .into_iter()
                .filter(|c| c.2)
                .min_by(|&(x, ex, _), &(y, ey, _)| a(y).cmp(&a(x)).then_with(|| by_share((x, ex), (y, ey))).then_with(|| self.place(s, x, y)))
                .map(|c| c.0);
        }
        scored.into_iter().min_by(|&(x, ex, _), &(y, ey, _)| by_share((x, ex), (y, ey)).then_with(|| tie(x, y))).map(|c| c.0)
    }

    /// The neighbours of `c` on the field: beside it in its row, in front of and behind it in
    /// its lanes; in (lane, row) order.
    pub fn neighbours(&self, s: &State, c: usize) -> Vec<usize> {
        let fc = &self.fighters[c];
        let mut out: Vec<usize> = (0..self.len())
            .filter(|&t| t != c && s.on_field(t) && self.fighters[t].side == fc.side)
            .filter(|&t| {
                let ft = &self.fighters[t];
                let shares = fc.lanes().any(|l| ft.lanes().contains(&l));
                if s.row[t] == s.row[c] {
                    !shares && fc.lanes().any(|l| ft.lanes().any(|m| (l as i32 - m as i32).abs() == 1))
                } else {
                    (s.row[t] as i32 - s.row[c] as i32).abs() == 1 && shares
                }
            })
            .collect();
        out.sort_by(|&x, &y| self.place(s, x, y));
        out
    }

    /// The most hurt of `h`'s allies on the field (itself included; the most HP missing, ties by
    /// lane then row), if any is hurt.
    pub fn heal_target(&self, s: &State, h: usize) -> Option<usize> {
        let side = self.fighters[h].side;
        let missing = |t: usize| self.fighters[t].max_hp * 100 - s.hp[t];
        (0..self.len())
            .filter(|&t| s.on_field(t) && self.fighters[t].side == side && !self.fighters[t].fixed && missing(t) > 0)
            .min_by(|&x, &y| missing(y).cmp(&missing(x)).then_with(|| self.place(s, x, y)))
    }

    fn side_down(&self, s: &State, side: Side) -> bool {
        !(0..self.len()).any(|i| self.fighters[i].side == side && s.on_field(i))
    }
}

impl<'a> Run<'a> {
    fn new(f: &'a Formation, log: bool) -> Run<'a> {
        let n = f.len();
        Run {
            f,
            s: State::new(f),
            log,
            events: Vec::new(),
            kills: Vec::new(),
            dealt: vec![0; n * n],
            fate: vec![Fate::Standing; n],
            rolled: vec![0; n],
            healed: vec![0; n],
            was_healed: vec![0; n],
            round: 0,
            pass: 0,
        }
    }

    fn run(mut self, rng: &mut impl BattleRng) -> Outcome {
        let f = self.f;
        debug_assert_eq!(f.validate(), Ok(()));
        let p = f.params;
        // The battle's start: chance first strikes, in fighter order.
        for i in 0..f.len() {
            let x = &f.fighters[i];
            if x.chance_first_strikes > 0 {
                self.rolled[i] = rng.rand(x.chance_first_strikes + 1, "Engagement first strikes");
            }
            self.s.speed[i] = x.speed + p.first_strike_speed * (x.first_strikes.max(0) + self.rolled[i]);
        }
        let mut ended = f.side_down(&self.s, Side::Attacker) || f.side_down(&self.s, Side::Defender);
        let mut round = 1;
        while round <= p.rounds && !ended {
            self.round = round as u8;
            let passes = (0..f.len())
                .filter(|&i| self.s.on_field(i))
                .map(|i| f.actions_at(i, self.s.row[i]))
                .filter(|a| a.kind != ActionKind::None)
                .map(|a| a.count)
                .max()
                .unwrap_or(0);
            for k in 1..=passes {
                self.pass = k as u8;
                let acts = |s: &State, i: usize| {
                    let a = f.actions_at(i, s.row[i]);
                    s.on_field(i) && a.kind != ActionKind::None && a.count >= k
                };
                let mut movers: Vec<(usize, i32)> = Vec::new();
                for i in 0..f.len() {
                    if acts(&self.s, i) {
                        movers.push((i, rng.rand(COIN, "Engagement order")));
                    }
                }
                movers.sort_by(|&(x, cx), &(y, cy)| self.s.speed[y].cmp(&self.s.speed[x]).then(cx.cmp(&cy)).then(x.cmp(&y)));
                for (m, _) in movers {
                    if !acts(&self.s, m) {
                        continue;
                    }
                    self.act(m, rng);
                    if f.side_down(&self.s, Side::Attacker) || f.side_down(&self.s, Side::Defender) {
                        ended = true;
                        break;
                    }
                }
                if ended {
                    break;
                }
            }
            round += 1;
        }
        let n = f.len();
        let fighters = (0..n)
            .map(|i| {
                let taken = (0..n).map(|h| self.dealt[h * n + i]).sum();
                let dealt = (0..n).map(|t| self.dealt[i * n + t]).sum();
                End {
                    hp: self.s.hp[i],
                    row: self.s.row[i],
                    speed: self.s.speed[i],
                    rolled_first_strikes: self.rolled[i],
                    fate: self.fate[i],
                    dealt,
                    taken,
                    healed: self.healed[i],
                    was_healed: self.was_healed[i],
                }
            })
            .collect();
        Outcome {
            fighters,
            kills: self.kills,
            rounds: self.round,
            down: [f.side_down(&self.s, Side::Attacker), f.side_down(&self.s, Side::Defender)],
            events: self.events,
            dealt: self.dealt,
        }
    }

    /// Fighter `h` acts once from its current row.
    fn act(&mut self, h: usize, rng: &mut impl BattleRng) {
        let f = self.f;
        let round = self.round as i32;
        let row_kind = f.actions_at(h, self.s.row[h]).kind;
        let kind = if row_kind == ActionKind::Charge && !f.ground.flanking() { ActionKind::Strike } else { row_kind };
        let mut ev = Event {
            round: self.round,
            pass: self.pass,
            actor: h as FighterIdx,
            kind,
            spell: if kind == ActionKind::Spell { f.fighters[h].spell.id } else { None },
            target: None,
            hits: Vec::new(),
            heal: None,
            knocked: None,
            note: None,
        };
        if kind == ActionKind::Heal {
            match f.heal_target(&self.s, h) {
                None => ev.note = Some(Note::NoOneHurt),
                Some(t) => {
                    let p = &f.params;
                    let r = p.rand_min + rng.rand(p.rand_max - p.rand_min + 1, "Engagement heal");
                    let amount = (p.base_damage.max(0) as i64 * p.heal_pct.max(0) as i64 * r as i64 / 100) as i32;
                    let got = amount.min(f.fighters[t].max_hp * 100 - self.s.hp[t]).max(0);
                    self.s.hp[t] += got;
                    self.healed[h] += got;
                    self.was_healed[t] += got;
                    ev.target = Some(t as FighterIdx);
                    ev.heal = Some(Heal { target: t as FighterIdx, amount: got, r });
                }
            }
            self.push(ev);
            return;
        }
        let list = f.reach(&self.s, h, kind);
        let Some(c) = f.choose(&self.s, h, kind, round, &list) else {
            ev.note = Some(Note::NoTarget);
            self.push(ev);
            return;
        };
        ev.target = Some(c as FighterIdx);
        if kind.physical() && rng.rand(100, "Engagement miss") < f.miss_chance(kind, h, c) {
            ev.hits.push(Hit { target: c as FighterIdx, dmg: 0, expected: 0, r: 0, crit: false, miss: true, killed: false, withdrew: false, splash: false });
            self.push(ev);
            return;
        }
        let crit = kind.can_crit() && rng.rand(100, "Engagement crit") < f.crit_chance(h);
        let spell_splash = kind == ActionKind::Spell && f.fighters[h].spell.splash;
        let collateral = f.collateral_splash(h, kind);
        let around = if spell_splash || collateral { f.neighbours(&self.s, c) } else { Vec::new() };
        let hit = self.hit(h, c, kind, crit, false, f.floor(h, c), rng);
        ev.hits.push(hit);
        for t in around {
            let floor = if collateral { f.splash_floor(h, t) } else { f.floor(h, t) };
            let shielded = collateral && f.fighters[t].collateral_protection >= 100;
            if self.s.on_field(t) && self.s.hp[t] > floor && !shielded {
                let hit = self.hit(h, t, kind, false, true, floor, rng);
                ev.hits.push(hit);
            }
        }
        if crit && kind.melee() {
            ev.knocked = self.knock_back(c);
        }
        self.push(ev);
    }

    /// One hit of `h` on `t` (`crit`, a `splash`), never below `floor`: r, then a withdrawal from
    /// a killing hit.
    #[allow(clippy::too_many_arguments)]
    fn hit(&mut self, h: usize, t: usize, kind: ActionKind, crit: bool, splash: bool, floor: i32, rng: &mut impl BattleRng) -> Hit {
        let f = self.f;
        let p = &f.params;
        let expected = f.expected(h, t, kind, self.round as i32, splash);
        let r = p.rand_min + rng.rand(p.rand_max - p.rand_min + 1, "Engagement hit");
        let pct = if crit { p.crit_damage_pct.max(0) as i64 } else { 100 };
        let raw = (expected as i64 * pct * r as i64 / 10_000).clamp(1, i32::MAX as i64) as i32;
        let dmg = raw.min((self.s.hp[t] - floor).max(0));
        let mut out = Hit { target: t as FighterIdx, dmg, expected, r, crit, miss: false, killed: false, withdrew: false, splash };
        if dmg >= self.s.hp[t] {
            let x = &f.fighters[t];
            let may_flee = x.withdraw_pct > 0 && !(x.side == Side::Defender && f.ground.city);
            if may_flee && rng.rand(100, "Engagement withdraw") < x.withdraw_pct {
                self.s.gone[t] = true;
                self.fate[t] = Fate::Withdrew { round: self.round };
                out.withdrew = true;
                out.dmg = 0;
            } else {
                out.dmg = self.s.hp[t];
                self.dealt[h * f.len() + t] += self.s.hp[t];
                self.s.hp[t] = 0;
                self.s.alive[t] = false;
                self.fate[t] = Fate::Fell { by: h as FighterIdx, round: self.round };
                self.kills.push(Kill { victim: t as FighterIdx, killer: h as FighterIdx, round: self.round });
                out.killed = true;
            }
        } else {
            self.s.hp[t] -= dmg;
            self.dealt[h * f.len() + t] += dmg;
        }
        out
    }

    /// A melee crit's knock-back: `t` moves back a row if it is on the field, not in the rear and
    /// every cell behind it is free.
    fn knock_back(&mut self, t: usize) -> Option<Knock> {
        let f = self.f;
        let row = self.s.row[t];
        if !self.s.on_field(t) || row as usize + 1 >= ROWS || f.fighters[t].fixed {
            return None;
        }
        let ft = &f.fighters[t];
        let blocked = (0..f.len()).any(|u| {
            u != t && self.s.on_field(u) && f.fighters[u].side == ft.side && self.s.row[u] == row + 1 && f.fighters[u].lanes().any(|l| ft.lanes().contains(&l))
        });
        if blocked {
            return None;
        }
        self.s.row[t] = row + 1;
        Some(Knock { unit: t as FighterIdx, from: row, to: row + 1 })
    }

    fn push(&mut self, ev: Event) {
        if self.log {
            self.events.push(ev);
        }
    }
}

/// The auto formation's order of a unit's kind (the action of the row it prefers), lowest first:
/// anti-flanking strikers, strikers, shooters, chargers, casters, healers, bombards, stabbers
/// (docs/ENGAGEMENTS.md, "The auto formation").
pub fn kind_pick_order(kind: ActionKind, anti_flank: bool) -> i32 {
    match kind {
        ActionKind::Strike if anti_flank => 1,
        ActionKind::Strike => 2,
        ActionKind::Shoot => 3,
        ActionKind::Charge => 4,
        ActionKind::Spell => 5,
        ActionKind::Heal => 6,
        ActionKind::Bombard => 7,
        ActionKind::Stab => 8,
        ActionKind::None => 9,
    }
}

/// A unit the auto formation may place ([`auto_arrange`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Its kind's place in the auto formation's order ([`kind_pick_order`], lowest first).
    pub pick_order: i32,
    /// Its rank preference: the rows in the order it is placed in them (`engage_row_order`).
    pub row_order: [u8; ROWS],
    pub large: bool,
    /// Its strength with its promotions (any consistent scale; a large unit counts double).
    pub value: i64,
    /// Not below `ENGAGEMENT_AUTO_MIN_HP_PERCENT` of its HP: preferred within its kind.
    pub healthy: bool,
    /// It may fill the river slots (`bRiver`: it crosses rivers freely): only such units fill the
    /// slots above `ordinary` ([`auto_arrange`]).
    pub river: bool,
}

/// Where the auto formation puts a candidate.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Index into the candidates.
    pub unit: usize,
    pub lane: u8,
    pub row: u8,
}

/// The rules' auto formation (the prototype's `autoArrange`): one unit of each kind first, in
/// the kinds' pick order, then the rest in that order, each kind's healthy units first and the
/// strongest first (a large unit's value counts double), while the slots last (a large unit
/// needs 2; units that cannot fill river slots at most `ordinary` of them: Q9's river rule);
/// then the large units placed first, each unit in the first row of its rank preference with
/// room, the lanes nearest the centre first (then the lower). A unit with no room is left out.
/// In candidate order.
pub fn auto_arrange(cands: &[Candidate], lanes: u8, slots: i32, ordinary: i32) -> Vec<Placement> {
    auto_arrange_around(cands, lanes, slots, ordinary, &[])
}

/// The cells of `n` Sentinels on a side of `lanes` lanes (docs/SIEGES.md): the front row, the
/// lanes nearest the centre first (then the lower); any beyond the front row's lanes in the
/// middle row the same way.
pub fn sentinel_cells(lanes: u8, n: usize) -> Vec<(u8, u8)> {
    let centre = |l: u8| (2 * l as i32 - (lanes as i32 - 1)).abs();
    let mut order: Vec<u8> = (0..lanes).collect();
    order.sort_by_key(|&l| (centre(l), l));
    (0..ROWS as u8).flat_map(|r| order.iter().map(move |&l| (l, r))).take(n).collect()
}

/// [`auto_arrange`] with the cells in `blocked` already taken (a city's Sentinels).
pub fn auto_arrange_around(cands: &[Candidate], lanes: u8, slots: i32, ordinary: i32, blocked: &[(u8, u8)]) -> Vec<Placement> {
    let value = |c: &Candidate| c.value * if c.large { 2 } else { 1 };
    let mut order: Vec<usize> = (0..cands.len()).collect();
    order.sort_by(|&x, &y| {
        let (a, b) = (&cands[x], &cands[y]);
        a.pick_order.cmp(&b.pick_order).then(b.healthy.cmp(&a.healthy)).then(value(b).cmp(&value(a))).then(x.cmp(&y))
    });
    let mut first: Vec<usize> = Vec::new();
    let mut rest: Vec<usize> = Vec::new();
    for &i in &order {
        if first.iter().any(|&j| cands[j].pick_order == cands[i].pick_order) {
            rest.push(i);
        } else {
            first.push(i);
        }
    }
    let (mut used, mut non_river) = (0, 0);
    let mut chosen: Vec<usize> = Vec::new();
    for i in first.into_iter().chain(rest) {
        let size = 1 + cands[i].large as i32;
        let river = cands[i].river;
        if used + size <= slots && (river || non_river + size <= ordinary) {
            chosen.push(i);
            used += size;
            if !river {
                non_river += size;
            }
        }
    }
    let lanes_n = lanes as usize;
    let mut grid = vec![[false; ROWS]; lanes_n];
    for &(l, r) in blocked {
        if let Some(cell) = grid.get_mut(l as usize).and_then(|g| g.get_mut(r as usize)) {
            *cell = true;
        }
    }
    // twice the distance from the centre lane: |2l - (lanes - 1)|, a large unit's from l + 1/2
    let centre = |l: usize, large: bool| (2 * l as i32 + large as i32 - (lanes as i32 - 1)).abs();
    let mut out: Vec<Placement> = Vec::new();
    let larges = chosen.iter().copied().filter(|&i| cands[i].large);
    let smalls = chosen.iter().copied().filter(|&i| !cands[i].large);
    for i in larges.chain(smalls).collect::<Vec<_>>() {
        let c = &cands[i];
        let mut cell = None;
        for r in c.row_order {
            let r = (r as usize).min(ROWS - 1);
            let mut free: Vec<usize> =
                (0..lanes_n).filter(|&l| if c.large { l + 1 < lanes_n && !grid[l][r] && !grid[l + 1][r] } else { !grid[l][r] }).collect();
            free.sort_by_key(|&l| (centre(l, c.large), l));
            if let Some(&l) = free.first() {
                cell = Some((l, r));
                break;
            }
        }
        if let Some((l, r)) = cell {
            grid[l][r] = true;
            if c.large {
                grid[l + 1][r] = true;
            }
            out.push(Placement { unit: i, lane: l as u8, row: r as u8 });
        }
    }
    out.sort_by_key(|p| p.unit);
    out
}
