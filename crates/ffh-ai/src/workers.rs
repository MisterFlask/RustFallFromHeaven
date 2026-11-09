//! The cities' work crews (docs/WORKS.md; the valuation of the AI's workers, which the crews
//! replaced): improve the plots our cities work, connect resources, and build roads between our
//! cities. [`choose_crew_build`] answers `Controller::choose_crew_build` for the AI's cities and,
//! as the advisor, for a human's.
//!
//! A build's value is the yield change of its improvement weighted by what the working city
//! needs (food while it can grow, production, commerce; improvements that grow, like cottages,
//! count their grown yield in part), plus the value of a resource it connects; plots being
//! worked now count fully, others at a discount. Roads come from the rules' plan
//! (`Game::player_road_plan`): the next missing road plot between two unconnected cities of ours,
//! and plots that would link an improved resource to its city. FfH's AI-only build bans
//! (`CvGameUtils.canBuild`) are respected: no farms for Fallow civs, no workshops
//! before Guilds (unless Fallow), no cottages under Aristocracy, no windmills under Arete.
//!
//! Chops ([`clearing_value`]): a plain feature removal (Remove Forest, Remove Jungle) is worth its
//! hammers only to a city building a settler or a world wonder, as an expert chops to
//! rush expansion, less the feature's yields on a worked plot. Some features are never chopped:
//! the Ancient Forest, a feature a civic of ours makes happy, a healthy feature of a city with no
//! health to spare, and a city's last three forests (lumber mills, health). Improvements on a
//! feature are valued as before, without counting the clearing.

use ffh_core::game::Game;
use ffh_core::ids::*;
use ffh_data::*;


/// FfH `CvGameUtils.canBuild` for AI players: false = the AI must not use this build here.
pub fn ai_build_allowed(g: &Game, p: PlayerId, plot: PlotIdx, build: BuildType) -> bool {
    BanState::new(g, p).rule(g.d, build).allows(g, plot)
}

/// What the bans of [`ai_build_allowed`] read of the player: its civics, traits and techs.
struct BanState {
    fallow: bool,
    arete: bool,
    guilds: bool,
    aristocracy: bool,
}

/// [`ai_build_allowed`] for one build and player, before the plot is known.
#[derive(Copy, Clone)]
enum BanRule {
    Banned,
    Allowed,
    /// Roads: not on water.
    OnLand,
}

impl BanState {
    fn new(g: &Game, p: PlayerId) -> BanState {
        let d = g.d;
        let pl = &g.players[p];
        let civic = |opt: &str, civ: &str| -> bool {
            match (d.ty::<CivicOptionType>(opt), d.ty::<CivicType>(civ)) {
                (Some(o), Some(c)) => pl.civics[o] == Some(c),
                _ => false,
            }
        };
        BanState {
            fallow: d.ty::<TraitType>("TRAIT_FALLOW").is_some_and(|t| pl.traits[t]),
            arete: civic("CIVICOPTION_LABOR", "CIVIC_ARETE"),
            guilds: d.ty::<TechType>("TECH_GUILDS").is_some_and(|t| g.team_is_has_tech(pl.team, Some(t))),
            aristocracy: civic("CIVICOPTION_GOVERNMENT", "CIVIC_ARISTOCRACY"),
        }
    }

    fn rule(&self, d: &GameData, build: BuildType) -> BanRule {
        let ok = match d.builds[build].base.type_.as_str() {
            "BUILD_FORT" => false,
            "BUILD_WINDMILL" => !self.arete,
            "BUILD_WORKSHOP" => self.fallow || self.guilds,
            "BUILD_COTTAGE" => !self.aristocracy,
            "BUILD_FARM" => !self.fallow,
            "BUILD_ROAD" => return BanRule::OnLand,
            _ => true,
        };
        if ok {
            BanRule::Allowed
        } else {
            BanRule::Banned
        }
    }
}

impl BanRule {
    fn allows(self, g: &Game, plot: PlotIdx) -> bool {
        match self {
            BanRule::Banned => false,
            BanRule::Allowed => true,
            BanRule::OnLand => !g.plot_is_water(plot),
        }
    }
}

