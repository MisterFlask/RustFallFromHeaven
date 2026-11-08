//! The battle magic of the great-person deck's battle cards (docs/CARDS.md, the battle magic
//! batch; their data keys: `ffh_data::BattleMagic`), as the resolver plays it. The rest of the
//! resolver calls in here at a few places (each marked "Card:" in `mod.rs`):
//!
//! - **The battle's start** ([`Run::magic_start`]): the pre-battle hexes of a fighter's presence
//!   (Gravity Well's grounding: the enemy's charges are strikes for the battle; the Whisperer's
//!   madness: the enemy fighter with the highest A, not a Sentinel nor an illusion, hits its own
//!   side in round 1), unless the enemy side has a hex ward on the field (the Abjurer); then the
//!   opening casts (Contingency): each fighter with one casts its damage spell once, round 0, in
//!   fighter order.
//! - **Each round's start** ([`Run::magic_round`]): a side with an Abjurer on the field ends the
//!   enemy side's charms (they are not cast again: a caster casts each charm once); the madness
//!   ends after round 1.
//! - **An action** ([`Run::magic_skip`], [`Run::magic_kind`], [`Run::magic_summon`],
//!   [`Run::magic_intercept`]): a fighter under the Lullaby skips its next actions; a grounded
//!   charge is a strike; a Spellsword's first strike each round is its spell; a caster with
//!   illusions casts them with its first spell action (they take their cells if free); a familiar
//!   takes the first spell or shot aimed at its caster.
//! - **A charm or hex** ([`Run::magic_charm_targets`], [`Run::magic_charm_landed`]): its rows
//!   (front, rear or the whole side), a warded side's immunity to hexes, the actions it makes
//!   skip, and the record of what landed (the Abjurer and Spellbreaker end charms).
//! - **A hit** ([`Run::magic_hit_pct`], [`Run::magic_landed`], [`Run::magic_recall`]): a
//!   dispelled fighter's A, Dv and magic defence without its dispellable promotions; a spell's
//!   hit on an illusion kills it; a Spellbreaker's hit dispels its target; Contingency's recall
//!   takes a fighter out of the battle in place of a killing hit.
//! - **A heal** ([`Run::magic_revive`]): the Halo brings back the first fighter of its side to
//!   fall, once per battle.
//!
//! Nothing here draws a random number but the opening casts' own (the same draws as any spell
//! action). Illusions are fighters of the formation that start off the field ([`State::on_field`]
//! is false while they are pending); they do not act and a side of illusions alone is down.

use ffh_data::{ActionKind, BattleCharm, Presence, RowActions, ROWS};

use super::*;

/// What a fighter loses while dispelled (Spellbreaker): its A, Dv and magic defence in percent of
/// what they are with its dispellable promotions, and the speed they give.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Bare {
    pub a_pct: i32,
    pub dv_pct: i32,
    pub md_pct: i32,
    pub speed: i32,
}

/// A fighter's battle magic (module comment); all off for an ordinary fighter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FighterMagic {
    /// While it is on the field, the enemy's hexes have no effect on its side.
    pub hex_ward: bool,
    /// While it is on the field, the enemy side's charms end at the start of each round after the
    /// first.
    pub end_charms: bool,
    /// Its pre-battle hex.
    pub presence: Option<Presence>,
    /// In place of its first strike each round it casts its `spell` (Spellsword).
    pub strike_spell: bool,
    /// Once per battle its heal also brings back the first fighter of its side that fell, at this
    /// percent of its HP (Halo); 0: none.
    pub revive: i32,
    /// Each hit it lands dispels its target.
    pub dispel: bool,
    /// What it loses while dispelled; `None`: nothing (no dispellable promotion).
    pub bare: Option<Bare>,
    /// It casts its damage spell before the first round (Contingency).
    pub opening_cast: bool,
    /// A hit that would kill it takes it out of the battle instead (Contingency).
    pub recall: bool,
    /// The fighter whose first spell or shot it takes (a familiar).
    pub guards: Option<FighterIdx>,
    /// An illusion: it does not act, does not hold the field, and a spell's hit kills it.
    pub illusion: bool,
    /// Its illusions' summoner: off the field until that fighter casts them.
    pub summoner: Option<FighterIdx>,
    /// The illusions it casts with its first spell action.
    pub illusions: Option<IllusionSpell>,
    /// Percent more damage its hits deal to illusions (and, through the pair table, to summons).
    pub damage_vs_summons: i32,
}

