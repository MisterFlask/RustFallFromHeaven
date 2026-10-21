//! Port of CvPlayerAI.cpp:142-3366 (FFH2 0.41 C++ AI).
//!
//! `AI_foundValue` (1847-2993) is not re-ported: it delegates to ffh-core's audited port
//! (`Game::player_ai_found_value`), passing this player's AI city sites (`m_aiAICitySites`,
//! which the C++ reads through `AI_isPlotCitySite`).
//!
//! Notes:
//! - `AI_reset` resets the AI state kept in this crate (`state.rs`); the AI members kept in
//!   ffh-core (`Player::ai_*`) are reset by ffh-core's own `AI_reset` when the player is
//!   initialized (PORT ADAPT).
//! - `AI_conquerCity` returns the raze decision; the rules raze the city (TASK_RAZE) or report
//!   `cityAcquiredAndKept` themselves.
//! - Iteration over a player's units (`firstUnit`/`nextUnit`) uses a snapshot of the unit list,
//!   re-checking that each unit still exists; units created while iterating (upgrades) cannot
//!   act this turn (they have no moves left), so not visiting them changes nothing.

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;
use ffh_core::setup::SlotStatus;

/// C++ `CvMap::calculatePathDistance(pSource, pDest)` from `&Game` (ffh-core's
/// `map_calculate_path_distance` takes `&mut Game` only to reach the step finder, a `RefCell`).
fn calculate_path_distance(g: &Game, source: PlotIdx, dest: PlotIdx) -> i32 {
    let (sx, sy) = g.map.xy(source);
    let (dx, dy) = g.map.xy(dest);

    let mut finder = g.step_finder.borrow_mut();
    if finder.generate_path(g, sx, sy, dx, dy, false, 0, true) {
        if let Some(node) = finder.last_node() {
            return node.data1;
        }
    }

    -1 // no passable path exists
}

impl Cai {
    /// C++ `void CvPlayerAI::AI_init()` (CvPlayerAI.cpp:142-157)
    pub fn player_ai_init(&self, g: &mut Game, p: PlayerId) {
        self.player_ai_reset(g, p, false);

        //--------------------------------
        // Init other game data
        let slot_status = g.setup.players[p.idx()].slot_status;
        if slot_status == SlotStatus::Taken || slot_status == SlotStatus::Computer {
            if let Some(personality) = g.players[p].personality_type {
                let info = &g.d.leader_heads[personality];
                let peace = info.base_peace_weight + self.rand(info.peace_weight_rand, "AI Peace Weight");
                let espionage = info.espionage_weight;
                self.player_ai_set_peace_weight(g, p, peace);
                self.player_ai_set_espionage_weight(g, p, espionage);
                //AI_setCivicTimer(((getMaxAnarchyTurns() == 0) ? (GC.getDefineINT("MIN_REVOLUTION_TURNS") * 2) : CIVIC_CHANGE_DELAY) / 2);
                self.player_ai_set_religion_timer(g, p, 1);
                let civic = if g.players[p].max_anarchy_turns == 0 { 1 } else { 2 };
                self.player_ai_set_civic_timer(g, p, civic);
            }
        }
        self.s().player(p).initialized = true;
    }

    /// C++ `void CvPlayerAI::AI_reset(bool bConstructor)` (CvPlayerAI.cpp:168-293)
    pub fn player_ai_reset(&self, g: &mut Game, p: PlayerId, constructor: bool) {
        // PORT ADAPT: the members ffh-core keeps (civic/religion timers, AI unit counts,
        // peacetime trade/grant values, gold traded, attitude extra, first contact, memory
        // counts, bonus values) are reset by ffh-core's `AI_reset` at player init; resetting
        // them here mid-game would break the rules' bookkeeping. This resets the rest: this
        // player's AI state (`state.rs` defaults are the C++ `AI_reset` values) and the other
        // players' entries about this player.
        let n = g.players.len();
        let d = g.d;
        self.s().ensure_players(n);
        *self.s().player(p) = PlayerAi::new(d, n);

        if !constructor {
            for other in 0..n {
                let mut st = self.s();
                let o = &mut st.players[other];
                o.same_religion_counter[p.idx()] = 0;
                o.different_religion_counter[p.idx()] = 0;
                o.favorite_civic_counter[p.idx()] = 0;
                o.bonus_trade_counter[p.idx()] = 0;
                for v in o.contact_timer[p.idx()].iter_mut() {
                    *v = 0;
                }
                o.close_borders_attitude_cache[p.idx()] = 0;
            }
        }
    }

    /// C++ `int CvPlayerAI::AI_getFlavorValue(FlavorTypes eFlavor) const` (CvPlayerAI.cpp:296-301)
    pub fn player_ai_get_flavor_value(&self, g: &Game, p: PlayerId, flavor: FlavorType) -> i32 {
        // PORT ADAPT: C++ asserts a personality; a player without one has no flavors.
        match g.players[p].personality_type {
            Some(personality) => g.d.leader_heads[personality].flavor_value[flavor],
            None => 0,
        }
    }

    /// C++ `void CvPlayerAI::AI_doTurnPre()` (CvPlayerAI.cpp:304-372)
    pub fn player_ai_do_turn_pre(&self, g: &mut Game, p: PlayerId) {
        self.player_ai_invalidate_close_borders_attitude_cache(g, p);
        self.player_ai_do_counter(g, p);
        // (ffh-core already ran AI_updateBonusValue for this player this turn; resetting the
        // cache again is harmless.)
        self.player_ai_update_bonus_value_2(g, p);
        self.player_ai_do_enemy_unit_data(g, p);
        if g.players[p].human {
            return;
        }

        // BETTER AI (better diplomatics) Sephi
        // Diplomatics
        // Decay Gold Traded
        if !g.players[p].human {
            for i in g.civ_player_ids() {
                if p != i {
                    let traded = self.player_ai_get_gold_traded_to(g, p, i);
                    let mut decay_gold_traded = traded / 40;

                    if g.players[i].alignment == g.players[p].alignment {
                        decay_gold_traded *= 2;
                    }
                    if g.players[i].favorite_religion == g.players[p].favorite_religion {
                        decay_gold_traded *= 2;
                    }

                    // FIX: C++ `AI_changeGoldTradedTo(i, std::min(AI_getGoldTradedTo(i),
                    // iDecayGoldTraded))` adds the "decay", so the gold traded grows by up to 10%
                    // a turn (until it overflows) and the AI stops trading gold to that player
                    // for good (AI_maxGoldTrade subtracts it). Decay it as intended.
                    self.player_ai_change_gold_traded_to(g, p, i, -traded.min(decay_gold_traded));
                }
            }
        }
        // BETTER AI (Naval AI) Sephi
        self.player_ai_do_expansion(g, p);

        self.player_ai_do_research(g, p);
        self.player_ai_do_commerce(g, p);
        self.player_ai_do_military(g, p);
        self.player_ai_do_civics(g, p);
        self.player_ai_do_religion(g, p);
        self.player_ai_do_check_financial_trouble(g, p);

        if g.player_is_barbarian(p) {
            return;
        }

        if g.player_is_minor_civ(p) {
            return;
        }
    }

