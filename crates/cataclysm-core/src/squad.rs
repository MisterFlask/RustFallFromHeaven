//! Ogre Battle 3x3 Squad — the atomic military unit on the strategic map.
//!
//! No loose warriors, no CvSelectionGroup sync, no separate Army vs Squad hierarchy: every moving
//! force on the map IS a Squad with a named Captain (using our embedded minor-leader portraits),
//! a 3x3 grid of up to 4 (or 5 after tier-2 Doctrine) places, and Wounded Hero return tracking.

use crate::data::{CaptainAura, GreatSuit, UnitClassId};
use crate::ids::{MemberId, PlayerId, PlotIdx, SquadId};

pub const GRID_ROWS: u8 = 3;
pub const GRID_LANES: u8 = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SquadOrder {
    None,
    Fortify,
    Raid,
}

#[derive(Clone, Debug)]
pub struct SquadMember {
    pub id: MemberId,
    pub class: UnitClassId,
    /// `0..=2` (0 = Front, 1 = Middle, 2 = Rear).
    pub row: u8,
    /// `0..=2` (a Large unit occupies `lane` and `lane + 1`, so `lane <= 1`).
    pub lane: u8,
    /// Current HP in centi-HP (`max_hp * 100` when full).
    pub hp_centi: i32,
    pub xp: i32,
    pub level: i32,
    /// Hero units return wounded to the capital after dying instead of being permanently lost.
    pub hero_name: Option<String>,
    pub hero_portrait: Option<&'static str>,
    pub hero_suit: Option<GreatSuit>,
    /// Permanent Wounds accumulated across deaths (-15% stats per Wound).
    pub wounds: i32,
}

impl SquadMember {
    pub fn new(id: MemberId, class: UnitClassId, row: u8, lane: u8) -> Self {
        let hp = class.info().hp * 100;
        Self {
            id,
            class,
            row,
            lane,
            hp_centi: hp,
            xp: 0,
            level: 1,
            hero_name: None,
            hero_portrait: None,
            hero_suit: None,
            wounds: 0,
        }
    }

    pub fn new_hero(id: MemberId, class: UnitClassId, name: String, portrait: &'static str, suit: GreatSuit) -> Self {
        let mut m = Self::new(id, class, class.info().row_order[0] as u8, 1);
        m.hero_name = Some(name);
        m.hero_portrait = Some(portrait);
        m.hero_suit = Some(suit);
        m.level = 3;
        m.hp_centi = m.max_hp() * 100;
        m
    }

    pub fn is_hero(&self) -> bool {
        self.hero_name.is_some()
    }

    pub fn places(&self) -> u8 {
        if self.class.info().large { 2 } else { 1 }
    }

    pub fn max_hp(&self) -> i32 {
        let base = self.class.info().hp;
        let lvl_bonus = (self.level - 1).max(0) * 10;
        let hero_bonus = if self.is_hero() { 30 } else { 0 };
        base + lvl_bonus + hero_bonus
    }

    pub fn hp_pct(&self) -> i32 {
        let max_c = (self.max_hp() * 100).max(1);
        ((self.hp_centi * 100) / max_c).clamp(0, 100)
    }

    pub fn heal_pct(&mut self, pct: i32) {
        let max_c = self.max_hp() * 100;
        self.hp_centi = (self.hp_centi + max_c * pct / 100).min(max_c);
    }

    pub fn grant_xp(&mut self, amount: i32) {
        self.xp += amount;
        while self.xp >= self.level * 12 {
            self.xp -= self.level * 12;
            self.level += 1;
            self.heal_pct(25);
        }
    }

    pub fn display_name(&self) -> &str {
        self.hero_name.as_deref().unwrap_or(self.class.info().name)
    }
}

/// A fallen Hero recovering at the capital before returning with +1 Wound.
#[derive(Clone, Debug)]
pub struct RecoveringHero {
    pub owner: PlayerId,
    pub member: SquadMember,
    pub turns_left: i32,
}

#[derive(Clone, Debug)]
pub struct Squad {
    pub id: SquadId,
    pub owner: PlayerId,
    pub name: String,
    pub captain_name: String,
    pub captain_portrait: &'static str,
    pub captain_aura: CaptainAura,
    pub plot: PlotIdx,
    pub members: Vec<SquadMember>,
    pub moves_left: i32,
    pub order: SquadOrder,
    /// Supply starvation state: 0 = supplied, 1-2 = Hungry, >=3 = Starving.
    pub unsupplied_turns: i32,
    /// Battles fought this turn (used to cycle defending squads fairly).
    pub fought_this_turn: i32,
    /// True if this squad is currently lent to an active Crusade Host.
    pub lent_by: Option<PlayerId>,
}

impl Squad {
    pub fn places_used(&self) -> u8 {
        self.members.iter().map(SquadMember::places).sum()
    }

    pub fn max_moves(&self, bonus: i32) -> i32 {
        let base = self.members.iter().map(|m| m.class.info().moves).min().unwrap_or(2);
        let aura = if self.captain_aura == CaptainAura::SwiftMarch { 1 } else { 0 };
        (base + aura + bonus).max(1)
    }