/// A caster's illusions: its spell (the caller's id), how many and their HP.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IllusionSpell {
    pub id: u32,
    pub count: i32,
    pub hp: i32,
}

/// The illusions of the casters among `fighters`, appended to it, off the field until cast: each
/// caster's `count` in the free cells of its side on `lanes` lanes (the front row first, the lanes
/// nearest the centre first), each with its caster's A and evasion, no action. Returns each new
/// illusion's caster (an index into `fighters`), in order.
pub fn add_illusions(fighters: &mut Vec<Fighter>, lanes: [u8; 2]) -> Vec<usize> {
    let mut casters = Vec::new();
    let base = fighters.len();
    for k in 0..base {
        let Some(sp) = fighters[k].magic.illusions else { continue };
        let side = fighters[k].side;
        let n = lanes[side.index()];
        let taken = |l: u8, r: u8, all: &[Fighter]| all.iter().any(|x| x.side == side && x.row == r && x.lanes().contains(&l));
        let cells: Vec<(u8, u8)> = sentinel_cells(n, n as usize * ROWS).into_iter().filter(|&(l, r)| !taken(l, r, fighters)).take(sp.count.max(0) as usize).collect();
        for (l, r) in cells {
            let mut x = Fighter::new(side, sp.hp);
            x.tag = fighters[k].tag;
            (x.lane, x.row) = (l, r);
            x.speed = 0;
            x.actions = [RowActions::default(); ROWS];
            x.a = fighters[k].a;
            x.evasion = fighters[k].evasion;
            x.crit = 0;
            x.magic.illusion = true;
            x.magic.summoner = Some(k as FighterIdx);
            fighters.push(x);
            casters.push(k);
        }
    }
    casters
}

/// A charm or hex that landed on a fighter.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    pub target: usize,
    pub caster: usize,
    pub charm: BattleCharm,
    /// It no longer counts (ended by an Abjurer or a Spellbreaker's hit).
    pub ended: bool,
}

/// The battle magic's state of a battle (`State::mx`).
#[derive(Clone, Debug, Default)]
pub struct MagicState {
    /// The charms and hexes that landed, in order.
    pub applied: Vec<Applied>,
    /// Actions each fighter is still to skip (the Lullaby).
    pub skip: Vec<i32>,
    /// Per side (attacker, defender): its charges are strikes (grounded).
    pub grounded: [bool; 2],
    /// Per side (attacker, defender): its fighter that hits its own side (madness), in round 1.
    pub mad: [Option<usize>; 2],
    /// Dispelled fighters.
    pub dispelled: Vec<bool>,
    /// The round in which a Spellsword last cast in place of a strike.
    pub struck: Vec<u8>,
    /// Halo holders that have revived, familiars that have intercepted, summoners that have cast
    /// their illusions.
    pub revived: Vec<bool>,
    pub intercepted: Vec<bool>,
    pub summoned: Vec<bool>,
    /// Illusions not on the field yet.
    pub pending: Vec<bool>,
}

impl MagicState {
    pub fn new(f: &Formation) -> MagicState {
        let n = f.len();
        MagicState {
            applied: Vec::new(),
            skip: vec![0; n],
            grounded: [false; 2],
            mad: [None; 2],
            dispelled: vec![false; n],
            struck: vec![0; n],
            revived: vec![false; n],
            intercepted: vec![false; n],
            summoned: vec![false; n],
            pending: f.fighters.iter().map(|x| x.magic.summoner.is_some()).collect(),
        }
    }
}

impl Formation {
    /// The side `h`'s hostile actions fall on: the other side, or its own while it is mad.
    pub(super) fn foe_of(&self, s: &State, h: usize) -> Side {
        if s.mx.mad[self.fighters[h].side.index()] == Some(h) {
            self.fighters[h].side
        } else {
            self.fighters[h].side.other()
        }
    }

    /// Whether side `side` has a fighter with a hex ward on the field.
    pub fn warded(&self, s: &State, side: Side) -> bool {
        (0..self.len()).any(|i| self.fighters[i].side == side && self.fighters[i].magic.hex_ward && s.on_field(i))
    }
}

