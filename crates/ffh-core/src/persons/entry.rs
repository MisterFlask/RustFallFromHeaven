//! Who enters the cast and who leaves it (docs/PERSONS.md, "The first build": The pool, Entry).

use ffh_data::*;

use super::loyalty::Holder;
use super::*;

/// A minor leader's title: the first paragraph of their civilopedia text when it is a short
/// title (`The Auditor of Oaths`); `None` when the text starts with prose.
pub fn leader_title(d: &GameData, l: LeaderHeadType) -> Option<String> {
    let text = d.text.get(&d.leader_heads[l].base.civilopedia);
    let (first, rest) = text.split_once("[NEWLINE][NEWLINE]")?;
    let first = first.trim();
    if first.is_empty() || rest.is_empty() || first.len() > 60 || first.contains('.') {
        return None;
    }
    Some(first.to_string())
}

/// A leader's name.
pub fn leader_name(d: &GameData, l: LeaderHeadType) -> String {
    d.text.name(&d.leader_heads[l].base.description).to_string()
}

impl Game {
    /// The persons player `p` could still draw (pure, in leader id order): its civilization's
    /// preferred minor leaders not in play as a leader and not drawn before in this game.
    pub fn persons_pool(&self, p: PlayerId) -> Vec<LeaderHeadType> {
        let Some(civ) = self.player_get_civilization_type(p) else { return Vec::new() };
        self.d
            .leader_heads
            .keys()
            .filter(|&l| {
                let info = &self.d.leader_heads[l];
                info.preferred_breakaway && info.minor_leader_of == Some(civ) && !crate::ffh::cards::sundering::leader_in_play(self, l)
            })
            .collect()
    }

    /// Draws a person for player `p` with the game RNG (a pool of one needs no draw).
    pub(crate) fn persons_draw(&mut self, p: PlayerId) -> Option<LeaderHeadType> {
        let pool = self.persons_pool(p);
        match pool.len() {
            0 => None,
            1 => Some(pool[0]),
            n => Some(pool[self.soren_rand_num(n as i32, "Persons: draw") as usize]),
        }
    }

    /// The person whose unit is `u`, if any (a scan of its owner's short cast).
    pub fn unit_person(&self, u: UnitId) -> Option<PersonId> {
        let owner = self.units.get(u)?.owner;
        self.players[owner].persons.iter().copied().find(|&x| self.persons.get(x).is_some_and(|person| person.unit == Some(u)))
    }

    /// Where a unit stands, for an entry line: its army's name, else its city's name.
    fn unit_where(&self, u: UnitId) -> Option<String> {
        if let Some(a) = self.units[u].army.filter(|&a| self.armies.contains(a)) {
            return Some(self.armies[a].name.clone());
        }
        let plot = self.unit_plot_opt(u)?;
        self.persons_city_at(plot).map(|c| self.persons_city_name(c))
    }

    /// `level 6 Axeman, 2nd Army`.
    fn unit_entry_line(&self, u: UnitId) -> String {
        let info = &self.d.units[self.units[u].unit_type];
        let what = format!("level {} {}", self.units[u].level, self.d.text.name(&info.base.description));
        match self.unit_where(u) {
            Some(w) => format!("{what}, {w}"),
            None => what,
        }
    }

    /// A new person of player `p` (entered this turn); `line` is what follows "enters the court:"
    /// in the message and the record.
    pub(crate) fn person_new(&mut self, p: PlayerId, origin: PersonOrigin, leader: Option<LeaderHeadType>, unit: Option<UnitId>, name: String, ambition: Ambition, line: &str) -> PersonId {
        let turn = self.game_turn();
        let x = self.persons.insert_with(|id| Person {
            id,
            owner: p,
            origin,
            leader,
            unit,
            name,
            ambition,
            post: Post::None,
            standing: Standing::default(),
            demand: None,
            demand_rest_until: turn,
            rival: None,
            deeds: Vec::new(),
            entered_turn: turn,
        });
        self.players[p].persons.push(x);
        if let Some(l) = leader {
            if !self.persons_drawn.contains(&l) {
                self.persons_drawn.push(l);
            }
        }
        self.person_add_deed(x, format!("Entered the court: {line}."));
        let name = self.persons[x].name.clone();
        let title = leader.and_then(|l| leader_title(self.d, l));
        self.person_msg(x, || match title {
            Some(t) => format!("{name}, {t}, enters the court: {line}."),
            None => format!("{name} enters the court: {line}."),
        });
        let how = match origin {
            PersonOrigin::Hero => "hero",
            PersonOrigin::Veteran => "veteran",
            PersonOrigin::GreatPerson => "great_person",
            PersonOrigin::Ruler => "ruler",
        };
        self.tel_person(p, "entered", how);
        x
    }