    pub fn sight_range(&self) -> i32 {
        let recon = self.members.iter().any(|m| matches!(m.class, UnitClassId::Ranger | UnitClassId::Assassin | UnitClassId::Horseman));
        if recon { 3 } else { 2 }
    }

    pub fn power_rating(&self) -> i32 {
        self.members
            .iter()
            .map(|m| {
                let c = m.class.info();
                let stat = c.atk.max(c.magic) + c.def;
                stat * m.hp_pct() / 100 + (m.level - 1) * 12
            })
            .sum()
    }

    /// Checks whether cell `(row, lane)` (and `lane+1` if `large`) is free, optionally ignoring `ignore_id`.
    pub fn cell_free(&self, row: u8, lane: u8, large: bool, ignore: Option<MemberId>) -> bool {
        if row >= GRID_ROWS || lane >= GRID_LANES || (large && lane + 1 >= GRID_LANES) {
            return false;
        }
        let need_l0 = lane;
        let need_l1 = if large { lane + 1 } else { lane };
        for m in &self.members {
            if Some(m.id) == ignore {
                continue;
            }
            if m.row != row {
                continue;
            }
            let ml0 = m.lane;
            let ml1 = if m.class.info().large { m.lane + 1 } else { m.lane };
            if need_l0 <= ml1 && ml0 <= need_l1 {
                return false;
            }
        }
        true
    }

    /// Finds the first open cell respecting a unit class's preferred row order (center lane first).
    pub fn find_open_cell(&self, class: UnitClassId) -> Option<(u8, u8)> {
        let info = class.info();
        let lane_order: &[u8] = if info.large { &[0, 1] } else { &[1, 0, 2] };
        for &r in &info.row_order {
            for &l in lane_order {
                if self.cell_free(r as u8, l, info.large, None) {
                    return Some((r as u8, l));
                }
            }
        }
        None
    }

    /// Automatically arranges every member into its best row, large units first, center lanes first.
    pub fn auto_arrange(&mut self) {
        let mut order: Vec<usize> = (0..self.members.len()).collect();
        order.sort_by(|&a, &b| {
            let ma = &self.members[a];
            let mb = &self.members[b];
            mb.class
                .info()
                .large
                .cmp(&ma.class.info().large)
                .then_with(|| ma.class.info().row_order[0].cmp(&mb.class.info().row_order[0]))
                .then_with(|| mb.max_hp().cmp(&ma.max_hp()))
                .then_with(|| ma.id.cmp(&mb.id))
        });
        let mut occupied = [[false; 3]; 3];
        for idx in order {
            let info = self.members[idx].class.info();
            let lane_order: &[u8] = if info.large { &[0, 1] } else { &[1, 0, 2] };
            let mut placed = false;
            'outer: for &r in &info.row_order {
                for &l in lane_order {
                    let ok = if info.large {
                        !occupied[r][l as usize] && !occupied[r][(l + 1) as usize]
                    } else {
                        !occupied[r][l as usize]
                    };
                    if ok {
                        self.members[idx].row = r as u8;
                        self.members[idx].lane = l;
                        occupied[r][l as usize] = true;
                        if info.large {
                            occupied[r][(l + 1) as usize] = true;
                        }
                        placed = true;
                        break 'outer;
                    }
                }
            }
            if !placed {
                // Fallback: any free cell
                'any: for r in 0..3 {
                    for l in 0..3 {
                        if info.large && l >= 2 {
                            continue;
                        }
                        let ok = if info.large { !occupied[r][l] && !occupied[r][l + 1] } else { !occupied[r][l] };
                        if ok {
                            self.members[idx].row = r as u8;
                            self.members[idx].lane = l as u8;
                            occupied[r][l] = true;
                            if info.large {
                                occupied[r][l + 1] = true;
                            }
                            break 'any;
                        }
                    }
                }
            }
        }
    }

    /// Moves a member to `(row, lane)`, swapping with a same-size occupant if needed.
    pub fn move_member(&mut self, member_id: MemberId, row: u8, lane: u8) -> Result<(), &'static str> {
        let Some(pos) = self.members.iter().position(|m| m.id == member_id) else {
            return Err("member not found in squad");
        };
        let large = self.members[pos].class.info().large;
        if row >= GRID_ROWS || lane >= GRID_LANES || (large && lane + 1 >= GRID_LANES) {
            return Err("target cell is out of bounds for this unit size");
        }
        if self.cell_free(row, lane, large, Some(member_id)) {
            self.members[pos].row = row;
            self.members[pos].lane = lane;
            return Ok(());
        }
        // Try exact swap with a single same-size occupant.
        if let Some(other_pos) = self
            .members
            .iter()
            .position(|m| m.id != member_id && m.row == row && m.lane == lane && m.class.info().large == large)
        {
            let (old_r, old_l) = (self.members[pos].row, self.members[pos].lane);
            self.members[pos].row = row;
            self.members[pos].lane = lane;
            self.members[other_pos].row = old_r;
            self.members[other_pos].lane = old_l;
            return Ok(());
        }
        Err("target cell is occupied by a unit of different size")
    }
}
