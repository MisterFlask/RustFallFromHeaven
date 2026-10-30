//! Governors (this game's own rules, not FFH2's; `GAMEOPTION_GOVERNORS`; docs/GOVERNORS.md).
//!
//! **Direct rule.** A player rules up to `GOVERNOR_DIRECT_RULE_CITIES` cities directly, the
//! capital always among them; they work as cities always have. Every other city is governed. A
//! new city takes a free direct slot if there is one, else it is governed: gaining a city never
//! takes direct rule away from another. The player may swap a governed city with a directly
//! ruled one (or promote one into a free slot); a city whose status changed keeps it for
//! `GOVERNOR_DIRECT_RULE_SWAP_COOLDOWN` turns. World wonders, rituals (projects) and settlers
//! are built only in directly ruled cities.
//!
//! **Governed cities.** Their production is automated (the controller's production choice, which
//! reads the governor's profile; ffh-ai `governor.rs`) and cannot be set by hand. Each runs in one
//! of two modes: *develop* (buildings, workers, local defenders) or *feed* an army (units for it,
//! sent in transit, until it reaches its target size). The governor's profile weighs the city
//! AI's flavors and sets the citizens' emphasis; its traits are small modifiers of the city
//! ([`Game::city_governor_mods`]).
//!
//! **Governors and the bench.** A governor is a persistent record (`Governor`): name, profile,
//! traits, origin, tenure. A player's governors without a city are its bench. Below the bench's
//! target (the vacant posts plus `GOVERNOR_BENCH_SURPLUS`), each turn adds a candidate with
//! `GOVERNOR_BENCH_GEN_CHANCE`% (the game's RNG), its profile and traits drawn toward the leader's
//! flavors. A governed city without a governor runs under the interim profile until one is
//! assigned: a newly governed city takes the best-matching candidate, and an interim city takes
//! the next candidate that appears. Replacing a governor costs the city
//! `GOVERNOR_SWAP_PENALTY`% of its production and commerce for `GOVERNOR_SWAP_TURNS` turns (both
//! cities when a governor moves from one to another); the replaced governor returns to the
//! bench. A city captured, flipped, traded or razed loses its governor to the game.

use std::collections::BTreeSet;

use ffh_data::*;

use crate::army::RecruitDest;
use crate::game::Game;
use crate::ids::*;

/// A governor (`Game::governors`; owner list `Player::governors`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Governor {
    pub id: GovernorId,
    pub owner: PlayerId,
    pub name: String,
    pub profile: GovernorProfileType,
    pub traits: Vec<GovernorTraitType>,
    /// Where it came from (0: the bench's own draw). Recorded for a later layer; no effect yet.
    pub origin: i32,
    /// The turn it joined its player.
    pub born_turn: i32,
    /// The city it governs; `None`: on the bench.
    pub city: Option<CityId>,
    /// The turn it took its city (its tenure; no effect yet).
    pub since_turn: i32,
}

/// What a governed city's governor (its traits) and a recent replacement change in it.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct GovMods {
    /// Yield rate modifiers (food, production, commerce), %.
    pub yield_modifier: [i32; YieldType::NUM],
    /// Commerce rate modifiers (gold, research, culture, espionage), %.
    pub commerce_modifier: [i32; CommerceType::NUM],
    pub happy: i32,
    pub health: i32,
    pub building_production: i32,
    pub unit_production: i32,
    pub experience: i32,
}

pub type GovResult = Result<(), &'static str>;

impl Game {
    /// Whether this game has governors.
    #[inline]
    pub fn governors_on(&self) -> bool {
        self.is_option(GameOptionType::GAMEOPTION_GOVERNORS)
    }

    /// Whether city `c` is governed (not directly ruled). Barbarian cities are never governed.
    pub fn city_is_governed(&self, c: CityId) -> bool {
        self.governors_on() && !self.cities[c].direct_rule && !self.player_is_barbarian(self.cities[c].owner)
    }

