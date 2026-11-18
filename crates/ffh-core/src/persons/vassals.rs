//! Every vassal is a House: the records the vassal rules make and end, the tribute, the levy,
//! freeing a House (docs/PERSONS.md, "The second build", rules 1, 2, 4, 6, 8).
//!
//! - **The records** ([`Game::team_vassal_houses`], called by `team_set_vassal` both ways): when a
//!   team becomes a vassal by any means (a capitulation, a vassal deal, a card's vassal birth, a
//!   founding, a colony), each of its living players that is no human gets a House record owned by
//!   the master team's leading player, headed by a person for its leader (a person of the
//!   liege's cast with that leader: a founder; else a new one, origin `Ruler`, without a unit).
//!   Its starting modifier: `Capitulated: -4 (100 turns)` for a capitulation, `Sought
//!   protection: +2 (60 turns)` otherwise (a founding, a card's birth and a colony clear it).
//!   When the vassalage ends by any means, or the House's player dies, the record ends and its
//!   head leaves the cast. A player that becomes a House loses its own cast (its own Houses are
//!   freed by the vassal rules: a vassal cannot have vassals).
//! - **Who may hold vassals** ([`Game::team_vassal_refusal`]): not a vassal, not under No Vassal
//!   States (the Civ4 rules). A liege under a direct-rule civic holds its vassals as Houses too: it
//!   grants them no city, and adopting direct rule is civil war (rule 11, `secession.rs`).
//! - **Tribute** ([`Game::player_persons_tribute`], the liege's turn processing): for each House
//!   in id order, its tribute percent of its cities' gold commerce, from its treasury (no more
//!   than it holds) to the liege's, and of their research commerce, added to the liege team's
//!   current research (lost when it researches nothing).
//! - **The levy**: an army's requisition takes the cities of its owner's Houses; a House Annoyed or
//!   worse takes none ([`Game::house_levy_refusal`]); each order placed is `Levied: -1 (20
//!   turns)` ([`Game::house_levied`]); the unit trained is the army owner's (`city/turn.rs`).
//! - **Freeing** ([`Game::player_free_house`]): the vassalage ends at peace.

use ffh_data::*;

use super::loyalty::Holder;
use super::*;

impl Game {
    // -----------------------------------------------------------------------------------------
    // Names and colours
    // -----------------------------------------------------------------------------------------

    /// A civilization's name in the Houses' lines: its own description when it has one of its
    /// own (`House Arne`, `Bannor of House Arne`), else its short description (`Hippus`).
    pub fn player_house_name(&self, p: PlayerId) -> String {
        let own = &self.setup.players[p.idx()].civ_description;
        if own.is_empty() {
            self.player_get_civilization_short_description(p)
        } else {
            own.clone()
        }
    }

    /// The player whose colour player `p` is drawn in on the map: a House's liege (the realm reads
    /// as one; the shields on the cities' plates tell the Houses apart), else `p`.
    pub fn player_color_player(&self, p: PlayerId) -> PlayerId {
        match self.player_house(p) {
            Some(h) => self.houses[h].owner,
            None => p,
        }
    }

    /// The House of city `c`'s owner (`None`: a city of a civilization that is no House).
    pub fn city_house(&self, c: CityId) -> Option<HouseId> {
        let owner = self.cities.get(c)?.owner;
        self.player_house(owner)
    }

    /// The liege of player `p` if it is a House.
    pub fn player_liege(&self, p: PlayerId) -> Option<PlayerId> {
        self.player_house(p).map(|h| self.houses[h].owner)
    }

    // -----------------------------------------------------------------------------------------
    // Who may hold vassals
    // -----------------------------------------------------------------------------------------