    /// A new person of player `p` without a word (no message, no record line): the head a House
    /// takes when a civilization becomes a vassal (origin `Ruler`), told by the vassalage's own
    /// message.
    pub(crate) fn person_new_quiet(&mut self, p: PlayerId, origin: PersonOrigin, leader: LeaderHeadType, line: &str) -> PersonId {
        let turn = self.game_turn();
        let name = leader_name(self.d, leader);
        let ambition = ambition_of_leader(self.d, leader);
        let x = self.persons.insert_with(|id| Person {
            id,
            owner: p,
            origin,
            leader: Some(leader),
            unit: None,
            name,
            ambition,
            post: Post::None,
            standing: Standing::default(),
            demand: None,
            demand_rest_until: turn,
            rival: None,
            deeds: Vec::new(),
            entered_turn: turn,
        });
        self.players[p].persons.push(x);
        if !self.persons_drawn.contains(&leader) {
            self.persons_drawn.push(leader);
        }
        self.person_add_deed(x, format!("Entered the court: {line}."));
        self.tel_person(p, "entered", "ruler");
        x
    }

    /// Entry in player `p`'s turn processing: every hero that is no person yet; then at most one
    /// veteran, the army unit of the highest level (ties: the lower id) at the bar.
    pub(crate) fn player_persons_entry(&mut self, p: PlayerId) {
        let n = names(self.d);
        let taken: Vec<UnitId> = self.players[p].persons.iter().filter_map(|&x| self.persons.get(x).and_then(|person| person.unit)).collect();
        let fresh = |g: &Game, u: UnitId| !taken.contains(&u) && !g.persons_barred.contains(&u);
        let heroes: Vec<UnitId> = self.players[p].units.iter().copied().filter(|&u| self.units[u].duration == 0 && self.unit_is_hero(u) && fresh(self, u)).collect();
        for u in heroes {
            let info = &self.d.units[self.units[u].unit_type];
            let ambition = match info.unit_combat_type {
                Some(c) if Some(c) == n.disciple => Ambition::Faith,
                Some(c) if Some(c) == n.adept => Ambition::Works,
                _ => Ambition::War,
            };
            let name = if self.units[u].name.is_empty() { self.d.text.name(&info.base.description).to_string() } else { self.units[u].name.clone() };
            let line = self.unit_entry_line(u);
            self.person_new(p, PersonOrigin::Hero, None, Some(u), name, ambition, &line);
        }
        // One veteran a turn.
        let veterans = self.players[p].persons.iter().filter(|&&x| self.persons.get(x).is_some_and(|person| person.origin == PersonOrigin::Veteran)).count() as i32;
        let bar = self.d.defines.person_veteran_level + veterans;
        let best = self.players[p]
            .units
            .iter()
            .copied()
            .filter(|&u| self.units[u].level >= bar && fresh(self, u) && self.unit_is_army_unit(u))
            .max_by_key(|&u| (self.units[u].level, std::cmp::Reverse(u)));
        let Some(u) = best else { return };
        let Some(l) = self.persons_draw(p) else { return };
        let name = leader_name(self.d, l);
        self.unit_set_name(u, name.clone());
        let line = self.unit_entry_line(u);
        let ambition = ambition_of_leader(self.d, l);
        self.person_new(p, PersonOrigin::Veteran, Some(l), Some(u), name, ambition, &line);
    }

