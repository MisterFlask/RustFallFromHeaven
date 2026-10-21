//! Port of CvUnitAI.cpp:14123-16967 (FFH2 0.41 C++ AI).

#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]

use crate::prelude::*;

impl Cai {
    /// C++ `bool CvUnitAI::AI_specialSeaTransportMissionary()` (CvUnitAI.cpp:14124-14333)
    pub fn unit_ai_special_sea_transport_missionary(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_specialSeaTransportMissionary")
    }

    /// C++ `bool CvUnitAI::AI_specialSeaTransportSpy()` (CvUnitAI.cpp:14337-14448)
    pub fn unit_ai_special_sea_transport_spy(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_specialSeaTransportSpy")
    }

    /// C++ `bool CvUnitAI::AI_carrierSeaTransport()` (CvUnitAI.cpp:14452-14632)
    pub fn unit_ai_carrier_sea_transport(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_carrierSeaTransport")
    }

    /// C++ `bool CvUnitAI::AI_connectPlot(CvPlot* pPlot, int iRange = 0)` (CvUnitAI.cpp:14636-14695)
    pub fn unit_ai_connect_plot(&self, g: &mut Game, u: UnitId, plot: PlotIdx, range: i32) -> bool {
        todo!("CvUnitAI::AI_connectPlot")
    }

    /// C++ `bool CvUnitAI::AI_improveCity(CvCity* pCity)` (CvUnitAI.cpp:14699-14751)
    pub fn unit_ai_improve_city(&self, g: &mut Game, u: UnitId, city: CityId) -> bool {
        todo!("CvUnitAI::AI_improveCity")
    }

    /// C++ `bool CvUnitAI::AI_improveLocalPlot(int iRange, CvCity* pIgnoreCity)` (CvUnitAI.cpp:14753-14894)
    pub fn unit_ai_improve_local_plot(&self, g: &mut Game, u: UnitId, range: i32, ignore_city: CityId) -> bool {
        todo!("CvUnitAI::AI_improveLocalPlot")
    }

    /// C++ `bool CvUnitAI::AI_nextCityToImprove(CvCity* pCity)` (CvUnitAI.cpp:14897-14980)
    pub fn unit_ai_next_city_to_improve(&self, g: &mut Game, u: UnitId, city: CityId) -> bool {
        todo!("CvUnitAI::AI_nextCityToImprove")
    }

    /// C++ `bool CvUnitAI::AI_nextCityToImproveAirlift()` (CvUnitAI.cpp:14984-15040)
    pub fn unit_ai_next_city_to_improve_airlift(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_nextCityToImproveAirlift")
    }

    /// C++ `bool CvUnitAI::AI_irrigateTerritory()` (CvUnitAI.cpp:15044-15186)
    pub fn unit_ai_irrigate_territory(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_irrigateTerritory")
    }

    /// C++ `bool CvUnitAI::AI_fortTerritory(bool bCanal, bool bAirbase)` (CvUnitAI.cpp:15188-15306)
    pub fn unit_ai_fort_territory(&self, g: &mut Game, u: UnitId, canal: bool, airbase: bool) -> bool {
        todo!("CvUnitAI::AI_fortTerritory")
    }

    /// C++ `bool CvUnitAI::AI_improveBonus(int iMinValue = 0, CvPlot** ppBestPlot = NULL, BuildTypes* peBestBuild = NULL, int* piBestValue = NULL)` (CvUnitAI.cpp:15309-15607)
    pub fn unit_ai_improve_bonus(&self, g: &mut Game, u: UnitId, min_value: i32, best_plot: &mut Option<PlotIdx>, best_build: &mut Option<BuildType>, best_value: &mut i32) -> bool {
        todo!("CvUnitAI::AI_improveBonus")
    }

    /// C++ `bool CvUnitAI::AI_improvePlot(CvPlot* pPlot, BuildTypes eBuild)` (CvUnitAI.cpp:15611-15638)
    pub fn unit_ai_improve_plot(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: BuildType) -> bool {
        todo!("CvUnitAI::AI_improvePlot")
    }

