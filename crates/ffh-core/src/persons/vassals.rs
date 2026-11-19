//! Every vassal is a House: the records the vassal rules make and end, the tribute, the levy,
//! freeing a House (docs/PERSONS.md, "The second build", rules 1, 2, 4, 6, 8).
//!
//! - **The records** ([`Game::team_vassal_houses`], called by `team_set_vassal` both ways): when a
//!   team becomes a vassal by any means (a capitulation, a vassal deal, a card's vassal birth, a
//!   founding, a colony), each of its living players that is no human gets a House record owned by
//!   the master team's leading player; its leader is its player's. Its starting modifier:
//!   `Capitulated: -4 (100 turns)` for a capitulation, `Sought protection: +2 (60 turns)`
//!   otherwise (a founding, a card's birth and a colony clear it). When the vassalage ends by any
//!   means, or the House's player dies, the record ends. A player that becomes a House has no
//!   Houses of its own (they are freed by the vassal rules: a vassal cannot have vassals).
//! - **Who may hold vassals** ([`Game::team_vassal_refusal`]): not a vassal, not under No Vassal
//!   States (the Civ4 rules). A liege under a direct-rule civic holds its vassals as Houses too: it
//!   grants them no city, and adopting direct rule is civil war (rule 11, `secession.rs`).
//! - **Tribute** ([`Game::player_persons_tribute`], the liege's turn processing): for each House
//!   in id order, its rate in force (`House::tribute_now`) of its cities' gold commerce, from its
//!   treasury (no more than it holds) to the liege's, and of their research commerce, added to the
//!   liege team's current research (lost when it researches nothing). The liege sets a rate
//!   (`House::tribute`, [`Game::player_set_house_tribute`]); the rate in force moves one step
//!   toward it every [`TRIBUTE_LAG_TURNS`] turns, both ways, counted from its last step or, when
//!   it was at rest, from the setting ([`Game::house_tribute_in_force_turn`]). A new House starts
//!   with its realm's default in force.
//! - **The levy**: an army's requisition takes the cities of its owner's Houses; a House Annoyed or
//!   worse takes none ([`Game::house_levy_refusal`]); each order placed is `Levied: -1 (20
//!   turns)` ([`Game::house_levied`]); the unit trained is the army owner's (`city/turn.rs`).
//! - **Freeing** ([`Game::player_free_house`]): the vassalage ends at peace.

