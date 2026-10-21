//! Port of the other FFH2 0.41 AI code outside the AI classes: the Python hooks
//! `CvGameUtils.AI_unitUpdate` (CvGameUtils.py:2957-2995), `AI_MageTurn` (3190-3561),
//! `AI_MagePromotion` (3562-3968), `AI_Mage_UPGRADE_MANA` (3969-4357),
//! `CustomFunctions.warScript`/`warScriptAllow`/`dogpile`/`dogPileAllow` (CustomFunctions.py:
//! 1169-1265), and the AI functions of the rules classes: `CvUnit::chooseSpell`
//! (CvUnit.cpp:15917-16088), `CvUnit::AI_canJoinGroup` (CvUnit.cpp:18499),
//! `CvPlot::AI_sumStrength` (CvPlot.cpp:2792-2831), `CvPlot::AI_neededPermDefenseReserve`
//! (CvPlot.cpp:10960-10990), `CvPlot::AI_neededBetterDefender` (CvPlot.cpp:10992-11097).
//! (`CvPlayer::AI_isSummonSuicideMode` is `Player::sum_sui_mode`; `AI_doFavoriteReligion` and
//! `AI_doTowerMastery` are ported in ffh-core.)
#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]
use crate::prelude::*;

impl Cai {
    /// Python `CvGameUtils.AI_unitUpdate(argsList = (pUnit,))`: true = the Python moved the
    /// unit (the DLL's `AI_update` returns).
    pub fn py_ai_unit_update(&self, g: &mut Game, u: UnitId) -> bool {
        todo!("CvGameUtils.AI_unitUpdate")
    }

    /// Python `CvGameUtils.AI_MageTurn(argsList = (pUnit,))`: the `lResult` the DLL reads
    /// (1 = the unit acted).
    pub fn py_ai_mage_turn(&self, g: &mut Game, u: UnitId) -> i32 {
        todo!("CvGameUtils.AI_MageTurn")
    }

    /// Python `CvGameUtils.AI_MagePromotion(argsList = (pUnit,))`: the promotion to take
    /// (`None` = Python -1).
    pub fn py_ai_mage_promotion(&self, g: &Game, u: UnitId) -> Option<PromotionType> {
        todo!("CvGameUtils.AI_MagePromotion")
    }

    /// Python `CvGameUtils.AI_Mage_UPGRADE_MANA(argsList = (pUnit,))`: the `lResult` the DLL
    /// reads (1 = acted, 2 = become a terraformer).
    pub fn py_ai_mage_upgrade_mana(&self, g: &mut Game, u: UnitId) -> i32 {
        todo!("CvGameUtils.AI_Mage_UPGRADE_MANA")
    }

    /// Python `CustomFunctions.warScript(iPlayer)` (called by `onBeginPlayerTurn` for
    /// non-human players). `startWar` is ported in ffh-core (`cf_start_war`).
    pub fn py_war_script(&self, g: &mut Game, p: PlayerId) {
        todo!("CustomFunctions.warScript")
    }

    /// Python `CustomFunctions.warScriptAllow(iPlayer, iPlayer2)`.
    pub fn py_war_script_allow(&self, g: &Game, p: PlayerId, p2: PlayerId) -> bool {
        todo!("CustomFunctions.warScriptAllow")
    }

    /// Python `CustomFunctions.dogpile(iPlayer, iVictim)`.
    pub fn py_dogpile(&self, g: &mut Game, p: PlayerId, victim: PlayerId) {
        todo!("CustomFunctions.dogpile")
    }

    /// Python `CustomFunctions.dogPileAllow(iPlayer, iPlayer2)`.
    pub fn py_dog_pile_allow(&self, g: &Game, p: PlayerId, p2: PlayerId) -> bool {
        todo!("CustomFunctions.dogPileAllow")
    }

    /// C++ `int CvUnit::chooseSpell()` (CvUnit.cpp:15917): the spell to cast (`None` =
    /// NO_SPELL).
    pub fn unit_choose_spell(&self, g: &Game, u: UnitId) -> Option<SpellType> {
        todo!("CvUnit::chooseSpell")
    }

    /// C++ `bool CvUnit::AI_canJoinGroup(CvSelectionGroup* pSelectionGroup) const`
    /// (CvUnit.cpp:18499).
    pub fn unit_ai_can_join_group(&self, g: &Game, u: UnitId, grp: GroupId) -> bool {
        todo!("CvUnit::AI_canJoinGroup")
    }

    /// C++ `int CvPlot::AI_sumStrength(PlayerTypes eOwner, PlayerTypes eAttackingPlayer =
    /// NO_PLAYER, DomainTypes eDomainType = NO_DOMAIN, bool bDefensiveBonuses = true, bool
    /// bTestAtWar = false, bool bTestPotentialEnemy = false) const` (CvPlot.cpp:2792).
    pub fn plot_ai_sum_strength(&self, g: &Game, plot: PlotIdx, owner: PlayerId, attacking_player: Option<PlayerId>, domain: Option<DomainType>, defensive_bonuses: bool, test_at_war: bool, test_potential_enemy: bool) -> i32 {
        todo!("CvPlot::AI_sumStrength")
    }

    /// C++ `int CvPlot::AI_neededPermDefenseReserve(int flag)` (CvPlot.cpp:10960).
    pub fn plot_ai_needed_perm_defense_reserve(&self, g: &Game, plot: PlotIdx, flag: i32) -> i32 {
        todo!("CvPlot::AI_neededPermDefenseReserve")
    }

    /// C++ `int CvPlot::AI_neededBetterDefender(int type)` (CvPlot.cpp:10992).
    pub fn plot_ai_needed_better_defender(&self, g: &Game, plot: PlotIdx, type_: i32) -> i32 {
        todo!("CvPlot::AI_neededBetterDefender")
    }
}