    /// Why team `master` may not take a vassal now (`None`: it may): under No Vassal States, or a
    /// vassal itself.
    pub fn team_vassal_refusal(&self, master: TeamId) -> Option<&'static str> {
        if self.is_option(GameOptionType::GAMEOPTION_NO_VASSAL_STATES) {
            return Some("Game option: no vassal states.");
        }
        if self.team_is_a_vassal(master) {
            return Some("A vassal cannot have vassals.");
        }
        None
    }

    // -----------------------------------------------------------------------------------------
    // The records
    // -----------------------------------------------------------------------------------------

    /// Hook of `team_set_vassal(t, master, value, capitulated)`: team `t` became `master`'s vassal
    /// (`value`) or stopped being it. Every living player of `t` that is no human gets a House
    /// record, or loses it.
    pub(crate) fn team_vassal_houses(&mut self, t: TeamId, master: TeamId, value: bool, capitulated: bool) {
        let members: Vec<PlayerId> = self.player_ids().filter(|&p| self.players[p].team == t).collect();
        // A vassal has no Crown limit (rule 14), a vassal freed has one again.
        for &p in &members {
            if self.players[p].alive {
                self.player_update_maintenance(p);
            }
        }
        if value {
            let Some(owner) = self.team_get_leader_id(master) else { return };
            if !self.player_has_persons(owner) {
                return;
            }
            let origin = if capitulated { HouseOrigin::Capitulated } else { HouseOrigin::Vassal };
            for hp in members {
                if !self.players[hp].alive || self.players[hp].human || self.player_is_barbarian(hp) {
                    continue;
                }
                if let Some(h) = self.player_house(hp) {
                    if self.houses[h].owner == owner {
                        continue;
                    }
                    self.house_record_end(h);
                }
                self.house_begin(hp, owner, origin);
            }
        } else {
            for hp in members {
                if let Some(h) = self.player_house(hp) {
                    if self.players[self.houses[h].owner].team == master {
                        self.house_record_end(h);
                    }
                }
            }
        }
    }

    /// Player `hp` becomes a House of `owner` (`origin`): the record, headed by the person of
    /// `owner`'s cast with `hp`'s leader (a founder), else a new `Ruler`; its starting modifier.
    pub(crate) fn house_begin(&mut self, hp: PlayerId, owner: PlayerId, origin: HouseOrigin) -> Option<HouseId> {
        // Its own cast leaves (a House has no persons of its own).
        for x in self.players[hp].persons.clone() {
            if self.persons.contains(x) {
                self.person_leave(x);
            }
        }
        self.players[hp].persons.clear();
        let leader = self.player_get_leader_type(hp);
        let name = self.player_house_name(hp);
        let existing = leader.and_then(|l| self.players[owner].persons.iter().copied().find(|&x| self.persons.get(x).is_some_and(|person| person.leader == Some(l) && !matches!(person.post, Post::Head(_)))));
        // A person of the cast with its leader founded it (a founding names it and tells it).
        let origin = if existing.is_some() { HouseOrigin::Founded } else { origin };
        let head = leader.map(|l| existing.unwrap_or_else(|| self.person_new_quiet(owner, PersonOrigin::Ruler, l, &format!("ruler of the {name}"))));
        let turn = self.game_turn();
        let color = self.house_new_color(owner);
        let tribute = self.players[owner].house_tribute;
        let mut standing = Standing::default();
        if let Some(x) = head {
            // A founder's own modifiers move to the House; a commander's command ends.
            standing.mods = std::mem::take(&mut self.persons[x].standing.mods);
            self.persons[x].standing = Standing::default();
            if let Post::Commander(a) = self.persons[x].post {
                if let Some(army) = self.armies.get_mut(a) {
                    if army.commander == Some(x) {
                        army.commander = None;
                    }
                    if army.commander_named == Some(x) {
                        army.commander_named = None;
                    }
                }
            }
        }
        let start = match origin {
            HouseOrigin::Capitulated => Some((LoyaltyCause::Capitulated, CAPITULATED)),
            HouseOrigin::Vassal => Some((LoyaltyCause::SoughtProtection, SOUGHT_PROTECTION)),
            HouseOrigin::Founded => None,
        };
        if let Some((cause, (amount, turns))) = start {
            standing.mods.push(LoyaltyMod { cause, amount, until: Some(turn + turns) });
        }
        let h = self.houses.insert_with(|id| House {
            id,
            owner,
            player: hp,
            origin,
            name,
            head,
            standing,
            founded_turn: turn,
            color,
            tribute,
            last_gold: 0,
            last_research: 0,
            last_turn: -1,
        });
        self.players[owner].houses.push(h);
        self.players[hp].house = Some(h);
        if let Some(x) = head {
            self.persons[x].post = Post::Head(h);
            let label = self.house_label(h);
            self.person_add_deed(x, format!("Heads {label}."));
        }
        // The state told so far is the one it starts in (no change to tell).
        let state = self.house_state(h);
        self.houses[h].standing.state = state;
        let label = self.house_label(h);
        let what = match origin {
            HouseOrigin::Capitulated => "capitulated",
            HouseOrigin::Vassal | HouseOrigin::Founded => "vassal",
        };
        if origin != HouseOrigin::Founded {
            self.house_msg(h, || format!("{label} is a House of ours: {}.", if origin == HouseOrigin::Capitulated { "it capitulated" } else { "it became our vassal" }));
        }
        self.tel_person(owner, "house", what);
        Some(h)
    }

    /// A card's vassal birth or a colony: House `h` of player `hp` takes no starting modifier.
    pub(crate) fn player_house_birth(&mut self, hp: PlayerId) {
        if let Some(h) = self.player_house(hp) {
            self.houses[h].standing.mods.retain(|m| m.cause != LoyaltyCause::SoughtProtection);
            let state = self.house_state(h);
            self.houses[h].standing.state = state;
        }
    }

    /// House `h`'s record ends (its vassalage ended, or its player died): its head leaves the
    /// cast, its liege's armies no longer levy its cities.
    pub(crate) fn house_record_end(&mut self, h: HouseId) {
        let Some(house) = self.houses.get(h) else { return };
        let (owner, hp, head) = (house.owner, house.player, house.head);
        if let Some(x) = head.filter(|&x| self.persons.contains(x)) {
            self.persons[x].post = Post::None;
            self.persons[x].standing = Standing::default();
            let label = self.house_label(h);
            self.person_add_deed(x, format!("Left the court with {label}."));
            self.person_leave(x);
        }
        self.players[owner].houses.retain(|&o| o != h);
        if self.players[hp].house == Some(h) {
            self.players[hp].house = None;
        }
        self.houses.remove(h);
        // The liege's armies requisition only from its own cities and its Houses'.
        for a in self.players[owner].armies.clone() {
            if !self.armies.contains(a) {
                continue;
            }
            let keep: Vec<CityId> = self.armies[a].requisition.iter().copied().filter(|&c| self.cities.contains(c) && (self.cities[c].owner == owner || self.player_liege(self.cities[c].owner) == Some(owner))).collect();
            self.armies[a].requisition = keep;
        }
        self.tel_person(owner, "house", "ended");
    }

    /// Hook of `player_set_alive(p, false)`: a House's player died, its record ends.
    pub(crate) fn player_persons_died(&mut self, p: PlayerId) {
        if let Some(h) = self.player_house(p) {
            self.house_record_end(h);
        }
    }

    /// The vassalage of team `vassal` to team `master` ends at peace: the deals that record it are
    /// cancelled (`team_free_vassal`), and the flag cleared if no deal recorded it.
    pub(crate) fn team_end_vassalage(&mut self, vassal: TeamId, master: TeamId) {
        self.team_free_vassal(master, vassal);
        if self.teams[vassal].vassal[master] {
            self.team_set_vassal(vassal, master, false, false);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Freeing a House
    // -----------------------------------------------------------------------------------------

    /// Whether player `p` may free House `h` (else why not).
    pub fn player_check_free_house(&self, p: PlayerId, h: HouseId) -> Result<(), &'static str> {
        if !self.houses.contains(h) || self.houses[h].owner != p {
            return Err("Not a House of yours.");
        }
        Ok(())
    }

    /// Player `p` frees House `h`: the vassalage ends at peace, the record ends, its head leaves
    /// the cast (docs/PERSONS.md, rule 8).
    pub fn player_free_house(&mut self, p: PlayerId, h: HouseId) -> Result<(), &'static str> {
        self.player_check_free_house(p, h)?;
        let hp = self.houses[h].player;
        let label = self.house_label(h);
        let civ = self.player_get_civilization_short_description(p);
        let (ht, pt) = (self.players[hp].team, self.players[p].team);
        if let Some(x) = self.houses[h].head.filter(|&x| self.persons.contains(x)) {
            self.person_add_deed(x, format!("{label} freed."));
        }
        self.house_msg(h, || format!("{label} is freed: it is no longer our vassal."));
        self.team_end_vassalage(ht, pt);
        // (The vassal rules ended the record; a vassalage no deal recorded too.)
        if self.houses.contains(h) {
            self.house_record_end(h);
        }
        let turn = self.game_turn();
        let to = self.audience_met(pt);
        self.log.msg_to(turn, to, || format!("{label} is freed by the {civ}."));
        self.tel_person(p, "house", "freed");
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Tribute
    // -----------------------------------------------------------------------------------------

    /// Whether `pct` is a tribute: 0 to [`TRIBUTE_MAX`] by [`TRIBUTE_STEP`].
    fn tribute_check(pct: i32) -> Result<(), &'static str> {
        if !(0..=TRIBUTE_MAX).contains(&pct) || pct % TRIBUTE_STEP != 0 {
            return Err("Tribute: 0 to 50 percent, in steps of 10.");
        }
        Ok(())
    }

    /// Whether player `p` may set House `h`'s tribute to `pct` percent (else why not).
    pub fn player_check_house_tribute(&self, p: PlayerId, h: HouseId, pct: i32) -> Result<(), &'static str> {
        if !self.houses.contains(h) || self.houses[h].owner != p {
            return Err("Not a House of yours.");
        }
        Self::tribute_check(pct)
    }

    /// Player `p` sets House `h`'s tribute to `pct` percent (its standing loyalty part follows).
    pub fn player_set_house_tribute(&mut self, p: PlayerId, h: HouseId, pct: i32) -> Result<(), &'static str> {
        self.player_check_house_tribute(p, h, pct)?;
        self.houses[h].tribute = pct;
        Ok(())
    }

    /// Player `p` sets its realm's default tribute (`Player::house_tribute`), which a new House
    /// takes.
    pub fn player_set_house_tribute_default(&mut self, p: PlayerId, pct: i32) -> Result<(), &'static str> {
        if !self.player_has_persons(p) {
            return Err("This civilization has no Houses.");
        }
        Self::tribute_check(pct)?;
        self.players[p].house_tribute = pct;
        Ok(())
    }

    /// The gold commerce and the research commerce of House `h`'s cities (a turn's).
    pub fn house_commerce(&self, h: HouseId) -> (i32, i32) {
        let mut gold = 0;
        let mut research = 0;
        for c in self.house_cities(h) {
            gold += self.city_commerce_rate(c, CommerceType::COMMERCE_GOLD);
            research += self.city_commerce_rate(c, CommerceType::COMMERCE_RESEARCH);
        }
        (gold, research)
    }

    /// What House `h` would pay its liege this turn at a tribute of `pct` percent: (gold, research).
    /// The gold is no more than its treasury holds; the research is 0 while the liege researches
    /// nothing.
    pub fn house_tribute_at(&self, h: HouseId, pct: i32) -> (i32, i32) {
        let Some(house) = self.houses.get(h) else { return (0, 0) };
        let (gold, research) = self.house_commerce(h);
        let gold = (gold * pct / 100).min(self.players[house.player].gold).max(0);
        let research = if self.player_get_current_research(house.owner).is_some() { (research * pct / 100).max(0) } else { 0 };
        (gold, research)
    }

    /// Player `p`'s Houses pay their tribute (its turn processing), in id order.
    pub(crate) fn player_persons_tribute(&mut self, p: PlayerId) {
        let mut houses: Vec<HouseId> = self.players[p].houses.iter().copied().filter(|&h| self.houses.contains(h)).collect();
        houses.sort();
        let team = self.players[p].team;
        let turn = self.game_turn();
        for h in houses {
            let hp = self.houses[h].player;
            if !self.players[hp].alive {
                continue;
            }
            let pct = self.houses[h].tribute;
            let (gold, mut research) = self.house_tribute_at(h, pct);
            if gold > 0 {
                self.players[hp].gold -= gold;
                self.players[p].gold += gold;
            }
            match self.player_get_current_research(p) {
                Some(tech) if research > 0 => self.team_change_research_progress(team, tech, research, p),
                _ => research = 0,
            }
            let house = &mut self.houses[h];
            house.last_gold = gold;
            house.last_research = research;
            house.last_turn = turn;
            self.tel_tribute(p, gold, research);
        }
    }

    // -----------------------------------------------------------------------------------------
    // The levy
    // -----------------------------------------------------------------------------------------

    /// Why House `h` takes no levy, if it takes none: `House Arne is Annoyed. It takes no levy.`
    pub fn house_levy_refusal(&self, h: HouseId) -> Option<String> {
        let state = self.house_state(h);
        state.withholds().then(|| format!("{} is {}. It takes no levy.", self.house_label(h), state.name()))
    }

    /// Whether city `c` belongs to a House of player `p` (a city `p`'s armies may levy).
    pub fn city_levied_by(&self, c: CityId, p: PlayerId) -> bool {
        self.cities.get(c).is_some_and(|city| self.player_liege(city.owner) == Some(p))
    }

    /// Why city `c` takes no levy order now, if it takes none (a city of a House Annoyed or
    /// worse); `None` for a city of no House.
    pub fn city_levy_refusal(&self, c: CityId) -> Option<String> {
        self.city_house(c).and_then(|h| self.house_levy_refusal(h))
    }

    /// The cities player `p`'s armies may requisition from beyond its own: its Houses' (in House
    /// order, then each House player's city order).
    pub fn player_levy_cities(&self, p: PlayerId) -> Vec<CityId> {
        self.players[p].houses.iter().filter(|&&h| self.houses.contains(h)).flat_map(|&h| self.house_cities(h)).collect()
    }

    /// A levy order was placed in a city of House `h`: `Levied: -1 (20 turns)`.
    pub(crate) fn house_levied(&mut self, h: HouseId) {
        if !self.houses.contains(h) {
            return;
        }
        self.holder_add_mod(Holder::House(h), LoyaltyCause::Levied, LEVIED);
        let owner = self.houses[h].owner;
        self.tel_person(owner, "levy", "placed");
    }
}