/// [`ai_build_allowed`] for every build of one player, resolved once (type names, civics,
/// traits, techs) for the many plots of one city's search, which changes nothing they read.
struct BuildBans {
    rules: Vec<BanRule>,
}

impl BuildBans {
    fn new(g: &Game, p: PlayerId) -> BuildBans {
        let state = BanState::new(g, p);
        BuildBans { rules: g.d.builds.keys().map(|b| state.rule(g.d, b)).collect() }
    }

    fn allowed(&self, g: &Game, plot: PlotIdx, build: BuildType) -> bool {
        self.rules[build.idx()].allows(g, plot)
    }
}

/// What [`build_value`] reads that does not depend on the build: the commerce weight (the
/// player's sliders), the food weight of each working city, and the yields of the current
/// improvement of the plot being scored. Filled on first use during one city's search, which
/// changes nothing in the game.
struct ValueCx {
    commerce_w: i32,
    food_w: Vec<(CityId, i32)>,
    old: Option<(PlotIdx, [i32; YieldType::NUM])>,
}

impl ValueCx {
    fn new(g: &Game, p: PlayerId) -> ValueCx {
        // Commerce matters more to an empire that cannot afford its research.
        let research = crate::economy::spend_pct(g, p);
        let commerce_w = if research < 50 { 11 * (200 - 2 * research) / 100 } else { 11 };
        ValueCx { commerce_w, food_w: Vec::new(), old: None }
    }

    /// What working city `c` needs: food while it can grow.
    fn food_weight(&mut self, g: &Game, c: CityId) -> i32 {
        if let Some(&(_, w)) = self.food_w.iter().find(|(o, _)| *o == c) {
            return w;
        }
        let surplus = g.city_yield_rate(c, YieldType::YIELD_FOOD) - g.city_food_consumption(c, false, 0);
        let happy_room = g.city_happy_level(c) - g.city_unhappy_level(c, 0);
        let w = if surplus < 2 && happy_room > 0 { 30 } else if happy_room > 0 { 22 } else { 12 };
        self.food_w.push((c, w));
        w
    }

    /// The yields player `p` gets from the improvement on `plot` now (0 without one).
    fn old_yields(&mut self, g: &Game, p: PlayerId, plot: PlotIdx) -> [i32; YieldType::NUM] {
        if let Some((q, old)) = self.old {
            if q == plot {
                return old;
            }
        }
        let mut old = [0; YieldType::NUM];
        if let Some(o) = g.map.plots[plot].improvement_type {
            for (y, slot) in old.iter_mut().enumerate() {
                *slot = g.plot_calculate_improvement_yield_change(plot, o, YieldType::from_usize(y), Some(p), false, false);
            }
        }
        self.old = Some((plot, old));
        old
    }
}

/// Forests a city keeps around it whatever a chop would give (lumber mills, health).
const KEEP_FORESTS: i32 = 3;
/// A chop's hammers count as this many turns of production on the plot when they rush a
/// settler or a world wonder.
const CHOP_TURNS: i32 = 10;

/// The feature `build` clears from `plot` for `p`: none when the build keeps it, or the
/// civilization maintains it (the elves' forests stay under their improvements).
fn cleared_feature(g: &Game, p: PlayerId, plot: PlotIdx, build: BuildType) -> Option<FeatureType> {
    let f = g.map.plots[plot].feature_type?;
    if !g.d.builds[build].feature_remove[f] {
        return None;
    }
    let civ = g.player_get_civilization_type(p)?;
    (!g.d.civilizations[civ].maintain_features[f]).then_some(f)
}