impl<'a> Run<'a> {
    /// The battle's start (module comment): the pre-battle hexes, then the opening casts.
    pub(super) fn magic_start(&mut self, rng: &mut impl BattleRng) {
        let f = self.f;
        let present = |side: Side, p: Presence, s: &State| (0..f.len()).any(|i| f.fighters[i].side == side && f.fighters[i].magic.presence == Some(p) && s.on_field(i));
        for side in [Side::Attacker, Side::Defender] {
            let foe = side.other();
            if f.warded(&self.s, foe) {
                continue;
            }
            if present(side, Presence::Grounding, &self.s) {
                self.s.mx.grounded[foe.index()] = true;
            }
            if present(side, Presence::Madness, &self.s) {
                // The enemy's highest A (ties: the lower index), not a Sentinel or an illusion.
                let top = (0..f.len())
                    .filter(|&i| f.fighters[i].side == foe && self.s.on_field(i) && !f.fighters[i].fixed && !f.fighters[i].magic.illusion)
                    .max_by(|&x, &y| f.fighters[x].a.cmp(&f.fighters[y].a).then(y.cmp(&x)));
                self.s.mx.mad[foe.index()] = top;
            }
        }
        // The opening casts, round 0.
        for i in 0..f.len() {
            if f.fighters[i].magic.opening_cast && self.s.on_field(i) && !f.side_down(&self.s, f.fighters[i].side.other()) {
                self.act_kind(i, Some(ActionKind::Spell), rng);
            }
        }
    }

    /// Round `round`'s start (module comment): an Abjurer ends the enemy side's charms; the
    /// madness ends after round 1.
    pub(super) fn magic_round(&mut self, round: i32) {
        let f = self.f;
        if round >= 2 {
            self.s.mx.mad = [None; 2];
            for side in [Side::Attacker, Side::Defender] {
                let breaker = (0..f.len()).any(|i| f.fighters[i].side == side && f.fighters[i].magic.end_charms && self.s.on_field(i));
                if breaker {
                    let foe = side.other();
                    let ended: Vec<usize> =
                        (0..self.s.mx.applied.len()).filter(|&k| !self.s.mx.applied[k].ended && !self.s.mx.applied[k].charm.enemy && f.fighters[self.s.mx.applied[k].target].side == foe).collect();
                    for k in ended {
                        self.end_applied(k);
                    }
                }
            }
        }
    }

    /// Takes charm or hex `k` off its target.
    fn end_applied(&mut self, k: usize) {
        let a = self.s.mx.applied[k];
        let (t, c) = (a.target, a.charm);
        self.s.dmg_mod[t] -= c.damage;
        self.s.taken_mod[t] -= c.taken;
        self.s.acc_mod[t] -= c.accuracy;
        self.s.eva_mod[t] -= c.evasion;
        self.s.crit_mod[t] -= c.crit;
        self.s.speed[t] -= c.speed;
        self.s.mx.applied[k].ended = true;
    }

    /// Whether fighter `h` skips this action (the Lullaby): its event says so.
    pub(super) fn magic_skip(&mut self, h: usize) -> bool {
        if self.s.mx.skip[h] <= 0 {
            return false;
        }
        self.s.mx.skip[h] -= 1;
        let kind = self.f.actions_at(h, self.s.row[h]).kind;
        let ev = Event { round: self.round, pass: self.pass, actor: h as FighterIdx, kind, spell: None, target: None, hits: Vec::new(), heal: None, knocked: None, note: Some(Note::Skipped), entered: Vec::new() };
        self.push(ev);
        true
    }

    /// The kind fighter `h` fights an action of `kind` as (after the ground's): a grounded side's
    /// charge is a strike; a Spellsword's first strike each round is its spell.
    pub(super) fn magic_kind(&mut self, h: usize, kind: ActionKind) -> ActionKind {
        let fh = &self.f.fighters[h];
        if kind == ActionKind::Charge && self.s.mx.grounded[fh.side.index()] {
            return ActionKind::Strike;
        }
        if kind == ActionKind::Strike && fh.magic.strike_spell && self.s.mx.struck[h] != self.round {
            self.s.mx.struck[h] = self.round;
            return ActionKind::Spell;
        }
        kind
    }

