//! Erebus (Creation 1.07/1.08), adapted from Rich Marinaccio's FFH2 Python script.
//! Original: Copyright 2007 Rich Marinaccio; 2019 assertion fixes by lfgr.
//! Reference: akachi10/FFH2-game-files, PrivateMaps/Erebus.py, blob
//! 1b66edd67b621e3c5c7b6c5cceb5f7d1d09446b4. See docs/MAPGEN.md for adaptations.
//!
//! Regions grow on river intersections, then become valleys or connected seas. A tree of
//! outflow gates supplies elevation, drainage and rainfall; mountain rims separate valleys.
//! Native seeded map randomness replaces Python's separate RNG. No generation state is global.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::game::Game;
use crate::mapgen::MapScript;
use crate::rng::Rng;
use ffh_data::*;

const CARDINAL: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
const AROUND: [(i32, i32); 8] = [
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];
mod terra;

const RIVER_THRESHOLD: f64 = 5.0;

fn shuffle<T>(a: &mut [T], rng: &mut Rng) {
    for i in (1..a.len()).rev() {
        let j = rng.get_i((i + 1) as i32, "Erebus shuffle") as usize;
        a.swap(i, j);
    }
}
fn random(rng: &mut Rng) -> f64 {
    rng.get(65535, "Erebus") as f64 / 65535.0
}
fn index(x: i32, y: i32, w: i32, h: i32) -> Option<usize> {
    (x >= 0 && y >= 0 && x < w && y < h).then_some((y * w + x) as usize)
}

#[derive(Clone, Debug)]
struct Region {
    seed: (i32, i32),
    water: bool,
    growing: bool,
    touching: bool,
    neighbors: Vec<usize>,
    parent: Option<usize>,
    altitude: Option<usize>,
    gate: Option<usize>,
    moisture: f64,
}
#[derive(Clone, Copy, Debug)]
struct Gate {
    a: usize,
    b: usize,
    corner: usize,
}

#[derive(Debug)]
struct Layout {
    w: i32,
    h: i32,
    regions: Vec<Region>,
    tiles: Vec<i32>,
    plots: Vec<PlotType>,
    flow: Vec<Option<usize>>,
    runoff: Vec<f64>,
    altitude: Vec<f64>,
    rainfall: Vec<f64>,
    old_world: Option<Vec<bool>>,
}