/// What clearing feature `f` from `plot` with `build` is worth (see the module docs); `None`
/// when the feature is kept. A plain chop (no improvement) is worth its hammers only to a city
/// rushing a settler or a world wonder.
fn clearing_value(g: &Game, p: PlayerId, plot: PlotIdx, build: BuildType, f: FeatureType, cx: &mut ValueCx) -> Option<i32> {
    let d = g.d;
    if d.ty::<FeatureType>("FEATURE_FOREST_ANCIENT") == Some(f) {
        return None;
    }
    let pl = &g.players[p];
    if d.civic_options.keys().filter_map(|o| pl.civics[o]).any(|c| d.civics[c].feature_happiness_changes[f] > 0) {
        return None;
    }
    let city = g.map.plots[plot].working_city.filter(|&c| g.cities[c].owner == p)?;
    if d.features[f].health_percent > 0 && g.city_good_health(city) - g.city_bad_health(city, false, 0) <= 1 {
        return None;
    }
    let same = (0..g.city_num_city_plots(city).max(0) as usize)
        .filter_map(|i| g.city_index_plot(city, i))
        .filter(|&q| g.map.plots[q].feature_type == Some(f) && g.map.plots[q].owner == Some(p))
        .count() as i32;
    if same <= KEEP_FORESTS && d.features[f].health_percent >= 0 && d.ty::<FeatureType>("FEATURE_FOREST") == Some(f) {
        return None;
    }
    // The feature's own yields, lost on a worked plot.
    let team = g.players[p].team;
    let mut value = 0;
    if g.plot_is_being_worked(plot) {
        let weights = [cx.food_weight(g, city), 18, cx.commerce_w];
        for (y, w) in weights.iter().enumerate() {
            let yt = YieldType::from_usize(y);
            let lost = g.plot_calculate_nature_yield(plot, yt, Some(team), false) - g.plot_calculate_nature_yield(plot, yt, Some(team), true);
            value -= lost * w;
        }
    }
    // The hammers, to the city that receives them.
    let (hammers, to) = g.plot_get_feature_production(plot, build, team);
    if let Some(c) = to.filter(|&c| g.cities[c].owner == p && !g.city_is_production_process(c)) {
        let wonder = g.city_production_building(c).is_some_and(|b| d.buildings[b].building_class_type.is_some_and(|k| ffh_core::utils::is_world_wonder_class(d, k)));
        let rush = g.city_is_food_production(c) || wonder;
        value += if rush { hammers * 18 / CHOP_TURNS } else { 0 };
    }
    Some(value)
}

/// Value of performing `build` on `plot` for player `p` (0 = not worth it).
fn build_value(g: &Game, roads: &[(PlotIdx, i32)], p: PlayerId, plot: PlotIdx, build: BuildType, cx: &mut ValueCx) -> i32 {
    let team = g.players[p].team;
    let d = g.d;
    let info = &d.builds[build];
    let pl = &g.map.plots[plot];
    if info.improvement.is_none() && info.route.is_none() {
        // A plain chop (Remove Forest): worth its hammers to a city rushing a settler or a world
        // wonder, unless the feature is kept.
        return cleared_feature(g, p, plot, build).and_then(|f| clearing_value(g, p, plot, build, f, cx)).map_or(0, |v| v.max(0));
    }
    if let Some(imp) = info.improvement {
        if pl.improvement_type == Some(imp) {
            return 0;
        }
        // The great-person cards' improvements (docs/CARDS.md).
        if let Some(value) = crate::cards_map_rules::build_value(g, p, plot, imp) {
            return value;
        }
        // A card's battle improvement (the Trench): where our units hold.
        if let Some(v) = crate::cards_battle_arms::build_value(g, p, plot, build) {
            return v;
        }
        // What the working city needs.
        let city = pl.working_city.filter(|&c| g.cities[c].owner == p);
        let (food_w, prod_w) = match city {
            Some(c) => (cx.food_weight(g, c), 18),
            None => (20, 18),
        };
        // Improvements that grow (cottages) count their grown form for two thirds.
        let mut last = imp;
        for _ in 0..4 {
            match d.improvements[last].improvement_upgrade {
                Some(next) if next != imp => last = next,
                _ => break,
            }
        }
        let weights = [food_w, prod_w, cx.commerce_w];
        let old_yields = cx.old_yields(g, p, plot);
        let mut value = 0;
        let mut new_total = 0;
        for y in 0..YieldType::NUM {
            let yt = YieldType::from_usize(y);
            let now = g.plot_calculate_improvement_yield_change(plot, imp, yt, Some(p), false, false);
            let mut new = now;
            let mut grown = None;
            if last != imp {
                let gy = g.plot_calculate_improvement_yield_change(plot, last, yt, Some(p), false, false);
                grown = Some(gy);
                new = (new + 2 * gy) / 3;
            }
            let old = old_yields[y];
            value += (new - old) * weights[y];
            value += crate::traits::threshold_value(g, p, plot, y, old, now, grown, weights[y]); // trait-play
            new_total += pl.yields[y] - old + new;
        }
        // A plot nobody would work (total yield 2 or less) is not worth improving.
        if new_total <= 2 && pl.bonus_type.is_none() {
            return 0;
        }
        if let Some(b) = pl.bonus_type {
            let revealed = d.bonuses[b].tech_reveal.map_or(true, |t| g.team_is_has_tech(team, Some(t)));
            let traded_now = pl.improvement_type.is_some_and(|o| d.improvements[o].improvement_bonus[b].bonus_trade);
            if revealed && d.improvements[imp].improvement_bonus[b].bonus_trade && !traded_now {
                value += 100 + g.player_ai_bonus_val(p, b, 1).min(400) / 2;
            } else if revealed && traded_now {
                // Never replace the improvement that connects a resource with one that does not.
                return 0;
            }
        }
        if value <= 0 {
            return 0;
        }
        let worked = g.plot_is_being_worked(plot);
        if !worked {
            value = value * 6 / 10;
        }
        if pl.improvement_type.is_some() && value < 60 {
            return 0; // do not churn improvements
        }
        value
    } else if info.route.is_some() {
        if pl.route_type.is_some() {
            return 0;
        }
        roads.iter().find(|(q, _)| *q == plot).map_or(0, |&(_, val)| val)
    } else {
        0
    }
}