use ffh_data::*;

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
            if !self.player_has_houses(owner) {
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

    /// Player `hp` becomes a House of `owner` (`origin`): the record and its starting modifier. A
    /// founding ([`Game::player_found_house`]) is known by the name its player was born with,
    /// `House <name>` with the name its liege has just taken for it, and by its cities, all
    /// acquired this turn (a House that broke away and becomes a vassal again is no founding).
    pub(crate) fn house_begin(&mut self, hp: PlayerId, owner: PlayerId, origin: HouseOrigin) -> Option<HouseId> {
        let name = self.player_house_name(hp);
        let turn = self.game_turn();
        let cities = &self.players[hp].cities;
        let born_now = !cities.is_empty() && cities.iter().all(|&c| self.cities.get(c).is_some_and(|city| city.game_turn_acquired == turn));
        let founded = origin != HouseOrigin::Capitulated && born_now && name.strip_prefix("House ").is_some_and(|n| self.players[owner].house_names.last().is_some_and(|l| l == n));
        let origin = if founded { HouseOrigin::Founded } else { origin };
        // A founding's civilization is described `House <name>`; the record keeps the name, and
        // its label adds `House` again.
        let name = match origin {
            HouseOrigin::Founded => name.strip_prefix("House ").map_or(name.clone(), str::to_string),
            _ => name,
        };
        let color = self.house_new_color(owner);
        let tribute = self.players[owner].house_tribute;
        let mut standing = Standing::default();
        let start = match origin {
            HouseOrigin::Capitulated => Some((LoyaltyCause::Capitulated, CAPITULATED)),
            HouseOrigin::Vassal => Some((LoyaltyCause::SoughtProtection, SOUGHT_PROTECTION)),
            HouseOrigin::Founded | HouseOrigin::Split => None,
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
            standing,
            demand: None,
            demand_rest_until: turn,
            deeds: Vec::new(),
            founded_turn: turn,
            color,
            tribute,
            tribute_now: tribute,
            tribute_since: turn,
            last_gold: 0,
            last_research: 0,
            last_turn: -1,
            refused: None,
            contract: None,
            ultimatums: 0,
        });
        self.players[owner].houses.push(h);
        self.players[hp].house = Some(h);
        let label = self.house_label(h);
        let civ = self.player_get_civilization_short_description(owner);
        let deed = match origin {
            HouseOrigin::Founded | HouseOrigin::Split => format!("A House of the {civ}."),
            HouseOrigin::Capitulated => format!("Capitulated to the {civ}."),
            HouseOrigin::Vassal => format!("Became a vassal of the {civ}."),
        };
        self.house_add_deed(h, deed);
        // The state told so far is the one it starts in (no change to tell).
        let state = self.house_state(h);
        self.houses[h].standing.state = state;
        let what = match origin {
            HouseOrigin::Capitulated => "capitulated",
            HouseOrigin::Vassal | HouseOrigin::Founded | HouseOrigin::Split => "vassal",
        };
        if !origin.founded() {
            self.house_msg(h, || format!("{label} is your House: {}.", if origin == HouseOrigin::Capitulated { "it capitulated" } else { "it became your vassal" }));
        }
        self.tel_house(owner, "record", what);
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

    /// House `h`'s record ends (its vassalage ended, or its player died): its liege's armies no
    /// longer levy its cities.
    pub(crate) fn house_record_end(&mut self, h: HouseId) {
        if !self.houses.contains(h) {
            return;
        }
        // A contract ends with the record (rule 6: its gold a turn and its resource trade stop).
        self.house_contract_end(h, false);
        let Some(house) = self.houses.get(h) else { return };
        let (owner, hp) = (house.owner, house.player);
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
        self.tel_house(owner, "record", "ended");
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

    /// Player `p` frees House `h`: the vassalage ends at peace, the record ends (docs/PERSONS.md,
    /// rule 8).
    pub fn player_free_house(&mut self, p: PlayerId, h: HouseId) -> Result<(), &'static str> {
        self.player_check_free_house(p, h)?;
        let hp = self.houses[h].player;
        let label = self.house_label(h);
        let civ = self.player_get_civilization_short_description(p);
        let (ht, pt) = (self.players[hp].team, self.players[p].team);
        self.house_msg(h, || format!("{label} is freed: it is no longer your vassal."));
        self.team_end_vassalage(ht, pt);
        // (The vassal rules ended the record; a vassalage no deal recorded too.)
        if self.houses.contains(h) {
            self.house_record_end(h);
        }
        let turn = self.game_turn();
        let to = self.audience_met(pt);
        self.log.msg_to(turn, to, || format!("{label} is freed by the {civ}."));
        self.tel_house(p, "record", "freed");
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

    /// Whether player `p` may set House `h`'s tribute to `pct` percent (else why not): a contract's
    /// bound refuses a rate above it (rule 6: `Your contract with House Arne forbids it until turn
    /// 160.`).
    pub fn player_check_house_tribute(&self, p: PlayerId, h: HouseId, pct: i32) -> Result<(), String> {
        if !self.houses.contains(h) || self.houses[h].owner != p {
            return Err("Not a House of yours.".into());
        }
        Self::tribute_check(pct)?;
        if let Some(why) = self.player_contract_refusal(p, ContractAct::Tribute { house: h, pct }) {
            return Err(why);
        }
        Ok(())
    }

    /// Player `p` sets House `h`'s tribute to `pct` percent: the rate the rate in force moves
    /// toward, one step every [`TRIBUTE_LAG_TURNS`] turns (the payment and the loyalty part follow
    /// the rate in force). A rate in force at rest starts its count now; one already moving keeps
    /// its count (a change of mind neither costs nor gains time).
    pub fn player_set_house_tribute(&mut self, p: PlayerId, h: HouseId, pct: i32) -> Result<(), String> {
        self.player_check_house_tribute(p, h, pct)?;
        let turn = self.game_turn();
        let house = &mut self.houses[h];
        if house.tribute_now == house.tribute && pct != house.tribute {
            house.tribute_since = turn;
        }
        house.tribute = pct;
        Ok(())
    }

    /// The game turn the next step of House `h`'s rate in force would take effect were its rate set
    /// to `pct` now (`pct` differs from the rate in force): [`TRIBUTE_LAG_TURNS`] after the count's
    /// start, which a new rate sets to now when the rate in force is at rest; at the earliest
    /// next turn (a step overdue takes effect at the end of this turn's processing).
    fn house_tribute_next_step(&self, h: HouseId, pct: i32) -> i32 {
        let house = &self.houses[h];
        let turn = self.game_turn();
        let since = if house.tribute_now == house.tribute && pct != house.tribute { turn } else { house.tribute_since };
        (since + TRIBUTE_LAG_TURNS).max(turn + 1)
    }

    /// The game turn from which a tribute of `pct` percent would be in force for House `h` were it
    /// set now (for the rate set: when it will be reached): this turn when it is the rate in
    /// force; else the next step's turn and [`TRIBUTE_LAG_TURNS`] more for each further step.
    pub fn house_tribute_in_force_turn(&self, h: HouseId, pct: i32) -> i32 {
        let now = self.houses[h].tribute_now;
        if pct == now {
            return self.game_turn();
        }
        let steps = (pct - now).abs() / TRIBUTE_STEP;
        self.house_tribute_next_step(h, pct) + (steps - 1) * TRIBUTE_LAG_TURNS
    }

    /// The turns until a tribute of `pct` percent would be in force for House `h` were it set now
    /// (0: it is the rate in force). For its own rate: the turns until it is reached.
    pub fn house_tribute_turns(&self, h: HouseId, pct: i32) -> i32 {
        self.house_tribute_in_force_turn(h, pct) - self.game_turn()
    }

    /// The rate in force of House `h` `turns` turns from now, were its rate set to `pct` now.
    pub fn house_tribute_in_force_after(&self, h: HouseId, pct: i32, turns: i32) -> i32 {
        let now = self.houses[h].tribute_now;
        if pct == now {
            return now;
        }
        let at = self.game_turn() + turns;
        let first = self.house_tribute_next_step(h, pct);
        if at < first {
            return now;
        }
        let steps = (1 + (at - first) / TRIBUTE_LAG_TURNS).min((pct - now).abs() / TRIBUTE_STEP);
        now + (pct - now).signum() * steps * TRIBUTE_STEP
    }

    /// Player `p` sets its realm's default tribute (`Player::house_tribute`), which a new House
    /// takes.
    pub fn player_set_house_tribute_default(&mut self, p: PlayerId, pct: i32) -> Result<(), &'static str> {
        if !self.player_has_houses(p) {
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

    /// Player `p`'s Houses pay their tribute at the rate in force (its turn processing), in id
    /// order; then each rate in force whose count has run steps toward its set rate, in force from
    /// the next turn.
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
            let pct = self.houses[h].tribute_now;
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
            self.house_tribute_step(h);
        }
    }

    /// House `h`'s rate in force steps toward its set rate when its count has run
    /// ([`TRIBUTE_LAG_TURNS`] turns from `House::tribute_since`): in force from the next turn,
    /// which starts the count again. At the end of its liege's turn processing.
    fn house_tribute_step(&mut self, h: HouseId) {
        let next = self.game_turn() + 1;
        let house = &mut self.houses[h];
        if house.tribute_now != house.tribute && next >= house.tribute_since + TRIBUTE_LAG_TURNS {
            house.tribute_now += (house.tribute - house.tribute_now).signum() * TRIBUTE_STEP;
            house.tribute_since = next;
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
        self.house_add_mod(h, LoyaltyCause::Levied, LEVIED);
        let owner = self.houses[h].owner;
        self.tel_house(owner, "levy", "placed");
    }
}
