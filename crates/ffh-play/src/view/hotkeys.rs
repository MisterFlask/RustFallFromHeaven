//! The action hotkeys and order priorities (design §7 P6 "hotkeys"): what the Civ4 interface
//! binds to each action and the order of the action buttons. The data has them (`hotkeys` and
//! `order_priority` of missions, commands, builds and automations: `ffh_data::Hotkey`); the
//! interface modes' bindings are listed in [`INTERFACE_MODES`] as FfH2's
//! `CIV4InterfaceModeInfos.xml` has them. The exe's dispatch of a key press is not in the DLL
//! source: the client takes, among the selection's enabled actions bound to the key, the one with
//! the highest hotkey priority, ties in the action list's order.

use crate::json::Json;

/// One binding: the key (`KB_F`, `KB_SPACE`, ...), the modifiers and the hotkey priority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub key: String,
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub priority: i32,
}

impl Binding {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "key" => self.key.as_str(), "alt" => self.alt, "shift" => self.shift, "ctrl" => self.ctrl, "priority" => self.priority }
    }
}

/// An action info's interface fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionKeys {
    pub bindings: Vec<Binding>,
    /// `iOrderPriority` (0 when absent, as `GetChildXmlValByName` leaves the default).
    pub order: i32,
}

impl ActionKeys {
    pub fn to_json(&self) -> Json {
        crate::jobj! { "keys" => Json::Arr(self.bindings.iter().map(Binding::to_json).collect()), "order" => self.order }
    }
}

/// The interface modes with a button (`bVisible`), their bindings (key, alt, shift, ctrl) and
/// hotkey priority as `Assets/XML/Interface/CIV4InterfaceModeInfos.xml` of FfH2 0.41 has them
/// (no `iOrderPriority` there).
pub const INTERFACE_MODES: &[(&str, &str, bool, bool, bool)] = &[
    ("INTERFACEMODE_GO_TO", "KB_G", false, false, false),
    ("INTERFACEMODE_GO_TO_TYPE", "KB_G", false, false, true),
    ("INTERFACEMODE_GO_TO_ALL", "KB_G", true, false, false),
    ("INTERFACEMODE_ROUTE_TO", "KB_R", true, false, false),
    ("INTERFACEMODE_AIRLIFT", "KB_A", false, true, false),
    ("INTERFACEMODE_NUKE", "KB_N", false, false, false),
    ("INTERFACEMODE_RECON", "KB_R", false, false, false),
    ("INTERFACEMODE_PARADROP", "KB_P", false, false, false),
    ("INTERFACEMODE_AIRBOMB", "KB_B", false, false, false),
    ("INTERFACEMODE_RANGE_ATTACK", "KB_B", false, false, false),
    ("INTERFACEMODE_AIRSTRIKE", "KB_S", false, false, false),
    ("INTERFACEMODE_REBASE", "KB_R", true, false, false),
];

/// The action keys of `kind` (`MissionType`, `CommandType`, `BuildType`, `AutomateType`) by type
/// name.
pub fn read(d: &ffh_data::GameData, kind: &str) -> Vec<(String, ActionKeys)> {
    fn keys(hotkeys: &[ffh_data::Hotkey], order: i32) -> ActionKeys {
        let bindings = hotkeys.iter().map(|b| Binding { key: b.key.clone(), alt: b.alt, shift: b.shift, ctrl: b.ctrl, priority: b.priority }).collect();
        ActionKeys { bindings, order }
    }
    match kind {
        "MissionType" => d.missions.iter_enumerated().map(|(t, m)| (d.reg.type_name(t).to_string(), keys(&m.hotkeys, m.order_priority))).collect(),
        "CommandType" => d.commands.iter_enumerated().map(|(t, c)| (d.reg.type_name(t).to_string(), keys(&c.hotkeys, c.order_priority))).collect(),
        "BuildType" => d.builds.iter_enumerated().map(|(t, b)| (d.reg.type_name(t).to_string(), keys(&b.hotkeys, b.order_priority))).collect(),
        "AutomateType" => d.automates.iter_enumerated().map(|(t, a)| (d.reg.type_name(t).to_string(), keys(&a.hotkeys, a.order_priority))).collect(),
        _ => Vec::new(),
    }
}

/// The interface modes for `/api/data`: `[{type, keys, order}]`.
pub fn interface_modes_json() -> Json {
    Json::Arr(
        INTERFACE_MODES
            .iter()
            .map(|&(ty, key, alt, shift, ctrl)| {
                let keys = ActionKeys { bindings: vec![Binding { key: key.into(), alt, shift, ctrl, priority: 0 }], order: 0 };
                keys.to_json().with("type", ty)
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_are_read_from_the_data() {
        let d = ffh_data::global();
        let missions = read(d, "MissionType");
        let get = |list: &[(String, ActionKeys)], t: &str| list.iter().find(|(n, _)| n == t).map(|(_, k)| k.clone()).unwrap_or_else(|| panic!("{t}"));
        let skip = get(&missions, "MISSION_SKIP");
        assert_eq!(skip.bindings[0], Binding { key: "KB_SPACE".into(), alt: false, shift: false, ctrl: false, priority: 1 });
        assert_eq!(skip.bindings[1].key, "KB_NUMPAD0");
        assert_eq!(skip.order, 13);
        assert_eq!(get(&missions, "MISSION_FORTIFY").bindings[0].key, "KB_F");
        assert_eq!(get(&missions, "MISSION_SENTRY").bindings[0].key, "KB_S");
        assert!(get(&missions, "MISSION_FOUND").bindings[0].priority > get(&missions, "MISSION_BOMBARD").bindings[0].priority);
        assert!(get(&missions, "MISSION_PILLAGE").bindings[0].shift);
        assert!(get(&missions, "MISSION_MOVE_TO").bindings.is_empty());
        // One entry per type.
        assert_eq!(missions.len(), crate::names::kind("MissionType").map(|k| (k.count)(d)).unwrap());
        let commands = read(d, "CommandType");
        assert_eq!(get(&commands, "COMMAND_DELETE").bindings[0].key, "KB_DELETE");
        assert!(get(&commands, "COMMAND_UNLOAD_ALL").bindings[0].shift);
        let automates = read(d, "AutomateType");
        assert_eq!(get(&automates, "AUTOMATE_BUILD").bindings[0].key, "KB_A");
        assert_eq!(get(&automates, "AUTOMATE_EXPLORE").bindings[0].key, "KB_E");
        assert_eq!(automates.len(), crate::names::kind("AutomateType").map(|k| (k.count)(d)).unwrap());
        let builds = read(d, "BuildType");
        assert_eq!(get(&builds, "BUILD_ROAD").bindings[0].key, "KB_R");
        assert!(get(&builds, "BUILD_REMOVE_FOREST").bindings[0].alt);
        assert_eq!(builds.len(), d.builds.len());
    }
}
