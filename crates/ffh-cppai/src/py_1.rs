//! Port of the FFH2 0.41 Python AI hooks `CvGameUtils.AI_chooseTech` (CvGameUtils.py:594-1239)
//! and `CvGameUtils.AI_chooseProduction` (CvGameUtils.py:1240-2956).
#![allow(unused_variables, unused_mut, unused_imports, clippy::all)]
use crate::prelude::*;

impl Cai {
    /// Python `CvGameUtils.AI_chooseTech(argsList = (ePlayer, bFree))` (CvGameUtils.py:594):
    /// the tech to research, or `None` (Python -1: the DLL then uses `AI_bestTech`).
    pub fn py_ai_choose_tech(&self, g: &Game, p: PlayerId, free: bool) -> Option<TechType> {
        todo!("CvGameUtils.AI_chooseTech")
    }

    /// Python `CvGameUtils.AI_chooseProduction(argsList = (pCity,))` (CvGameUtils.py:1240):
    /// true = the Python chose (pushed an order; the DLL returns), false = the DLL chooses.
    pub fn py_ai_choose_production(&self, g: &mut Game, c: CityId) -> bool {
        todo!("CvGameUtils.AI_chooseProduction")
    }
}