    /// A caster's illusions, cast with its first spell action: each illusion it summons whose cell
    /// is free takes it, at its HP. `false`: it has none to cast.
    pub(super) fn magic_summon(&mut self, h: usize, mut ev: Event) -> bool {
        let f = self.f;
        let Some(id) = f.fighters[h].magic.illusions.map(|i| i.id) else { return false };
        if self.s.mx.summoned[h] {
            return false;
        }
        self.s.mx.summoned[h] = true;
        ev.spell = Some(id);
        for t in 0..f.len() {
            if f.fighters[t].magic.summoner != Some(h as FighterIdx) || !self.s.mx.pending[t] {
                continue;
            }
            let ft = &f.fighters[t];
            let row = ft.row;
            let taken = (0..f.len()).any(|u| u != t && self.s.on_field(u) && f.fighters[u].side == ft.side && self.s.row[u] == row && f.fighters[u].lanes().any(|l| ft.lanes().contains(&l)));
            if taken {
                continue;
            }
            self.s.mx.pending[t] = false;
            self.s.row[t] = row;
            ev.entered.push(Entry { unit: t as FighterIdx, lane: ft.lane, row, hp: self.s.hp[t] });
        }
        if ev.entered.is_empty() {
            ev.note = Some(Note::NoTarget);
        }
        ev.target = ev.entered.first().map(|e| e.unit);
        self.push(ev);
        true
    }

    /// The target of `h`'s action of `kind` once its choice is `c`: a familiar takes the first
    /// spell or shot aimed at the caster it guards.
    pub(super) fn magic_intercept(&mut self, h: usize, c: usize, kind: ActionKind) -> usize {
        if !matches!(kind, ActionKind::Spell | ActionKind::Shoot) {
            return c;
        }
        let f = self.f;
        let guard = (0..f.len()).find(|&g| f.fighters[g].magic.guards == Some(c as FighterIdx) && !self.s.mx.intercepted[g] && self.s.on_field(g) && f.fighters[g].side != f.fighters[h].side);
        match guard {
            Some(g) => {
                self.s.mx.intercepted[g] = true;
                g
            }
            None => c,
        }
    }

    /// The fighters a charm or hex `c` cast by `h` lands on: its side's on the field, of its rows
    /// (the front row as it stands, the rear row, or all); none on a side warded against hexes.
    pub(super) fn magic_charm_targets(&self, h: usize, c: &BattleCharm) -> Vec<usize> {
        let f = self.f;
        let side = if c.enemy { f.fighters[h].side.other() } else { f.fighters[h].side };
        if c.enemy && f.warded(&self.s, side) {
            return Vec::new();
        }
        let on: Vec<usize> = (0..f.len()).filter(|&t| self.s.on_field(t) && f.fighters[t].side == side && !f.fighters[t].magic.illusion).collect();
        // The rear row: the side's rearmost row in use (the middle row of an army of two rows).
        let rear = on.iter().map(|&t| self.s.row[t]).max().unwrap_or(0);
        on.into_iter().filter(|&t| c.whole_side || if c.rear { self.s.row[t] == rear } else { self.s.row[t] == 0 }).collect()
    }

    /// A charm or hex `c` of `h` landed on `t`: its skips, and its record.
    pub(super) fn magic_charm_landed(&mut self, h: usize, t: usize, c: &BattleCharm) {
        self.s.mx.skip[t] += c.skip.max(0);
        self.s.mx.applied.push(Applied { target: t, caster: h, charm: *c, ended: false });
    }

