//! This game's own battle magic of unit types and promotions (docs/CARDS.md, section 2, "New data
//! keys"; the great-person deck's battle cards): how a unit's spells, presence and hits change a
//! formation battle (docs/ENGAGEMENTS.md). One block, [`BattleMagic`], read the same way for a
//! unit type (`units.jsonc`) and a promotion (`promotions.jsonc`): a unit has what its type and its
//! promotions have.

use crate::enums::*;
use crate::infos::AttackPattern;
use crate::registry::Rd;

/// A pre-battle hex of a unit's presence (`battle_presence`): it takes when the unit is in the
/// battle's line at the start, unless the enemy side has a hex ward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    /// `GROUNDING`: for the battle, the enemy's charges are strikes (front reach, no first-round
    /// bonus).
    Grounding,
    /// `MADNESS`: in round 1 the enemy's unit with the highest attack (not a Sentinel) hits its
    /// own side.
    Madness,
}

impl Presence {
    pub const ALL: [Presence; 2] = [Presence::Grounding, Presence::Madness];

    pub fn token(self) -> &'static str {
        match self {
            Presence::Grounding => "GROUNDING",
            Presence::Madness => "MADNESS",
        }
    }

    pub fn parse(s: &str) -> Option<Presence> {
        Presence::ALL.into_iter().find(|p| p.token() == s.trim())
    }
}

/// A unit type's or promotion's battle magic (module comment). Every field is 0, false or `None`
/// without its key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BattleMagic {
    /// `battle_spell_power`: percent more damage of its damage spells in a battle, the magic bolt
    /// included.
    pub spell_power: i32,
    /// `battle_damage_only`: in a battle it casts no charm, hex or heal.
    pub damage_only: bool,
    /// `battle_spell_pattern`: its damage spells hit with this pattern in place of their own.
    pub spell_pattern: Option<AttackPattern>,
    /// `battle_front_caster`: from the front row it casts as many spells as from its best row.
    pub front_caster: bool,
    /// `battle_hex_ward`: while it is on the field, the enemy's hexes have no effect on its side
    /// (the pre-battle hexes of `battle_presence` included).
    pub hex_ward: bool,
    /// `battle_end_charms`: while it is on the field, the enemy side's charms end at the start of
    /// each round after the first.
    pub end_charms: bool,
    /// `battle_presence`: a pre-battle hex of its presence.
    pub presence: Option<Presence>,
    /// `battle_revive`: once per battle, its heal also brings back the first unit of its side that
    /// fell in the battle, at this percent of its HP (never a Sentinel).
    pub revive: i32,
    /// `battle_dispel`: each hit it lands ends the target's charms and the battle effects of its
    /// dispellable promotions for the rest of the battle.
    pub dispel: bool,
    /// `battle_damage_vs_summons`: percent more damage to summoned units (a unit with a duration, a
    /// battle's illusions).
    pub damage_vs_summons: i32,
    /// `battle_spell`: it knows this battle spell, whatever the spell's own prerequisites. A unit
    /// holds at most one promotion with it.
    pub spell: Option<SpellType>,
    /// `strike_spell`: in place of its first strike each round it casts this spell, with half its
    /// attack as its magic. A unit holds at most one promotion with it.
    pub strike_spell: Option<SpellType>,
    /// `opening_cast`: when its plot is attacked it casts its damage spell before the battle's
    /// first round (the magic bolt when it knows none); the promotion is then lost.
    pub opening_cast: bool,
    /// `death_recall`: when a hit would kill it in a battle, it leaves the battle and returns to
    /// its owner's capital with 1 HP; the promotion is then lost.
    pub death_recall: bool,
    /// `battle_familiar` (a unit type): it binds to a caster in its stack; in a battle it takes the
    /// first spell or shot aimed at that caster, and the caster gains 1 XP a turn while they
    /// share a plot.
    pub familiar: bool,
}

impl BattleMagic {
    /// `a` with `b` added: numbers summed, flags or-ed, the first named spell, pattern and
    /// presence kept.
    pub fn plus(self, b: &BattleMagic) -> BattleMagic {
        BattleMagic {
            spell_power: self.spell_power + b.spell_power,
            damage_only: self.damage_only || b.damage_only,
            spell_pattern: self.spell_pattern.or(b.spell_pattern),
            front_caster: self.front_caster || b.front_caster,
            hex_ward: self.hex_ward || b.hex_ward,
            end_charms: self.end_charms || b.end_charms,
            presence: self.presence.or(b.presence),
            revive: self.revive.max(b.revive),
            dispel: self.dispel || b.dispel,
            damage_vs_summons: self.damage_vs_summons + b.damage_vs_summons,
            spell: self.spell.or(b.spell),
            strike_spell: self.strike_spell.or(b.strike_spell),
            opening_cast: self.opening_cast || b.opening_cast,
            death_recall: self.death_recall || b.death_recall,
            familiar: self.familiar || b.familiar,
        }
    }

    /// Whether it has any key.
    pub fn any(&self) -> bool {
        *self != BattleMagic::default()
    }
}

/// The [`BattleMagic`] keys of an entry (`what` names it in warnings).
pub fn read_battle_magic(rd: &Rd, what: &str) -> BattleMagic {
    let pattern = rd.has("battle_spell_pattern").then(|| rd.string("battle_spell_pattern")).and_then(|t| {
        let p = AttackPattern::parse(&t);
        if p.is_none() {
            rd.cx.warn(format!("{what}: unknown battle_spell_pattern {t:?}"));
        }
        p
    });
    let presence = rd.has("battle_presence").then(|| rd.string("battle_presence")).and_then(|t| {
        let p = Presence::parse(&t);
        if p.is_none() {
            rd.cx.warn(format!("{what}: unknown battle_presence {t:?}"));
        }
        p
    });
    BattleMagic {
        spell_power: rd.int("battle_spell_power"),
        damage_only: rd.bool("battle_damage_only"),
        spell_pattern: pattern,
        front_caster: rd.bool("battle_front_caster"),
        hex_ward: rd.bool("battle_hex_ward"),
        end_charms: rd.bool("battle_end_charms"),
        presence,
        revive: rd.int("battle_revive"),
        dispel: rd.bool("battle_dispel"),
        damage_vs_summons: rd.int("battle_damage_vs_summons"),
        spell: rd.rf::<SpellType>("battle_spell"),
        strike_spell: rd.rf::<SpellType>("strike_spell"),
        opening_cast: rd.bool("opening_cast"),
        death_recall: rd.bool("death_recall"),
        familiar: rd.bool("battle_familiar"),
    }
}

/// A spell's `battle_illusions` (`{"count", "hp"}`): cast with its caster's first spell action in a
/// battle, it puts `count` illusory soldiers of `hp` HP each in free cells of its side (the front
/// row first). They do nothing, give no XP, vanish when a spell hits them and exist only in the
/// battle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleIllusions {
    pub count: i32,
    pub hp: i32,
}

/// A spell's `battle_illusions`, if any (a count or HP below 1 warns and reads as none).
pub fn read_battle_illusions(rd: &Rd, spell: &str) -> Option<BattleIllusions> {
    let c = rd.child("battle_illusions")?;
    let v = BattleIllusions { count: c.int("count"), hp: c.int("hp") };
    if v.count < 1 || v.hp < 1 {
        rd.cx.warn(format!("{spell}: battle_illusions needs a count and HP above 0"));
        return None;
    }
    Some(v)
}