    /// C++ `void CvPlayerAI::AI_doTurnPost()` (CvPlayerAI.cpp:375-412)
    pub fn player_ai_do_turn_post(&self, g: &mut Game, p: PlayerId) {
        if g.players[p].human {
            return;
        }

        if g.player_is_barbarian(p) {
            return;
        }

        if g.player_is_minor_civ(p) {
            return;
        }

        self.player_ai_do_diplo(g, p);

        // UNOFFICIAL_PATCH 06/16/09 jdog5000: AI_doSplit moved to AI_doTurnUnitsPost.

        for victory in g.d.victories.keys() {
            self.player_ai_launch(g, p, victory);
        }
    }

    /// C++ `void CvPlayerAI::AI_doTurnUnitsPre()` (CvPlayerAI.cpp:415-440)
    pub fn player_ai_do_turn_units_pre(&self, g: &mut Game, p: PlayerId) {
        self.player_ai_update_found_values(g, p, false);

        if self.rand(8, "AI Update Area Targets") == 0 {
            // XXX personality???
            self.player_ai_update_area_targets(g, p);
        }

        if g.players[p].human {
            return;
        }

        if g.player_is_barbarian(p) {
            return;
        }

        //	if (AI_isDoStrategy(AI_STRATEGY_CRUSH))
        //	{
        //		AI_convertUnitAITypesForCrush();
        //	}
    }

    /// C++ `void CvPlayerAI::AI_doTurnUnitsPost()` (CvPlayerAI.cpp:443-723)
    pub fn player_ai_do_turn_units_post(&self, g: &mut Game, p: PlayerId) {
        if !g.players[p].human || g.players[p].options[PlayerOptionType::PLAYEROPTION_AUTO_PROMOTION] {
            for u in g.players[p].units.clone() {
                if g.units.contains(u) {
                    self.unit_ai_promote(g, u);
                }
            }
        }

        if g.players[p].human {
            // ADDON (automatic spellcasting) Sephi
            for u in g.players[p].units.clone() {
                if !g.units.contains(u) {
                    continue;
                }
                if let Some(spell) = g.units[u].auto_spell_cast {
                    if g.unit_can_cast(u, spell, false) {
                        g.unit_cast(u, spell);
                    }
                }
            }
            return;
        }

        // BETTER AI (Immobile Units regroup) Sephi
        // Immobile Units should leave their group if not head of the group
        for u in g.players[p].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            if g.unit_get_immobile_timer(u) > 0 {
                g.unit_join_group(u, None, true);
            }
            if g.units.contains(u) && g.unit_moves_left(u) < 1 {
                g.unit_join_group(u, None, true);
            }
        }

        // BETTER AI (Spellcasting UNITAI_MAGE) Sephi
        for u in g.players[p].units.clone() {
            if !g.units.contains(u) {
                continue;
            }
            if g.units[u].ai_type == Some(UnitAIType::UNITAI_MAGE) && g.units[u].ai_groupflag == GROUPFLAG_PERMDEFENSE {
                self.unit_ai_mage_cast(g, u);
            }
        }

        // FfH: Modified by Kael 11/26/2008 (the BtS upgrade/disband passes are commented out)
        let mut best_unit: Option<UnitType> = None;
        let mut best_value = -1;
        let mut value;
        if g.player_is_barbarian(p) {
            for u in g.players[p].units.clone() {
                if !g.units.contains(u) {
                    continue;
                }
                best_unit = None;
                // C++ quirk: iBestValue is not reset per unit.
                for unit_type in g.d.units.keys() {
                    if g.unit_can_upgrade(u, Some(unit_type), false) {
                        value = 1 + self.rand(10000, "Upgrade");
                        if value > best_value {
                            best_value = value;
                            best_unit = Some(unit_type);
                        }
                    }
                }
                if best_unit.is_some() {
                    g.unit_upgrade(u, best_unit);
                    if g.units.contains(u) {
                        g.unit_do_delayed_death(u);
                    }
                }
            }
        } else {
            let pyre_zombie = g.d.defines.unit_pyre_zombie;
            for pass in 0..3 {
                for u in g.players[p].units.clone() {
                    if !g.units.contains(u) {
                        continue;
                    }
                    let valid = match pass {
                        0 => {
                            let unit_plot = g.unit_plot(u);
                            self.player_ai_get_plot_danger(g, p, unit_plot, 1, false) > 0
                        }
                        1 => g.units[u].level > 3,
                        _ => g.units[u].level <= 3,
                    };
                    if valid {
                        // C++ quirk: eBestUnit and iBestValue are reset neither per unit nor
                        // per pass, so a later unit may be upgraded to an earlier unit's choice
                        // (`upgrade` rejects it when the unit cannot upgrade to it).
                        for unit_type in g.d.units.keys() {
                            if g.unit_can_upgrade(u, Some(unit_type), false) {
                                value = 1 + self.rand(10000, "Upgrade");

                                // BETTER AI (upgrading Units) Sephi
                                // Block City counters from becoming Pyre Zombies
                                if pyre_zombie != -1 {
                                    if unit_type.i() == pyre_zombie {
                                        if g.units[u].ai_type == Some(UnitAIType::UNITAI_CITY_COUNTER) {
                                            value = -10000;
                                        }
                                    }
                                }

                                if value > best_value {
                                    best_value = value;
                                    best_unit = Some(unit_type);
                                }
                            }
                        }
                        if best_unit.is_some() {
                            g.unit_upgrade(u, best_unit);
                            if g.units.contains(u) {
                                g.unit_do_delayed_death(u);
                            }
                        }
                    }
                }
            }
        }
        // FfH: End Modify

