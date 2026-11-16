//! Erebus Terra: two regional continents, Old World starts and exclusive overseas resources.
use super::*;
use crate::ids::AreaId;

/// Luxuries and a late strategic resource; early food, metals, horses and reagents remain normal.
const RESERVED: [&str; 4] = ["BONUS_GEMS", "BONUS_SILK", "BONUS_INCENSE", "BONUS_MITHRIL"];

fn habitable(s: &Layout) -> usize {
    s.plots
        .iter()
        .filter(|&&k| matches!(k, PlotType::PLOT_LAND | PlotType::PLOT_HILLS))
        .count()
}

impl Layout {
    fn continent(side: i32, rng: &mut Rng) -> Self {
        let n = (side * side) as usize;
        let mut s = Self {
            w: side,
            h: side,
            regions: Vec::new(),
            tiles: vec![-1; n],
            plots: vec![PlotType::PLOT_OCEAN; n],
            flow: vec![None; ((side + 1) * (side + 1)) as usize],
            runoff: vec![0.0; ((side + 1) * (side + 1)) as usize],
            altitude: vec![0.0; n],
            rainfall: vec![0.0; n],
            old_world: None,
        };
        s.grow_regions(rng);
        // Offshore regions surround an irregular interior of valleys. Unlike ordinary Erebus,
        // the drainage roots are the surrounding ocean rather than a central sea.
        let distance = |r: &Region| {
            let dx = (r.seed.0 as f64 - side as f64 * 0.5) / (side as f64 * 0.37);
            let dy = (r.seed.1 as f64 - side as f64 * 0.5) / (side as f64 * 0.42);
            dx * dx + dy * dy
        };
        for r in &mut s.regions {
            r.water = distance(r) > 1.0;
        }
        if s.regions.iter().all(|r| r.water) {
            let center = (0..s.regions.len())
                .min_by(|&a, &b| distance(&s.regions[a]).total_cmp(&distance(&s.regions[b])))
                .unwrap();
            s.regions[center].water = false;
        }
        if s.regions.iter().all(|r| !r.water) {
            let outside = (0..s.regions.len())
                .max_by(|&a, &b| distance(&s.regions[a]).total_cmp(&distance(&s.regions[b])))
                .unwrap();
            s.regions[outside].water = true;
        }
        let mut q: VecDeque<_> = s
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, &r)| (r >= 0 && s.regions[r as usize].water).then_some(i))
            .collect();
        while let Some(i) = q.pop_front() {
            let (x, y) = (i as i32 % side, i as i32 / side);
            for (dx, dy) in CARDINAL {
                if let Some(j) = index(x + dx, y + dy, side, side) {
                    if s.tiles[j] < 0 {
                        s.tiles[j] = s.tiles[i];
                        q.push_back(j);
                    }
                }
            }
        }
        s.drainage(rng);
        s.plot_types(rng);
        // An ocean rim makes the continents fully separate even on the nonwrapping board.
        for i in 0..n {
            let (x, y) = (i as i32 % side, i as i32 / side);
            if x == 0 || y == 0 || x == side - 1 || y == side - 1 {
                s.plots[i] = PlotType::PLOT_OCEAN;
            }
        }
        // Strip offshore rock fragments: there are exactly two land areas in the combined map.
        let mut seen = vec![false; n];
        let mut largest = Vec::new();
        for start in 0..n {
            if seen[start] || s.plots[start] == PlotType::PLOT_OCEAN {
                continue;
            }
            let mut component = Vec::new();
            let mut q = VecDeque::from([start]);
            seen[start] = true;
            while let Some(i) = q.pop_front() {
                component.push(i);
                let (x, y) = (i as i32 % side, i as i32 / side);
                for (dx, dy) in AROUND {
                    if let Some(j) = index(x + dx, y + dy, side, side) {
                        if !seen[j] && s.plots[j] != PlotType::PLOT_OCEAN {
                            seen[j] = true;
                            q.push_back(j);
                        }
                    }
                }
            }
            if component.len() > largest.len() {
                largest = component;
            }
        }
        let mut keep = vec![false; n];
        for i in largest {
            keep[i] = true;
        }
        for (i, &land) in keep.iter().enumerate() {
            if !land {
                s.plots[i] = PlotType::PLOT_OCEAN;
            }
        }
        s
    }

    pub(super) fn generate_terra(w: i32, h: i32, rng: &mut Rng) -> Self {
        let mut old = Self::continent(h, rng);
        // Keep the starting continent roomy even for a sparse coastal draw.
        for _ in 0..12 {
            if habitable(&old) >= (h * h / 4) as usize {
                break;
            }
            let next = Self::continent(h, rng);
            if habitable(&next) > habitable(&old) {
                old = next;
            }
        }
        let capacity = w - h - 12;
        let target = habitable(&old) / 3;
        let mut side = ((h as f64 / 3_f64.sqrt()).round() as i32).clamp(12, capacity);
        let mut new = Self::continent(side, rng);
        let mut error = habitable(&new).abs_diff(target);
        for _ in 0..24 {
            if error <= (target / 8).max(3) {
                break;
            }
            let count = habitable(&new).max(1);
            side = ((side as f64 * (target as f64 / count as f64).sqrt()).round() as i32)
                .clamp(12, capacity);
            let candidate = Self::continent(side, rng);
            let e = habitable(&candidate).abs_diff(target);
            if e < error {
                new = candidate;
                error = e;
            }
            // Estimate from the retained best candidate, not a failed draw's dimensions.
            side = new.w;
        }
        let minimum = habitable(&old) * 28 / 100;
        while habitable(&new) < minimum {
            let shoulder = (0..new.plots.len())
                .filter(|&i| new.plots[i] == PlotType::PLOT_PEAK)
                .find(|&i| {
                    let (x, y) = (i as i32 % new.w, i as i32 / new.w);
                    AROUND.iter().any(|&(dx, dy)| {
                        index(x + dx, y + dy, new.w, new.h).is_some_and(|j| {
                            matches!(new.plots[j], PlotType::PLOT_LAND | PlotType::PLOT_HILLS)
                        })
                    })
                });
            let Some(i) = shoulder else { break };
            new.plots[i] = PlotType::PLOT_HILLS;
        }
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
            old_world: Some(vec![false; n]),
        };
        for (part, ox, oy, main) in [
            (&old, 0, 0, true),
            (
                &new,
                h + 12 + (capacity - new.w) / 2,
                (h - new.h) / 2,
                false,
            ),
        ] {
            for y in 0..part.h {
                for x in 0..part.w {
                    let a = (y * part.w + x) as usize;
                    let b = ((y + oy) * w + x + ox) as usize;
                    s.plots[b] = part.plots[a];
                    s.altitude[b] = part.altitude[a];
                    s.rainfall[b] = part.rainfall[a];
                    s.old_world.as_mut().unwrap()[b] =
                        main && part.plots[a] != PlotType::PLOT_OCEAN;
                }
            }
            for y in 0..=part.h {
                for x in 0..=part.w {
                    let a = (y * (part.w + 1) + x) as usize;
                    let b = ((y + oy) * (w + 1) + x + ox) as usize;
                    s.runoff[b] = part.runoff[a];
                    if let Some(to) = part.flow[a] {
                        let (tx, ty) = (to as i32 % (part.w + 1), to as i32 / (part.w + 1));
                        s.flow[b] = Some(((ty + oy) * (w + 1) + tx + ox) as usize);
                    }
                }
            }
        }
        if rng.get(2, "Erebus Terra orientation") == 0 {
            for row in s.plots.chunks_mut(w as usize) {
                row.reverse();
            }
            for row in s.altitude.chunks_mut(w as usize) {
                row.reverse();
            }
            for row in s.rainfall.chunks_mut(w as usize) {
                row.reverse();
            }
            for row in s.old_world.as_mut().unwrap().chunks_mut(w as usize) {
                row.reverse();
            }
            for row in s.runoff.chunks_mut((w + 1) as usize) {
                row.reverse();
            }
            let mirror = |c: usize| {
                c / (w + 1) as usize * (w + 1) as usize + w as usize - c % (w + 1) as usize
            };
            let mut flow = vec![None; s.flow.len()];
            for (from, to) in s.flow.iter().enumerate() {
                flow[mirror(from)] = to.map(mirror);
            }
            s.flow = flow;
        }
        s
    }
}

