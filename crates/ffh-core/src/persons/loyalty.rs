//! Loyalty: the modifiers, the sum, the four states and the Disloyal countdown
//! (docs/PERSONS.md, "The first build": Loyalty, States).
//!
//! A head's loyalty is their House's (`House::standing`); everyone else's is their own
//! (`Person::standing`). The sum is the modifiers still running plus the standing part `State
//! religion opposed: -2` (the state religion's alignment the opposite of the person's; a House's
//! is its head's). An empty seat is Loyal whatever its sum.

use ffh_data::*;

use super::*;

/// Whose loyalty: a House's or a person's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Holder {
    House(HouseId),
    Person(PersonId),
}

impl Game {
    /// Whose loyalty person `x` answers to: their House's if they head one, else their own.
    pub(crate) fn person_holder(&self, x: PersonId) -> Holder {
        match self.person_house(x) {
            Some(h) => Holder::House(h),
            None => Holder::Person(x),
        }
    }

    pub(crate) fn holder_standing(&self, hd: Holder) -> &Standing {
        match hd {
            Holder::House(h) => &self.houses[h].standing,
            Holder::Person(x) => &self.persons[x].standing,
        }
    }

    pub(crate) fn holder_standing_mut(&mut self, hd: Holder) -> &mut Standing {
        match hd {
            Holder::House(h) => &mut self.houses[h].standing,
            Holder::Person(x) => &mut self.persons[x].standing,
        }
    }

    /// The person whose alignment and demands a holder has: the House's head, or the person.
    pub(crate) fn holder_person(&self, hd: Holder) -> Option<PersonId> {
        match hd {
            Holder::House(h) => self.houses[h].head.filter(|&x| self.persons.contains(x)),
            Holder::Person(x) => Some(x),
        }
    }

    /// The holder's label: `House Arne`, `Hrolf, 2nd Army`, `Hrolf`.
    pub(crate) fn holder_label(&self, hd: Holder) -> String {
        match hd {
            Holder::House(h) => self.house_label(h),
            Holder::Person(x) => self.person_subject(x),
        }
    }

    /// The standing part `State religion opposed` for person `x` under state religion `religion`:
    /// [`RELIGION_OPPOSED`] when the religion's alignment is the opposite of the person's.
    pub(crate) fn person_religion_part(&self, x: PersonId, religion: Option<ReligionType>) -> i32 {
        let Some(r) = religion else { return 0 };
        if self.alignments_opposed(self.d.religions[r].alignment, self.person_alignment(x)) {
            RELIGION_OPPOSED
        } else {
            0
        }
    }

    /// The holder's standing religion part under its owner's state religion.
    pub(crate) fn holder_religion_part(&self, hd: Holder) -> i32 {
        let Some(x) = self.holder_person(hd) else { return 0 };
        let religion = self.player_get_state_religion(self.persons[x].owner);
        self.person_religion_part(x, religion)
    }

    /// The sum of the holder's running modifiers (without the standing religion part).
    pub(crate) fn holder_mods_sum(&self, hd: Holder) -> i32 {
        let turn = self.game_turn();
        self.holder_standing(hd).mods.iter().filter(|m| m.until.map_or(true, |u| u > turn)).map(|m| m.amount).sum()
    }

    /// The holder's loyalty: its running modifiers plus the standing religion part.
    pub(crate) fn holder_loyalty(&self, hd: Holder) -> i32 {
        self.holder_mods_sum(hd) + self.holder_religion_part(hd)
    }

    /// The holder's state at loyalty `total` (an empty seat is Loyal).
    pub(crate) fn holder_state_at(&self, hd: Holder, total: i32) -> LoyaltyState {
        if let Holder::House(h) = hd {
            if self.houses[h].head.is_none() {
                return LoyaltyState::Loyal;
            }
        }
        LoyaltyState::of(total)
    }

    /// The holder's state now.
    pub(crate) fn holder_state(&self, hd: Holder) -> LoyaltyState {
        self.holder_state_at(hd, self.holder_loyalty(hd))
    }

    /// House `h`'s loyalty (its head's state religion part included).
    pub fn house_loyalty(&self, h: HouseId) -> i32 {
        self.holder_loyalty(Holder::House(h))
    }

    /// House `h`'s state (an empty seat: Loyal).
    pub fn house_state(&self, h: HouseId) -> LoyaltyState {
        self.holder_state(Holder::House(h))
    }

    /// Person `x`'s loyalty: their House's for a head, else their own.
    pub fn person_loyalty(&self, x: PersonId) -> i32 {
        self.holder_loyalty(self.person_holder(x))
    }