    /// The profile an interim governor runs a city with (the XML's `bInterim` one).
    pub fn governor_interim_profile(&self) -> Option<GovernorProfileType> {
        self.d.governor_profiles.keys().find(|&k| self.d.governor_profiles[k].interim)
    }

    /// The profile city `c` is governed with: its governor's, else the interim one. `None` for a
    /// directly ruled city.
    pub fn city_governor_profile(&self, c: CityId) -> Option<GovernorProfileType> {
        if !self.city_is_governed(c) {
            return None;
        }
        match self.cities[c].governor.and_then(|g| self.governors.get(g)) {
            Some(gov) => Some(gov.profile),
            None => self.governor_interim_profile(),
        }
    }

    /// The modifiers of city `c` from its governor's traits and a recent replacement (all zero
    /// for a directly ruled city).
    pub fn city_governor_mods(&self, c: CityId) -> GovMods {
        let mut m = GovMods::default();
        if !self.city_is_governed(c) {
            return m;
        }
        let city = &self.cities[c];
        if let Some(gov) = city.governor.and_then(|g| self.governors.get(g)) {
            for &t in &gov.traits {
                let info = &self.d.governor_traits[t];
                for y in 0..YieldType::NUM {
                    m.yield_modifier[y] += info.yield_modifier[y];
                }
                for i in 0..CommerceType::NUM {
                    m.commerce_modifier[i] += info.commerce_modifier[i];
                }
                m.happy += info.happy;
                m.health += info.health;
                m.building_production += info.building_production_percent;
                m.unit_production += info.unit_production_percent;
                m.experience += info.experience;
            }
        }
        if city.governor_penalty_turns > 0 {
            let penalty = self.d.defines.governor_swap_penalty;
            m.yield_modifier[YieldType::YIELD_PRODUCTION.idx()] -= penalty;
            m.yield_modifier[YieldType::YIELD_COMMERCE.idx()] -= penalty;
        }
        m
    }

    /// Player `p`'s directly ruled cities.
    pub fn player_direct_cities(&self, p: PlayerId) -> Vec<CityId> {
        self.players[p].cities.iter().copied().filter(|&c| self.cities[c].direct_rule).collect()
    }

    /// The number of cities a player may rule directly.
    pub fn governor_direct_slots(&self) -> usize {
        self.d.defines.governor_direct_rule_cities.max(1) as usize
    }

    /// Player `p`'s bench: its governors without a city, in the order they joined.
    pub fn player_bench(&self, p: PlayerId) -> Vec<GovernorId> {
        self.players[p].governors.iter().copied().filter(|&g| self.governors.get(g).is_some_and(|gov| gov.city.is_none())).collect()
    }

    /// Player `p`'s governed cities without a governor (vacant posts).
    pub fn player_vacant_posts(&self, p: PlayerId) -> Vec<CityId> {
        self.players[p].cities.iter().copied().filter(|&c| self.city_is_governed(c) && self.cities[c].governor.is_none()).collect()
    }

    /// Whether city `c`'s rule may change now (its cooldown after the last change is over).
    pub fn city_rule_change_ready(&self, c: CityId) -> bool {
        let t = self.cities[c].rule_changed_turn;
        t < 0 || self.game_turn() >= t + self.d.defines.governor_direct_rule_swap_cooldown
    }

    // ---------------------------------------------------------------------------------------
    // A city's arrival and departure
    // ---------------------------------------------------------------------------------------

