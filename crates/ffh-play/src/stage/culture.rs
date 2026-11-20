//! `ffh-play stage culture DIR`: the Culture screen's scene (docs/PERSONS.md, "The fourth build as
//! built, part 2A"; tools/web-culture-smoke.mjs). It is the `persons` stage ([`super::persons`]:
//! the human courts the neighbour's House, or its refusal is listed in `hand_set`; the neighbour
//! courts the human's House `pleased` past two thirds of its requirement) with one religion in
//! every city of the human, set by hand and listed in `hand_set`: so Spread Religion has targets,
//! and one of them refused (the human's own civilization, every city of which has it).

use std::path::Path;
use std::sync::Arc;

use ffh_data::*;

use super::StagedPersons;
use crate::human::AdvisorAnswerer;
use crate::session::{Session, SessionOptions};

/// Writes the scene into `dir` (a game directory) as `culture.ffhsave` (the `persons` stage's
/// save is written there too).
pub fn culture(dir: &Path) -> Result<StagedPersons, String> {
    let mut st = super::persons(dir, false)?;
    let (bytes, sidecar) = crate::save::read_bundle(&st.save)?;
    let opts = SessionOptions { saves_dir: Some(dir.to_path_buf()), autosave_keep: 0, journal: false, ..Default::default() };
    let mut s = Session::load(&bytes, sidecar.as_ref(), Arc::new(AdvisorAnswerer), opts).map_err(|e| format!("load: {e:?}"))?;
    let h = s.human;
    let g = &mut s.g;
    let d = g.d;
    let r = d.religions.keys().next().ok_or("no religion in the data")?;
    for c in g.players[h].cities.clone() {
        if g.cities.contains(c) {
            g.city_set_has_religion(c, r, true, false, false);
        }
    }
    st.hand_set.push(format!("{} in every city of the human", d.text.name(&d.religions[r].base.description)));
    // What the scene promises: Spread Religion has an allowed target and a refused one.
    let spread = g.culture_action(CultureEffect::SpreadReligion).ok_or("no Spread Religion action in the data")?;
    let options = g.culture_channel_options(h, spread, Some(r));
    if !options.iter().any(|o| o.refusal.is_some()) || !options.iter().any(|o| o.refusal.is_none()) {
        return Err(format!("Spread Religion's targets are not one refused and one allowed: {options:?}"));
    }
    let paths = s.save("culture").map_err(|e| format!("save: {e:?}"))?;
    st.save = paths.save;
    st.stage = "culture";
    Ok(st)
}
