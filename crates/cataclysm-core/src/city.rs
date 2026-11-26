//! Macro cities: Size 1..10, 4-6 structural District slots, zero Worker units (automatic Work Crews),
//! persistent Wall Sentinels, Raids/Sallies, and Lifeboat Wonders.

use crate::data::{DistrictKind, UnitClassId, WonderKind};
use crate::ids::{CityId, HouseId, PlayerId, PlotIdx};
use crate::world::{Improvement, PlotKind, Resource, Terrain, World};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SallyPosture {
    Never,
    Favoured,
    Always,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildOrder {
    District(DistrictKind),
    Squad(Vec<UnitClassId>),
    Reinforce(UnitClassId),
    Caravan,
    Wonder(WonderKind),
}

impl BuildOrder {
    pub fn cost(&self) -> i32 {
        match self {
            BuildOrder::District(d) => d.info().cost,
            BuildOrder::Squad(classes) => classes.iter().map(|c| c.info().cost_industry).sum::<i32>().max(30),
            BuildOrder::Reinforce(c) => c.info().cost_industry,
            BuildOrder::Caravan => 45,
            BuildOrder::Wonder(w) => w.cost(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            BuildOrder::District(d) => format!("District: {}", d.info().name),
            BuildOrder::Squad(classes) => {
                if let Some(first) = classes.first() {
                    format!("Squad ({} x{})", first.info().name, classes.len())
                } else {
                    "Squad".into()
                }
            }
            BuildOrder::Reinforce(c) => format!("Recruit: {}", c.info().name),
            BuildOrder::Caravan => "Trade Caravan".into(),
            BuildOrder::Wonder(w) => format!("Wonder: {}", w.name()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkCrewTask {
    pub plot: PlotIdx,
    pub build_road: bool,
    pub improvement: Improvement,
    pub progress: i32,
    pub needed: i32,
}

#[derive(Clone, Debug)]
pub struct City {
    pub id: CityId,
    pub owner: PlayerId,
    pub original_owner: PlayerId,
    pub name: String,
    pub plot: PlotIdx,
    pub capital: bool,
    pub is_carcosa: bool,
    pub is_outpost: bool,
    pub size: i32,
    pub food_stored: i32,
    pub culture: i32,
    pub districts: Vec<DistrictKind>,
    pub wonders: Vec<WonderKind>,
    pub house: Option<HouseId>,
    /// Persistent damage (0..=100; 100 = breached) per Sentinel wall section.
    pub sentinel_damage: Vec<i32>,
    /// True if an enemy squad stands adjacent (stops Sentinel repair and city healing).
    pub besieged: bool,
    /// Turns raided continuously; at 5, countryside is stripped for 10 turns.
    pub raid_streak: i32,
    pub stripped_turns: i32,
    pub sally: SallyPosture,
    pub building: Option<BuildOrder>,
    pub build_progress: i32,
    pub crew_task: Option<WorkCrewTask>,
    /// Optional rally plot for newly trained squads.
    pub waypoint: Option<PlotIdx>,
}

impl City {
    pub fn new(id: CityId, owner: PlayerId, name: String, plot: PlotIdx, capital: bool, is_outpost: bool) -> Self {
        Self {
            id,
            owner,
            original_owner: owner,
            name,
            plot,
            capital,
            is_carcosa: false,
            is_outpost,
            size: if capital { 2 } else { 1 },
            food_stored: 0,
            culture: if capital { 15 } else { 0 },
            districts: Vec::new(),
            wonders: Vec::new(),
            house: None,
            sentinel_damage: if capital { vec![0] } else { Vec::new() },
            besieged: false,
            raid_streak: 0,
            stripped_turns: 0,
            sally: SallyPosture::Favoured,
            building: None,
            build_progress: 0,
            crew_task: None,
            waypoint: None,
        }
    }

    pub fn has_district(&self, kind: DistrictKind) -> bool {
        self.districts.contains(&kind)
    }

    pub fn has_wonder(&self, kind: WonderKind) -> bool {
        self.wonders.contains(&kind)
    }

    /// Structural District slots unlocked by Size (max 6 for capitals/Megacities, 4-5 for normal).
    pub fn max_district_slots(&self, megacity: bool) -> usize {
        if self.is_outpost {
            return 1;
        }
        let cap = if megacity || self.capital { 6 } else { 4 };
        ((self.size + 1) / 2 + 1).clamp(2, cap) as usize
    }

    pub fn food_needed(&self) -> i32 {
        18 + self.size * 12
    }

    pub fn max_sentinels(&self, has_masonry_wards: bool) -> usize {
        let mut n = if self.capital { 1 } else { 0 };
        if self.has_district(DistrictKind::Bulwark) {
            n = if has_masonry_wards { 3 } else { 2 };
        }
        if self.is_carcosa {
            n = n.max(3);
        }
        n
    }

    pub fn sync_sentinels(&mut self, has_masonry_wards: bool) {
        let target = self.max_sentinels(has_masonry_wards);
        while self.sentinel_damage.len() < target {
            self.sentinel_damage.push(if self.besieged { 100 } else { 0 });
        }
        self.sentinel_damage.truncate(target);
    }

    pub fn standing_sentinels(&self) -> usize {
        self.sentinel_damage.iter().filter(|&&d| d < 100).count()
    }

    pub fn defense_pct(&self) -> i32 {
        let mut d = 15 + (self.culture / 40).min(45);
        if self.has_district(DistrictKind::Bulwark) {
            d += 40;
        }
        if self.is_carcosa {
            d += 50;
        }
        d
    }

    /// Automatically picks the highest-value unimproved countryside tile or road for this city's Work Crew.
    pub fn pick_crew_task(&self, world: &World) -> Option<WorkCrewTask> {
        let mut best: Option<(i32, WorkCrewTask)> = None;
        let center = self.plot;
        let (cx, cy) = (world.plot(center).x, world.plot(center).y);
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let Some(p) = world.idx(cx + dx, cy + dy) else { continue };
                let pl = world.plot(p);
                if pl.owner != Some(self.owner) || pl.city.is_some() {
                    continue;
                }
                // Priority 1: connect adjacent tiles with roads.
                if pl.kind.is_passable_land() && !pl.road && dx.abs().max(dy.abs()) == 1 {
                    let score = 40;
                    if best.as_ref().is_none_or(|(s, _)| score > *s) {
                        best = Some((score, WorkCrewTask { plot: p, build_road: true, improvement: Improvement::None, progress: 0, needed: 3 }));
                    }
                }
                // Priority 2: improve unimproved tiles.
                if pl.improvement == Improvement::None {
                    let imp = if pl.kind == PlotKind::Coast && pl.resource == Resource::Fish {
                        Some((Improvement::FishingBoats, 70))
                    } else if !pl.kind.is_passable_land() {
                        None
                    } else if pl.resource.is_mana() {
                        Some((Improvement::ManaNode, 90))
                    } else if matches!(pl.resource, Resource::Iron | Resource::Mithril | Resource::GoldOre | Resource::Gems) || pl.kind == PlotKind::Hills {
                        Some((Improvement::Mine, 65))
                    } else if matches!(pl.resource, Resource::Cow | Resource::Horse) {
                        Some((Improvement::Pasture, 65))
                    } else if pl.feature.is_woods() {
                        Some((Improvement::Lumbermill, 50))
                    } else if pl.terrain == Terrain::Grass || pl.resource == Resource::Wheat {
                        Some((Improvement::Farm, 55))
                    } else if pl.terrain == Terrain::Plains {
                        Some((Improvement::Cottage, 48))
                    } else {
                        None
                    };
                    if let Some((imp, score)) = imp {
                        if best.as_ref().is_none_or(|(s, _)| score > *s) {
                            best = Some((score, WorkCrewTask { plot: p, build_road: false, improvement: imp, progress: 0, needed: 4 }));
                        }
                    }
                }
            }
        }
        best.map(|(_, t)| t)
    }
}
