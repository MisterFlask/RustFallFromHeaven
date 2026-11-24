//! The AI's squads (docs/ENGAGEMENTS.md, "Squads"): every squad of an AI player stays Auto, and
//! each body (an army, a garrison, the loose units on a plot) of two or more squads is kept
//! massed (`Game::squad_mass_sort_body`): its best units together in the first squad (the auto
//! formation of them all), the next best in the second, and so on, in as few squads as hold them.
//! One battle is one squad against one squad, and a battle won in the field pushes the whole
//! enemy stack back, so the strongest squad fighting first is what an expert would field. Done at
//! the start of the player's turn (for its attacks) and at its end (for the battles it defends
//! before its next turn).

use ffh_core::game::Game;
use ffh_core::ids::PlayerId;

/// Masses player `p`'s bodies whose squads are not massed (nothing to do: no sync).
pub fn turn(g: &mut Game, p: PlayerId) {
    for (plot, body) in g.squad_bodies_to_mass(p) {
        let _ = g.squad_mass_sort_body(p, plot, body);
    }
}

#[cfg(test)]
mod tests {
    use ffh_core::army::SquadBody;
    use ffh_core::engagement::{HitCtx, Side};
    use ffh_core::game::Game;
    use ffh_core::ids::*;
    use ffh_core::setup::GameSetup;
    use ffh_data::*;

    /// Eight axemen of one army, made one at a time (squads of one, then the snake deal of two
    /// squads of four): `turn` masses them, the auto formation's five in the first squad.
    #[test]
    fn turn_masses_a_body_of_eight() {
        let d = ffh_data::global();
        let mut g = Game::new(d, GameSetup::standard(d, 2, "WORLDSIZE_DUEL", 5));
        g.game_init();
        let p = PlayerId(0);
        let at = g
            .map
            .plot_ids()
            .find(|&q| !g.plot_is_water(q) && !g.plot_is_peak(q) && g.map.plots[q].units.is_empty() && g.map.plots[q].owner.is_none())
            .expect("free land");
        let (x, y) = g.map.xy(at);
        let ty = d.t::<UnitType>("UNIT_AXEMAN");
        let units: Vec<UnitId> = (0..8)
            .map(|i| {
                let u = g.player_init_unit(p, ty, x, y, None, None);
                g.unit_change_extra_combat_percent(u, 10 * i);
                g.player_army_sync(p);
                u
            })
            .collect();
        g.army_form(p, &units).expect("an army");
        let army = SquadBody::Army(g.units[units[0]].army.expect("in the army"));
        g.squad_auto_sort_body(p, at, army).expect("sorted");
        assert_eq!(g.squad_layouts_at(p, at).len(), 2);
        assert_eq!(g.squad_bodies_to_mass(p), vec![(at, army)]);
        super::turn(&mut g, p);
        let layouts = g.squad_layouts_at(p, at);
        assert_eq!(layouts.len(), 2);
        let mut first: Vec<UnitId> = layouts[0].units().into_iter().map(|(u, _)| u).collect();
        first.sort();
        let mut best: Vec<UnitId> = g.engage_auto_formation(&units, Side::Attacker, &g.engage_squad_slots(), &HitCtx::at(at)).iter().map(|q| q.unit).collect();
        best.sort();
        assert_eq!(first, best);
        assert!(g.squad_bodies_to_mass(p).is_empty());
    }
}