    /// City `c` is new to its owner (founded, captured, traded, flipped): it takes a free direct
    /// slot (always, as the capital), else it is governed. A founded city (`assign`) takes the
    /// best-matching candidate of the bench at once; a city that changed hands is interim until
    /// its owner's turn ends (`player_governance_turn` fills it then), so that the candidate is
    /// matched against the city as it is restored, not the blank one `player_acquire_city`
    /// creates, and a city razed or given away at once costs no governor.
    pub fn city_governance_init(&mut self, c: CityId, assign: bool) {
        if !self.governors_on() {
            return;
        }
        let p = self.cities[c].owner;
        self.cities[c].governor = None;
        self.cities[c].feed_army = None;
        self.cities[c].governor_penalty_turns = 0;
        self.cities[c].rule_changed_turn = -1;
        if self.player_is_barbarian(p) {
            self.cities[c].direct_rule = true;
            return;
        }
        let direct = self.player_direct_cities(p).into_iter().filter(|&o| o != c).count();
        self.cities[c].direct_rule = direct < self.governor_direct_slots() || self.city_is_capital(c);
        if !self.city_is_governed(c) {
            return;
        }
        if assign {
            self.city_become_governed(c);
        } else {
            self.cities[c].production_automated = true;
            self.cities[c].recruit_default = None;
            self.city_apply_governor_emphasis(c);
        }
    }

    /// City `c` was just acquired (`player_acquire_city`, after its state was restored and the
    /// raze decision taken): a governed city has production at once (the governor's, whoever's turn
    /// it is), and its owner learns who will run it.
    pub fn city_governance_acquired(&mut self, c: CityId) {
        if !self.governors_on() || !self.cities.contains(c) || !self.city_is_governed(c) {
            return;
        }
        if self.cities[c].order_queue.is_empty() {
            self.city_choose_production(c, None, None, None, false, false);
        }
        let p = self.cities[c].owner;
        let turn = self.game_turn();
        let name = self.d.text.name(&self.cities[c].name).to_string();
        let bench = !self.player_bench(p).is_empty();
        self.log.msg_at(turn, Some(p), (self.cities[c].x, self.cities[c].y), || {
            if bench {
                format!("{name} is governed: a governor from your bench takes it at the end of the turn.")
            } else {
                format!("{name} is run by an interim governor: no candidate is on your bench.")
            }
        });
    }

    /// City `c` leaves its owner (captured, traded, flipped, razed): its governor leaves the game.
    pub fn city_governance_lost(&mut self, c: CityId) {
        if let Some(g) = self.cities[c].governor.take() {
            if let Some(gov) = self.governors.get(g) {
                let owner = gov.owner;
                self.governors.remove(g);
                self.players[owner].governors.retain(|&x| x != g);
            }
        }
    }

    /// City `c`, ruled directly until now, goes to a governor, who takes over its production: the
    /// orders the player queued (a settler or a wonder among them, which a governed city may not
    /// build) give way to the governor's choice.
    fn city_hand_over(&mut self, c: CityId) {
        self.cities[c].direct_rule = false;
        self.cities[c].rule_changed_turn = self.game_turn();
        self.city_become_governed(c);
        self.city_clear_order_queue(c);
        self.city_choose_production(c, None, None, None, false, false);
    }

    /// City `c` just became governed: production automated, its units for its own garrison unless
    /// its mode says otherwise (the default destination the player gave it under direct rule
    /// cannot be changed while governed), the best candidate of the bench.
    fn city_become_governed(&mut self, c: CityId) {
        let p = self.cities[c].owner;
        self.cities[c].production_automated = true;
        self.cities[c].recruit_default = None;
        let best = self.player_bench(p).into_iter().max_by_key(|&g| (self.governor_match(g, c), std::cmp::Reverse(g)));
        match best {
            Some(g) => self.governor_take_post(g, c),
            None => {
                let turn = self.game_turn();
                let name = self.d.text.name(&self.cities[c].name).to_string();
                self.log.msg_at(turn, Some(p), (self.cities[c].x, self.cities[c].y), || format!("{name} is run by an interim governor: no candidate is on your bench."));
                self.city_apply_governor_emphasis(c);
            }
        }
    }

    /// Governor `g` (on the bench) takes city `c` (vacant).
    fn governor_take_post(&mut self, g: GovernorId, c: CityId) {
        let turn = self.game_turn();
        self.governors[g].city = Some(c);
        self.governors[g].since_turn = turn;
        self.cities[c].governor = Some(g);
        self.city_apply_governor_emphasis(c);
        self.city_governor_changed(c);
    }

