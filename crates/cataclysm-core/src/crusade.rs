//! Crusades and Allied Loans: committed war forces lent to a temporary Host.

use crate::ids::{CrusadeId, PlayerId, SquadId};

/// Turns a Crusade lasts.
pub const CRUSADE_TURNS: i32 = 18;

#[derive(Clone, Debug)]
pub struct Crusade {
    pub id: CrusadeId,
    pub caller: PlayerId,
    pub target: PlayerId,
    pub turns_left: i32,
    pub active: bool,
    /// Squads lent by allies (returns to lender on dissolution if still alive).
    pub lent_squads: Vec<(PlayerId, SquadId)>,
    /// Credit earned per lender (captures & kills).
    pub credit: Vec<(PlayerId, i32)>,
}

impl Crusade {
    pub fn new(id: CrusadeId, caller: PlayerId, target: PlayerId) -> Self {
        Self {
            id,
            caller,
            target,
            turns_left: CRUSADE_TURNS,
            active: true,
            lent_squads: Vec::new(),
            credit: Vec::new(),
        }
    }

    pub fn add_credit(&mut self, lender: PlayerId, amount: i32) {
        if let Some((_, c)) = self.credit.iter_mut().find(|(p, _)| *p == lender) {
            *c += amount;
        } else {
            self.credit.push((lender, amount));
        }
    }
}
