//! FFH2 AI: a new design (see docs/AI.md), not a port of the C++ AI.
//!
//! [`Ai`] implements [`ffh_core::Controller`]. Each turn (`play_turn`) it builds a
//! [`view::View`] (enemies, danger map), manages the empire (research, civics, religion,
//! sliders), diplomacy, then gives every group that needs orders one decision through the
//! rules' mission API, and finally casts end-of-turn spells as FfH's AI does. City
//! production is chosen when the rules ask (`choose_production`).
//!
//! All decisions are deterministic functions of the game state: the AI draws no random
//! numbers and its caches are recomputed whenever the turn changes.

use std::sync::{Arc, Mutex};

use ffh_core::controller::{Controller, ControllerRef, OfferVerdict};
use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_core::structs::*;
use ffh_data::*;

pub mod armies;
pub mod governor;
pub mod automation;
pub mod campaign;
pub mod casters;
pub mod cards;
pub mod cards_battle_arms;
pub mod cards_battle_magic;
pub mod cards_battle_more;
pub mod cards_deck_ui;
pub mod cards_economy;
pub mod cards_map_rules;
pub mod cards_new_civs;
pub mod cards_new_civs_2;
pub mod cards_world_spells;
pub mod cards_world_spells_2;
pub mod casting;
pub mod combat_sim;
pub mod civs;
pub mod code_effects;
pub mod council;
pub mod culture;
pub mod defense;
pub mod crusade;
pub mod diplomacy;
pub mod economy;
pub mod empire;
pub mod events;
pub mod expansion;
pub mod finance;
pub mod forecast;
pub mod invasion;
pub mod lanun_khazad;
pub mod magic;
pub mod mana;
pub mod military;
pub mod naval;
pub mod offers;
pub mod outposts;
pub mod pacts;
pub mod predict;
pub mod probes;
pub mod production;
pub mod reach;
pub mod research;
pub mod research_path;
pub mod sea_ice;
pub mod sites;
pub mod supply;
pub mod switches;
pub mod tactics;
pub mod tech_plan;
pub mod threat;
#[cfg(test)]
pub(crate) mod testkit;
pub mod trade;
pub mod traits;
pub mod unflagged;
pub mod units;
pub mod values;
pub mod vassals;
pub mod victory;
pub mod warships;
pub mod view;
pub mod workers;
pub mod world_rituals;
pub mod world_spells;

/// Debug tracing: `FFH_AI_TRACE=all` or `FFH_AI_TRACE=<player id>` prints AI decisions to
/// stderr. Read once; it only prints, so it cannot change the game.
pub fn trace_on(p: PlayerId) -> bool {
    static TRACE: std::sync::OnceLock<Option<i32>> = std::sync::OnceLock::new();
    let t = TRACE.get_or_init(|| match std::env::var("FFH_AI_TRACE") {
        Ok(v) if v == "all" => Some(-1),
        Ok(v) => v.parse().ok(),
        Err(_) => None,
    });
    matches!(*t, Some(-1)) || *t == Some(p.0 as i32)
}

#[macro_export]
macro_rules! trace {
    ($g:expr, $p:expr, $($arg:tt)*) => {
        if $crate::trace_on($p) {
            eprintln!("[t{} p{}] {}", $g.game_turn(), $p.0, format!($($arg)*));
        }
    };
}

/// The AI controller. One instance may serve any number of players.
#[derive(Default)]
pub struct Ai {
    pub(crate) sites: Mutex<sites::SiteCaches>,
    /// Game turn of each player's last full `play_turn` (C++ calls `AI_unitUpdate` on every
    /// `update` until the turn ends; empire-level decisions are made once per turn). Only a
    /// shortcut: `play_turn` also derives "already played" from the game state.
    played: Mutex<std::collections::BTreeMap<PlayerId, i32>>,
    /// Each player's battle forecasts of its turn (`forecast::Cache`), lent to its view while it
    /// plays.
    forecasts: Mutex<std::collections::BTreeMap<PlayerId, forecast::Cache>>,
}

impl Ai {
    pub fn new() -> Ai {
        Ai::default()
    }

    /// Player `p`'s forecast cache for this game turn (`forecast::Cache`), to lend to its view.
    fn lend_forecasts(&self, g: &Game, p: PlayerId) -> forecast::Cache {
        self.forecasts.lock().unwrap().remove(&p).unwrap_or_default().for_turn(g.game_turn())
    }

    /// The cache back from the view `v`; with telemetry on, the forecasts asked for and played
    /// on this call reported (`decisions.forecast`).
    fn return_forecasts(&self, g: &Game, v: &mut view::View) {
        let c = std::mem::take(&mut v.forecasts);
        if g.telemetry.on() && c.calls.get() > 0 {
            g.tel_decision(v.p, "forecast", "calls", "", c.calls.get(), String::new);
            g.tel_decision(v.p, "forecast", "played", "", c.played.get(), String::new);
        }
        c.calls.set(0);
        c.played.set(0);
        self.forecasts.lock().unwrap().insert(v.p, c);
    }
}