    /// The governor of city `c` changed (or its penalty ended): the cached rates follow.
    fn city_governor_changed(&mut self, c: CityId) {
        for i in 0..CommerceType::NUM {
            self.city_update_commerce(c, CommerceType::from_usize(i));
        }
        self.cities[c].ai_assign_work_dirty = true;
    }

    /// The citizens' emphasis of city `c`: its governor's profile's (none when directly ruled:
    /// the player's own choice is kept from then on).
    fn city_apply_governor_emphasis(&mut self, c: CityId) {
        let Some(profile) = self.city_governor_profile(c) else { return };
        let wanted: Vec<EmphasizeType> = self.d.governor_profiles[profile].emphasizes.clone();
        for e in self.d.emphasizes.keys() {
            let on = wanted.contains(&e);
            if self.cities[c].ai_emphasize[e] != on {
                self.city_ai_set_emphasize(c, e, on);
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Commands (the human's buttons; the AI uses them too)
    // ---------------------------------------------------------------------------------------

    /// Makes `to_rule` (governed) directly ruled and, if no direct slot is free, `to_govern`
    /// (directly ruled, not the capital) governed. Both must be past their cooldown.
    pub fn player_set_direct_rule(&mut self, p: PlayerId, to_rule: CityId, to_govern: Option<CityId>) -> GovResult {
        self.player_check_direct_rule(p, to_rule, to_govern)?;
        let turn = self.game_turn();
        if let Some(o) = to_govern {
            self.city_hand_over(o);
        }
        self.city_become_direct(to_rule);
        self.cities[to_rule].rule_changed_turn = turn;
        Ok(())
    }

    /// Whether `player_set_direct_rule(p, to_rule, to_govern)` would be done (else why not).
    pub fn player_check_direct_rule(&self, p: PlayerId, to_rule: CityId, to_govern: Option<CityId>) -> GovResult {
        if !self.governors_on() {
            return Err("governors are off in this game");
        }
        if !self.cities.contains(to_rule) || self.cities[to_rule].owner != p || !self.city_is_governed(to_rule) {
            return Err("not a governed city of yours");
        }
        if !self.city_rule_change_ready(to_rule) {
            return Err("its rule changed too recently");
        }
        let full = self.player_direct_cities(p).len() >= self.governor_direct_slots();
        match to_govern {
            Some(o) => {
                if !self.cities.contains(o) || self.cities[o].owner != p || self.city_is_governed(o) {
                    return Err("not a directly ruled city of yours");
                }
                if self.city_is_capital(o) {
                    return Err("the capital is always ruled directly");
                }
                if !self.city_rule_change_ready(o) {
                    return Err("its rule changed too recently");
                }
            }
            None if full => return Err("no direct slot is free: hand a city to a governor"),
            None => {}
        }
        Ok(())
    }

    /// Hands directly ruled city `c` (not the capital) to a governor, freeing its slot.
    pub fn city_hand_to_governor(&mut self, p: PlayerId, c: CityId) -> GovResult {
        self.city_check_hand_to_governor(p, c)?;
        self.city_hand_over(c);
        Ok(())
    }

    /// Whether `city_hand_to_governor(p, c)` would be done (else why not).
    pub fn city_check_hand_to_governor(&self, p: PlayerId, c: CityId) -> GovResult {
        if !self.governors_on() {
            return Err("governors are off in this game");
        }
        if !self.cities.contains(c) || self.cities[c].owner != p || self.city_is_governed(c) {
            return Err("not a directly ruled city of yours");
        }
        if self.city_is_capital(c) {
            return Err("the capital is always ruled directly");
        }
        if !self.city_rule_change_ready(c) {
            return Err("its rule changed too recently");
        }
        Ok(())
    }

    /// City `c` becomes directly ruled: its governor returns to the bench, its production is the
    /// player's again.
    fn city_become_direct(&mut self, c: CityId) {
        // The emphasis was the governor's: the player chooses it again.
        if let Some(profile) = self.city_governor_profile(c) {
            for e in self.d.governor_profiles[profile].emphasizes.clone() {
                if self.cities[c].ai_emphasize[e] {
                    self.city_ai_set_emphasize(c, e, false);
                }
            }
        }
        self.cities[c].direct_rule = true;
        if let Some(g) = self.cities[c].governor.take() {
            if let Some(gov) = self.governors.get_mut(g) {
                gov.city = None;
            }
        }
        self.cities[c].feed_army = None;
        self.cities[c].governor_penalty_turns = 0;
        self.cities[c].production_automated = false;
        self.city_governor_changed(c);
    }

    /// Governor `g` of player `p` (on the bench or in another city) governs city `c`. The governor
    /// it replaces returns to the bench. A replacement costs the city the penalty; a move from
    /// another city costs both cities; filling a vacant post from the bench costs nothing.
    pub fn city_assign_governor(&mut self, p: PlayerId, c: CityId, g: GovernorId) -> GovResult {
        self.city_check_assign_governor(p, c, g)?;
        if self.cities[c].governor == Some(g) {
            return Ok(());
        }
        let turns = self.d.defines.governor_swap_turns.max(0);
        // A move between two cities costs both; the city it leaves is interim.
        if let Some(from) = self.governors[g].city {
            self.cities[from].governor = None;
            self.cities[from].governor_penalty_turns = turns;
            self.cities[c].governor_penalty_turns = turns;
            self.city_apply_governor_emphasis(from);
            self.city_governor_changed(from);
        }
        self.governors[g].city = None;
        // The governor it replaces goes back to the bench; the city pays.
        if let Some(old) = self.cities[c].governor.take() {
            if let Some(gov) = self.governors.get_mut(old) {
                gov.city = None;
            }
            self.cities[c].governor_penalty_turns = turns;
        }
        self.governor_take_post(g, c);
        Ok(())
    }

    /// Whether `city_assign_governor(p, c, g)` would be done (else why not).
    pub fn city_check_assign_governor(&self, p: PlayerId, c: CityId, g: GovernorId) -> GovResult {
        if !self.governors_on() {
            return Err("governors are off in this game");
        }
        if !self.cities.contains(c) || self.cities[c].owner != p || !self.city_is_governed(c) {
            return Err("not a governed city of yours");
        }
        if !self.governors.contains(g) || self.governors[g].owner != p {
            return Err("not a governor of yours");
        }
        Ok(())
    }

    /// Governed city `c` feeds army `a` (`None`: develops). A new mode takes effect at once: the
    /// governor chooses the city's production again (what it was building keeps its progress for
    /// later, as any order taken off a queue).
    pub fn city_set_feed_army(&mut self, p: PlayerId, c: CityId, a: Option<ArmyId>) -> GovResult {
        self.city_check_feed_army(p, c, a)?;
        if self.cities[c].feed_army != a {
            self.cities[c].feed_army = a;
            self.city_clear_order_queue(c);
            self.city_choose_production(c, None, None, None, false, false);
        }
        Ok(())
    }

    /// Whether `city_set_feed_army(p, c, a)` would be done (else why not).
    pub fn city_check_feed_army(&self, p: PlayerId, c: CityId, a: Option<ArmyId>) -> GovResult {
        if !self.governors_on() {
            return Err("governors are off in this game");
        }
        if !self.cities.contains(c) || self.cities[c].owner != p || !self.city_is_governed(c) {
            return Err("not a governed city of yours");
        }
        if let Some(a) = a {
            if !self.armies_on() {
                return Err("armies are off in this game");
            }
            if !self.recruit_dest_valid(p, RecruitDest::Army(a)) {
                return Err("no such army of yours (or it is disbanding)");
            }
        }
        Ok(())
    }

    /// Whether governed city `c`'s governor may queue hostile cultural actions.
    pub fn city_set_hostile_culture(&mut self, p: PlayerId, c: CityId, on: bool) -> GovResult {
        if !self.cities.contains(c) || self.cities[c].owner != p {
            return Err("not a city of yours");
        }
        self.cities[c].governor_hostile_culture = on;
        Ok(())
    }

    // ---------------------------------------------------------------------------------------
    // The turn
    // ---------------------------------------------------------------------------------------

    /// Player `p`'s governance in its turn processing: the capital ruled directly, no more direct
    /// cities than slots, the replacement penalties wearing off, feeding stopped for armies that
    /// are gone, the bench drawing candidates, interim cities taking them.
    pub fn player_governance_turn(&mut self, p: PlayerId) {
        if !self.governors_on() || self.player_is_barbarian(p) || !self.players[p].alive {
            return;
        }
        self.player_governance_settle(p);
        let turn = self.game_turn();
        for c in self.players[p].cities.clone() {
            if let Some(a) = self.cities[c].feed_army {
                if !self.recruit_dest_valid(p, RecruitDest::Army(a)) {
                    self.cities[c].feed_army = None;
                    if self.city_is_governed(c) {
                        self.city_clear_order_queue(c);
                        self.city_choose_production(c, None, None, None, false, false);
                    }
                    let name = self.d.text.name(&self.cities[c].name).to_string();
                    self.log.msg(turn, Some(p), || format!("{name} has no army to feed any more: it develops."));
                }
            }
            if self.city_is_governed(c) && !self.cities[c].production_automated {
                self.cities[c].production_automated = true;
            }
        }
        // The bench.
        let vacant = self.player_vacant_posts(p);
        let target = vacant.len() as i32 + self.d.defines.governor_bench_surplus.max(0);
        if (self.player_bench(p).len() as i32) < target && self.soren_rand_num(100, "Governor candidate") < self.d.defines.governor_bench_gen_chance {
            let g = self.governor_generate(p);
            let name = self.governors[g].name.clone();
            let profile = self.d.text.name(&self.d.governor_profiles[self.governors[g].profile].base.description).to_string();
            self.log.msg(turn, Some(p), || format!("{name}, a {profile}, joins your bench of governors."));
        }
        // The governors' own initiative (a human's governed cities: their cultural actions).
        let controller = self.controller(p);
        controller.govern_cities(self, p);
        // Interim cities take the candidates, the best match first.
        for c in self.player_vacant_posts(p) {
            let Some(g) = self.player_bench(p).into_iter().max_by_key(|&g| (self.governor_match(g, c), std::cmp::Reverse(g))) else { break };
            self.governor_take_post(g, c);
            let name = self.governors[g].name.clone();
            let city = self.d.text.name(&self.cities[c].name).to_string();
            self.log.msg(turn, Some(p), || format!("{name} now governs {city}."));
        }
    }

    /// The end of player `p`'s turn processing (after its cities' production and commerce): the
    /// replacement penalties wear off, so a penalty of N turns costs N turns' output.
    pub fn player_governance_turn_end(&mut self, p: PlayerId) {
        if !self.governors_on() {
            return;
        }
        for c in self.players[p].cities.clone() {
            if self.cities.contains(c) && self.cities[c].governor_penalty_turns > 0 {
                self.cities[c].governor_penalty_turns -= 1;
                if self.cities[c].governor_penalty_turns == 0 {
                    self.city_governor_changed(c);
                }
            }
        }
    }

    /// Player `p`'s direct rule settled: the capital directly ruled, at most the slots' number of
    /// directly ruled cities (the cities made direct last go back to governors).
    pub fn player_governance_settle(&mut self, p: PlayerId) {
        if !self.governors_on() || self.player_is_barbarian(p) {
            return;
        }
        if let Some(cap) = self.player_get_capital_city(p) {
            if self.city_is_governed(cap) {
                self.city_become_direct(cap);
            }
        }
        let mut direct = self.player_direct_cities(p);
        let slots = self.governor_direct_slots();
        if direct.len() <= slots {
            return;
        }
        direct.retain(|&c| !self.city_is_capital(c));
        direct.sort_by_key(|&c| (std::cmp::Reverse(self.cities[c].rule_changed_turn), std::cmp::Reverse(c)));
        let turn = self.game_turn();
        let over = self.player_direct_cities(p).len() - slots;
        for &c in direct.iter().take(over) {
            self.city_hand_over(c);
            let name = self.d.text.name(&self.cities[c].name).to_string();
            self.log.msg(turn, Some(p), || format!("{name} goes to a governor: your new capital takes its direct slot."));
        }
    }

    // ---------------------------------------------------------------------------------------
    // Candidates
    // ---------------------------------------------------------------------------------------

    /// A new candidate on player `p`'s bench: a profile and up to `GOVERNOR_MAX_TRAITS` traits
    /// drawn toward the leader's flavors (the game's RNG), a name of its civilization's list.
    pub fn governor_generate(&mut self, p: PlayerId) -> GovernorId {
        let d = self.d;
        let leader = self.players[p].personality_type.or(self.player_get_leader_type(p));
        let flavors: Vec<FlavorType> = (0..d.reg.count::<FlavorType>()).map(FlavorType::from_usize).collect();
        let lf = |f: FlavorType| leader.map_or(0, |l| d.leader_heads[l].flavor_value[f]);
        // The profile: weight 10 + the leader's flavors times the profile's.
        let profiles: Vec<(GovernorProfileType, i32)> = d
            .governor_profiles
            .keys()
            .filter(|&k| !d.governor_profiles[k].interim)
            .map(|k| (k, 10 + flavors.iter().map(|&f| lf(f) * d.governor_profiles[k].flavor_value[f]).sum::<i32>().max(0)))
            .collect();
        let profile = self.weighted_pick(&profiles, "Governor profile").expect("a governor profile");
        // Traits: 0..=max, each weight 1 + its flavors times the leader's / 5; no two of one kind.
        let max_traits = d.defines.governor_max_traits.max(0);
        let n = self.soren_rand_num(max_traits + 1, "Governor traits");
        let mut traits: Vec<GovernorTraitType> = Vec::new();
        for _ in 0..n {
            let pool: Vec<(GovernorTraitType, i32)> = d
                .governor_traits
                .keys()
                .filter(|&t| traits.iter().all(|&o| !traits_clash(&d.governor_traits[o], &d.governor_traits[t])))
                .map(|t| (t, 1 + flavors.iter().map(|&f| lf(f) * d.governor_traits[t].flavor_value[f]).sum::<i32>().max(0) / 5))
                .collect();
            match self.weighted_pick(&pool, "Governor trait") {
                Some(t) => traits.push(t),
                None => break,
            }
        }
        let name = self.governor_new_name(p);
        let turn = self.game_turn();
        let g = self.governors.insert_with(|id| Governor { id, owner: p, name, profile, traits, origin: 0, born_turn: turn, city: None, since_turn: -1 });
        self.players[p].governors.push(g);
        self.players[p].governor_count += 1;
        g
    }

    /// One of `items` drawn by weight (the game's RNG); `None` when empty.
    fn weighted_pick<T: Copy>(&mut self, items: &[(T, i32)], log: &str) -> Option<T> {
        let total: i32 = items.iter().map(|&(_, w)| w.max(0)).sum();
        if total <= 0 {
            return None;
        }
        let mut r = self.soren_rand_num(total, log);
        for &(t, w) in items {
            let w = w.max(0);
            if r < w {
                return Some(t);
            }
            r -= w;
        }
        None
    }

    /// A name for a new governor of `p`: the first of its civilization's list none of its governors
    /// has, else of the generic list, else "Governor N".
    fn governor_new_name(&self, p: PlayerId) -> String {
        let d = self.d;
        let civ = self.player_get_civilization_type(p);
        let taken: BTreeSet<&str> = self.players[p].governors.iter().filter_map(|&g| self.governors.get(g)).map(|g| g.name.as_str()).collect();
        let lists = [d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_some() && d.governor_names[k].civilization == civ), d.governor_names.keys().find(|&k| d.governor_names[k].civilization.is_none())];
        for list in lists.into_iter().flatten() {
            if let Some(n) = d.governor_names[list].names.iter().find(|n| !taken.contains(n.as_str())) {
                return n.clone();
            }
        }
        format!("Governor {}", self.players[p].governor_count + 1)
    }

    /// How well governor `g` suits city `c`: its profile's flavors weighed by what the city needs.
    pub fn governor_match(&self, g: GovernorId, c: CityId) -> i32 {
        let d = self.d;
        let profile = &d.governor_profiles[self.governors[g].profile];
        let need = self.city_governor_needs(c);
        (0..d.reg.count::<FlavorType>()).map(FlavorType::from_usize).map(|f| profile.flavor_value[f] * need[f]).sum()
    }

    /// What city `c` needs, by flavor (0-10): growth while small or starving, production while it
    /// makes little, military near a threat or a rival, science and gold with much commerce,
    /// culture while its plot's culture is contested, religion with a religion of its own.
    pub fn city_governor_needs(&self, c: CityId) -> IdxVec<FlavorType, i32> {
        let d = self.d;
        let mut need = IdxVec::from_elem(1, d.reg.count::<FlavorType>());
        let city = &self.cities[c];
        let p = city.owner;
        let food = self.city_yield_rate(c, YieldType::YIELD_FOOD) - self.city_food_consumption(c, false, 0);
        let production = self.city_yield_rate(c, YieldType::YIELD_PRODUCTION);
        let commerce = self.city_yield_rate(c, YieldType::YIELD_COMMERCE);
        let set = |need: &mut IdxVec<FlavorType, i32>, name: &str, v: i32| {
            if let Some(f) = d.ty::<FlavorType>(name) {
                need[f] = v.clamp(0, 10);
            }
        };
        set(&mut need, "FLAVOR_GROWTH", if food <= 1 { 7 } else if city.population < 5 { 5 } else { 2 });
        set(&mut need, "FLAVOR_PRODUCTION", if production < city.population { 6 } else { 3 });
        let plot = self.city_plot(c);
        let (x, y) = (city.x, city.y);
        let team = self.players[p].team;
        let rival_near = self.player_ids().any(|q| {
            let pl = &self.players[q];
            q != p && pl.alive && pl.team != team && pl.cities.iter().any(|&o| self.plot_distance(x, y, self.cities[o].x, self.cities[o].y) <= 7)
        });
        let at_war_near = rival_near && self.team_ids().into_iter().any(|t| t != team && self.at_war(team, t));
        set(&mut need, "FLAVOR_MILITARY", if at_war_near { 8 } else if rival_near { 5 } else { 1 });
        set(&mut need, "FLAVOR_SCIENCE", 2 + commerce / 4);
        set(&mut need, "FLAVOR_GOLD", 1 + commerce / 4);
        let ours = self.plot_calculate_culture_percent(plot, p);
        set(&mut need, "FLAVOR_CULTURE", if ours < 70 { 6 } else { 2 });
        let religious = d.religions.keys().any(|r| city.has_religion[r]);
        set(&mut need, "FLAVOR_RELIGION", if religious { 4 } else { 1 });
        need
    }
}

/// Whether two traits change the same thing (a governor never has both).
fn traits_clash(a: &GovernorTraitInfo, b: &GovernorTraitInfo) -> bool {
    let touch = |t: &GovernorTraitInfo| -> [bool; 7] {
        [
            t.building_production_percent != 0,
            t.unit_production_percent != 0,
            t.happy != 0,
            t.health != 0,
            t.experience != 0,
            t.yield_modifier.iter().any(|&v| v != 0),
            t.commerce_modifier.iter().any(|&v| v != 0),
        ]
    };
    let (x, y) = (touch(a), touch(b));
    std::ptr::eq(a, b) || x.iter().zip(y.iter()).any(|(&p, &q)| p && q)
}