impl Layout {
    fn generate(w: i32, h: i32, water_density: f64, rng: &mut Rng) -> Self {
        let n = (w * h) as usize;
        let mut s = Self {
            w,
            h,
            regions: Vec::new(),
            tiles: vec![-1; n],
            plots: vec![PlotType::PLOT_OCEAN; n],
            flow: vec![None; ((w + 1) * (h + 1)) as usize],
            runoff: vec![0.0; ((w + 1) * (h + 1)) as usize],
            altitude: vec![0.0; n],
            rainfall: vec![0.0; n],
            old_world: None,
        };
        s.grow_regions(rng);
        s.choose_seas(water_density, rng);
        s.drainage(rng);
        s.plot_types(rng);
        s
    }
    fn tile(&self, x: i32, y: i32) -> i32 {
        index(x, y, self.w, self.h).map_or(-1, |i| self.tiles[i])
    }
    fn corner_neighbors(&self, c: usize) -> Vec<usize> {
        let rw = self.w + 1;
        let (x, y) = (c as i32 % rw, c as i32 / rw);
        CARDINAL
            .iter()
            .filter_map(|&(dx, dy)| index(x + dx, y + dy, rw, self.h + 1))
            .collect()
    }
    fn corner_tiles(&self, c: usize) -> [i32; 4] {
        let (x, y) = (c as i32 % (self.w + 1), c as i32 / (self.w + 1));
        [
            self.tile(x - 1, y - 1),
            self.tile(x, y - 1),
            self.tile(x - 1, y),
            self.tile(x, y),
        ]
    }
    fn pure_corner(&self, c: usize) -> i32 {
        let ts = self.corner_tiles(c);
        if ts.iter().all(|&r| r == ts[0]) {
            ts[0]
        } else {
            -1
        }
    }
    fn tile_corners(&self, i: usize) -> [usize; 4] {
        let (x, y) = (i as i32 % self.w, i as i32 / self.w);
        let c = (y * (self.w + 1) + x) as usize;
        [
            c,
            c + 1,
            c + (self.w + 1) as usize,
            c + (self.w + 2) as usize,
        ]
    }
    fn grow_regions(&mut self, rng: &mut Rng) {
        let (rw, rh) = (self.w + 1, self.h + 1);
        let mut rx = vec![-1; (rw * rh) as usize];
        let mut queue = VecDeque::new();
        let edge = 5.min(self.w.min(self.h) / 4).max(2);
        let count = ((self.w * self.h) as f64 * 0.009) as usize;
        for _ in 0..count.max(2) {
            let mut seed = None;
            for _ in 0..10000 {
                let x = edge + rng.get_i(rw - 2 * edge, "Erebus region x");
                let y = edge + rng.get_i(rh - 2 * edge, "Erebus region y");
                if self
                    .regions
                    .iter()
                    .all(|r| (r.seed.0 - x).abs() >= 5 || (r.seed.1 - y).abs() >= 5)
                {
                    seed = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = seed else { break };
            let r = self.regions.len();
            self.regions.push(Region {
                seed: (x, y),
                water: false,
                growing: true,
                touching: false,
                neighbors: Vec::new(),
                parent: None,
                altitude: None,
                gate: None,
                moisture: 1.0,
            });
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let c = index(x + dx, y + dy, rw, rh).unwrap();
                    rx[c] = r as i32;
                    queue.push_back((r, x + dx, y + dy));
                }
            }
        }
        // A bounded queue also handles small/custom dimensions without Python's endless-loop errors.
        for _ in 0..(rw * rh * 200) {
            let Some((r, x, y)) = queue.pop_front() else {
                break;
            };
            if !self.regions[r].growing {
                continue;
            }
            let mut room = false;
            for (dx, dy) in CARDINAL {
                let (nx, ny) = (x + dx, y + dy);
                let Some(c) = index(nx, ny, rw, rh) else {
                    continue;
                };
                let mut free = rx[c] < 0;
                if free {
                    for (ax, ay) in AROUND {
                        if let Some(j) = index(nx + ax, ny + ay, rw, rh) {
                            if rx[j] >= 0 && rx[j] != r as i32 {
                                self.regions[r].touching = true;
                                free = false;
                            }
                        }
                    }
                }
                let boundary = nx < 2 || ny < 2 || nx >= rw - 2 || ny >= rh - 2;
                if free && (boundary || rng.get(4, "Erebus grow") == 0) {
                    rx[c] = r as i32;
                    queue.push_back((r, nx, ny));
                }
                if boundary && self.regions[r].touching {
                    self.regions[r].growing = false;
                }
                if free && !boundary {
                    room = true;
                }
            }
            if room {
                queue.push_back((r, x, y));
            }
        }
        // A river intersection claims its four touching plots, in the original scan order.
        for y in 0..rh {
            for x in 0..rw {
                let r = rx[(y * rw + x) as usize];
                if r < 0 {
                    continue;
                }
                for (dx, dy) in [(0, 0), (-1, 0), (0, -1), (-1, -1)] {
                    if let Some(i) = index(x + dx, y + dy, self.w, self.h) {
                        self.tiles[i] = r;
                    }
                }
            }
        }
        for y in 0..self.h {
            for x in 0..self.w {
                let r = self.tile(x, y);
                if r < 0 {
                    continue;
                }
                for (dx, dy) in CARDINAL {
                    let b = self.tile(x + dx, y + dy);
                    if b >= 0
                        && b != r
                        && !self.regions[r as usize].neighbors.contains(&(b as usize))
                    {
                        self.regions[r as usize].neighbors.push(b as usize);
                    }
                }
            }
        }
    }
    fn choose_seas(&mut self, density: f64, rng: &mut Rng) {
        let center = self.tile(self.w / 2, self.h / 2);
        let root = if center >= 0 { center as usize } else { 0 };
        self.regions[root].water = true;
        let mut order: Vec<_> = (0..self.regions.len()).collect();
        for _ in 0..((self.w * self.h) as f64 * density) as usize {
            shuffle(&mut order, rng);
            order.sort_by_key(|&r| {
                self.regions[r]
                    .neighbors
                    .iter()
                    .filter(|&&b| self.regions[b].water)
                    .count()
            });
            if let Some(&r) = order.iter().find(|&&r| {
                !self.regions[r].water
                    && self.regions[r]
                        .neighbors
                        .iter()
                        .any(|&b| self.regions[b].water)
            }) {
                self.regions[r].water = true;
            }
        }
        // Absorb unclaimed spaces adjoining the sea so its basins are not split by thin walls.
        let mut queue: VecDeque<_> = self
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, &r)| (r >= 0 && self.regions[r as usize].water).then_some(i))
            .collect();
        while let Some(i) = queue.pop_front() {
            let (x, y) = (i as i32 % self.w, i as i32 / self.w);
            for (dx, dy) in CARDINAL {
                if let Some(j) = index(x + dx, y + dy, self.w, self.h) {
                    if self.tiles[j] < 0 {
                        self.tiles[j] = self.tiles[i];
                        queue.push_back(j);
                    }
                }
            }
        }
    }
    fn drainage(&mut self, rng: &mut Rng) {
        let len = self.flow.len();
        let pure: Vec<_> = (0..len).map(|c| self.pure_corner(c)).collect();
        // A gate must touch exactly two regions and have an interior intersection on both sides.
        let mut gates = Vec::new();
        for c in 0..len {
            let mut rs = self.corner_tiles(c).to_vec();
            if rs.iter().any(|&r| r < 0) {
                continue;
            }
            rs.sort_unstable();
            rs.dedup();
            if rs.len() != 2 {
                continue;
            }
            let ns = self.corner_neighbors(c);
            if rs.iter().all(|&r| ns.iter().any(|&j| pure[j] == r)) {
                gates.push(Gate {
                    a: rs[0] as usize,
                    b: rs[1] as usize,
                    corner: c,
                });
            }
        }
        for r in &mut self.regions {
            if r.water {
                r.altitude = Some(0);
            }
        }
        let mut order: Vec<_> = (0..self.regions.len()).collect();
        for _ in 0..self.regions.len() {
            let mut changed = false;
            shuffle(&mut order, rng);
            for &r in &order {
                if self.regions[r].altitude.is_some() {
                    continue;
                }
                let choices: Vec<_> = gates
                    .iter()
                    .filter_map(|gate| {
                        let b = if gate.a == r {
                            gate.b
                        } else if gate.b == r {
                            gate.a
                        } else {
                            return None;
                        };
                        self.regions[b].altitude.map(|_| (b, gate.corner))
                    })
                    .collect();
                if choices.is_empty() {
                    continue;
                }
                let (b, c) = choices[rng.get_i(choices.len() as i32, "Erebus gate") as usize];
                self.regions[r].parent = Some(b);
                self.regions[r].gate = Some(c);
                self.regions[r].altitude = Some(self.regions[b].altitude.unwrap() + 1);
                changed = true;
            }
            if !changed {
                break;
            }
        }
        let highest = self
            .regions
            .iter()
            .filter_map(|r| r.altitude)
            .max()
            .unwrap_or(1)
            .max(1);
        let mut wet = self
            .regions
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| r.altitude.unwrap_or(0))
            .map(|(r, _)| r)
            .unwrap();
        while self.regions[wet].altitude.unwrap_or(0) > 1 {
            wet = self.regions[wet].parent.unwrap();
        }
        let wet_c = self.regions[wet].gate.unwrap_or(0);
        let (wx, wy) = (wet_c as i32 % (self.w + 1), wet_c as i32 / (self.w + 1));
        let mut max_distance: f64 = 1.0;
        for r in &self.regions {
            if let Some(c) = r.gate {
                let dx = c as i32 % (self.w + 1) - wx;
                let dy = c as i32 / (self.w + 1) - wy;
                max_distance = max_distance.max(((dx * dx + dy * dy) as f64).sqrt());
            }
        }
        for r in &mut self.regions {
            if let Some(c) = r.gate {
                let dx = c as i32 % (self.w + 1) - wx;
                let dy = c as i32 / (self.w + 1) - wy;
                r.moisture = 1.0 - ((dx * dx + dy * dy) as f64).sqrt() / max_distance;
            }
        }
        let mut height = vec![-1.0; len];
        let mut owner = pure.clone();
        for r in 0..self.regions.len() {
            let Some(c) = self.regions[r].gate else {
                continue;
            };
            let parent = self.regions[r].parent.unwrap() as i32;
            owner[c] = r as i32;
            self.flow[c] = self
                .corner_neighbors(c)
                .into_iter()
                .find(|&j| pure[j] == parent);
            height[c] = 0.01;
            let mut frontier = vec![c];
            while !frontier.is_empty() {
                shuffle(&mut frontier, rng);
                let mut next = Vec::new();
                for i in frontier {
                    for j in self.corner_neighbors(i) {
                        if pure[j] == r as i32 && height[j] < 0.0 {
                            height[j] = height[i] * (1.01 + random(rng) * 0.19);
                            next.push(j);
                        }
                    }
                }
                frontier = next;
            }
        }
        for c in 0..len {
            let r = owner[c];
            if r < 0
                || self.regions[r as usize].water
                || height[c] < 0.0
                || self.regions[r as usize].gate == Some(c)
            {
                continue;
            }
            let gate = self.regions[r as usize].gate;
            let choices: Vec<_> = self
                .corner_neighbors(c)
                .into_iter()
                .filter(|&j| {
                    (pure[j] == r || gate == Some(j)) && height[j] >= 0.0 && height[j] < height[c]
                })
                .collect();
            if !choices.is_empty() {
                self.flow[c] =
                    Some(choices[rng.get_i(choices.len() as i32, "Erebus flow") as usize]);
            }
        }
        let mut corners: Vec<_> = (0..len).filter(|&c| self.flow[c].is_some()).collect();
        corners.sort_by(|&a, &b| {
            let aa = self.regions[owner[a] as usize].altitude.unwrap_or(0);
            let ba = self.regions[owner[b] as usize].altitude.unwrap_or(0);
            ba.cmp(&aa)
                .then_with(|| height[b].total_cmp(&height[a]))
                .then_with(|| a.cmp(&b))
        });
        // Accumulate in topological order instead of rewalking every tributary's whole path.
        for c in corners {
            let r = owner[c] as usize;
            self.runoff[c] += 0.25 + 0.75 * self.regions[r].moisture;
            if let Some(next) = self.flow[c] {
                self.runoff[next] += self.runoff[c];
            }
        }
        for i in 0..self.tiles.len() {
            let r = self.tiles[i];
            if r < 0 || self.regions[r as usize].altitude.is_none() {
                continue;
            }
            let region = &self.regions[r as usize];
            let river = self
                .tile_corners(i)
                .iter()
                .map(|&c| self.runoff[c])
                .sum::<f64>()
                / 4.0;
            self.altitude[i] = ((region.altitude.unwrap() + 1) as f64 - river.min(10.0) * 0.2)
                / (highest + 1) as f64;
            self.rainfall[i] = region.moisture + river.min(50.0) / 50.0 * 0.2;
        }
    }
    fn plot_types(&mut self, rng: &mut Rng) {
        let mut order: Vec<_> = (0..self.tiles.len()).collect();
        shuffle(&mut order, rng);
        let highest = self
            .regions
            .iter()
            .filter_map(|r| r.altitude)
            .max()
            .unwrap_or(1);
        for &i in &order {
            let r = self.tiles[i];
            if r < 0 || self.regions[r as usize].altitude.is_none() {
                self.plots[i] = PlotType::PLOT_PEAK;
                continue;
            }
            let reg = &self.regions[r as usize];
            if reg.water {
                continue;
            }
            let (x, y) = (i as i32 % self.w, i as i32 / self.w);
            let mut kind = PlotType::PLOT_LAND;
            for (dx, dy) in AROUND {
                let Some(j) = index(x + dx, y + dy, self.w, self.h) else {
                    kind = PlotType::PLOT_PEAK;
                    break;
                };
                let other = self.tiles[j];
                if other < 0 || self.plots[j] == PlotType::PLOT_PEAK {
                    continue;
                }
                if self.regions[other as usize].water && reg.altitude == Some(1) {
                    kind = PlotType::PLOT_LAND;
                    break;
                }
                if self.regions[other as usize].water && reg.altitude == Some(2) && highest >= 3 {
                    kind = PlotType::PLOT_HILLS;
                    break;
                }
                if other != r {
                    kind = PlotType::PLOT_PEAK;
                }
            }
            self.plots[i] = kind;
        }
        for &i in &order {
            if self.plots[i] != PlotType::PLOT_LAND {
                continue;
            }
            let a = self.altitude[i].clamp(0.0, 1.0);
            if random(rng) < 0.15 + a * 0.75 {
                self.plots[i] = PlotType::PLOT_HILLS;
            }
            if random(rng) < a * 0.20 {
                self.plots[i] = PlotType::PLOT_PEAK;
            }
            let river = self
                .tile_corners(i)
                .iter()
                .map(|&c| self.runoff[c])
                .sum::<f64>()
                / 4.0;
            if random(rng) < (river / 50.0).min(1.0) {
                self.plots[i] = PlotType::PLOT_LAND;
            }
        }
        // Cut traversable outlets through the mountain rims. Every valley has one downstream
        // gate; rivers and armies can both cross it, even for a short, low-rainfall tributary.
        let gates: Vec<_> = self.regions.iter().filter_map(|r| r.gate).collect();
        for c in gates {
            let (x, y) = (c as i32 % (self.w + 1), c as i32 / (self.w + 1));
            for (dx, dy) in [(0, 0), (-1, 0), (0, -1), (-1, -1)] {
                if let Some(i) = index(x + dx, y + dy, self.w, self.h) {
                    if self.plots[i] == PlotType::PLOT_PEAK {
                        self.plots[i] = PlotType::PLOT_HILLS;
                    }
                }
            }
        }
        // Rivers cut paths through internal high ground as well as the inter-valley gates.
        for c in 0..self.flow.len() {
            let Some(next) = self.flow[c].filter(|_| self.runoff[c] > RIVER_THRESHOLD) else {
                continue;
            };
            let rw = self.w + 1;
            let (x, y) = (c as i32 % rw, c as i32 / rw);
            let (nx, ny) = (next as i32 % rw, next as i32 / rw);
            let touching = if y == ny {
                [(x.min(nx), y - 1), (x.min(nx), y)]
            } else {
                [(x - 1, y.min(ny)), (x, y.min(ny))]
            };
            for (tx, ty) in touching {
                if let Some(i) = index(tx, ty, self.w, self.h) {
                    if self.plots[i] == PlotType::PLOT_PEAK {
                        self.plots[i] = PlotType::PLOT_HILLS;
                    }
                }
            }
        }
        for i in 0..self.plots.len() {
            let (x, y) = (i as i32 % self.w, i as i32 / self.w);
            if (x == 0 || y == 0 || x == self.w - 1 || y == self.h - 1)
                && self.plots[i] != PlotType::PLOT_OCEAN
            {
                self.plots[i] = PlotType::PLOT_PEAK;
            }
        }
        // As in Creation, inaccessible pockets are mountains rather than tempting, unusable land.
        let mut seen = vec![false; self.plots.len()];
        let mut queue = VecDeque::new();
        for i in 0..self.plots.len() {
            if self.plots[i] == PlotType::PLOT_OCEAN {
                seen[i] = true;
                queue.push_back(i);
            }
        }
        while let Some(i) = queue.pop_front() {
            let (x, y) = (i as i32 % self.w, i as i32 / self.w);
            for (dx, dy) in AROUND {
                if let Some(j) = index(x + dx, y + dy, self.w, self.h) {
                    if !seen[j] && self.plots[j] != PlotType::PLOT_PEAK {
                        seen[j] = true;
                        queue.push_back(j);
                    }
                }
            }
        }
        for (i, visited) in seen.into_iter().enumerate() {
            if !visited {
                self.plots[i] = PlotType::PLOT_PEAK;
            }
        }
    }
}