    /// The percentage of `h`'s hit of `kind` on `t` while either is dispelled: the expected
    /// damage with the dispelled one's A or Dv (its magic defence against a spell) without its
    /// dispellable promotions, over the expected damage with them; 100 otherwise.
    pub(super) fn magic_hit_pct(&self, h: usize, t: usize, kind: ActionKind) -> i64 {
        let f = self.f;
        let bare_h = f.fighters[h].magic.bare.filter(|_| self.s.mx.dispelled[h]);
        let bare_t = f.fighters[t].magic.bare.filter(|_| self.s.mx.dispelled[t]);
        if bare_h.is_none() && bare_t.is_none() {
            return 100;
        }
        let p = f.pair(h, t);
        let (a, dv) = if kind == ActionKind::Spell { (p.ma as i64, p.md as i64) } else { (p.a as i64, p.dv as i64) };
        let a2 = match bare_h {
            Some(b) if kind != ActionKind::Spell => a * b.a_pct.max(1) as i64 / 100,
            _ => a,
        }
        .max(1);
        let dv2 = match bare_t {
            Some(b) if kind == ActionKind::Spell => dv * b.md_pct.max(1) as i64 / 100,
            Some(b) => dv * b.dv_pct.max(1) as i64 / 100,
            None => dv,
        }
        .max(1);
        // (a2 / (a2 + dv2)) / (a / (a + dv)), in percent.
        100 * a2 * (a + dv) / (a * (a2 + dv2)).max(1)
    }

    /// A hit of `h` on `t` of `kind` landed (`dmg` its damage before the target's HP caps it):
    /// a spell's hit on an illusion takes all its HP; a Spellbreaker's hit dispels its target.
    pub(super) fn magic_landed(&mut self, h: usize, t: usize, kind: ActionKind, dmg: i32) -> i32 {
        let f = self.f;
        if f.fighters[h].magic.dispel && !self.s.mx.dispelled[t] {
            self.dispel(t);
        }
        if kind == ActionKind::Spell && f.fighters[t].magic.illusion {
            return self.s.hp[t];
        }
        dmg
    }

    /// Dispels `t`: its charms end, and its dispellable promotions' speed goes.
    fn dispel(&mut self, t: usize) {
        self.s.mx.dispelled[t] = true;
        let ended: Vec<usize> = (0..self.s.mx.applied.len()).filter(|&k| !self.s.mx.applied[k].ended && self.s.mx.applied[k].target == t && !self.s.mx.applied[k].charm.enemy).collect();
        for k in ended {
            self.end_applied(k);
        }
        if let Some(b) = self.f.fighters[t].magic.bare {
            self.s.speed[t] += b.speed;
        }
    }

    /// Whether a killing hit on `t` takes it out of the battle instead (Contingency's recall,
    /// whatever the ground and the hitter).
    pub(super) fn magic_recall(&self, t: usize) -> bool {
        self.f.fighters[t].magic.recall
    }

    /// After a heal action of `h` (`ev` its event, as cast): a Halo holder that has not revived
    /// brings back the first fighter of its side that fell (not a Sentinel or an illusion), in
    /// its cell if free, else the rear cell of its lanes, at its percent of HP; the kill is
    /// undone. `true`: it revived one.
    pub(super) fn magic_revive(&mut self, h: usize, ev: &Event) -> bool {
        let f = self.f;
        let pct = f.fighters[h].magic.revive;
        if pct <= 0 || self.s.mx.revived[h] {
            return false;
        }
        let side = f.fighters[h].side;
        let Some(k) = self.kills.iter().position(|k| {
            let v = k.victim as usize;
            f.fighters[v].side == side && !f.fighters[v].fixed && !f.fighters[v].magic.illusion && !self.s.alive[v]
        }) else {
            return false;
        };
        let v = self.kills[k].victim as usize;
        let fv = &f.fighters[v];
        let free = |row: u8, s: &State| !(0..f.len()).any(|u| u != v && s.on_field(u) && f.fighters[u].side == side && s.row[u] == row && f.fighters[u].lanes().any(|l| fv.lanes().contains(&l)));
        let rear = (ROWS - 1) as u8;
        let row = if free(self.s.row[v], &self.s) {
            self.s.row[v]
        } else if free(rear, &self.s) {
            rear
        } else {
            return false;
        };
        self.s.mx.revived[h] = true;
        let hp = (fv.max_hp * pct.clamp(1, 100)).max(1);
        self.s.alive[v] = true;
        self.s.hp[v] = hp;
        self.s.row[v] = row;
        self.fate[v] = Fate::Standing;
        self.kills.remove(k);
        let mut e = ev.clone();
        e.kind = ActionKind::Heal;
        e.target = Some(v as FighterIdx);
        e.hits.clear();
        e.heal = None;
        e.knocked = None;
        e.note = None;
        e.entered = vec![Entry { unit: v as FighterIdx, lane: f.fighters[v].lane, row, hp }];
        self.push(e);
        true
    }
}