pub fn ai_controller() -> ControllerRef {
    Arc::new(Ai::new())
}

impl Controller for Ai {
    fn name(&self) -> &str {
        "ffh-ai"
    }

    fn play_turn(&self, g: &mut Game, p: PlayerId) {
        if !g.players[p].alive {
            return;
        }
        // A crusade Host (docs/CRUSADE.md) plays only its war.
        if g.player_is_crusade_host(p) {
            crusade::host_turn(g, p);
            supply::turn(g, p);
            tactics::turn(g, p);
            return;
        }
        let turn = g.game_turn();
        // The empire decisions are made on the first call of the player's turn. A controller
        // that did not see the earlier calls (a game saved mid-turn, resumed with a fresh AI)
        // recognises them from the game state: the rules call again within a turn only while
        // some unit is still moving, i.e. has spent moves this turn.
        let seen = self.played.lock().unwrap().insert(p, turn) == Some(turn);
        let first_call = !seen && !g.players[p].units.iter().any(|&u| g.units.contains(u) && g.unit_has_moved(u));
        if first_call {
            lanun_khazad::delve_travel(g, p); // units take the Khazad's tunnels before they play
        }
        // The first call's empire decisions read only the view's header (diplomacy, offers, the
        // exposure diagnostic), and the view is rebuilt after them: analyse it now only when the
        // units play on it (a later call, or the barbarians).
        let mut v = view::View::header(g, p);
        v.forecasts = self.lend_forecasts(g, p);
        if !first_call || v.barbarian {
            v.analyze(g);
        }
        if first_call && !v.barbarian {
            military::report_exposure(g, &v, p);
        }
        if !first_call {
            // Called again in the same turn: only groups that still need orders.
            units::play(self, g, &mut v);
            self.return_forecasts(g, &mut v);
            return;
        }
        if !v.barbarian {
            if trace_on(p) {
                trace!(g, p, "{}", victory::describe(g, p, &victory::plan(g, p)));
            }
            if g.telemetry.on() {
                telemetry_intents(g, p);
            }
            research_path::turn(g, p); // research paths (docs/AI.md, "Research paths")
            empire::manage(g, p);
            production::emergency(self, g, p);
            production::trouble_rebuild(self, g, p);
            production::wonder_emphasis(g, p);
            expansion::prepare_escorts(g, p); // early-expansion
            diplomacy::reconsider_deals(g, p);
            diplomacy::manage(g, &v);
            offers::to_humans(g, &v); // ai-offers: deals offered to human players
            // Crusades and loans (docs/CRUSADE.md): pledged units leave before ours move.
            crusade::deliver(g, p);
            crusade::ask_for_loans(g, p);
            // War declarations change the picture.
            // (`View::new`, the forecast cache carried over.)
            let forecasts = std::mem::take(&mut v.forecasts);
            v = view::View::header(g, p);
            v.forecasts = forecasts;
            v.analyze(g);
            civs::turn(self, g, &mut v); // civ-mechanics
            traits::turn(g, &v); // trait-play
            governor::turn(self, g, p); // governed cities: priorities, empire orders (docs/GOVERNORS.md)
            armies::turn(g, p); // armies' reinforcements (docs/ARMIES.md)
            supply::turn(g, p); // telemetry only: the armies' supply counters (docs/SUPPLY.md)
            culture::turn(g, p); // cultural channels (docs/CULTURE.md), after the governors' production
            cards::turn(g, &v); // the great-person cards' content: world spells, rituals (docs/CARDS.md)
            world_rituals::turn(g, &v); // the civilization's world ritual (docs/CARDS.md)
            lanun_khazad::turn(g, &v); // the Lanun's storm, the Khazad's tunnels (docs/CARDS.md)
            outposts::turn(g, p); // outposts raised to cities (docs/NAVAL.md, 9.2)
        }
        if !v.barbarian {
            campaign::plan(g, &mut v);
            casting::barrage(g, &v);
        }
        units::play(self, g, &mut v);
        if !v.barbarian {
            tactics::turn(g, p); // battle tactics, once the stacks of the turn are made (docs/TACTICS.md)
            cards::end_turn(g, p); // the cards' content that acts once the units have moved
        }
        sea_ice::end_turn(g, p); // Icebreak near our cities (ffh_core world_ritual_spells::illians)
        casting::end_turn(g, &v);
        // Spells may have summoned units or used moves: make sure nothing is left ready.
        units::finalize(g, p);
        self.return_forecasts(g, &mut v);
    }

    fn choose_production(&self, g: &mut Game, c: CityId) {
        // A crusade Host has no economy: its cities build nothing.
        if g.player_is_crusade_host(g.cities[c].owner) {
            return;
        }
        production::choose(self, g, c);
    }