#[derive(Default)]
pub struct ErebusMapScript {
    layout: Mutex<Option<Layout>>,
    terra: bool,
}

impl ErebusMapScript {
    pub fn terra() -> Self { Self { terra: true, ..Self::default() } }
}

impl MapScript for ErebusMapScript {
    fn name(&self) -> &str {
        if self.terra { "ErebusTerra" } else { "Erebus" }
    }
    fn get_wrap_x(&self, _: &Game) -> Option<bool> {
        Some(false)
    }
    fn get_wrap_y(&self, _: &Game) -> Option<bool> {
        Some(false)
    }
    fn get_grid_size(&self, g: &Game, world: WorldSizeType) -> Option<(i32, i32)> {
        let cells = match g.d.type_name(world) {
            "WORLDSIZE_DUEL" => 8,
            "WORLDSIZE_TINY" => 9,
            "WORLDSIZE_SMALL" => 10,
            "WORLDSIZE_STANDARD" => 13,
            "WORLDSIZE_LARGE" => 15,
            "WORLDSIZE_HUGE" => 22,
            _ => return None,
        };
        let scale = match g.d.type_name(g.setup.sea_level) {
            "SEALEVEL_LOW" => (0.007_f64 / 0.008).sqrt(),
            "SEALEVEL_HIGH" => (0.007_f64 / 0.003).sqrt(),
            _ => 1.0,
        };
        let side = (cells as f64 * scale) as i32;
        Some(if self.terra { (side + (side * 3 + 3) / 4 + 3, side) } else { (side, side) })
    }
    fn generate_plot_types(&self, g: &mut Game) -> Option<Vec<PlotType>> {
        let density = match g.d.type_name(g.setup.sea_level) {
            "SEALEVEL_LOW" => 0.001,
            "SEALEVEL_HIGH" => 0.004,
            _ => 0.002,
        };
        let s = if self.terra {
            Layout::generate_terra(g.map.grid_width, g.map.grid_height, &mut g.map_rand)
        } else {
            Layout::generate(g.map.grid_width, g.map.grid_height, density, &mut g.map_rand)
        };
        let plots = s.plots.clone();
        *self.layout.lock().unwrap() = Some(s);
        Some(plots)
    }
    fn generate_terrain_types(&self, g: &mut Game) -> Option<Vec<TerrainType>> {
        let lock = self.layout.lock().unwrap();
        let s = lock.as_ref()?;
        let climate = Climate::new(g);
        let terrain = |name| g.d.t::<TerrainType>(name);
        let (grass, plains, desert, tundra, snow) = (
            terrain("TERRAIN_GRASS"),
            terrain("TERRAIN_PLAINS"),
            terrain("TERRAIN_DESERT"),
            terrain("TERRAIN_TUNDRA"),
            terrain("TERRAIN_SNOW"),
        );
        let mut out = Vec::with_capacity(s.plots.len());
        for i in 0..s.plots.len() {
            if s.plots[i] == PlotType::PLOT_OCEAN {
                out.push(
                    g.map.plots[crate::ids::PlotIdx(i as u32)]
                        .terrain_type
                        .unwrap_or(terrain("TERRAIN_OCEAN")),
                );
                continue;
            }
            let rain = s.rainfall[i];
            let mut t = if rain < climate.desert {
                if rain < climate.desert * (0.5 + random(&mut g.map_rand) * 0.5) {
                    desert
                } else {
                    plains
                }
            } else if rain < climate.plains {
                if rain
                    < climate.desert
                        + (climate.plains - climate.desert) * (0.5 + random(&mut g.map_rand) * 0.5)
                {
                    plains
                } else {
                    grass
                }
            } else {
                grass
            };
            if s.altitude[i] > climate.snow {
                t = snow;
            } else if s.altitude[i] > climate.tundra {
                t = tundra;
            } else if t == desert && s.altitude[i] > climate.max_desert {
                t = plains;
            }
            out.push(t);
        }
        let ground = out.clone();
        for i in 0..out.len() {
            if s.plots[i] != PlotType::PLOT_PEAK {
                continue;
            }
            out[i] = tundra;
            let (x, y) = (i as i32 % s.w, i as i32 / s.w);
            for (dx, dy) in AROUND {
                if let Some(j) = index(x + dx, y + dy, s.w, s.h) {
                    if matches!(s.plots[j], PlotType::PLOT_LAND | PlotType::PLOT_HILLS) {
                        out[i] = ground[j];
                        break;
                    }
                }
            }
        }
        Some(out)
    }
    fn add_rivers(&self, g: &mut Game) -> bool {
        let lock = self.layout.lock().unwrap();
        let Some(s) = lock.as_ref() else { return false };
        for i in 0..s.plots.len() {
            let [sw, se, _nw, ne] = s.tile_corners(i);
            let p = crate::ids::PlotIdx(i as u32);
            let rw = (s.w + 1) as usize;
            if s.runoff[ne] > RIVER_THRESHOLD && s.flow[ne] == ne.checked_sub(rw) {
                g.plot_set_w_of_river(
                    p,
                    true,
                    Some(CardinalDirectionType::CARDINALDIRECTION_SOUTH),
                );
            }
            if s.runoff[sw] > RIVER_THRESHOLD && s.flow[sw] == Some(sw + 1) {
                g.plot_set_n_of_river(p, true, Some(CardinalDirectionType::CARDINALDIRECTION_EAST));
            }
            if s.runoff[se] > RIVER_THRESHOLD && s.flow[se] == Some(se + rw) {
                g.plot_set_w_of_river(
                    p,
                    true,
                    Some(CardinalDirectionType::CARDINALDIRECTION_NORTH),
                );
            } else if s.runoff[se] > RIVER_THRESHOLD && s.flow[se] == se.checked_sub(1) {
                g.plot_set_n_of_river(p, true, Some(CardinalDirectionType::CARDINALDIRECTION_WEST));
            }
        }
        true
    }
    fn add_lakes(&self, _: &mut Game) -> bool {
        true
    }
    fn add_features(&self, g: &mut Game) -> bool {
        let lock = self.layout.lock().unwrap();
        let Some(s) = lock.as_ref() else { return false };
        let c = Climate::new(g);
        for i in 0..s.plots.len() {
            let p = crate::ids::PlotIdx(i as u32);
            if g.plot_is_water(p) || g.plot_is_peak(p) {
                continue;
            }
            let desert = g.map.plots[p].terrain_type == g.d.ty("TERRAIN_DESERT");
            let mut feature = None;
            if !desert && random(&mut g.map_rand) < s.rainfall[i] {
                if s.altitude[i] < c.leafy && s.rainfall[i] >= c.jungle {
                    if g.plot_is_flatlands(p) && random(&mut g.map_rand) < 0.30 {
                        g.plot_set_terrain_type(p, g.d.t("TERRAIN_MARSH"), false);
                        if random(&mut g.map_rand) >= 0.33 {
                            feature = g.d.ty("FEATURE_JUNGLE");
                        }
                    } else {
                        feature = g.d.ty("FEATURE_JUNGLE");
                    }
                } else {
                    feature = g.d.ty("FEATURE_FOREST");
                }
            }
            if desert && g.plot_is_flatlands(p) {
                if random(&mut g.map_rand) <= s.rainfall[i] * 3.0 {
                    feature = g.d.ty("FEATURE_SCRUB");
                }
                if g.plot_is_river(p) {
                    feature = g.d.ty("FEATURE_FLOOD_PLAINS");
                } else if random(&mut g.map_rand) < 0.08 {
                    let (x, y) = (i as i32 % s.w, i as i32 / s.w);
                    let surrounded = AROUND.iter().all(|&(dx, dy)| {
                        g.map.plot_idx(x + dx, y + dy).is_some_and(|j| {
                            g.map.plots[j].terrain_type == g.d.ty("TERRAIN_DESERT")
                                && g.map.plots[j].feature_type != g.d.ty("FEATURE_OASIS")
                        })
                    });
                    if surrounded {
                        feature = g.d.ty("FEATURE_OASIS");
                    }
                }
            }
            if let Some(f) = feature {
                if g.plot_can_have_feature(p, Some(f)) {
                    g.plot_set_feature_type(p, Some(f));
                }
            }
        }
        true
    }
    fn add_bonus_type(&self, g: &mut Game, bonus: BonusType) -> bool {
        self.terra && terra::place_reserved_type(g, bonus)
    }
    fn find_starting_area(&self, g: &Game, _: crate::ids::PlayerId) -> Option<i32> {
        if self.terra && !g.final_initialized { terra::old_world_area(g).map(|a| a.i()) } else { None }
    }
    fn after_starting_resources(&self, g: &mut Game) {
        if self.terra { terra::finish_resources(g); }
    }
    fn normalize_add_river(&self, _: &mut Game) -> bool {
        true
    }
    fn normalize_add_lakes(&self, _: &mut Game) -> bool {
        true
    }
    fn normalize_remove_peaks(&self, _: &mut Game) -> bool {
        true
    }
    fn normalize_add_good_terrain(&self, _: &mut Game) -> bool {
        true
    }
    fn normalize_remove_bad_terrain(&self, _: &mut Game) -> bool {
        true
    }
    fn is_bonus_ignore_latitude(&self, _: &Game) -> Option<bool> {
        Some(true)
    }
}