pub(super) fn old_world_area(g: &Game) -> Option<AreaId> {
    g.map
        .areas
        .iter()
        .filter(|(_, a)| !a.water)
        .map(|(a, _)| {
            let count = g
                .map
                .plot_ids()
                .filter(|&p| g.map.plots[p].area == Some(a) && !g.plot_is_peak(p))
                .count();
            (a, count)
        })
        .max_by_key(|&(_, n)| n)
        .map(|(a, _)| a)
}

pub(super) fn place_reserved_type(g: &mut Game, bonus: BonusType) -> bool {
    // Skip ordinary placement. Guarantees are placed after features, starts and their
    // normalization have settled; none can leak back into a starting city radius.
    RESERVED.contains(&g.d.type_name(bonus))
}

pub(super) fn finish_resources(g: &mut Game) {
    let Some(old) = old_world_area(g) else { return };
    let new: Vec<_> = g
        .map
        .plot_ids()
        .filter(|&p| !g.plot_is_water(p) && !g.plot_is_peak(p) && g.map.plots[p].area != Some(old))
        .collect();
    for name in RESERVED {
        let bonus = g.d.t::<BonusType>(name);
        let old_copies: Vec<_> = g
            .map
            .plot_ids()
            .filter(|&p| {
                g.map.plots[p].area == Some(old) && g.map.plots[p].bonus_type == Some(bonus)
            })
            .collect();
        for p in &old_copies {
            g.plot_set_bonus_type(*p, None);
        }
        let existing = new
            .iter()
            .filter(|&&p| g.map.plots[p].bonus_type == Some(bonus))
            .count();
        let target = (new.len() / 160).max(2).max(existing + old_copies.len());
        for _ in existing..target {
            let mut fits: Vec<_> = new
                .iter()
                .copied()
                .filter(|&p| {
                    g.map.plots[p].improvement_type.is_none()
                        && g.plot_can_have_bonus(p, Some(bonus), true)
                })
                .collect();
            if fits.is_empty() {
                let empty: Vec<_> = new
                    .iter()
                    .copied()
                    .filter(|&p| {
                        g.map.plots[p].bonus_type.is_none()
                            && g.map.plots[p].improvement_type.is_none()
                    })
                    .collect();
                if empty.is_empty() {
                    break;
                }
                let p = empty[g.map_rand_num(empty.len() as i32, "Terra resource biome") as usize];
                g.plot_set_feature_type(p, None);
                g.plot_set_terrain_type(
                    p,
                    g.d.t(if name == "BONUS_INCENSE" {
                        "TERRAIN_PLAINS"
                    } else {
                        "TERRAIN_GRASS"
                    }),
                    false,
                );
                g.plot_set_plot_type(
                    p,
                    if name == "BONUS_MITHRIL" {
                        PlotType::PLOT_HILLS
                    } else {
                        PlotType::PLOT_LAND
                    },
                    false,
                );
                let feature = match name {
                    "BONUS_GEMS" => Some("FEATURE_JUNGLE"),
                    "BONUS_SILK" => Some("FEATURE_FOREST"),
                    _ => None,
                };
                if let Some(f) = feature {
                    g.plot_set_feature_type(p, Some(g.d.t(f)));
                }
                if g.plot_can_have_bonus(p, Some(bonus), true) {
                    fits.push(p);
                }
            }
            if fits.is_empty() {
                break;
            }
            let p = fits[g.map_rand_num(fits.len() as i32, "Terra overseas resource") as usize];
            g.plot_set_bonus_type(p, Some(bonus));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_continents_have_a_small_habitable_new_world_and_a_deep_ocean_gap() {
        for h in [28, 32, 40, 52, 88, 132] {
            for seed in 1..=40 {
                let w = h + (h * 3 + 3) / 4 + 12;
                let s = Layout::generate_terra(w, h, &mut Rng::new(seed));
                let mask = s.old_world.as_ref().unwrap();
                let old = s
                    .plots
                    .iter()
                    .enumerate()
                    .filter(|&(i, k)| {
                        mask[i] && matches!(*k, PlotType::PLOT_LAND | PlotType::PLOT_HILLS)
                    })
                    .count();
                let new = s
                    .plots
                    .iter()
                    .enumerate()
                    .filter(|&(i, k)| {
                        !mask[i] && matches!(*k, PlotType::PLOT_LAND | PlotType::PLOT_HILLS)
                    })
                    .count();
                assert!(
                    new * 100 >= old * 27 && new * 100 <= old * 43,
                    "habitable ratio: {old}/{new}, side {h}, seed {seed}"
                );
                assert!(new >= 30);
                let mut seen = vec![false; s.plots.len()];
                let mut components = 0;
                for start in 0..s.plots.len() {
                    if seen[start] || s.plots[start] == PlotType::PLOT_OCEAN {
                        continue;
                    }
                    components += 1;
                    let main = mask[start];
                    let mut queue = VecDeque::from([start]);
                    seen[start] = true;
                    while let Some(i) = queue.pop_front() {
                        assert_eq!(mask[i], main, "no land bridge between worlds");
                        let (x, y) = (i as i32 % w, i as i32 / w);
                        for (dx, dy) in AROUND {
                            if let Some(j) = index(x + dx, y + dy, w, h) {
                                if !seen[j] && s.plots[j] != PlotType::PLOT_OCEAN {
                                    seen[j] = true;
                                    queue.push_back(j);
                                }
                            }
                        }
                    }
                }
                assert_eq!(components, 2);
                let ocean_columns = (0..w)
                    .filter(|&x| {
                        (0..h).all(|y| s.plots[(y * w + x) as usize] == PlotType::PLOT_OCEAN)
                    })
                    .count();
                assert!(
                    ocean_columns >= 12,
                    "deep-water crossing, no coastal stepping stones"
                );
            }
        }
    }

    #[test]
    fn terra_is_seeded_and_reproducible() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        let x = Layout::generate_terra(82, 40, &mut a);
        let y = Layout::generate_terra(82, 40, &mut b);
        assert_eq!(x.plots, y.plots);
        assert_eq!(x.flow, y.flow);
        assert_eq!(x.old_world, y.old_world);
        assert_eq!(a, b);
        assert_ne!(
            x.plots,
            Layout::generate_terra(82, 40, &mut Rng::new(8)).plots
        );
    }
}
