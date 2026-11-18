//! Commanders (docs/PERSONS.md, "The first build": Commanders).
//!
//! A person is *with* an army when their unit is in the army's group (a veteran in it, a hero
//! grouped with it). An army's commander is the person with it that the player named
//! (`Army::commander_named`, [`Game::player_set_commander`]), else the one of the highest level
//! (ties: the earlier in the cast). A House's head never commands; a fleet has no commander. The
//! command follows the unit: a person who leaves the group loses it, without a grievance.

use super::*;
use crate::army::ArmyDomain;

impl Game {
    /// The persons of `a`'s owner with army `a`, in cast order: their unit in its group, no
    /// House's head. None for a fleet.
    pub fn army_persons_with(&self, a: ArmyId) -> Vec<PersonId> {
        let army = &self.armies[a];
        if army.domain != ArmyDomain::Land {
            return Vec::new();
        }
        let Some(grp) = army.group.filter(|&g| self.groups.contains(g)) else { return Vec::new() };
        self.players[army.owner]
            .persons
            .iter()
            .copied()
            .filter(|&x| {
                self.persons.contains(x)
                    && self.person_house(x).is_none()
                    && self.person_unit(x).is_some_and(|u| self.units[u].group == Some(grp) && self.units[u].march.is_none())
            })
            .collect()
    }

    /// The commander army `a` would have by the rule: the named person if with it, else the
    /// person with it of the highest level (ties: the earlier in the cast).
    fn army_commander_by_rule(&self, a: ArmyId) -> Option<PersonId> {
        let with = self.army_persons_with(a);
        if let Some(named) = self.armies[a].commander_named.filter(|x| with.contains(x)) {
            return Some(named);
        }
        let mut best: Option<(i32, PersonId)> = None;
        for &x in &with {
            let level = self.person_unit(x).map_or(0, |u| self.units[u].level);
            if best.map_or(true, |(l, _)| level > l) {
                best = Some((level, x));
            }
        }
        best.map(|(_, x)| x)
    }

    /// Player `p`'s armies take their commanders by the rule (after the army sync and in the
    /// persons turn). A person who loses a command has no post; one who gains it is told.
    pub fn player_persons_commanders(&mut self, p: PlayerId) {
        if self.players[p].persons.is_empty() && self.players[p].armies.iter().all(|&a| self.armies.get(a).map_or(true, |army| army.commander.is_none())) {
            return;
        }
        let mut chosen: Vec<(ArmyId, Option<PersonId>)> = Vec::new();
        for a in self.players[p].armies.clone() {
            if !self.armies.contains(a) {
                continue;
            }
            // A named commander who is not with it any more is forgotten.
            if let Some(named) = self.armies[a].commander_named {
                if !self.army_persons_with(a).contains(&named) {
                    self.armies[a].commander_named = None;
                }
            }
            chosen.push((a, self.army_commander_by_rule(a)));
        }
        // Who loses a command.
        for x in self.players[p].persons.clone() {
            if let Post::Commander(a) = self.persons[x].post {
                if !chosen.iter().any(|&(b, c)| b == a && c == Some(x)) {
                    self.persons[x].post = Post::None;
                    let name = self.persons[x].name.clone();
                    let army = self.armies.get(a).map(|army| army.name.clone());
                    if let Some(army) = army {
                        self.person_add_deed(x, format!("Left the command of the {army}."));
                        self.person_msg(x, || format!("{name} no longer commands the {army}."));
                    }
                }
            }
        }
        // Who gains one.
        for (a, c) in chosen {
            self.armies[a].commander = c;
            let Some(x) = c else { continue };
            if self.persons[x].post != Post::Commander(a) {
                self.persons[x].post = Post::Commander(a);
                let name = self.persons[x].name.clone();
                let army = self.armies[a].name.clone();
                self.person_add_deed(x, format!("Took command of the {army}."));
                self.person_msg(x, || format!("{name} commands the {army}."));
                self.tel_person(p, "posted", "commander");
            }
        }
    }

    /// The commander of the army unit `u` is in (or is with, a hero grouped with it), if any.
    pub fn unit_army_commander(&self, u: UnitId) -> Option<PersonId> {
        let unit = self.units.get(u)?;
        let a = unit.army.filter(|&a| self.armies.contains(a)).or_else(|| {
            let grp = unit.group?;
            self.players[unit.owner].armies.iter().copied().find(|&a| self.armies.get(a).is_some_and(|army| army.group == Some(grp)))
        })?;
        self.armies[a].commander.filter(|&x| self.persons.contains(x))
    }

    /// Hook of a capture (`unit_set_xy`): the city now on `plot` was taken by a unit of its owner
    /// whose army had `commander`; it records the commander's claim and deed.
    pub(crate) fn city_persons_taken(&mut self, plot: PlotIdx, commander: Option<PersonId>) {
        let Some(x) = commander.filter(|&x| self.persons.contains(x)) else { return };
        let Some(c) = self.persons_city_at(plot) else { return };
        if self.cities[c].owner != self.persons[x].owner {
            return;
        }
        let turn = self.game_turn();
        self.cities[c].taken_by = Some((x, turn));
        let city = self.persons_city_name(c);
        let name = self.persons[x].name.clone();
        let army = self.person_army(x).map(|a| self.armies[a].name.clone());
        self.person_add_deed(x, format!("Took {city}."));
        self.person_msg(x, || match army {
            Some(a) => format!("{name} took {city} with the {a}."),
            None => format!("{name} took {city}."),
        });
    }

    /// Player `p` names person `x` commander of army `a` (among the persons with it).
    pub fn player_set_commander(&mut self, p: PlayerId, a: ArmyId, x: PersonId) -> Result<(), &'static str> {
        if !self.armies.contains(a) || self.armies[a].owner != p {
            return Err("Not an army of yours.");
        }
        if self.armies[a].domain != ArmyDomain::Land {
            return Err("A fleet has no commander.");
        }
        if !self.persons.contains(x) || self.persons[x].owner != p {
            return Err("Not a person of yours.");
        }
        if self.person_house(x).is_some() {
            return Err("A House's head does not command.");
        }
        if !self.army_persons_with(a).contains(&x) {
            return Err("Not with this army.");
        }
        self.armies[a].commander_named = Some(x);
        self.player_persons_commanders(p);
        Ok(())
    }
}