    /// C++ `BuildTypes CvUnitAI::AI_betterPlotBuild(CvPlot* pPlot, BuildTypes eBuild)` (CvUnitAI.cpp:15640-15796)
    pub fn unit_ai_better_plot_build(&self, g: &mut Game, u: UnitId, plot: PlotIdx, build: BuildType) -> Option<BuildType> {
        todo!("CvUnitAI::AI_betterPlotBuild")
    }

    /// C++ `bool CvUnitAI::AI_connectBonus(bool bTestTrade = true)` (CvUnitAI.cpp:15800-15838)
    pub fn unit_ai_connect_bonus(&self, g: &mut Game, u: UnitId, test_trade: bool) -> bool {
        todo!("CvUnitAI::AI_connectBonus")
    }

    /// C++ `bool CvUnitAI::AI_connectCity()` (CvUnitAI.cpp:15842-15881)
    pub fn unit_ai_connect_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_connectCity")
    }

    /// C++ `bool CvUnitAI::AI_routeCity()` (CvUnitAI.cpp:15885-15927)
    pub fn unit_ai_route_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_routeCity")
    }

    /// C++ `bool CvUnitAI::AI_routeTerritory(bool bImprovementOnly = false)` (CvUnitAI.cpp:15931-16023)
    pub fn unit_ai_route_territory(&self, g: &mut Game, u: UnitId, improvement_only: bool) -> bool {
        todo!("CvUnitAI::AI_routeTerritory")
    }

    /// C++ `bool CvUnitAI::AI_travelToUpgradeCity()` (CvUnitAI.cpp:16027-16134)
    pub fn unit_ai_travel_to_upgrade_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_travelToUpgradeCity")
    }

    /// C++ `bool CvUnitAI::AI_retreatToCity(bool bPrimary = false, bool bAirlift = false, int iMaxPath = MAX_INT)` (CvUnitAI.cpp:16137-16263)
    pub fn unit_ai_retreat_to_city(&self, g: &mut Game, u: UnitId, primary: bool, airlift: bool, max_path: i32) -> bool {
        todo!("CvUnitAI::AI_retreatToCity")
    }

    /// C++ `bool CvUnitAI::AI_pickup(UnitAITypes eUnitAI)` (CvUnitAI.cpp:16267-16363)
    pub fn unit_ai_pickup(&self, g: &mut Game, u: UnitId, unit_ai: UnitAIType) -> bool {
        todo!("CvUnitAI::AI_pickup")
    }

    /// C++ `bool CvUnitAI::AI_airOffensiveCity()` (CvUnitAI.cpp:16367-16484)
    pub fn unit_ai_air_offensive_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airOffensiveCity")
    }

    /// C++ `bool CvUnitAI::AI_airDefensiveCity()` (CvUnitAI.cpp:16488-16550)
    pub fn unit_ai_air_defensive_city(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airDefensiveCity")
    }

    /// C++ `bool CvUnitAI::AI_airCarrier()` (CvUnitAI.cpp:16554-16634)
    pub fn unit_ai_air_carrier(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airCarrier")
    }

    /// C++ `bool CvUnitAI::AI_missileLoad(UnitAITypes eTargetUnitAI, int iMaxOwnUnitAI = -1, bool bStealthOnly = false)` (CvUnitAI.cpp:16636-16688)
    pub fn unit_ai_missile_load(&self, g: &mut Game, u: UnitId, target_unit_ai: UnitAIType, max_own_unit_ai: i32, stealth_only: bool) -> bool {
        todo!("CvUnitAI::AI_missileLoad")
    }

    /// C++ `bool CvUnitAI::AI_airStrike()` (CvUnitAI.cpp:16692-16785)
    pub fn unit_ai_air_strike(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airStrike")
    }

    /// C++ `bool CvUnitAI::AI_airBombPlots()` (CvUnitAI.cpp:16787-16873)
    pub fn unit_ai_air_bomb_plots(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airBombPlots")
    }

    /// C++ `bool CvUnitAI::AI_airBombDefenses()` (CvUnitAI.cpp:16876-16967)
    pub fn unit_ai_air_bomb_defenses(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvUnitAI::AI_airBombDefenses")
    }

}