struct Climate {
    desert: f64,
    plains: f64,
    jungle: f64,
    tundra: f64,
    snow: f64,
    max_desert: f64,
    leafy: f64,
}
impl Climate {
    fn new(g: &Game) -> Self {
        let mut c = Self {
            desert: 0.30,
            plains: 0.50,
            jungle: 0.90,
            tundra: 0.74,
            snow: 0.84,
            max_desert: 0.65,
            leafy: 0.30,
        };
        match g.d.type_name(g.setup.climate) {
            "CLIMATE_ARID" => {
                c.desert = 0.70;
                c.plains = 0.80;
                c.jungle = 0.98;
            }
            "CLIMATE_TROPICAL" => {
                c.desert = 0.10;
                c.plains = 0.35;
                c.jungle = 0.50;
                c.leafy = 0.40;
            }
            "CLIMATE_COLD" => {
                c.tundra = 0.35;
                c.snow = 0.55;
                c.max_desert = 0.25;
                c.leafy = 0.20;
            }
            _ => {}
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regional_worlds_have_connected_seas_bounded_edges_and_acyclic_drainage() {
        for side in [28, 32, 40, 52, 88, 132] {
            for seed in 1..=200 {
                let s = Layout::generate(side, side, 0.002, &mut Rng::new(seed));
                for (r, region) in s.regions.iter().enumerate() {
                    if region.altitude.is_none() {
                        assert!(
                            s.tiles
                                .iter()
                                .enumerate()
                                .filter(|(_, &owner)| owner == r as i32)
                                .all(|(i, _)| s.plots[i] == PlotType::PLOT_PEAK),
                            "ungated regions are inaccessible mountains"
                        );
                    }
                }
                let n = s.plots.len();
                let water = s
                    .plots
                    .iter()
                    .filter(|&&k| k == PlotType::PLOT_OCEAN)
                    .count();
                let passable = s
                    .plots
                    .iter()
                    .filter(|&&k| matches!(k, PlotType::PLOT_LAND | PlotType::PLOT_HILLS))
                    .count();
                assert!(
                    water > n / 20 && passable > n / 10,
                    "usable land and sea: side {side}, seed {seed}"
                );
                assert!(
                    s.plots
                        .iter()
                        .filter(|&&k| k == PlotType::PLOT_PEAK)
                        .count()
                        >= s.regions.len()
                );
                for i in 0..n {
                    let (x, y) = (i as i32 % side, i as i32 / side);
                    if x == 0 || y == 0 || x == side - 1 || y == side - 1 {
                        assert!(
                            matches!(s.plots[i], PlotType::PLOT_OCEAN | PlotType::PLOT_PEAK),
                            "no playable land across a nonwrapping boundary"
                        );
                    }
                }
                let start = s
                    .plots
                    .iter()
                    .position(|&k| k == PlotType::PLOT_OCEAN)
                    .unwrap();
                let mut seen = vec![false; n];
                let mut q = VecDeque::from([start]);
                seen[start] = true;
                let mut reached_water = 0;
                while let Some(i) = q.pop_front() {
                    if s.plots[i] == PlotType::PLOT_OCEAN {
                        reached_water += 1;
                    }
                    let (x, y) = (i as i32 % side, i as i32 / side);
                    for (dx, dy) in CARDINAL {
                        if let Some(j) = index(x + dx, y + dy, side, side) {
                            if !seen[j] && s.plots[j] == PlotType::PLOT_OCEAN {
                                seen[j] = true;
                                q.push_back(j);
                            }
                        }
                    }
                }
                assert_eq!(reached_water, water, "one connected sea");
                for c in 0..s.flow.len() {
                    let mut at = c;
                    let mut steps = 0;
                    while let Some(next) = s.flow[at] {
                        assert!(
                            s.corner_neighbors(at).contains(&next),
                            "rivers use adjacent intersections"
                        );
                        at = next;
                        steps += 1;
                        assert!(steps < s.flow.len(), "drainage must terminate");
                    }
                }
            }
        }
    }

    #[test]
    fn same_seed_reproduces_geography_and_rng_state() {
        let mut a = Rng::new(31);
        let mut b = a.clone();
        let x = Layout::generate(40, 40, 0.002, &mut a);
        let y = Layout::generate(40, 40, 0.002, &mut b);
        assert_eq!(x.plots, y.plots);
        assert_eq!(x.flow, y.flow);
        assert_eq!(x.altitude, y.altitude);
        assert_eq!(x.rainfall, y.rainfall);
        assert_eq!(a, b);
        assert_ne!(
            x.plots,
            Layout::generate(40, 40, 0.002, &mut Rng::new(32)).plots
        );
    }
}