        // UNOFFICIAL_PATCH 06/16/09 jdog5000: moved here per alexman's suggestion
        self.player_ai_do_split(g, p);
    }

    /// C++ `void CvPlayerAI::AI_doPeace()` (CvPlayerAI.cpp:726-1048)
    pub fn player_ai_do_peace(&self, g: &mut Game, p: PlayerId) {
        debug_assert!(!g.players[p].human);
        debug_assert!(!g.player_is_barbarian(p));

        let team = g.players[p].team;
        let mut contacted = vec![false; g.teams.len()];
        let (contact_rand, contact_delay) = match g.players[p].personality_type {
            Some(leader) => {
                let info = &g.d.leader_heads[leader];
                (info.contact_rand[ContactType::CONTACT_PEACE_TREATY], info.contact_delay[ContactType::CONTACT_PEACE_TREATY])
            }
            None => (0, 0),
        };

        for i in g.civ_player_ids() {
            if !g.players[i].alive || i == p {
                continue;
            }
            if !(g.player_can_contact(p, i) && self.player_ai_is_willing_to_talk(g, p, i)) {
                continue;
            }
            let their_team = g.players[i].team;
            if !(!g.team_is_human(team) && (g.players[i].human || !g.team_is_human(their_team))) {
                continue;
            }
            if !g.teams[team].at_war[their_team] {
                continue;
            }
            if !(!g.players[i].human || g.team_get_leader_id(team) == Some(p)) {
                continue;
            }
            debug_assert!(!g.player_is_barbarian(i));
            debug_assert!(their_team != team);

            if self.team_ai_get_at_war_counter(g, team, their_team) <= 10 {
                continue;
            }
            if self.player_ai_get_contact_timer(g, p, i, ContactType::CONTACT_PEACE_TREATY) != 0 {
                continue;
            }

            let mut offered = false;

            let item = set_trade_item(TradeableItem::TRADE_SURRENDER, 0);

            if g.player_can_trade_item(p, i, &item, true) {
                let our_list = vec![item];
                let their_list: Vec<TradeData> = Vec::new();

                offered = true;

                if g.players[i].human {
                    if !contacted[their_team.idx()] {
                        self.player_ai_change_contact_timer(g, p, i, ContactType::CONTACT_PEACE_TREATY, contact_delay);
                        // AI_DIPLOCOMMENT_OFFER_PEACE: gDLL->beginDiplomacy (human popup) omitted.
                        contacted[their_team.idx()] = true;
                    }
                } else {
                    implement_ai_deal(g, p, i, &our_list, &their_list);
                }
            }

            if !offered {
                if self.rand(contact_rand, "AI Diplo Peace Treaty") == 0 {
                    let item = set_trade_item(TradeableItem::TRADE_PEACE_TREATY, 0);

                    if g.player_can_trade_item(p, i, &item, true) && g.player_can_trade_item(i, p, &item, true) {
                        let mut our_value = self.team_ai_end_war_val(g, team, their_team);
                        let mut their_value = self.team_ai_end_war_val(g, their_team, team);

                        let mut best_receive_tech: Option<TechType> = None;
                        let mut best_give_tech: Option<TechType> = None;

                        let mut receive_gold = 0;
                        let mut give_gold = 0;

                        let mut best_receive_city: Option<CityId> = None;
                        let mut best_give_city: Option<CityId> = None;

                        if their_value > our_value {
                            if their_value > our_value {
                                let mut best_value = 0;

                                for tech in g.d.techs.keys() {
                                    let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.i());

                                    if g.player_can_trade_item(i, p, &item, true) {
                                        let value = 1 + self.rand(10000, "AI Peace Trading (Tech #1)");

                                        if value > best_value {
                                            best_value = value;
                                            best_receive_tech = Some(tech);
                                        }
                                    }
                                }

                                if let Some(tech) = best_receive_tech {
                                    our_value += self.team_ai_tech_trade_val(g, team, tech, their_team);
                                }
                            }

                            let gold = (their_value - our_value).min(self.player_ai_max_gold_trade(g, i, p));

                            if gold > 0 {
                                let item = set_trade_item(TradeableItem::TRADE_GOLD, gold);

                                if g.player_can_trade_item(i, p, &item, true) {
                                    receive_gold = gold;
                                    our_value += gold;
                                }
                            }

                            if their_value > our_value {
                                let mut best_value = 0;

                                for c in g.players[i].cities.clone() {
                                    let item = set_trade_item(TradeableItem::TRADE_CITIES, c.0 as i32);

                                    if g.player_can_trade_item(i, p, &item, true) {
                                        let value = g.plot_calculate_culture_percent(g.city_plot(c), p);

                                        if value > best_value {
                                            best_value = value;
                                            best_receive_city = Some(c);
                                        }
                                    }
                                }

                                if let Some(c) = best_receive_city {
                                    our_value += self.player_ai_city_trade_val(g, p, c);
                                }
                            }
                        } else if our_value > their_value {
                            let mut best_value = 0;

                            for tech in g.d.techs.keys() {
                                let item = set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.i());

                                if g.player_can_trade_item(p, i, &item, true) {
                                    if self.team_ai_tech_trade_val(g, their_team, tech, team) <= (our_value - their_value) {
                                        let value = 1 + self.rand(10000, "AI Peace Trading (Tech #2)");

                                        if value > best_value {
                                            best_value = value;
                                            best_give_tech = Some(tech);
                                        }
                                    }
                                }
                            }

                            if let Some(tech) = best_give_tech {
                                their_value += self.team_ai_tech_trade_val(g, their_team, tech, team);
                            }

                            let gold = (our_value - their_value).min(self.player_ai_max_gold_trade(g, p, i));

                            if gold > 0 {
                                let item = set_trade_item(TradeableItem::TRADE_GOLD, gold);

                                if g.player_can_trade_item(p, i, &item, true) {
                                    give_gold = gold;
                                    their_value += gold;
                                }
                            }

                            let mut best_value = 0;

                            for c in g.players[p].cities.clone() {
                                let item = set_trade_item(TradeableItem::TRADE_CITIES, c.0 as i32);

                                if g.player_can_trade_item(p, i, &item, true) {
                                    if self.player_ai_city_trade_val(g, i, c) <= (our_value - their_value) {
                                        let value = g.plot_calculate_culture_percent(g.city_plot(c), i);

                                        if value > best_value {
                                            best_value = value;
                                            best_give_city = Some(c);
                                        }
                                    }
                                }
                            }

                            if let Some(c) = best_give_city {
                                their_value += self.player_ai_city_trade_val(g, i, c);
                            }
                        }

                        let accept = if g.players[i].human {
                            our_value >= their_value
                        } else {
                            (our_value > ((their_value * 3) / 5)) && (their_value > ((our_value * 3) / 5))
                        };
                        if accept {
                            let mut our_list: Vec<TradeData> = Vec::new();
                            let mut their_list: Vec<TradeData> = Vec::new();

                            let item = set_trade_item(TradeableItem::TRADE_PEACE_TREATY, 0);

                            our_list.push(item.clone());
                            their_list.push(item);

                            if let Some(tech) = best_give_tech {
                                our_list.push(set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.i()));
                            }

                            if let Some(tech) = best_receive_tech {
                                their_list.push(set_trade_item(TradeableItem::TRADE_TECHNOLOGIES, tech.i()));
                            }

                            if give_gold != 0 {
                                our_list.push(set_trade_item(TradeableItem::TRADE_GOLD, give_gold));
                            }

                            if receive_gold != 0 {
                                their_list.push(set_trade_item(TradeableItem::TRADE_GOLD, receive_gold));
                            }

                            if let Some(c) = best_give_city {
                                our_list.push(set_trade_item(TradeableItem::TRADE_CITIES, c.0 as i32));
                            }

                            if let Some(c) = best_receive_city {
                                their_list.push(set_trade_item(TradeableItem::TRADE_CITIES, c.0 as i32));
                            }

                            if g.players[i].human {
                                if !contacted[their_team.idx()] {
                                    self.player_ai_change_contact_timer(g, p, i, ContactType::CONTACT_PEACE_TREATY, contact_delay);
                                    // AI_DIPLOCOMMENT_OFFER_PEACE: gDLL->beginDiplomacy (human
                                    // popup) omitted.
                                    contacted[their_team.idx()] = true;
                                }
                            } else {
                                // C++: the other AI's agreement is the 3/5 value test above
                                // (no AI_considerOffer); see implement_ai_deal.
                                implement_ai_deal(g, p, i, &our_list, &their_list);
                            }
                        }
                    }
                }
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_updateFoundValues(bool bStartingLoc = false) const` (CvPlayerAI.cpp:1051-1127)
    pub fn player_ai_update_found_values(&self, g: &mut Game, p: PlayerId, starting_loc: bool) {
        for a in g.map.areas.ids() {
            g.map.areas[a].best_found_value[p] = 0;
        }

        if starting_loc {
            for i in 0..g.map.num_plots() {
                g.map.plots[i].found_value[p] = -1;
            }
        } else {
            let barbarian = g.player_is_barbarian(p);
            if !barbarian {
                let min = self.player_ai_get_min_found_value(g, p);
                self.player_ai_invalidate_city_sites(g, p, min);
            }
            // `AI_foundValue` reads the city sites (`AI_isPlotCitySite`): the list as left by
            // AI_invalidateCitySites (it does not change during the loop).
            let city_sites = self.s().player(p).ai_city_sites.clone();
            let team = g.players[p].team;
            let no_cities = g.players[p].num_cities() == 0;
            for i in 0..g.map.num_plots() {
                let plot = PlotIdx(i as u32);
                // BETTER AI (first settler) Sephi
                if g.map.plots[i].revealed[team] || no_cities {
                    // GC.getUSE_GET_CITY_FOUND_VALUE_CALLBACK(): Python getCityFoundValue
                    // returns -1 in FFH2 (and the callback is off), so AI_foundValue is used.
                    let (x, y) = g.map.xy(plot);
                    let value = g.player_ai_found_value(p, x, y, -1, false, &city_sites);

                    // C++ `m_aiFoundValue` is a `short` array: the stored value truncates.
                    g.map.plots[i].found_value[p] = value as i16 as i32;

                    if let Some(a) = g.map.plots[i].area {
                        if value > g.map.areas[a].best_found_value[p] {
                            g.map.areas[a].best_found_value[p] = value;
                        }
                    }
                }
            }
            if !barbarian {
                let max_city_count = 4;
                let min = self.player_ai_get_min_found_value(g, p);
                self.player_ai_update_city_sites(g, p, min, max_city_count);
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_updateAreaTargets()` (CvPlayerAI.cpp:1130-1149)
    pub fn player_ai_update_area_targets(&self, g: &mut Game, p: PlayerId) {
        for a in g.map.areas.ids() {
            if !g.map.areas[a].water {
                if self.rand(3, "AI Target City") == 0 {
                    g.map.areas[a].target_cities[p] = None;
                } else {
                    let target = self.player_ai_find_target_city(g, p, a);
                    g.map.areas[a].target_cities[p] = target;
                }
            }
        }
    }

    /// C++ `int CvPlayerAI::AI_movementPriority(CvSelectionGroup* pGroup) const` (CvPlayerAI.cpp:1153-1256)
    /// Returns priority for unit movement (lower values move first...)
    pub fn player_ai_movement_priority(&self, g: &Game, p: PlayerId, group: GroupId) -> i32 {
        let Some(head) = g.group_get_head_unit(group) else {
            return 16;
        };

        if g.unit_has_cargo(head) {
            if g.unit_special_cargo(head).is_none() {
                return 0;
            } else {
                return 1;
            }
        }

        if g.unit_get_domain_type(head) == DomainType::DOMAIN_AIR {
            return 2;
        }

        let unit_ai = g.units[head].ai_type;

        if unit_ai == Some(UnitAIType::UNITAI_WORKER) || unit_ai == Some(UnitAIType::UNITAI_WORKER_SEA) {
            return 3;
        }

        if unit_ai == Some(UnitAIType::UNITAI_EXPLORE) || unit_ai == Some(UnitAIType::UNITAI_EXPLORE_SEA) {
            return 4;
        }

        if g.unit_bombard_rate(head) > 0 {
            return 5;
        }

        if g.unit_collateral_damage(head) > 0 {
            return 6;
        }

        if g.unit_can_fight(head) {
            if g.unit_withdrawal_probability(head) > 20 {
                return 7;
            }

            if g.unit_withdrawal_probability(head) > 0 {
                return 8;
            }

            let mut curr_combat = g.unit_curr_combat_str(head, None, None, None);
            let best_combat = g.best_land_unit_combat() * 100;

            if g.unit_no_defensive_bonus(head) {
                curr_combat *= 3;
                curr_combat /= 2;
            }

            if self.unit_ai_is_city_ai_type(g, head) {
                curr_combat /= 2;
            }

            if curr_combat > best_combat {
                return 9;
            } else if curr_combat > ((best_combat * 4) / 5) {
                return 10;
            } else if curr_combat > ((best_combat * 3) / 5) {
                return 11;
            } else if curr_combat > ((best_combat * 2) / 5) {
                return 12;
            } else if curr_combat > ((best_combat * 1) / 5) {
                return 13;
            } else {
                return 14;
            }
        }

        15
    }

    /// C++ `void CvPlayerAI::AI_unitUpdate()` (CvPlayerAI.cpp:1259-1372)
    pub fn player_ai_unit_update(&self, g: &mut Game, p: PlayerId) {
        if g.player_has_busy_unit(p) {
            return;
        }

        for grp in g.players[p].group_cycle.clone() {
            if !g.groups.contains(grp) {
                continue;
            }
            if self.group_ai_is_force_separate(g, grp) {
                // do not split groups that are in the midst of attacking
                if g.groups[grp].force_update || !self.group_ai_is_group_attack(g, grp) {
                    self.group_ai_separate(g, grp); // pointers could become invalid...
                }
            }
        }

        if g.players[p].human {
            for grp in g.players[p].group_cycle.clone() {
                // group might have been killed by a previous group update
                if g.groups.contains(grp) {
                    if self.group_ai_update(g, grp) {
                        break; // pointers could become invalid...
                    }
                }
            }
        } else {
            // C++ moves the groups of the cycle to the final list in passes iValue = 0, 1, ...
            // taking those with AI_movementPriority <= iValue: a stable sort by priority (the
            // priority is a pure function of the group, so it is computed once).
            let mut final_group_cycle: Vec<(i32, GroupId)> = Vec::new();
            for grp in g.players[p].group_cycle.clone() {
                if g.groups.contains(grp) {
                    final_group_cycle.push((self.player_ai_movement_priority(g, p, grp), grp));
                }
            }
            final_group_cycle.sort_by_key(|&(priority, _)| priority);

            let is_civ = p.idx() < g.num_civ_players();
            for (_, grp) in final_group_cycle {
                // group might have been killed by a previous group update
                if g.groups.contains(grp) {
                    // TEMPFIX Sephi
                    if self.group_ai_update(g, grp) {
                        if is_civ {
                            break; // pointers could become invalid...
                        }
                    }
                }
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_makeAssignWorkDirty()` (CvPlayerAI.cpp:1375-1384)
    pub fn player_ai_make_assign_work_dirty(&self, g: &mut Game, p: PlayerId) {
        for c in g.players[p].cities.clone() {
            if g.cities.contains(c) {
                self.city_ai_set_assign_work_dirty(g, c, true);
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_assignWorkingPlots()` (CvPlayerAI.cpp:1387-1396)
    pub fn player_ai_assign_working_plots(&self, g: &mut Game, p: PlayerId) {
        for c in g.players[p].cities.clone() {
            if g.cities.contains(c) {
                self.city_ai_assign_working_plots(g, c);
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_updateAssignWork()` (CvPlayerAI.cpp:1399-1408)
    pub fn player_ai_update_assign_work(&self, g: &mut Game, p: PlayerId) {
        for c in g.players[p].cities.clone() {
            if g.cities.contains(c) {
                self.city_ai_update_assign_work(g, c);
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_makeProductionDirty()` (CvPlayerAI.cpp:1411-1422)
    pub fn player_ai_make_production_dirty(&self, g: &mut Game, p: PlayerId) {
        debug_assert!(!g.players[p].human, "isHuman did not return false as expected");

        for c in g.players[p].cities.clone() {
            if g.cities.contains(c) {
                self.city_ai_set_choose_production_dirty(g, c, true);
            }
        }
    }

    /// C++ `void CvPlayerAI::AI_conquerCity(CvCity* pCity)` (CvPlayerAI.cpp:1425-1545)
    /// Returns true to raze: the rules then raze the city (`doTask(TASK_RAZE)`), or else report
    /// `cityAcquiredAndKept` (the C++ function's other side effects).
    pub fn player_ai_conquer_city(&self, g: &Game, p: PlayerId, city: CityId) -> bool {
        let mut raze = false;

        if g.player_can_raze(p, city) {
            let mut raze_value = 0;
            if g.elapsed_game_turns > 20 {
                if g.players[p].num_cities() > 4 {
                    if !g.city_is_any_holy_city(city) && !g.city_has_active_world_wonder(city) {
                        let team = g.players[p].team;
                        let (cx, cy) = (g.cities[city].x, g.cities[city].y);
                        let area = g.city_area(city);
                        let previous_owner = g.cities[city].previous_owner;
                        let barbarian = g.barbarian_player();

                        if previous_owner != Some(barbarian) {
                            let nearest_city = g.map_find_city(cx, cy, None, Some(team), true, false, None, None, Some(city));

                            match nearest_city {
                                None => {
                                    if let Some(previous_owner) = previous_owner {
                                        let previous_team = g.players[previous_owner].team;
                                        if g.team_count_num_cities_by_area(previous_team, area) > 3 {
                                            raze_value += 30;
                                        }
                                    }
                                }
                                Some(nearest_city) => {
                                    let (nx, ny) = (g.cities[nearest_city].x, g.cities[nearest_city].y);
                                    let distance = g.plot_distance(cx, cy, nx, ny);
                                    if distance > 12 {
                                        raze_value += distance * 2;
                                    }
                                }
                            }
                        }

                        if g.map.areas[area].cities_per_player[p] > 0 {
                            if self.player_ai_is_financial_trouble(g, p) {
                                raze_value += 70 - 15 * g.cities[city].population;
                            }
                        }

                        if let Some(state_religion) = g.player_get_state_religion(p) {
                            if g.cities[city].has_religion[state_religion] {
                                if g.team_has_shrine(team, Some(state_religion)) {
                                    raze_value -= 50;
                                } else {
                                    raze_value -= 10;
                                }
                            }
                        }

                        // `pCity->AI_playerCloseness(getID())`: CvCity declares iMaxDistance = 7.
                        let closeness = self.city_ai_player_closeness(g, city, p, 7);
                        if closeness > 0 {
                            raze_value -= 25;
                            raze_value -= closeness * 2;
                        } else {
                            raze_value += 60;
                        }

                        if g.map.areas[area].cities_per_player[p] > 0 {
                            if previous_owner != Some(barbarian) {
                                if let Some(personality) = g.players[p].personality_type {
                                    raze_value += g.d.leader_heads[personality].raze_city_prob;
                                }
                            }
                        }

                        if raze_value > 0 {
                            for trait_type in g.d.traits.keys() {
                                if g.players[p].traits[trait_type] {
                                    raze_value *= 100 - g.d.traits[trait_type].upkeep_modifier;
                                    raze_value /= 100;
                                }
                            }

                            if self.rand(100, "AI Raze City") < raze_value {
                                raze = true;
                                // pCity->doTask(TASK_RAZE): done by the rules on `true`.
                            }
                        }
                    }
                }
            }
        }

        // !bRaze: CvEventReporter::cityAcquiredAndKept(getID(), pCity) -- done by the rules.
        raze
    }

    /// C++ `bool CvPlayerAI::AI_acceptUnit(CvUnit* pUnit) const` (CvPlayerAI.cpp:1548-1578)
    pub fn player_ai_accept_unit(&self, g: &Game, p: PlayerId, unit: UnitId) -> bool {
        if g.players[p].human {
            return true;
        }

        if self.player_ai_is_financial_trouble(g, p) {
            let unit_ai = g.units[unit].ai_type;
            if unit_ai == Some(UnitAIType::UNITAI_WORKER) {
                if self.player_ai_needed_workers(g, p, g.unit_area(unit)) > 0 {
                    return true;
                }
            }

            if unit_ai == Some(UnitAIType::UNITAI_WORKER_SEA) {
                return true;
            }

            if unit_ai == Some(UnitAIType::UNITAI_MISSIONARY) {
                return true; //XXX
            }
            return false;
        }

        true
    }

    /// C++ `bool CvPlayerAI::AI_captureUnit(UnitTypes eUnit, CvPlot* pPlot) const` (CvPlayerAI.cpp:1581-1610)
    pub fn player_ai_capture_unit(&self, g: &Game, p: PlayerId, unit: UnitType, plot: PlotIdx) -> bool {
        let team = g.players[p].team;

        if g.plot_get_team(plot) == Some(team) {
            return true;
        }

        let (x, y) = g.map.xy(plot);
        let nearest_city = g.map_find_city(x, y, None, Some(team), true, false, None, None, None);

        if let Some(nearest_city) = nearest_city {
            if g.plot_distance(x, y, g.cities[nearest_city].x, g.cities[nearest_city].y) <= 4 {
                return true;
            }
        }

        // FfH: Added by Kael 01/19/2008
        if g.d.units[unit].equipment_promotion.is_some() {
            return true;
        }
        // FfH: End Add

        false
    }

    /// C++ `DomainTypes CvPlayerAI::AI_unitAIDomainType(UnitAITypes eUnitAI) const` (CvPlayerAI.cpp:1613-1695)
    pub fn player_ai_unit_ai_domain_type(&self, g: &Game, p: PlayerId, unit_ai: UnitAIType) -> Option<DomainType> {
        use UnitAIType as U;
        match unit_ai {
            U::UNITAI_UNKNOWN => None,

            U::UNITAI_ANIMAL
            | U::UNITAI_SETTLE
            | U::UNITAI_WORKER
            | U::UNITAI_ATTACK
            | U::UNITAI_ATTACK_CITY
            | U::UNITAI_COLLATERAL
            | U::UNITAI_PILLAGE
            | U::UNITAI_RESERVE
            | U::UNITAI_COUNTER
            | U::UNITAI_PARADROP
            | U::UNITAI_CITY_DEFENSE
            | U::UNITAI_CITY_COUNTER
            | U::UNITAI_CITY_SPECIAL
            | U::UNITAI_EXPLORE
            | U::UNITAI_MISSIONARY
            | U::UNITAI_PROPHET
            | U::UNITAI_ARTIST
            | U::UNITAI_SCIENTIST
            | U::UNITAI_GENERAL
            | U::UNITAI_MERCHANT
            | U::UNITAI_ENGINEER
            | U::UNITAI_SPY
            | U::UNITAI_ATTACK_CITY_LEMMING
            // BETTER AI (New UNITAI) Sephi
            | U::UNITAI_MAGE
            | U::UNITAI_TERRAFORMER
            | U::UNITAI_MANA_UPGRADE
            | U::UNITAI_WARWIZARD
            | U::UNITAI_BARBSMASHER
            | U::UNITAI_HERO
            | U::UNITAI_FEASTING
            | U::UNITAI_MEDIC => Some(DomainType::DOMAIN_LAND),

            U::UNITAI_ICBM => Some(DomainType::DOMAIN_IMMOBILE),

            U::UNITAI_WORKER_SEA
            | U::UNITAI_ATTACK_SEA
            | U::UNITAI_RESERVE_SEA
            | U::UNITAI_ESCORT_SEA
            | U::UNITAI_EXPLORE_SEA
            | U::UNITAI_ASSAULT_SEA
            | U::UNITAI_SETTLER_SEA
            | U::UNITAI_MISSIONARY_SEA
            | U::UNITAI_SPY_SEA
            | U::UNITAI_CARRIER_SEA
            | U::UNITAI_MISSILE_CARRIER_SEA
            | U::UNITAI_PIRATE_SEA => Some(DomainType::DOMAIN_SEA),

            U::UNITAI_ATTACK_AIR | U::UNITAI_DEFENSE_AIR | U::UNITAI_CARRIER_AIR | U::UNITAI_MISSILE_AIR => {
                Some(DomainType::DOMAIN_AIR)
            }

            _ => {
                debug_assert!(false, "AI_unitAIDomainType: unknown unit AI {:?}", unit_ai);
                None
            }
        }
    }

    /// C++ `int CvPlayerAI::AI_yieldWeight(YieldTypes eYield) const` (CvPlayerAI.cpp:1698-1706)
    pub fn player_ai_yield_weight(&self, g: &Game, p: PlayerId, yield_: YieldType) -> i32 {
        if yield_ == YieldType::YIELD_PRODUCTION {
            let era = g.current_era().map_or(-1, |e| e.i());
            let production_modifier = 100 + (30 * 0.max(era - 1) / 1.max(g.d.eras.len() as i32 - 2));
            return (g.d.yields[yield_].ai_weight_percent * production_modifier) / 100;
        }
        g.d.yields[yield_].ai_weight_percent
    }

    /// C++ `int CvPlayerAI::AI_commerceWeight(CommerceTypes eCommerce, CvCity* pCity = NULL) const` (CvPlayerAI.cpp:1709-1844)
    pub fn player_ai_commerce_weight(&self, g: &Game, p: PlayerId, commerce: CommerceType, city: Option<CityId>) -> i32 {
        let mut weight = g.d.commerces[commerce].ai_weight_percent;

        //sorry but the merchant descrimination must stop.
        weight = weight.min(110);

        //XXX Add something for 100%/0% type situations
        match commerce {
            CommerceType::COMMERCE_RESEARCH => {
                if self.player_ai_avoid_science(g, p) {
                    if g.player_is_no_research_available(p) {
                        weight = 0;
                    } else {
                        weight /= 8;
                    }
                }
            }
            CommerceType::COMMERCE_GOLD => {
                let gold_percent = g.players[p].commerce_percent[CommerceType::COMMERCE_GOLD.idx()];
                if gold_percent == 100 {
                    //avoid strikes
                    if g.players[p].gold_per_turn < 0 {
                        weight += 15;
                    }
                } else if gold_percent == 0 {
                    //put more money towards other commerce types
                    if g.players[p].gold_per_turn > 0 {
                        weight -= 15;
                    }
                }
            }
            CommerceType::COMMERCE_CULTURE => {
                // COMMERCE_CULTURE AIWeightPercent is 25% in default xml

                if let Some(c) = city {
                    let last_level = CultureLevelType::from_usize(g.d.culture_levels.len() - 1);
                    if g.cities[c].culture[p] >= 100 * g.culture_threshold(last_level) {
                        weight /= 50;
                    } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3) {
                        let culture_rate_rank = g.city_find_commerce_rate_rank(c, CommerceType::COMMERCE_CULTURE);
                        let cultural_victory_num_culture_cities = g.num_culture_victory_cities;

                        // if one of the currently best cities, then focus hard, *4
                        if culture_rate_rank <= cultural_victory_num_culture_cities {
                            weight *= 4;
                        }
                        // if one of the 3 close to the top, then still emphasize culture some, *2
                        else if culture_rate_rank <= cultural_victory_num_culture_cities + 3 {
                            weight *= 2;
                        }
                    } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2) {
                        weight *= 3;
                    } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1) {
                        weight *= 2;
                    }

                    weight += 100 - g.plot_calculate_culture_percent(g.city_plot(c), p);

                    // C++ `getCultureLevel() <= (CultureLevelTypes)1` (NO_CULTURELEVEL is -1).
                    if g.cities[c].culture_level.map_or(-1, |l| l.i()) <= 1 {
                        weight = weight.max(800);
                    }
                }
                // pCity == NULL
                else {
                    if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE3) {
                        weight *= 3;
                        weight /= 4;
                    } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE2) {
                        weight *= 2;
                        weight /= 3;
                    } else if self.player_ai_is_do_strategy(g, p, AI_STRATEGY_CULTURE1) {
                        weight /= 2;
                    } else {
                        weight /= 3;
                    }
                }
            }
            CommerceType::COMMERCE_ESPIONAGE => {
                let team = g.players[p].team;
                let mut esp_multiplier = 0;
                for t in g.civ_team_ids() {
                    if g.team_is_alive(t) && t != team && !g.teams[t].vassal[team] && !g.teams[team].vassal[t] {
                        let point_diff = g.teams[t].espionage_points_against_team[team] - g.teams[team].espionage_points_against_team[t];
                        if point_diff > 0 {
                            esp_multiplier += 1;
                            break;
                        }
                    }
                }
                esp_multiplier += if esp_multiplier > 0 { 3 } else { 0 };
                weight *= esp_multiplier;
                weight *= self.player_ai_get_espionage_weight(g, p);
                weight /= 100;
            }
            _ => {}
        }

        weight
    }

    /// C++ `int CvPlayerAI::AI_foundValue(int iX, int iY, int iMinRivalRange = -1, bool bStartingLoc = false) const` (CvPlayerAI.cpp:1847-2993)
    pub fn player_ai_found_value(&self, g: &Game, p: PlayerId, x: i32, y: i32, min_rival_range: i32, starting_loc: bool) -> i32 {
        // ffh-core's audited port; the C++ reads this player's AI city sites
        // (`AI_isPlotCitySite`), which live in this crate's state.
        let city_sites = self.s().player(p).ai_city_sites.clone();
        g.player_ai_found_value(p, x, y, min_rival_range, starting_loc, &city_sites)
    }

    /// C++ `bool CvPlayerAI::AI_isAreaAlone(CvArea* pArea) const` (CvPlayerAI.cpp:2996-2999)
    pub fn player_ai_is_area_alone(&self, g: &Game, p: PlayerId, area: AreaId) -> bool {
        let team = g.players[p].team;
        (g.map.areas[area].num_cities - g.team_count_num_cities_by_area(g.barbarian_team(), area))
            == g.team_count_num_cities_by_area(team, area)
    }

    /// C++ `bool CvPlayerAI::AI_isCapitalAreaAlone() const` (CvPlayerAI.cpp:3002-3014)
    pub fn player_ai_is_capital_area_alone(&self, g: &Game, p: PlayerId) -> bool {
        if let Some(capital_city) = g.player_get_capital_city(p) {
            return self.player_ai_is_area_alone(g, p, g.city_area(capital_city));
        }

        false
    }

    /// C++ `bool CvPlayerAI::AI_isPrimaryArea(CvArea* pArea) const` (CvPlayerAI.cpp:3017-3042)
    pub fn player_ai_is_primary_area(&self, g: &Game, p: PlayerId, area: AreaId) -> bool {
        if g.map.areas[area].water {
            return false;
        }

        if g.map.areas[area].cities_per_player[p] > 2 {
            return true;
        }

        if let Some(capital_city) = g.player_get_capital_city(p) {
            if g.city_area(capital_city) == area {
                return true;
            }
        }

        false
    }

    /// C++ `int CvPlayerAI::AI_militaryWeight(CvArea* pArea) const` (CvPlayerAI.cpp:3045-3048)
    pub fn player_ai_military_weight(&self, g: &Game, p: PlayerId, area: AreaId) -> i32 {
        let a = &g.map.areas[area];
        a.population_per_player[p] + a.cities_per_player[p] + 1
    }

    /// C++ `int CvPlayerAI::AI_targetCityValue(CvCity* pCity, bool bRandomize, bool bIgnoreAttackers = false) const` (CvPlayerAI.cpp:3051-3131)
    pub fn player_ai_target_city_value(&self, g: &Game, p: PlayerId, city: CityId, randomize: bool, ignore_attackers: bool) -> i32 {
        let team = g.players[p].team;
        let c = &g.cities[city];
        let (cx, cy) = (c.x, c.y);
        let city_plot = g.city_plot(city);

        let mut value = 1;

        value += (c.population * (50 + g.city_calculate_culture_percent(city, p))) / 100;

        if c.defense_damage > 0 {
            value += (c.defense_damage / 30) + 1;
        }

        if g.city_is_coastal(city, g.d.defines.min_water_size_for_ocean) {
            value += 1;
        }

        if g.city_has_active_world_wonder(city) {
            value += 1 + c.num_world_wonders;
        }

        if g.city_is_any_holy_city(city) {
            value += 2;
        }

        if c.ever_owned[p] {
            value += 3;
        }
        if !ignore_attackers {
            value += self.player_ai_adjacent_potential_attackers(g, p, city_plot, false);
        }

        for i in 0..NUM_CITY_PLOTS {
            if let Some(loop_plot) = g.plot_city(cx, cy, i) {
                if g.plot_get_bonus_type(loop_plot, Some(team)).is_some() {
                    value += 1;
                }

                if g.map.plots[loop_plot].owner == Some(p) {
                    value += 1;
                }

                if g.plot_is_adjacent_player(loop_plot, p, true) {
                    value += 1;
                }
            }
        }

        let nearest_city = g.map_find_city(cx, cy, Some(p), None, true, false, None, None, None);

        if let Some(nearest_city) = nearest_city {
            let path_distance = calculate_path_distance(g, g.city_plot(nearest_city), city_plot);
            value += 1.max((g.map.max_step_distance() * 2) - path_distance);
        }

        if randomize {
            value += self.rand((g.cities[city].population / 2) + 1, "AI Target City Value");
        }

        value
    }

    /// C++ `CvCity* CvPlayerAI::AI_findTargetCity(CvArea* pArea) const` (CvPlayerAI.cpp:3134-3170)
    pub fn player_ai_find_target_city(&self, g: &Game, p: PlayerId, area: AreaId) -> Option<CityId> {
        let team = g.players[p].team;
        let mut best_value = 0;
        let mut best_city = None;

        for i in g.civ_player_ids() {
            if g.players[i].alive {
                if g.is_potential_enemy(team, Some(g.players[i].team)) {
                    for &c in &g.players[i].cities {
                        if g.city_area(c) == area {
                            let value = self.player_ai_target_city_value(g, p, c, true, false);

                            if value > best_value {
                                best_value = value;
                                best_city = Some(c);
                            }
                        }
                    }
                }
            }
        }

        best_city
    }

    /// C++ `bool CvPlayerAI::AI_isCommercePlot(CvPlot* pPlot) const` (CvPlayerAI.cpp:3173-3176)
    pub fn player_ai_is_commerce_plot(&self, g: &Game, p: PlayerId, plot: PlotIdx) -> bool {
        g.map.plots[plot].yields[YieldType::YIELD_FOOD.idx()] >= g.d.defines.food_consumption_per_population
    }

    /// C++ `int CvPlayerAI::AI_getPlotDanger(CvPlot* pPlot, int iRange = -1, bool bTestMoves = true) const` (CvPlayerAI.cpp:3179-3271)
    pub fn player_ai_get_plot_danger(&self, g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool) -> i32 {
        let (mut count, border_danger) = danger_count(g, p, plot, range, test_moves, false);

        if border_danger > 0 {
            if !g.players[p].human && !g.plot_is_city(plot, false, None) {
                count += border_danger;
            }
        }

        count
    }

    /// C++ `int CvPlayerAI::AI_getUnitDanger(CvUnit* pUnit, int iRange = -1, bool bTestMoves = true, bool bAnyDanger = true) const` (CvPlayerAI.cpp:3274-3366)
    pub fn player_ai_get_unit_danger(&self, g: &Game, p: PlayerId, unit: UnitId, range: i32, test_moves: bool, any_danger: bool) -> i32 {
        // (bAnyDanger is unused in the C++.)
        let plot = g.unit_plot(unit);
        let (mut count, border_danger) = danger_count(g, p, plot, range, test_moves, true);

        if border_danger > 0 {
            if !g.players[p].human || g.unit_is_automated(unit) {
                count += border_danger;
            }
        }

        count
    }
}

