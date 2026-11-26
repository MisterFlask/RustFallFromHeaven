//! Re-exports the pure 3x3 formation battle resolver from `ffh-battle` and adapts `ffh-core`'s
//! RNG and `ffh-data`'s `Defines` onto it.

pub use ffh_battle::*;

use ffh_data::Defines;

impl BattleRng for crate::rng::Rng {
    fn rand(&mut self, n: i32, label: &'static str) -> i32 {
        self.get_i(n, label)
    }
}

/// Builds formation battle `Params` from FFH2's `ENGAGEMENT_*` defines.
pub fn params_from_defines(d: &Defines) -> Params {
    Params {
        base_damage: d.engagement_base_damage,
        rand_min: d.engagement_damage_rand_min,
        rand_max: d.engagement_damage_rand_max.max(d.engagement_damage_rand_min),
        rounds: d.engagement_rounds,
        first_strike_speed: d.engagement_first_strike_speed,
        shoot_miss_pct: d.engagement_shoot_miss_percent,
        bombard_miss_pct: d.engagement_bombard_miss_percent,
        crit_damage_pct: d.engagement_crit_damage_percent,
        charge_bonus: d.engagement_charge_bonus,
        splash_pct: d.engagement_splash_percent,
        bolt_pct: d.engagement_bolt_percent,
        heal_pct: d.engagement_heal_percent,
        volley_pct: d.engagement_volley_percent,
        pierce_pct: d.engagement_pierce_percent,
        sweep_pct: d.engagement_sweep_percent,
        cleave_pct: d.engagement_cleave_percent,
        trample_pct: d.engagement_trample_percent,
        missile_pct: d.engagement_missile_percent,
        city_attacker_shot_pct: d.engagement_city_attacker_shot_percent,
        forest_ranged_pct: d.engagement_forest_ranged_percent,
        hills_attacker_pct: d.engagement_hills_attacker_percent,
        hills_defender_ranged_pct: d.engagement_hills_defender_ranged_percent,
        forest_stab_pct: d.engagement_forest_stab_percent,
        open_large_pct: d.engagement_open_large_percent,
        rough_large_pct: d.engagement_rough_large_percent,
        wading_pct: d.engagement_wading_percent,
    }
}
