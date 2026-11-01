//! The `crusades` query (docs/CRUSADE.md): the running crusades and loan generals, the human's
//! part in each (its call, what it delivered and earned, whether it may lend), the units it
//! could lend, and its past crusades. Crusades are public (a council's call, a general's war);
//! the Host's units are counted, and located only where the human sees them (`host_at`: its
//! largest stack in view, for the map to show).

use ffh_core::crusade::{CrusadeKind, Objective, Pledge};

use crate::json::Json;
use crate::view::Viewer;

fn kind_json(k: CrusadeKind) -> Json {
    match k {
        CrusadeKind::Council(vs) => crate::jobj! { "kind" => "council", "source" => Json::idx(vs) },
        CrusadeKind::Loan { borrower } => crate::jobj! { "kind" => "loan", "borrower" => borrower },
        CrusadeKind::Direct => crate::jobj! { "kind" => "direct" },
    }
}

/// `{enabled, wars, rules, military_strength, lendable, crusades, reports}` for the human.
pub fn crusades(v: &Viewer) -> Json {
    let g = v.g;
    let h = v.h;
    let rules = g.crusade_rules();
    let lendable: Vec<Json> = g.players[h]
        .units
        .iter()
        .copied()
        .filter(|&u| g.units.contains(u) && g.crusade_unit_lendable(u))
        .map(|u| {
            let unit = &g.units[u];
            crate::jobj! {
                "unit" => u,
                "type" => Json::idx(unit.unit_type),
                "strength" => g.crusade_unit_strength(u),
                "x" => unit.x,
                "y" => unit.y,
            }
        })
        .collect();
    let crusades: Vec<Json> = g
        .crusades
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let objective = match &c.objective {
                Objective::Term => crate::jobj! { "kind" => "term" },
                Objective::Capture(plots) => crate::jobj! {
                    "kind" => "capture",
                    "plots" => Json::Arr(plots.iter().map(|&q| { let (x, y) = g.map.xy(q); crate::jarr![x, y] }).collect()),
                },
                Objective::Recover => crate::jobj! { "kind" => "recover" },
            };
            let call = c.calls.iter().find(|x| x.player == h).map(|x| {
                crate::jobj! {
                    "obligation" => x.obligation,
                    "answer" => match x.answer {
                        None => Json::Null,
                        Some(Pledge::Refuse) => Json::from("refuse"),
                        Some(Pledge::Strength(s)) => Json::from(s),
                    },
                    "settled" => x.settled,
                }
            });
            let mine = c.lenders.iter().find(|l| l.player == h);
            let can_lend = match g.crusade_can_lend(c.id, h) {
                Ok(()) => Json::Bool(true),
                Err(e) => Json::from(format!("{e:?}")),
            };
            let host_units = g.players[c.host].units.iter().filter(|&&u| g.units.contains(u) && !g.units[u].death_delay).count();
            // The Host's largest stack in view, and the target's city nearest the human's
            // capital among those it has revealed (where the map centres for "Show").
            let mut stacks: Vec<(ffh_core::ids::PlotIdx, usize)> = Vec::new();
            for &u in &g.players[c.host].units {
                if g.units.contains(u) && !g.units[u].death_delay && v.unit_visible(u) {
                    let q = g.unit_plot(u);
                    match stacks.iter_mut().find(|s| s.0 == q) {
                        Some(s) => s.1 += 1,
                        None => stacks.push((q, 1)),
                    }
                }
            }
            let host_at = stacks.iter().max_by_key(|&&(q, n)| (n, std::cmp::Reverse(q))).map(|&(q, _)| {
                let (x, y) = g.map.xy(q);
                crate::jarr![x, y]
            });
            let home = g.players[h].capital_city.filter(|&cc| g.cities.contains(cc)).map(|cc| (g.cities[cc].x, g.cities[cc].y));
            let target_at = g
                .player_ids()
                .filter(|&q| g.players[q].alive && g.players[q].team == c.target)
                .flat_map(|q| g.players[q].cities.iter().copied())
                .filter(|&cc| v.reveal || g.map.plots[g.city_plot(cc)].revealed[v.team])
                .min_by_key(|&cc| (home.map_or(0, |(hx, hy)| g.plot_distance(hx, hy, g.cities[cc].x, g.cities[cc].y)), cc))
                .map(|cc| crate::jarr![g.cities[cc].x, g.cities[cc].y]);
            crate::jobj! {
                "id" => c.id.0,
                "kind" => kind_json(c.kind),
                "host" => c.host,
                "host_name" => crate::view::player_name(g, c.host),
                "target" => c.target,
                "victim" => c.victim,
                "defend" => c.defend,
                "objective" => objective,
                "start" => c.start_turn,
                "end" => c.end_turn,
                "deadline" => c.deadline_turn,
                "dissolving" => c.dissolving,
                "state" => format!("{:?}", c.ai.state).to_lowercase(),
                "call" => call,
                "delivered" => mine.map_or(0, |l| l.delivered),
                "serving" => g.crusade_lent_units(idx, Some(h)).len(),
                "credit" => mine.map_or(0, |l| l.credit),
                "term_end" => mine.map(|l| l.term_end),
                "at_war_with_target" => g.at_war(v.team, c.target),
                "can_lend" => can_lend,
                "host_units" => host_units,
                "host_at" => host_at,
                "target_at" => target_at,
                "lenders" => Json::Arr(c.lenders.iter().map(|l| crate::jobj! { "player" => l.player, "delivered" => l.delivered, "credit" => l.credit }).collect()),
            }
        })
        .collect();
    let reports: Vec<Json> = g
        .crusade_reports
        .iter()
        .filter_map(|r| {
            let me = r.lenders.iter().find(|l| l.player == h)?;
            Some(crate::jobj! {
                "id" => r.id.0,
                "kind" => kind_json(r.kind),
                "target" => r.target,
                "start" => r.start_turn,
                "end" => r.end_turn,
                "reason" => format!("{:?}", r.reason).to_lowercase(),
                "pool" => r.pool,
                "called" => me.called,
                "refused" => me.refused,
                "delivered" => me.delivered,
                "credit" => me.credit,
                "spoils" => me.spoils,
                "cities" => me.cities,
            })
        })
        .collect();
    // Who fights whom among the teams the human has met (as the foreign advisor shows it), for
    // the loans it may propose.
    let met: Vec<ffh_core::ids::TeamId> = g.civ_team_ids().filter(|&t| g.team_is_alive(t) && (t == v.team || g.teams[v.team].has_met[t] || v.reveal)).collect();
    let mut wars = Vec::new();
    for (i, &a) in met.iter().enumerate() {
        for &b in &met[i + 1..] {
            if g.at_war(a, b) && !g.team_is_crusade_host(a) && !g.team_is_crusade_host(b) {
                wars.push(crate::jarr![a, b]);
            }
        }
    }
    crate::jobj! {
        "enabled" => g.crusades_enabled(),
        "wars" => Json::Arr(wars),
        "rules" => crate::jobj! {
            "loan_max_turns" => ffh_core::crusade::speed_turns(g, rules.loan_max_turns),
            "obligation_percent" => rules.obligation_percent,
        },
        "military_strength" => g.crusade_military_strength(h),
        "lendable" => Json::Arr(lendable),
        "crusades" => Json::Arr(crusades),
        "reports" => Json::Arr(reports),
    }
}