/// The shared loop of C++ `AI_getPlotDanger` and `AI_getUnitDanger`: (enemy units that can
/// reach `plot`, border danger). `at_war_units`: AI_getUnitDanger tests `atWar(unit team,
/// getTeam())` where AI_getPlotDanger tests `pLoopUnit->isEnemy(getTeam())`.
fn danger_count(g: &Game, p: PlayerId, plot: PlotIdx, range: i32, test_moves: bool, at_war_units: bool) -> (i32, i32) {
    let team = g.players[p].team;
    let plot_area = g.map.plots[plot].area;
    let (px, py) = g.map.xy(plot);

    let mut count = 0;
    let mut border_danger = 0;

    let range = if range == -1 { DANGER_RANGE } else { range };

    for dx in -range..=range {
        for dy in -range..=range {
            let Some(loop_plot) = g.plot_xy(px, py, dx, dy) else {
                continue;
            };

            if g.map.plots[loop_plot].area != plot_area {
                continue;
            }

            let (lx, ly) = g.map.xy(loop_plot);
            let distance = g.step_distance(px, py, lx, ly);
            if g.at_war_opt(g.plot_get_team(loop_plot), Some(team)) {
                if distance == 1 {
                    border_danger += 1;
                } else if distance == 2 && g.plot_is_route(loop_plot) {
                    border_danger += 1;
                }
            }

            for &loop_unit in &g.map.plots[loop_plot].units {
                let enemy = if at_war_units {
                    g.at_war(g.unit_get_team(loop_unit), team)
                } else {
                    g.unit_is_enemy(loop_unit, Some(team), None)
                };
                if enemy {
                    if g.unit_can_attack(loop_unit) {
                        if !g.unit_is_invisible(loop_unit, team, false) {
                            if g.unit_can_move_or_attack_into(loop_unit, plot, false) {
                                if !test_moves {
                                    count += 1;
                                } else {
                                    let mut danger_range = g.unit_base_moves(loop_unit);
                                    danger_range += if g.plot_is_valid_route(loop_plot, loop_unit) { 1 } else { 0 };
                                    if danger_range >= distance {
                                        count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (count, border_danger)
}

/// C++ `GC.getGameINLINE().implementDeal(..)` of a deal with another AI player, which the C++
/// signs at once (every AI player ran the same AI). PORT ADAPT: when the other player is played
/// by a different controller (ffh-ai in head-to-head games), that controller must agree first.
fn implement_ai_deal(g: &mut Game, p: PlayerId, other: PlayerId, our_list: &[TradeData], their_list: &[TradeData]) {
    let c = g.controller(other);
    if c.name() != "cppai" && !c.consider_offer(g, other, p, our_list, their_list) {
        return;
    }
    g.implement_deal(p, other, our_list, their_list, false);
}
