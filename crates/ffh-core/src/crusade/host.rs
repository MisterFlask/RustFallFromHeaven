//! The Host player: a reserved slot brought alive for one crusade and freed at dissolution.
//!
//! `GameSetup::from_options` reserves `CrusadeRules::host_slots` open slots (flagged
//! `PlayerSetup::host`) after the spare slots when crusades are on; FfH's mid-game players
//! (Infernals, Mercurians, `formEmpire`) never take them. A Host is initialized like a player
//! FfH adds mid-game (`CvGame::addPlayer`: team and player reset, which also clears every other
//! player's and team's entries about the slot), with:
//!
//! - the target's civilization (so the cities it captures keep their buildings by class) and the
//!   barbarian leader (no leader traits), the civilization trait removed and a neutral alignment:
//!   lent units gain nothing by changing owner;
//! - no techs, no research, no economy: its commerce goes to gold, which is the crusade's sacked
//!   wealth (`player_calculate_pre_inflated_costs` is 0 for a Host: no upkeep, no maintenance);
//! - no diplomacy (`player_can_contact` is false), no events, and wars only through the crusade:
//!   at war with the target and its vassals (set here, without the declaration's cascades) and
//!   the barbarians, at peace with everyone else, open borders with its lenders and the victim.
//!
//! When freed, the slot dies (`setAlive(false)`) and its culture is wiped from the map, so a
//! later Host in the same slot starts clean.

use crate::game::Game;
use crate::ids::*;
use crate::setup::SlotStatus;
use ffh_data::*;

impl Game {
    /// A reserved Host slot that is free now.
    pub fn crusade_free_host_slot(&self) -> Option<PlayerId> {
        self.civ_player_ids().find(|&p| self.player_is_host_slot(p) && !self.players[p].alive && self.crusade_of_host(p).is_none())
    }

    /// Bring slot `p` alive as the Host of a crusade against `target`, named `name`.
    pub(crate) fn crusade_activate_host(&mut self, p: PlayerId, target: TeamId, name: String) {
        let d = self.d;
        let target_leader = self.team_get_leader_id(target).expect("crusade target without a player");
        let civ = self.player_get_civilization_type(target_leader).expect("crusade target without a civilization");
        let leader = opt_from_i32::<LeaderHeadType>(d.defines.barbarian_leader).expect("BARBARIAN_LEADER");
        let team = TeamId(p.0);

        // CvGame::addPlayer, on the slot's own team.
        self.setup_set_team(p, team);
        self.setup.players[p.idx()].leader = Some(leader);
        self.setup.players[p.idx()].civ = Some(civ);
        self.setup.players[p.idx()].leader_name = name.clone();
        self.setup.players[p.idx()].civ_description = name;
        self.setup_set_slot_status(p, SlotStatus::Computer);
        self.team_init(team);
        // A call between player turns (a council vote, at the start of the game turn) must not
        // start the Host's turn at once (`setAlive` does when no turn is active): it plays in
        // slot order like everyone else.
        let active = self.num_game_turn_active;
        self.num_game_turn_active = active.max(1);
        self.player_init(p);
        self.num_game_turn_active = active;

        // Nothing changes for a unit by joining the Host.
        if let Some(civ_trait) = d.civilizations[civ].civ_trait {
            if self.players[p].traits[civ_trait] {
                self.player_set_has_trait(p, civ_trait, false);
            }
        }
        self.players[p].alignment = Some(AlignmentType::ALIGNMENT_NEUTRAL);
        for c in 0..CommerceType::NUM {
            let c = CommerceType::from_usize(c);
            let value = if c == CommerceType::COMMERCE_GOLD { 100 } else { 0 };
            self.player_set_commerce_percent(p, c, value);
        }

        // At war with the barbarians, as every civ.
        let barbarian = self.barbarian_team();
        self.crusade_set_host_war(team, barbarian);
    }

    /// Host team `host` goes to war with `enemy` (no declaration cascade: no defensive pacts, no
    /// vassal chains, no memories; the Host fights only this war).
    pub(crate) fn crusade_set_host_war(&mut self, host: TeamId, enemy: TeamId) {
        if host == enemy || self.teams[host].at_war[enemy] {
            return;
        }
        let members = |g: &Game, t: TeamId| g.player_ids().filter(|&p| g.players[p].team == t).collect::<Vec<_>>();
        let both: Vec<PlayerId> = members(self, host).into_iter().chain(members(self, enemy)).collect();
        for &p in &both {
            self.player_update_plunder(p, -1, false);
        }
        self.teams[host].at_war[enemy] = true;
        self.teams[enemy].at_war[host] = true;
        for &p in &both {
            self.player_update_plunder(p, 1, false);
        }
        if !self.team_is_barbarian(enemy) {
            self.team_meet(host, enemy, false);
        }
        self.teams[host].ai_at_peace_counter[enemy] = 0;
        self.teams[enemy].ai_at_peace_counter[host] = 0;
        self.team_ai_set_war_plan(host, enemy, Some(WarPlanType::WARPLAN_TOTAL), true);
        self.team_ai_set_war_plan(enemy, host, Some(WarPlanType::WARPLAN_ATTACKED_RECENT), true);
        self.map_verify_unit_valid_plot();
        for &p in &both {
            if self.players[p].alive {
                self.player_update_war_weariness_percent_anger(p);
                self.player_update_plot_groups(p);
                self.player_update_trade_routes(p);
            }
        }
    }

    /// Passage for the Host of crusade `idx` through team `t`'s lands (a lender or the victim).
    pub(crate) fn crusade_grant_passage(&mut self, idx: usize, t: TeamId) {
        let host = self.players[self.crusades[idx].host].team;
        if host == t {
            return;
        }
        self.team_meet(host, t, false);
        if !self.teams[host].open_borders[t] {
            self.team_set_open_borders(host, t, true);
        }
        if !self.teams[t].open_borders[host] {
            self.team_set_open_borders(t, host, true);
        }
    }

    /// Is a plot owned by team `plot_team` friendly ground for healing the Host on team `t`
    /// (a lender's or the victim's land)?
    pub fn crusade_host_heals_in(&self, t: TeamId, plot_team: Option<TeamId>) -> bool {
        let (Some(i), Some(plot_team)) = (self.crusade_of_host_team(t), plot_team) else { return false };
        let c = &self.crusades[i];
        c.lenders.iter().any(|l| self.players[l.player].team == plot_team) || c.victim.is_some_and(|v| self.players[v].team == plot_team)
    }

    /// Free the Host slot `p` (dissolution, last step): the player dies with whatever it still
    /// owns and its culture is wiped.
    pub(crate) fn crusade_free_host(&mut self, p: PlayerId) {
        let team = self.players[p].team;
        // Every war of the Host ends with it.
        for t in self.team_ids() {
            if t != team && self.teams[team].at_war[t] {
                self.team_make_peace(team, t, true);
            }
            // (one-sided barbarian flags)
            self.teams[team].at_war[t] = false;
            self.teams[t].at_war[team] = false;
        }
        if self.players[p].alive {
            self.player_set_alive(p, false);
        }
        for plot in self.map.plot_ids() {
            if self.map.plots[plot].culture[p] != 0 {
                self.plot_set_culture(plot, p, 0, true, true);
            }
        }
        for c in self.cities.ids() {
            if self.cities[c].culture[p] != 0 {
                self.city_set_culture_times100(c, p, 0, false, true);
            }
        }
        self.setup_set_slot_status(p, SlotStatus::Open);
    }
}
