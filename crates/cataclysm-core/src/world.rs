//! Square-grid world map, procedural generation, fog-of-war, Hell corruption, and Carcosa emergence.
//!
//! Uses a square grid so 100% of the existing HD-2D renderer, isometric fallback, terrain blending,
//! raised riverbanks, and town kits work natively with zero new map art.

use crate::ids::{CityId, PlayerId, PlotIdx};
use crate::rng::Rng;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlotKind {
    Ocean,
    Coast,
    Land,
    Hills,
    Peak,
}

impl PlotKind {
    pub const fn is_water(self) -> bool {
        matches!(self, PlotKind::Ocean | PlotKind::Coast)
    }

    pub const fn is_passable_land(self) -> bool {
        matches!(self, PlotKind::Land | PlotKind::Hills)
    }

    /// Index matching `ffh-data` `PlotType` (`PLOT_PEAK=0, PLOT_HILLS=1, PLOT_LAND=2, PLOT_OCEAN=3`).
    pub const fn art_plot_type(self) -> i32 {
        match self {
            PlotKind::Peak => 0,
            PlotKind::Hills => 1,
            PlotKind::Land => 2,
            PlotKind::Coast | PlotKind::Ocean => 3,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Terrain {
    Grass,
    Plains,
    Desert,
    Tundra,
    Snow,
    Marsh,
    Coast,
    Ocean,
    BurningSands,
    BrokenLands,
    FieldsOfPerdition,
}

impl Terrain {
    pub const fn art_key(self) -> &'static str {
        match self {
            Terrain::Grass => "TERRAIN_GRASS",
            Terrain::Plains => "TERRAIN_PLAINS",
            Terrain::Desert => "TERRAIN_DESERT",
            Terrain::Tundra => "TERRAIN_TUNDRA",
            Terrain::Snow => "TERRAIN_SNOW",
            Terrain::Marsh => "TERRAIN_MARSH",
            Terrain::Coast => "TERRAIN_COAST",
            Terrain::Ocean => "TERRAIN_OCEAN",
            Terrain::BurningSands => "TERRAIN_BURNING_SANDS",
            Terrain::BrokenLands => "TERRAIN_BROKEN_LANDS",
            Terrain::FieldsOfPerdition => "TERRAIN_FIELDS_OF_PERDITION",
        }
    }

    pub const fn is_hell(self) -> bool {
        matches!(self, Terrain::BurningSands | Terrain::BrokenLands | Terrain::FieldsOfPerdition)
    }

    pub const fn hell_version(self) -> Terrain {
        match self {
            Terrain::Desert => Terrain::BurningSands,
            Terrain::Plains | Terrain::Tundra | Terrain::Snow => Terrain::FieldsOfPerdition,
            Terrain::Grass | Terrain::Marsh => Terrain::BrokenLands,
            other => other,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    None,
    Forest,
    AncientForest,
    Jungle,
    Oasis,
    FloodPlains,
    BurntForest,
    Flames,
}

impl Feature {
    pub const fn art_key(self) -> Option<&'static str> {
        match self {
            Feature::None => None,
            Feature::Forest => Some("FEATURE_FOREST"),
            Feature::AncientForest => Some("FEATURE_FOREST_ANCIENT"),
            Feature::Jungle => Some("FEATURE_JUNGLE"),
            Feature::Oasis => Some("FEATURE_OASIS"),
            Feature::FloodPlains => Some("FEATURE_FLOOD_PLAINS"),
            Feature::BurntForest => Some("FEATURE_FOREST_BURNT"),
            Feature::Flames => Some("FEATURE_FLAMES"),
        }
    }

    pub const fn is_woods(self) -> bool {
        matches!(self, Feature::Forest | Feature::AncientForest | Feature::Jungle)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Resource {
    None,
    Wheat,
    Cow,
    Horse,
    Iron,
    Mithril,
    GoldOre,
    Gems,
    Wine,
    Fish,
    ManaRaw,
    ManaFire,
    ManaLife,
    ManaDeath,
    ManaNature,
    ManaMind,
}

impl Resource {
    pub const fn art_key(self) -> Option<&'static str> {
        match self {
            Resource::None => None,
            Resource::Wheat => Some("BONUS_WHEAT"),
            Resource::Cow => Some("BONUS_COW"),
            Resource::Horse => Some("BONUS_HORSE"),
            Resource::Iron => Some("BONUS_IRON"),
            Resource::Mithril => Some("BONUS_MITHRIL"),
            Resource::GoldOre => Some("BONUS_GOLD"),
            Resource::Gems => Some("BONUS_GEMS"),
            Resource::Wine => Some("BONUS_WINE"),
            Resource::Fish => Some("BONUS_FISH"),
            Resource::ManaRaw => Some("BONUS_MANA"),
            Resource::ManaFire => Some("BONUS_MANA_FIRE"),
            Resource::ManaLife => Some("BONUS_MANA_LIFE"),
            Resource::ManaDeath => Some("BONUS_MANA_DEATH"),
            Resource::ManaNature => Some("BONUS_MANA_NATURE"),
            Resource::ManaMind => Some("BONUS_MANA_MIND"),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Resource::None => "None",
            Resource::Wheat => "Wheat",
            Resource::Cow => "Cattle",
            Resource::Horse => "Horses",
            Resource::Iron => "Iron",
            Resource::Mithril => "Mithril",
            Resource::GoldOre => "Gold Vein",
            Resource::Gems => "Gems",
            Resource::Wine => "Wine",
            Resource::Fish => "Fish",
            Resource::ManaRaw => "Raw Mana",
            Resource::ManaFire => "Fire Mana",
            Resource::ManaLife => "Life Mana",
            Resource::ManaDeath => "Death Mana",
            Resource::ManaNature => "Nature Mana",
            Resource::ManaMind => "Mind Mana",
        }
    }

    pub const fn is_mana(self) -> bool {
        matches!(
            self,
            Resource::ManaRaw | Resource::ManaFire | Resource::ManaLife | Resource::ManaDeath | Resource::ManaNature | Resource::ManaMind
        )
    }

    pub const fn is_tradeable(self) -> bool {
        matches!(
            self,
            Resource::Horse | Resource::Iron | Resource::Mithril | Resource::GoldOre | Resource::Gems | Resource::Wine
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Improvement {
    None,
    Farm,
    Mine,
    Pasture,
    Cottage,
    Lumbermill,
    FishingBoats,
    Fort,
    ManaNode,
    RuinsLair,
}

impl Improvement {
    pub const fn art_key(self) -> Option<&'static str> {
        match self {
            Improvement::None => None,
            Improvement::Farm => Some("IMPROVEMENT_FARM"),
            Improvement::Mine => Some("IMPROVEMENT_MINE"),
            Improvement::Pasture => Some("IMPROVEMENT_PASTURE"),
            Improvement::Cottage => Some("IMPROVEMENT_COTTAGE"),
            Improvement::Lumbermill => Some("IMPROVEMENT_LUMBERMILL"),
            Improvement::FishingBoats => Some("IMPROVEMENT_FISHING_BOATS"),
            Improvement::Fort => Some("IMPROVEMENT_FORT"),
            Improvement::ManaNode => Some("IMPROVEMENT_MANA_FIRE"),
            Improvement::RuinsLair => Some("IMPROVEMENT_BARROW"),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Improvement::None => "None",
            Improvement::Farm => "Farm",
            Improvement::Mine => "Mine",
            Improvement::Pasture => "Pasture",
            Improvement::Cottage => "Cottage",
            Improvement::Lumbermill => "Lumbermill",
            Improvement::FishingBoats => "Fishing Boats",
            Improvement::Fort => "Fort",
            Improvement::ManaNode => "Mana Node",
            Improvement::RuinsLair => "Ancient Lair",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Visibility {
    Unrevealed = 0,
    Fogged = 1,
    Visible = 2,
}

#[derive(Clone, Debug)]
pub struct Plot {
    pub x: i32,
    pub y: i32,
    pub kind: PlotKind,
    pub terrain: Terrain,
    pub base_terrain: Terrain,
    pub feature: Feature,
    pub resource: Resource,
    pub improvement: Improvement,
    pub road: bool,
    /// River bitmask matching `ffh-play`: bit 0 = south edge, bit 1 = east edge.
    pub river_bits: i32,
    pub owner: Option<PlayerId>,
    pub city: Option<CityId>,
    /// True while a lair has not yet been explored.
    pub lair_unexplored: bool,
    /// Culture pressure per player index (for borders & House sympathy).
    pub culture: Vec<i32>,
}

impl Plot {
    pub fn yields(&self) -> (i32, i32, i32) {
        if self.kind.is_water() {
            let mut f = if self.kind == PlotKind::Coast { 1 } else { 0 };
            let g = if self.kind == PlotKind::Coast { 1 } else { 0 };
            if self.resource == Resource::Fish {
                f += 2;
            }
            if self.improvement == Improvement::FishingBoats {
                f += 1;
            }
            return (f, 0, g);
        }
        if self.kind == PlotKind::Peak {
            return (0, 0, 0);
        }
        let (mut food, mut ind, mut gold) = match self.terrain {
            Terrain::Grass => (2, 0, 0),
            Terrain::Plains => (1, 1, 0),
            Terrain::Desert | Terrain::BurningSands => (0, 0, 1),
            Terrain::Tundra => (1, 0, 0),
            Terrain::Snow => (0, 0, 0),
            Terrain::Marsh => (1, 0, 0),
            Terrain::BrokenLands | Terrain::FieldsOfPerdition => (0, 1, 1),
            Terrain::Coast | Terrain::Ocean => (0, 0, 0),
        };
        if self.kind == PlotKind::Hills {
            ind += 1;
        }
        match self.feature {
            Feature::Forest | Feature::AncientForest => ind += 1,
            Feature::FloodPlains => food += 2,
            Feature::Oasis => {
                food += 2;
                gold += 2;
            }
            _ => {}
        }
        match self.resource {
            Resource::Wheat => food += 2,
            Resource::Cow => {
                food += 1;
                ind += 1;
            }
            Resource::Horse => {
                ind += 1;
                gold += 1;
            }
            Resource::Iron => ind += 2,
            Resource::Mithril => {
                ind += 3;
                gold += 1;
            }
            Resource::GoldOre | Resource::Gems => gold += 3,
            Resource::Wine => {
                food += 1;
                gold += 2;
            }
            _ => {}
        }
        match self.improvement {
            Improvement::Farm => food += 2,
            Improvement::Mine => ind += 2,
            Improvement::Pasture => {
                food += 1;
                ind += 1;
            }
            Improvement::Cottage => gold += 2,
            Improvement::Lumbermill => ind += 2,
            _ => {}
        }
        if self.river_bits != 0 {
            gold += 1;
        }
        (food, ind, gold)
    }

    pub fn move_cost(&self, forest_free: bool) -> i32 {
        if self.kind == PlotKind::Peak || self.kind.is_water() {
            return 99;
        }
        if self.road {
            return 1;
        }
        let mut cost = 1;
        if self.kind == PlotKind::Hills {
            cost += 1;
        }
        if self.feature.is_woods() && !forest_free {
            cost += 1;
        }
        if self.terrain == Terrain::Marsh {
            cost += 1;
        }
        cost
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub w: i32,
    pub h: i32,
    pub wrap_x: bool,
    pub plots: Vec<Plot>,
    /// Per-player visibility grid: `vis[player][plot]`.
    pub vis: Vec<Vec<Visibility>>,
    /// Reserved water tile where Carcosa rises at AC 100.
    pub carcosa_plot: PlotIdx,
    /// Land bridge plots created when Carcosa rises (passable by land and ships).
    pub carcosa_bridges: Vec<PlotIdx>,
    /// Incremented whenever map tiles visually change.
    pub rev: i64,
}

impl World {
    pub fn len(&self) -> usize {
        self.plots.len()
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> Option<PlotIdx> {
        if y < 0 || y >= self.h {
            return None;
        }
        let wx = if self.wrap_x {
            x.rem_euclid(self.w)
        } else if x < 0 || x >= self.w {
            return None;
        } else {
            x
        };
        Some(PlotIdx((y * self.w + wx) as u32))
    }

    #[inline]
    pub fn plot(&self, p: PlotIdx) -> &Plot {
        &self.plots[p.idx()]
    }

    #[inline]
    pub fn plot_mut(&mut self, p: PlotIdx) -> &mut Plot {
        &mut self.plots[p.idx()]
    }

    pub fn distance(&self, a: PlotIdx, b: PlotIdx) -> i32 {
        let pa = self.plot(a);
        let pb = self.plot(b);
        let mut dx = (pa.x - pb.x).abs();
        if self.wrap_x {
            dx = dx.min(self.w - dx);
        }
        let dy = (pa.y - pb.y).abs();
        dx.max(dy)
    }

    pub fn neighbors(&self, p: PlotIdx) -> Vec<PlotIdx> {
        let pl = self.plot(p);
        let mut out = Vec::with_capacity(8);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if let Some(n) = self.idx(pl.x + dx, pl.y + dy) {
                    out.push(n);
                }
            }
        }
        out
    }

    pub fn cardinal_neighbors(&self, p: PlotIdx) -> Vec<PlotIdx> {
        let pl = self.plot(p);
        let mut out = Vec::with_capacity(4);
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            if let Some(n) = self.idx(pl.x + dx, pl.y + dy) {
                out.push(n);
            }
        }
        out
    }

    pub fn is_coastal(&self, p: PlotIdx) -> bool {
        !self.plot(p).kind.is_water() && self.neighbors(p).into_iter().any(|n| self.plot(n).kind.is_water())
    }

    pub fn clear_active_vision(&mut self, player: PlayerId) {
        let v = &mut self.vis[player.idx()];
        for cell in v.iter_mut() {
            if *cell == Visibility::Visible {
                *cell = Visibility::Fogged;
            }
        }
    }

    pub fn reveal_radius(&mut self, player: PlayerId, center: PlotIdx, radius: i32) {
        let pc = self.plot(center);
        let (cx, cy) = (pc.x, pc.y);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs().max(dy.abs()) <= radius {
                    if let Some(p) = self.idx(cx + dx, cy + dy) {
                        self.vis[player.idx()][p.idx()] = Visibility::Visible;
                    }
                }
            }
        }
    }

    pub fn is_revealed(&self, player: PlayerId, p: PlotIdx) -> bool {
        self.vis[player.idx()][p.idx()] != Visibility::Unrevealed
    }

    pub fn is_visible(&self, player: PlayerId, p: PlotIdx) -> bool {
        self.vis[player.idx()][p.idx()] == Visibility::Visible
    }

    /// Procedurally generates an interesting apocalyptic fantasy map with an inland sea / ocean
    /// holding a reserved Carcosa emergence site connected by future land bridges.
    pub fn generate(w: i32, h: i32, num_players: usize, rng: &mut Rng) -> (World, Vec<PlotIdx>) {
        let n = (w * h) as usize;
        // Generate smooth multi-octave elevation & moisture fields on a torus-in-X grid.
        let mut elev = vec![0i32; n];
        let mut moist = vec![0i32; n];

        // Place 6..10 continent lobes around a central sea so Carcosa rises dramatically in the middle!
        let cx = w as f64 * 0.5;
        let cy = h as f64 * 0.5;
        let num_lobes = 7usize;
        let mut lobes = Vec::with_capacity(num_lobes);
        for k in 0..num_lobes {
            let angle = (k as f64) * std::f64::consts::TAU / (num_lobes as f64) + (rng.range(40) as f64 - 20.0) * 0.01;
            let rad_x = (w as f64) * (0.28 + (rng.range(10) as f64) * 0.01);
            let rad_y = (h as f64) * (0.28 + (rng.range(10) as f64) * 0.01);
            let lx = (cx + angle.cos() * rad_x).clamp(2.0, (w - 3) as f64);
            let ly = (cy + angle.sin() * rad_y).clamp(2.0, (h - 3) as f64);
            let strn = 65 + rng.range(30);
            lobes.push((lx, ly, strn));
        }

        // Value noise grid for organic coastlines.
        let mut noise = vec![0i32; n];
        for v in &mut noise {
            *v = rng.range(40) - 20;
        }
        let mut smooth_noise = vec![0i32; n];
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0;
                let mut cnt = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let ny = y + dy;
                        if ny >= 0 && ny < h {
                            let nx = (x + dx).rem_euclid(w);
                            sum += noise[(ny * w + nx) as usize];
                            cnt += 1;
                        }
                    }
                }
                smooth_noise[(y * w + x) as usize] = sum / cnt.max(1);
            }
        }

        let mut mnoise = vec![0i32; n];
        for v in &mut mnoise {
            *v = rng.range(100);
        }

        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                let mut best = -40i32;
                for &(lx, ly, strn) in &lobes {
                    let mut dx = (x as f64 - lx).abs();
                    dx = dx.min(w as f64 - dx);
                    let dy = (y as f64 - ly).abs();
                    let d = (dx * dx + dy * dy).sqrt();
                    let falloff = (d * 100.0 / ((w.min(h) as f64) * 0.42)) as i32;
                    let val = strn - falloff;
                    if val > best {
                        best = val;
                    }
                }
                // Carve central sea basin for Carcosa!
                let cdx = (x as f64 - cx) / (w as f64 * 0.16).max(2.5);
                let cdy = (y as f64 - cy) / (h as f64 * 0.16).max(2.5);
                let cdist = (cdx * cdx + cdy * cdy).sqrt();
                if cdist < 1.15 {
                    best -= ((1.15 - cdist) * 85.0) as i32;
                }
                // Polar water borders top & bottom.
                if y == 0 || y == h - 1 {
                    best = -80;
                }
                elev[i] = best + smooth_noise[i];
                // Smooth moisture
                let mut msum = 0;
                let mut mcnt = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let ny = y + dy;
                        if ny >= 0 && ny < h {
                            let nx = (x + dx).rem_euclid(w);
                            msum += mnoise[(ny * w + nx) as usize];
                            mcnt += 1;
                        }
                    }
                }
                moist[i] = msum / mcnt.max(1);
            }
        }

        let mut plots = Vec::with_capacity(n);
        for y in 0..h {
            let lat = ((y as f64 - cy).abs() / (h as f64 * 0.5)).clamp(0.0, 1.0);
            for x in 0..w {
                let i = (y * w + x) as usize;
                let e = elev[i];
                let m = moist[i];
                let (kind, terrain) = if e < 4 {
                    (PlotKind::Ocean, Terrain::Ocean)
                } else {
                    let kind = if e > 64 {
                        PlotKind::Peak
                    } else if e > 42 {
                        PlotKind::Hills
                    } else {
                        PlotKind::Land
                    };
                    let terrain = if lat > 0.84 {
                        if m > 45 { Terrain::Snow } else { Terrain::Tundra }
                    } else if lat > 0.68 {
                        Terrain::Tundra
                    } else if m < 30 && lat < 0.45 {
                        Terrain::Desert
                    } else if m > 68 && kind == PlotKind::Land && lat < 0.35 {
                        Terrain::Marsh
                    } else if m > 46 {
                        Terrain::Grass
                    } else {
                        Terrain::Plains
                    };
                    (kind, terrain)
                };
                plots.push(Plot {
                    x,
                    y,
                    kind,
                    terrain,
                    base_terrain: terrain,
                    feature: Feature::None,
                    resource: Resource::None,
                    improvement: Improvement::None,
                    road: false,
                    river_bits: 0,
                    owner: None,
                    city: None,
                    lair_unexplored: false,
                    culture: vec![0; num_players],
                });
            }
        }

        let mut world = World {
            w,
            h,
            wrap_x: true,
            plots,
            vis: vec![vec![Visibility::Unrevealed; n]; num_players],
            carcosa_plot: PlotIdx(((h / 2) * w + (w / 2)) as u32),
            carcosa_bridges: Vec::new(),
            rev: 1,
        };

        // Mark coasts adjacent to land.
        for i in 0..n {
            let p = PlotIdx(i as u32);
            if world.plot(p).kind == PlotKind::Ocean {
                let near_land = world.neighbors(p).into_iter().any(|nb| !world.plot(nb).kind.is_water());
                if near_land {
                    world.plot_mut(p).kind = PlotKind::Coast;
                    world.plot_mut(p).terrain = Terrain::Coast;
                    world.plot_mut(p).base_terrain = Terrain::Coast;
                }
            }
        }

        // Guarantee Carcosa site is deep water near center.
        let mut best_carcosa = PlotIdx(((h / 2) * w + (w / 2)) as u32);
        let mut best_cscore = i32::MIN;
        for i in 0..n {
            let p = PlotIdx(i as u32);
            let pl = world.plot(p);
            if !pl.kind.is_water() || pl.y <= 2 || pl.y >= h - 3 {
                continue;
            }
            let d = (pl.x - w / 2).abs() + (pl.y - h / 2).abs();
            let water_nb = world.neighbors(p).into_iter().filter(|&nb| world.plot(nb).kind.is_water()).count() as i32;
            let score = water_nb * 10 - d * 3;
            if score > best_cscore {
                best_cscore = score;
                best_carcosa = p;
            }
        }
        world.carcosa_plot = best_carcosa;
        // Ensure Carcosa tile itself is water until AC 100.
        world.plot_mut(best_carcosa).kind = PlotKind::Ocean;
        world.plot_mut(best_carcosa).terrain = Terrain::Ocean;
        world.plot_mut(best_carcosa).base_terrain = Terrain::Ocean;

        // Add forests, jungles, oases, rivers, resources, Mana Nodes, and ancient lairs.
        for i in 0..n {
            let p = PlotIdx(i as u32);
            let (kind, t) = {
                let pl = world.plot(p);
                (pl.kind, pl.terrain)
            };
            if kind == PlotKind::Land || kind == PlotKind::Hills {
                if (t == Terrain::Grass || t == Terrain::Plains || t == Terrain::Tundra) && rng.chance(24) {
                    world.plot_mut(p).feature = if rng.chance(15) { Feature::AncientForest } else { Feature::Forest };
                } else if t == Terrain::Desert && kind == PlotKind::Land && rng.chance(8) {
                    world.plot_mut(p).feature = Feature::Oasis;
                }
                // Sprinkle river edges near hills.
                if kind == PlotKind::Land && rng.chance(10) {
                    let bit = if rng.chance(50) { 1 } else { 2 };
                    world.plot_mut(p).river_bits |= bit;
                    if world.plot(p).terrain == Terrain::Desert && world.plot(p).feature == Feature::None {
                        world.plot_mut(p).feature = Feature::FloodPlains;
                    }
                }
            }
        }

        // Resources & Mana Nodes & Lairs.
        for i in 0..n {
            let p = PlotIdx(i as u32);
            let kind = world.plot(p).kind;
            let terrain = world.plot(p).terrain;
            if kind == PlotKind::Coast && rng.chance(7) {
                world.plot_mut(p).resource = Resource::Fish;
            } else if kind.is_passable_land() {
                if rng.chance(4) {
                    let mana = match rng.range(6) {
                        0 => Resource::ManaFire,
                        1 => Resource::ManaLife,
                        2 => Resource::ManaDeath,
                        3 => Resource::ManaNature,
                        4 => Resource::ManaMind,
                        _ => Resource::ManaRaw,
                    };
                    world.plot_mut(p).resource = mana;
                } else if rng.chance(10) {
                    let res = match (kind, terrain) {
                        (PlotKind::Hills, _) => match rng.range(4) {
                            0 => Resource::Iron,
                            1 => Resource::GoldOre,
                            2 => Resource::Gems,
                            _ => Resource::Mithril,
                        },
                        (_, Terrain::Grass) => match rng.range(3) {
                            0 => Resource::Wheat,
                            1 => Resource::Cow,
                            _ => Resource::Wine,
                        },
                        (_, Terrain::Plains) => match rng.range(3) {
                            0 => Resource::Horse,
                            1 => Resource::Wheat,
                            _ => Resource::Cow,
                        },
                        (_, Terrain::Desert) => Resource::GoldOre,
                        _ => Resource::Iron,
                    };
                    world.plot_mut(p).resource = res;
                } else if rng.chance(3) && world.plot(p).feature != Feature::Oasis {
                    world.plot_mut(p).improvement = Improvement::RuinsLair;
                    world.plot_mut(p).lair_unexplored = true;
                }
            }
        }

        // Pick well-spaced starting plots for the active (non-barbarian) civilizations.
        let civ_count = num_players.saturating_sub(1);
        let starts = world.pick_starts(civ_count, rng);
        for &sp in &starts {
            // Clean up starting tile & guarantee fertile neighborhood.
            let pl = world.plot_mut(sp);
            pl.kind = PlotKind::Land;
            if matches!(pl.terrain, Terrain::Desert | Terrain::Snow | Terrain::Marsh) {
                pl.terrain = Terrain::Grass;
                pl.base_terrain = Terrain::Grass;
            }
            pl.feature = Feature::None;
            pl.improvement = Improvement::None;
            pl.lair_unexplored = false;
            pl.road = true;
            // Ensure at least one nearby food/iron/mana tile.
            let nbs = world.neighbors(sp);
            for (k, &nb) in nbs.iter().enumerate() {
                let np = world.plot_mut(nb);
                if np.kind == PlotKind::Peak {
                    np.kind = PlotKind::Hills;
                    np.terrain = Terrain::Plains;
                    np.base_terrain = Terrain::Plains;
                }
                if np.improvement == Improvement::RuinsLair {
                    np.improvement = Improvement::None;
                    np.lair_unexplored = false;
                }
                if k == 0 && np.kind.is_passable_land() && np.resource == Resource::None {
                    np.resource = Resource::Wheat;
                } else if k == 2 && np.kind.is_passable_land() && np.resource == Resource::None {
                    np.resource = Resource::ManaRaw;
                }
            }
        }

        (world, starts)
    }

    fn pick_starts(&self, count: usize, rng: &mut Rng) -> Vec<PlotIdx> {
        let mut candidates: Vec<(i32, PlotIdx)> = Vec::new();
        for i in 0..self.plots.len() {
            let p = PlotIdx(i as u32);
            let pl = self.plot(p);
            if pl.kind != PlotKind::Land || pl.y <= 2 || pl.y >= self.h - 3 {
                continue;
            }
            let land_nb = self.neighbors(p).into_iter().filter(|&n| self.plot(n).kind.is_passable_land()).count() as i32;
            if land_nb < 4 {
                continue;
            }
            let (f, ind, g) = pl.yields();
            let score = land_nb * 6 + f * 5 + ind * 4 + g * 2 + rng.range(10);
            candidates.push((score, p));
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

        let mut starts = Vec::with_capacity(count);
        let mut min_dist = (self.w.min(self.h) / 3).max(5);
        while starts.len() < count && min_dist >= 2 {
            for &(_, p) in &candidates {
                if starts.len() >= count {
                    break;
                }
                if starts.iter().all(|&s| self.distance(s, p) >= min_dist) {
                    starts.push(p);
                }
            }
            min_dist -= 1;
        }
        // Absolute fallback if map is tiny.
        if starts.len() < count {
            for i in 0..self.plots.len() {
                if starts.len() >= count {
                    break;
                }
                let p = PlotIdx(i as u32);
                if self.plot(p).kind.is_passable_land() && !starts.contains(&p) {
                    starts.push(p);
                }
            }
        }
        starts
    }

    /// Raises Carcosa from the sea and builds 4 narrow land bridges to the nearest shore in each
    /// cardinal direction. Returns the Carcosa plot and the newly raised bridge plots.
    pub fn raise_carcosa(&mut self) -> PlotIdx {
        let cp = self.carcosa_plot;
        {
            let pl = self.plot_mut(cp);
            pl.kind = PlotKind::Hills;
            pl.terrain = Terrain::FieldsOfPerdition;
            pl.base_terrain = Terrain::FieldsOfPerdition;
            pl.feature = Feature::None;
            pl.resource = Resource::ManaMind;
            pl.road = true;
        }
        let (cx, cy) = (self.plot(cp).x, self.plot(cp).y);
        let mut bridges = Vec::new();
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let mut x = cx + dx;
            let mut y = cy + dy;
            for _ in 0..10 {
                let Some(p) = self.idx(x, y) else { break };
                if !self.plot(p).kind.is_water() {
                    break;
                }
                let pl = self.plot_mut(p);
                pl.kind = PlotKind::Land;
                pl.terrain = Terrain::BrokenLands;
                pl.base_terrain = Terrain::BrokenLands;
                pl.road = true;
                bridges.push(p);
                x += dx;
                y += dy;
            }
        }
        self.carcosa_bridges = bridges;
        self.rev += 1;
        cp
    }

    /// Corrupts up to `count` non-hell land plots (preferring tiles owned by evil or wild plots).
    pub fn spread_hell(&mut self, count: usize, prefer_evil_owners: &[PlayerId], rng: &mut Rng) -> usize {
        let mut pool: Vec<PlotIdx> = Vec::new();
        for i in 0..self.plots.len() {
            let p = PlotIdx(i as u32);
            let pl = self.plot(p);
            if !pl.kind.is_passable_land() || pl.terrain.is_hell() {
                continue;
            }
            if pl.owner.is_some_and(|o| prefer_evil_owners.contains(&o)) || pl.owner.is_none() {
                pool.push(p);
            }
        }
        if pool.is_empty() {
            for i in 0..self.plots.len() {
                let p = PlotIdx(i as u32);
                let pl = self.plot(p);
                if pl.kind.is_passable_land() && !pl.terrain.is_hell() {
                    pool.push(p);
                }
            }
        }
        rng.shuffle(&mut pool);
        let mut changed = 0;
        for p in pool.into_iter().take(count) {
            let pl = self.plot_mut(p);
            pl.terrain = pl.terrain.hell_version();
            if pl.feature.is_woods() {
                pl.feature = Feature::BurntForest;
            }
            changed += 1;
        }
        if changed > 0 {
            self.rev += 1;
        }
        changed
    }

    /// Cleanses Hell terrain within `radius` of `center`.
    pub fn cleanse_hell(&mut self, center: PlotIdx, radius: i32) -> usize {
        let (cx, cy) = (self.plot(center).x, self.plot(center).y);
        let mut changed = 0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if let Some(p) = self.idx(cx + dx, cy + dy) {
                    let pl = self.plot_mut(p);
                    if pl.terrain.is_hell() && !pl.base_terrain.is_hell() {
                        pl.terrain = pl.base_terrain;
                        if pl.feature == Feature::BurntForest {
                            pl.feature = Feature::Forest;
                        }
                        changed += 1;
                    }
                }
            }
        }
        if changed > 0 {
            self.rev += 1;
        }
        changed
    }
}