    /// Person `x`'s state: their House's for a head, else their own.
    pub fn person_state(&self, x: PersonId) -> LoyaltyState {
        self.holder_state(self.person_holder(x))
    }

    /// Adds a modifier `(amount, turns)` with `cause` to the holder.
    pub(crate) fn holder_add_mod(&mut self, hd: Holder, cause: LoyaltyCause, (amount, turns): (i32, i32)) {
        let until = Some(self.game_turn() + turns);
        self.holder_standing_mut(hd).mods.push(LoyaltyMod { cause, amount, until });
    }

    /// Player `p`'s holders: its Houses, then its persons who head none, in their list orders.
    pub(crate) fn player_holders(&self, p: PlayerId) -> Vec<Holder> {
        let mut out: Vec<Holder> = self.players[p].houses.iter().copied().filter(|&h| self.houses.contains(h)).map(Holder::House).collect();
        out.extend(self.players[p].persons.iter().copied().filter(|&x| self.persons.contains(x) && self.person_house(x).is_none()).map(Holder::Person));
        out
    }

    /// Modifiers whose turns ran out end (in player `p`'s turn processing).
    pub(crate) fn player_persons_expire(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        for hd in self.player_holders(p) {
            self.holder_standing_mut(hd).mods.retain(|m| m.until.map_or(true, |u| u > turn));
        }
    }

    /// The states of player `p`'s holders: a change is told; Disloyal starts the countdown (a
    /// commander's records its sworn units), above it the countdown stops; a person without a
    /// post who is Disloyal leaves the cast.
    pub(crate) fn player_persons_states(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        for hd in self.player_holders(p) {
            let alive = match hd {
                Holder::House(h) => self.houses.contains(h),
                Holder::Person(x) => self.persons.contains(x),
            };
            if !alive {
                continue;
            }
            let state = self.holder_state(hd);
            let old = self.holder_standing(hd).state;
            if state == LoyaltyState::Disloyal {
                if let Holder::Person(x) = hd {
                    if self.persons[x].post == Post::None {
                        let name = self.persons[x].name.clone();
                        self.person_msg(x, || format!("{name} is Disloyal and leaves the court."));
                        self.tel_person(p, "state", "left");
                        self.person_leave(x);
                        continue;
                    }
                }
                if self.holder_standing(hd).countdown.is_none() {
                    let sworn = match hd {
                        Holder::Person(x) => {
                            let mut sworn = self.person_army(x).map(|a| self.army_units(a)).unwrap_or_default();
                            if let Some(u) = self.person_unit(x) {
                                if !sworn.contains(&u) {
                                    sworn.push(u);
                                }
                            }
                            sworn
                        }
                        Holder::House(_) => Vec::new(),
                    };
                    let turns = self.d.defines.person_disloyal_turns;
                    let st = self.holder_standing_mut(hd);
                    st.countdown = Some(turn + turns);
                    st.sworn = sworn;
                }
            } else if self.holder_standing(hd).countdown.is_some() {
                let st = self.holder_standing_mut(hd);
                st.countdown = None;
                st.sworn.clear();
            }
            if state != old {
                self.holder_standing_mut(hd).state = state;
                self.tel_person(p, "state", state.name());
                let label = match hd {
                    Holder::House(h) => self.house_label(h),
                    Holder::Person(x) => self.persons[x].name.clone(),
                };
                let outcome = match (state, hd) {
                    (LoyaltyState::Disloyal, Holder::House(h)) => format!(" {}", self.house_secession_outcome(h)),
                    (LoyaltyState::Disloyal, Holder::Person(x)) => format!(" {}", self.commander_secession_outcome(x)),
                    _ => String::new(),
                };
                let text = format!("{label} is {}.{outcome}", state.name());
                match hd {
                    Holder::House(h) => self.house_msg(h, || text),
                    Holder::Person(x) => self.person_msg(x, || text),
                }
            }
        }
    }

    /// The countdowns of player `p` that have run out (pure): Houses first, then commanders.
    /// The persons turn's secession acts on them (`secession.rs`).
    pub fn persons_countdowns_due(&self, p: PlayerId) -> Vec<CountdownDue> {
        let turn = self.game_turn();
        let mut out = Vec::new();
        for &h in &self.players[p].houses {
            if self.houses.get(h).is_some_and(|house| house.standing.countdown.is_some_and(|c| c <= turn)) {
                out.push(CountdownDue::House(h));
            }
        }
        for &x in &self.players[p].persons {
            if self.persons.get(x).is_some_and(|person| matches!(person.post, Post::Commander(_)) && person.standing.countdown.is_some_and(|c| c <= turn)) {
                out.push(CountdownDue::Commander(x));
            }
        }
        out
    }
}