    /// The unit a city trains for an army's requisition (docs/ARMIES.md): siege up to a third
    /// of the army, then the best attacker against cities (`armies::requisition_unit`).
    fn requisition_unit(&self, g: &Game, c: CityId, a: ArmyId) -> Option<(UnitType, UnitAIType)> {
        armies::requisition_unit(g, c, a)
    }

    /// What a city's work crews build next (docs/WORKS.md): `workers::choose_crew_build`.
    fn choose_crew_build(&self, g: &Game, c: CityId) -> Option<(PlotIdx, BuildType)> {
        workers::choose_crew_build(g, c)
    }

    fn govern_cities(&self, g: &mut Game, p: PlayerId) {
        // The AI's own cities are its play (`play_turn`); a human's governors act here.
        if g.players[p].human {
            culture::govern(g, p);
        }
    }

    fn choose_research(&self, g: &Game, p: PlayerId) -> Option<TechType> {
        research::choose(g, p)
    }

    fn raze_conquered_city(&self, g: &Game, p: PlayerId, c: CityId) -> bool {
        diplomacy::raze_conquered_city(g, p, c)
    }

    fn consider_offer(&self, g: &Game, p: PlayerId, from: PlayerId, they_give: &[TradeData], we_give: &[TradeData]) -> bool {
        diplomacy::consider_offer(g, p, from, they_give, we_give)
    }

    fn judge_offer(&self, g: &Game, p: PlayerId, from: PlayerId, they_give: &[TradeData], we_give: &[TradeData]) -> OfferVerdict {
        diplomacy::judge_offer(g, p, from, they_give, we_give, false)
    }

    fn trade_value(&self, g: &Game, p: PlayerId, other: PlayerId, item: &TradeData, p_gets: bool) -> Option<i32> {
        trade::item_value(g, p, other, item, p_gets)
    }

    fn attitude(&self, g: &Game, p: PlayerId, towards: PlayerId) -> AttitudeType {
        diplomacy::attitude(g, p, towards)
    }

    fn update_automated_group(&self, g: &mut Game, grp: GroupId) -> bool { automation::update(self, g, grp) }

    fn choose_event(&self, g: &Game, p: PlayerId, t: &EventTriggeredData, options: &[EventType]) -> Option<EventType> {
        events::choose(g, p, t, options)
    }

    fn choose_election(&self, g: &Game, p: PlayerId, vs: &VoteSelectionData) -> Option<usize> {
        council::choose_election(g, p, vs)
    }

    fn diplo_vote(&self, g: &Game, p: PlayerId, vote: &VoteSelectionSubData, source: VoteSourceType) -> PlayerVote {
        council::diplo_vote(g, p, vote, source)
    }

    fn crusade_pledge(&self, g: &Game, p: PlayerId, c: ffh_core::crusade::CrusadeId) -> Option<ffh_core::crusade::Pledge> {
        crusade::pledge(g, p, c)
    }

    fn crusade_draft_pick(&self, g: &Game, p: PlayerId, _c: ffh_core::crusade::CrusadeId, budget: i32, offered: &[(CityId, i32)]) -> Option<CityId> {
        crusade::draft_pick(g, p, budget, offered)
    }

    fn consider_loan(&self, g: &Game, p: PlayerId, proposal: &ffh_core::crusade::loan::LoanProposal) -> ffh_core::crusade::loan::LoanAnswer {
        crusade::consider_loan(g, p, proposal)
    }

    fn choose_loan_units(&self, g: &Game, p: PlayerId, terms: &ffh_core::crusade::loan::LoanTerms) -> Vec<UnitId> {
        crusade::pick_units(g, p, terms.strength as i64)
    }

    fn choose_free_promotion(&self, g: &Game, p: PlayerId, _u: UnitId, options: &[PromotionType]) -> Option<PromotionType> {
        cards_economy::choose_free_promotion(g, p, options)
    }

    fn choose_free_bonus(&self, g: &Game, p: PlayerId, _c: CityId, _b: BuildingType, options: &[BonusType]) -> Option<BonusType> {
        cards_economy::choose_free_bonus(g, p, options)
    }
}

/// The plans telemetry follows as they change (ffh_core::telemetry, docs/TELEMETRY.md): the victory
/// goal, the finances, the war being prepared.
fn telemetry_intents(g: &Game, p: PlayerId) {
    let plan = victory::plan_ours(g, p);
    g.tel_intent(p, "goal", || plan.goal().map_or_else(|| "none".to_string(), |goal| format!("{goal:?}")));
    let f = finance::state(g, p);
    let finance = if f.structural() {
        "structural_deficit"
    } else if f.support_heavy() {
        "support_heavy"
    } else if f.rate < 0 {
        "losing_gold"
    } else {
        "ok"
    };
    g.tel_intent(p, "finance", || finance.to_string());
    let team = g.players[p].team;
    g.tel_intent(p, "war_prep", || diplomacy::preparing_target(g, team).map_or_else(|| "none".to_string(), |t| format!("team {}", t.0)));
    if let Some(pr) = probes::probe(g, p) {
        g.tel_intent(p, "probe", || probes::name(pr).to_string());
    }
}
