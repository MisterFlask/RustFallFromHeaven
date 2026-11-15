//! Mana spheres belong to the resource. A node improvement connects its matching resource;
//! building, pillaging or dispelling it never changes the sphere.

use crate::game::Game;
use ffh_data::*;

/// The standard mana resources and the improvements that connect them, in build order.
/// Ice remains special to Letum Frigus: there is no ordinary Ice node build.
pub fn mana_nodes(d: &GameData) -> Vec<(BonusType, ImprovementType)> {
    let mut out = Vec::new();
    for build in d.builds.iter() {
        let Some(imp) = build.improvement else { continue };
        for bonus in d.bonuses.keys() {
            let harvest = &d.improvements[imp].improvement_bonus[bonus];
            if is_mana(d, bonus) && harvest.bonus_makes_valid && harvest.bonus_trade && !out.iter().any(|&(b, _)| b == bonus) {
                out.push((bonus, imp));
            }
        }
    }
    out
}

pub fn is_mana(d: &GameData, bonus: BonusType) -> bool {
    d.bonuses[bonus].bonus_class_type.is_some_and(|c| c.i() == d.defines.bonusclass_mana)
}

impl Game {
    /// Random standard sphere for a node discovered during play (lairs, Rites of Oghma).
    pub fn random_mana_bonus(&mut self) -> BonusType {
        let nodes = mana_nodes(self.d);
        nodes[self.soren_rand_num(nodes.len() as i32, "Mana sphere") as usize].0
    }

    /// Assign spheres after resource placement and start normalization, including far shores.
    /// Raw mana is only a placement template: keeping it until this pass preserves the total
    /// number and spacing of nodes. Only the map RNG is used, in plot order.
    pub fn mapgen_type_mana_nodes(&mut self) {
        let raw = self.d.t::<BonusType>("BONUS_MANA");
        let nodes = mana_nodes(self.d);
        for q in self.map.plot_ids() {
            if self.map.plots[q].bonus_type == Some(raw) {
                let bonus = nodes[self.map_rand_num(nodes.len() as i32, "Mana sphere") as usize].0;
                self.plot_set_bonus_type(q, Some(bonus));
            }
        }
    }
}