    /// Person `x` leaves the cast: their command is empty, their veteran's unit loses the name and
    /// stays an ordinary unit, a hero's unit stays a hero without a person; neither unit enters
    /// again. (A House's head leaves only with their House's record: `house_record_end`.)
    pub(crate) fn person_leave(&mut self, x: PersonId) {
        let Some(person) = self.persons.get(x) else { return };
        let p = person.owner;
        let origin = person.origin;
        let unit = self.person_unit(x);
        let post = person.post;
        match post {
            Post::Commander(a) => {
                if let Some(army) = self.armies.get_mut(a) {
                    if army.commander == Some(x) {
                        army.commander = None;
                    }
                    if army.commander_named == Some(x) {
                        army.commander_named = None;
                    }
                }
            }
            Post::Head(h) => {
                if self.houses.contains(h) && self.houses[h].head == Some(x) {
                    self.houses[h].head = None;
                }
            }
            Post::None => {}
        }
        self.persons.remove(x);
        self.players[p].persons.retain(|&y| y != x);
        if let Some(u) = unit {
            if origin == PersonOrigin::Veteran {
                self.unit_set_name(u, String::new());
            }
            if !self.persons_barred.contains(&u) {
                self.persons_barred.push(u);
            }
        }
    }

    /// Hook of `unit_convert`: unit `u` replaces `from` (an upgrade or a conversion); a person
    /// follows their unit when the owner stays (a unit that changes owner leaves the cast with
    /// `from`'s death).
    pub(crate) fn unit_persons_converted(&mut self, u: UnitId, from: UnitId) {
        if self.units[u].owner != self.units[from].owner {
            return;
        }
        if let Some(x) = self.unit_person(from) {
            self.persons[x].unit = Some(u);
            if self.persons[x].origin == PersonOrigin::Veteran {
                let name = self.persons[x].name.clone();
                self.unit_set_name(u, name);
            }
            let st = self.holder_standing_mut(self.person_holder(x));
            for s in st.sworn.iter_mut() {
                if *s == from {
                    *s = u;
                }
            }
        }
        if let Some(i) = self.persons_barred.iter().position(|&b| b == from) {
            self.persons_barred[i] = u;
        }
    }

    /// Hook of `unit_kill`: unit `u` dies; its person leaves the cast.
    pub(crate) fn unit_persons_lost(&mut self, u: UnitId) {
        if let Some(x) = self.unit_person(u) {
            let name = self.persons[x].name.clone();
            self.person_msg(x, || format!("{name} is dead."));
            self.person_leave(x);
        }
        self.persons_barred.retain(|&b| b != u);
    }

    /// Drops references to what is gone (start of the persons turn): a person whose unit died
    /// without the hook (removed outright) leaves; rivals that left are forgotten; armies gone
    /// release their commanders; barred units that died are forgotten.
    pub(crate) fn player_persons_drop_dead(&mut self, p: PlayerId) {
        for x in self.players[p].persons.clone() {
            if !self.persons.contains(x) {
                self.players[p].persons.retain(|&y| y != x);
                continue;
            }
            if let Some(u) = self.persons[x].unit {
                if !self.units.contains(u) || self.units[u].owner != p {
                    self.person_leave(x);
                    continue;
                }
            }
            if self.persons[x].rival.is_some_and(|r| !self.persons.contains(r)) {
                self.persons[x].rival = None;
            }
            if let Post::Commander(a) = self.persons[x].post {
                if !self.armies.contains(a) || self.armies[a].commander != Some(x) {
                    self.persons[x].post = Post::None;
                }
            }
            // A head whose House is gone leads a civilization of their own: out of the cast.
            if let Post::Head(h) = self.persons[x].post {
                if !self.houses.contains(h) {
                    self.person_leave(x);
                    continue;
                }
            }
            let hd = self.person_holder(x);
            if let Holder::Person(_) = hd {
                let units = &self.units;
                self.persons[x].standing.sworn.retain(|&s| units.contains(s));
            }
        }
        let units = &self.units;
        self.persons_barred.retain(|&b| units.contains(b));
    }
}