/// What city `c`'s crews work on next (`Controller::choose_crew_build`): the build worth the
/// most for the turns it takes, among the plots the crews may work (`Game::city_crew_plots`).
pub fn choose_crew_build(g: &Game, c: CityId) -> Option<(PlotIdx, BuildType)> {
    let d = g.d;
    let p = g.cities[c].owner;
    let roads = g.player_road_plan(p);
    // Builds with an improvement, a route, or hammers from a cleared feature (a chop).
    let builds: Vec<BuildType> = d
        .builds
        .keys()
        .filter(|&b| {
            let info = &d.builds[b];
            info.city && (info.improvement.is_some() || info.route.is_some() || info.feature_production.iter().any(|&h| h > 0))
        })
        .collect();
    let bans = BuildBans::new(g, p);
    let mut cx = ValueCx::new(g, p);
    let rate = g.city_crew_rate(c).max(1);
    let mut best: Option<(PlotIdx, BuildType, i64)> = None;
    for q in g.city_crew_plots(c, &roads) {
        let pl = &g.map.plots[q];
        let in_radius = pl.working_city.is_some_and(|o| g.cities[o].owner == p);
        let road = roads.iter().any(|(rq, _)| *rq == q);
        for &b in &builds {
            // `build_value`'s first zero cases, checked before the costlier tests (all of them
            // pure, so the order does not matter): the improvement is already there; a route
            // where there is one, or that the road plan does not ask for; a chop outside our
            // cities' plots or where there is nothing to clear.
            let info = &d.builds[b];
            match (info.improvement, info.route) {
                (Some(imp), _) => {
                    if pl.improvement_type == Some(imp) {
                        continue;
                    }
                }
                (None, Some(_)) => {
                    if pl.route_type.is_some() || !road {
                        continue;
                    }
                }
                (None, None) => {
                    if !in_radius || cleared_feature(g, p, q, b).is_none() {
                        continue;
                    }
                }
            }
            if !bans.allowed(g, q, b) || !g.city_crews_can_build(c, q, b) {
                continue;
            }
            let value = build_value(g, &roads, p, q, b, &mut cx);
            if value <= 0 {
                continue;
            }
            // The turns left on this plot (a feature to clear first takes longer).
            let time = (g.plot_get_build_time(q, b) - pl.build_progress[b]).max(1) / rate;
            let score = value as i64 * 100 / (time as i64 + 3);
            if best.map_or(true, |(_, _, s)| score > s) {
                best = Some((q, b, score));
            }
        }
    }
    if let Some((q, b, score)) = best {
        crate::trace!(g, p, "crews of {:?}: {} at {:?} (score {score})", c, d.builds[b].base.type_, g.map.xy(q));
    }
    best.map(|(q, b, _)| (q, b))
}
