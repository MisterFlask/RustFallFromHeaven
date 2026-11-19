//! Ultimatums and contracts (docs/PERSONS.md, "The third build", rules 5 and 6: they replace the
//! second build's Furious countdown).
//!
//! - **The ultimatum** ([`Game::house_ultimatum`], pure): in its liege's turn processing, a House
//!   that is Furious and has no contract running demands one thing its liege can do this turn, the
//!   first of: (a) its refused request again, while its `Request refused` runs (a Crown city
//!   granted, war on that civilization, a state religion adopted); (b) a lower tribute, when its
//!   rate set is 20% or more (the highest rate whose loyalty once in force is -4 or more, else 0%);
//!   (c) peace with a civilization at war with its liege that took cities of the House's, when that
//!   civilization's controller would accept peace now; (d) war on a civilization the House
//!   remembers a wrong from, at peace with its liege, outside its realm, that its liege can declare
//!   war on now; (e) a resource its liege has a spare source of and the House has none of; (f)
//!   gold a turn, always: per House city `max(1, (world unit + 12) / 24)`, half that again for each
//!   earlier ultimatum of the House, at most half its liege's total commerce a turn.
//! - **The answer** ([`Game::house_issue_ultimatum`]): the liege's controller
//!   (`Controller::consider_ultimatum`: the AI's rule, or a human's blocking decision). Refused, the
//!   House breaks away now (`secession.rs`, the hold contest as built). Accepted, the demand is
//!   carried out ([`Game::house_ultimatum_accept`]) and a **contract** binds for
//!   `PERSON_ULTIMATUM_TURNS` (20) turns.
//! - **The contract** (rule 6): the House's loyalty counts as at least Annoyed (-5) for every rule
//!   (`loyalty.rs`), and its liege cannot do what would void it: a tribute set above the bound
//!   (`player_check_house_tribute`), war on the civilization peace was made with
//!   (`team_can_declare_war`), peace with the civilization war was declared on (the trade item
//!   `TRADE_PEACE_TREATY`), cancelling the resource trade (`deal_is_cancelable`), a change of state
//!   religion (`player_can_convert`). Each refuses with `Your contract with House Aldric forbids it
//!   until turn 160.` ([`Game::player_contract_refusal`]). Gold a turn is the liege's expense in its
//!   budget (`gold_per_turn_by_player`, as a deal's gold a turn: an empty treasury has the usual
//!   consequences). At its end ([`Game::house_contract_end`]) the gold stops and the resource trade
//!   ends; a House Furious again issues a new ultimatum.

use ffh_data::*;

use super::secession::Why;
use super::*;
use crate::deal::Deal;
use crate::structs::TradeData;

/// An act of a liege that a contract of one of its Houses may forbid (rule 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractAct {
    /// Setting House `house`'s tribute to `pct` percent.
    Tribute { house: HouseId, pct: i32 },
    /// Declaring war on this team.
    DeclareWar(TeamId),
    /// Making peace with this team.
    MakePeace(TeamId),
    /// Cancelling this deal.
    CancelDeal(DealId),
    /// Changing the state religion.
    ChangeReligion,
}

/// A House's contract as the screens read it ([`Game::house_contract_view`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractView {
    pub kind: UltimatumKind,
    /// The game turn it ends, and the turns left.
    pub until: i32,
    pub turns_left: i32,
    /// The card's line: `Contract until turn 160: no tribute above 10%.`
    pub line: String,
    /// The loyalty part's line: `Contract until turn 160: at least Annoyed.`
    pub floor: String,
}

fn item(item_type: TradeableItem, data: i32) -> TradeData {
    TradeData { item_type, data, offering: false, hidden: false }
}

impl Game {
    // -----------------------------------------------------------------------------------------
    // The contract: reads and bounds
    // -----------------------------------------------------------------------------------------

    /// House `h`'s contract, while it runs (it ends at its `until` turn).
    pub fn house_contract(&self, h: HouseId) -> Option<Contract> {
        let c = self.houses.get(h)?.contract?;
        (c.until > self.game_turn()).then_some(c)
    }

    /// Whether House `h`'s contract runs.
    pub fn house_contract_runs(&self, h: HouseId) -> bool {
        self.house_contract(h).is_some()
    }

    /// A civilization as the demands name it: `the Sheaim`.
    fn ultimatum_civ(&self, q: PlayerId) -> String {
        format!("the {}", self.player_get_civilization_short_description(q))
    }

    fn ultimatum_bonus(&self, b: BonusType) -> String {
        self.d.text.name(&self.d.bonuses[b].base.description).to_string()
    }

    fn ultimatum_religion_name(&self, r: ReligionType) -> String {
        self.d.text.name(&self.d.religions[r].base.description).to_string()
    }

    /// What ultimatum `kind` of House `h` demands, as a sentence: `Ventry granted to House
    /// Aldric.`, `War on the Sheaim.`, `The Veil adopted as state religion.`, `Its tribute set to
    /// 10%.`, `Peace with the Sheaim.`, `Wine given to House Aldric for 20 turns.`, `6 gold a turn
    /// to House Aldric for 20 turns.`
    pub fn ultimatum_demand_text(&self, h: HouseId, kind: UltimatumKind) -> String {
        let house = self.house_label(h);
        let turns = self.d.defines.person_ultimatum_turns;
        match kind {
            UltimatumKind::GrantCity(plot) => format!("{} granted to {house}.", self.demand_city_name(plot)),
            UltimatumKind::War(q) => format!("War on {}.", self.ultimatum_civ(q)),
            UltimatumKind::AdoptReligion(r) => format!("{} adopted as state religion.", self.ultimatum_religion_name(r)),
            UltimatumKind::Tribute(pct) => format!("Its tribute set to {pct}%."),
            UltimatumKind::Peace(q) => format!("Peace with {}.", self.ultimatum_civ(q)),
            UltimatumKind::Resource(b) => format!("{} given to {house} for {turns} turns.", self.ultimatum_bonus(b)),
            UltimatumKind::Gold(gold) => format!("{gold} gold a turn to {house} for {turns} turns."),
        }
    }

    /// What a contract of `kind` binds its liege to (rule 6), `named`: with the House's name where
    /// the bound concerns it (`no tribute above 10% for House Aldric`; on the House's card: `no
    /// tribute above 10%`). `None` for a city granted (nothing binds).
    pub fn contract_bound_text(&self, h: HouseId, kind: UltimatumKind, named: bool) -> Option<String> {
        let house = self.house_label(h);
        let for_house = if named { format!(" for {house}") } else { String::new() };
        let to_house = if named { format!(" to {house}") } else { String::new() };
        Some(match kind {
            UltimatumKind::GrantCity(_) => return None,
            UltimatumKind::War(q) => format!("no peace with {}", self.ultimatum_civ(q)),
            UltimatumKind::AdoptReligion(_) => "no change of state religion".to_string(),
            UltimatumKind::Tribute(pct) => format!("no tribute above {pct}%{for_house}"),
            UltimatumKind::Peace(q) => format!("no war on {}", self.ultimatum_civ(q)),
            UltimatumKind::Resource(b) => format!("the {}{to_house} is not taken back", self.ultimatum_bonus(b)),
            UltimatumKind::Gold(gold) => format!("{gold} gold a turn{to_house}"),
        })
    }

    /// House `h`'s contract as the screens read it, while it runs.
    pub fn house_contract_view(&self, h: HouseId) -> Option<ContractView> {
        let c = self.house_contract(h)?;
        let what = match self.contract_bound_text(h, c.kind, false) {
            Some(bound) => bound,
            None => self.ultimatum_demand_text(h, c.kind).trim_end_matches('.').to_string(),
        };
        Some(ContractView {
            kind: c.kind,
            until: c.until,
            turns_left: c.until - self.game_turn(),
            line: format!("Contract until turn {}: {what}.", c.until),
            floor: format!("Contract until turn {}: at least Annoyed.", c.until),
        })
    }

    /// Whether team `t` is `q`'s team or bound to it as vassal or master (a war or a peace with one
    /// reaches the other).
    fn contract_reaches(&self, q: PlayerId, t: TeamId) -> bool {
        let qt = self.players[q].team;
        t == qt || self.teams[t].vassal[qt] || self.teams[qt].vassal[t]
    }

    /// Whether contract `c` of House `h` forbids act `act`.
    pub(crate) fn contract_forbids(&self, h: HouseId, c: &Contract, act: ContractAct) -> bool {
        match (c.kind, act) {
            (UltimatumKind::Tribute(max), ContractAct::Tribute { house, pct }) => house == h && pct > max,
            (UltimatumKind::War(q), ContractAct::MakePeace(t)) => self.contract_reaches(q, t),
            (UltimatumKind::Peace(q), ContractAct::DeclareWar(t)) => self.contract_reaches(q, t),
            (UltimatumKind::Resource(_), ContractAct::CancelDeal(d)) => c.deal == Some(d),
            (UltimatumKind::AdoptReligion(_), ContractAct::ChangeReligion) => true,
            _ => false,
        }
    }

    /// Why player `p` may not do `act`, when a contract of one of its Houses forbids it (rule 6):
    /// `Your contract with House Aldric forbids it until turn 160.` (the first such House by id).
    pub fn player_contract_refusal(&self, p: PlayerId, act: ContractAct) -> Option<String> {
        if self.houses.is_empty() {
            return None;
        }
        let mut houses: Vec<HouseId> = self.players.get(p)?.houses.iter().copied().filter(|&h| self.houses.contains(h)).collect();
        houses.sort();
        houses.into_iter().find_map(|h| {
            let c = self.house_contract(h)?;
            self.contract_forbids(h, &c, act).then(|| format!("Your contract with {} forbids it until turn {}.", self.house_label(h), c.until))
        })
    }

    /// Why team `t` may not do `act`, when a contract of a House of one of its players forbids it.
    pub fn team_contract_refusal(&self, t: TeamId, act: ContractAct) -> Option<String> {
        if self.houses.is_empty() {
            return None;
        }
        self.player_ids().filter(|&p| self.players[p].team == t && self.players[p].alive).find_map(|p| self.player_contract_refusal(p, act))
    }

    /// Whether a contract holds deal `d` (its resource given, rule 5 (e)): it cannot be cancelled.
    pub fn deal_contract_refusal(&self, d: DealId) -> Option<String> {
        if self.houses.is_empty() {
            return None;
        }
        let deal = self.deals.get(d)?;
        [deal.first_player, deal.second_player].into_iter().find_map(|p| self.player_contract_refusal(p, ContractAct::CancelDeal(d)))
    }

    // -----------------------------------------------------------------------------------------
    // The demand (rule 5): the first that applies, each only when its liege can meet it now
    // -----------------------------------------------------------------------------------------

    /// Whether House `h`'s liege can be asked to declare war on `q` now (rule 5 (a) and (d)): a
    /// living civilization (not the barbarians, a minor civilization or a crusade Host) outside the
    /// realm, at peace with the liege, that the liege can declare war on.
    fn ultimatum_war_ok(&self, h: HouseId, q: PlayerId) -> bool {
        let p = self.houses[h].owner;
        let (pt, qt) = (self.players[p].team, self.players[q].team);
        q != p
            && self.players[q].alive
            && !self.player_is_barbarian(q)
            && !self.player_is_minor_civ(q)
            && !self.player_is_crusade_host(q)
            && qt != pt
            && !self.teams[qt].vassal[pt]
            && !self.teams[pt].vassal[qt]
            && !self.teams[pt].at_war[qt]
            && self.team_can_declare_war(pt, qt)
    }

    /// The state religion an ultimatum asks House `h`'s liege to adopt (rule 5 (a)): one in the
    /// liege's cities it can adopt now, not opposed to the House leader's alignment, in the most of
    /// its cities (ties: the lower id).
    fn ultimatum_religion(&self, h: HouseId) -> Option<ReligionType> {
        let p = self.houses[h].owner;
        let alignment = self.house_alignment(h);
        self.d
            .religions
            .keys()
            .filter(|&r| self.player_can_convert(p, Some(r)) && !self.alignments_opposed(self.d.religions[r].alignment, alignment))
            .map(|r| (self.players[p].cities.iter().filter(|&&c| self.cities.contains(c) && self.cities[c].has_religion[r]).count(), r))
            .filter(|&(n, _)| n > 0)
            .max_by_key(|&(n, r)| (n, std::cmp::Reverse(r)))
            .map(|(_, r)| r)
    }

    /// (a) House `h`'s refused request again, while its `Request refused` runs and the request can
    /// be met now: the city granted (a Crown city its liege can grant it), war on that civilization
    /// (as (d)), a state religion adopted.
    fn ultimatum_request(&self, h: HouseId) -> Option<UltimatumKind> {
        let house = &self.houses[h];
        let refused = house.refused?;
        let turn = self.game_turn();
        if !house.standing.mods.iter().any(|m| m.cause == LoyaltyCause::DemandRefused && m.until.map_or(true, |u| u > turn)) {
            return None;
        }
        let p = house.owner;
        match refused {
            DemandKind::GrantCity(plot) => {
                let c = self.persons_city_at(plot)?;
                (self.cities[c].owner == p && self.player_check_grant(p, c, GrantTo::House(h)).is_ok()).then_some(UltimatumKind::GrantCity(plot))
            }
            DemandKind::War(q) => self.ultimatum_war_ok(h, q).then_some(UltimatumKind::War(q)),
            DemandKind::AdoptReligion => self.ultimatum_religion(h).map(UltimatumKind::AdoptReligion),
            DemandKind::TakeCity(_) | DemandKind::SpreadReligion(_) => None,
        }
    }

    /// House `h`'s loyalty as its parts make it once a tribute of `pct` is in force (no contract's
    /// floor: an ultimatum is issued without one).
    fn ultimatum_loyalty_at(&self, h: HouseId, pct: i32) -> i32 {
        self.house_loyalty_raw(h) - tribute_part(self.houses[h].tribute_now) + tribute_part(pct)
    }

    /// (b) A lower tribute, when House `h`'s rate set is [`ULTIMATUM_TRIBUTE_FROM`] or more: the
    /// highest rate whose loyalty once in force is [`ULTIMATUM_TRIBUTE_LOYALTY`] or more, else 0%,
    /// when below the rate set.
    fn ultimatum_tribute(&self, h: HouseId) -> Option<i32> {
        let set = self.houses[h].tribute;
        if set < ULTIMATUM_TRIBUTE_FROM {
            return None;
        }
        let pct = (0..=TRIBUTE_MAX / TRIBUTE_STEP).rev().map(|k| k * TRIBUTE_STEP).find(|&pct| self.ultimatum_loyalty_at(h, pct) >= ULTIMATUM_TRIBUTE_LOYALTY).unwrap_or(0);
        (pct < set).then_some(pct)
    }

    /// (c) Peace with a civilization House `h`'s liege is at war with and that the House remembers
    /// taking its cities (its `MEMORY_TOOK_CITY` toward it above 0), when that civilization would
    /// accept peace now (its controller is asked, as an offer of peace is; a human is not asked
    /// inside another's turn: no peace with a human is demanded). The first by player id.
    fn ultimatum_peace(&self, h: HouseId) -> Option<PlayerId> {
        let (p, hp) = (self.houses[h].owner, self.houses[h].player);
        let pt = self.players[p].team;
        let peace = [item(TradeableItem::TRADE_PEACE_TREATY, 0)];
        self.player_ids().find(|&q| {
            let qt = self.players[q].team;
            self.players[q].alive
                && !self.players[q].human
                && !self.player_is_barbarian(q)
                && !self.player_is_minor_civ(q)
                && !self.player_is_crusade_host(q)
                && self.player_house(q).is_none()
                && !self.team_is_a_vassal(qt)
                && self.teams[pt].at_war[qt]
                && self.players[hp].ai_memory_count[q][MemoryType::MEMORY_TOOK_CITY] > 0
                && self.player_can_trade_item(p, q, &peace[0], false)
                && self.player_can_trade_item(q, p, &peace[0], false)
                && self.controller(q).consider_offer(self, q, p, &peace, &peace)
        })
    }

    /// (d) War on a civilization House `h` remembers a wrong from (its `MEMORY_DECLARED_WAR` or
    /// `MEMORY_TOOK_CITY` toward it above 0) that its liege can be asked to declare war on now
    /// (`ultimatum_war_ok`). The first by player id.
    fn ultimatum_war(&self, h: HouseId) -> Option<PlayerId> {
        let hp = self.houses[h].player;
        let memory = &self.players[hp].ai_memory_count;
        self.player_ids().find(|&q| (memory[q][MemoryType::MEMORY_DECLARED_WAR] > 0 || memory[q][MemoryType::MEMORY_TOOK_CITY] > 0) && self.ultimatum_war_ok(h, q))
    }

    /// (e) A resource House `h`'s liege has a spare source of (more than one it can trade) and the
    /// House has none of, the trade possible (their trade networks joined, the resource not
    /// obsolete for either). The first by id.
    fn ultimatum_resource(&self, h: HouseId) -> Option<BonusType> {
        let (p, hp) = (self.houses[h].owner, self.houses[h].player);
        let (pt, ht) = (self.players[p].team, self.players[hp].team);
        if !self.player_can_trade_network_with(p, hp) {
            return None;
        }
        self.d.bonuses.keys().find(|&b| {
            self.player_get_num_tradeable_bonuses(p, b) > 1 && self.player_get_num_available_bonuses(hp, b) == 0 && !self.team_is_bonus_obsolete(pt, b) && !self.team_is_bonus_obsolete(ht, b)
        })
    }

    /// (f) The gold a turn House `h` would demand: per city of it, `max(1, (world unit +
    /// ULTIMATUM_GOLD_ROUND) / ULTIMATUM_GOLD_DIVISOR)` and half of that again for each earlier
    /// ultimatum of the House; at most half its liege's total commerce a turn; at least 1.
    pub fn ultimatum_gold(&self, h: HouseId) -> i32 {
        let house = &self.houses[h];
        let base = ((self.world_commerce_unit() + ULTIMATUM_GOLD_ROUND) / ULTIMATUM_GOLD_DIVISOR).max(1);
        let per_city = (base * (2 + house.ultimatums.max(0)) / 2).max(1);
        let cities = self.house_cities(h).len() as i32;
        let cap = self.player_total_commerce_rate(house.owner) / 2;
        (per_city * cities.max(1)).min(cap).max(1)
    }

    /// The ultimatum House `h` would issue now (rule 5): the first of (a) to (f) that applies, each
    /// only when its liege can meet it this turn; (f) always applies. Pure, but for (c), which asks
    /// the other side's controller. `None` for a House that does not stand.
    pub fn house_ultimatum(&self, h: HouseId) -> Option<Ultimatum> {
        let house = self.houses.get(h)?;
        if !self.players[house.owner].alive || !self.players[house.player].alive {
            return None;
        }
        let until = self.game_turn() + self.d.defines.person_ultimatum_turns;
        let (kind, request) = match self.ultimatum_request(h) {
            Some(k) => (k, true),
            None => {
                let k = self
                    .ultimatum_tribute(h)
                    .map(UltimatumKind::Tribute)
                    .or_else(|| self.ultimatum_peace(h).map(UltimatumKind::Peace))
                    .or_else(|| self.ultimatum_war(h).map(UltimatumKind::War))
                    .or_else(|| self.ultimatum_resource(h).map(UltimatumKind::Resource))
                    .unwrap_or_else(|| UltimatumKind::Gold(self.ultimatum_gold(h)));
                (k, false)
            }
        };
        Some(Ultimatum { house: h, kind, request, until })
    }

    /// What refusing House `h`'s ultimatum does now (the break-away, the hold contest included):
    /// `House Aldric breaks away, taking Ventry and Corona: war. Corona returns to you: your army's
    /// 4 units outnumber its 2.`, or `House Aldric ends: every city is held. ...`
    pub fn house_refusal_outcome(&self, h: HouseId) -> String {
        let label = self.house_label(h);
        let (going, _) = self.house_cities_held(h);
        let contests = self.house_hold_text(h);
        if going.is_empty() {
            format!("{label} ends: every city is held.{contests}")
        } else {
            format!("{label} breaks away, taking {}: war.{contests}", self.city_names_of(&going))
        }
    }

    // -----------------------------------------------------------------------------------------
    // Issuing, accepting
    // -----------------------------------------------------------------------------------------

    /// House `h` issues its ultimatum (in its liege's turn processing, Furious with no contract
    /// running): its liege's controller answers at once. Accepted: carried out, a contract binds.
    /// Refused: the House breaks away now.
    pub(crate) fn house_issue_ultimatum(&mut self, h: HouseId) {
        let Some(u) = self.house_ultimatum(h) else { return };
        let p = self.houses[h].owner;
        self.houses[h].ultimatums += 1;
        let label = self.house_label(h);
        let demand = self.ultimatum_demand_text(h, u.kind);
        self.house_add_deed(h, format!("Ultimatum: {}", demand));
        self.house_msg(h, || format!("{label} issues an ultimatum: {demand}"));
        let accept = self.controller(p).consider_ultimatum(self, p, &u);
        if !self.houses.contains(h) {
            return;
        }
        if accept {
            self.house_ultimatum_accept(&u);
        } else {
            self.house_msg(h, || format!("You refuse {label}'s ultimatum."));
            self.tel_house(p, "ultimatum", "refused");
            self.house_break_away(h, Why::Ultimatum);
        }
    }

    /// Carries out ultimatum `u`'s demand, accepted by its liege: the act, then the contract
    /// (rule 6: `PERSON_ULTIMATUM_TURNS` turns, its kind's bound, the House at least Annoyed).
    /// When the act cannot be done after all (a world that changed between the choice and the
    /// answer), the House takes (f), gold a turn, instead. Returns the contract's kind.
    pub fn house_ultimatum_accept(&mut self, u: &Ultimatum) -> UltimatumKind {
        let h = u.house;
        let p = self.houses[h].owner;
        let (mut kind, mut deal) = (u.kind, None);
        if self.ultimatum_carry_out(h, kind, &mut deal).is_err() {
            kind = UltimatumKind::Gold(self.ultimatum_gold(h));
            let _ = self.ultimatum_carry_out(h, kind, &mut deal);
        }
        if !self.houses.contains(h) {
            return kind;
        }
        let until = u.until;
        let house = &mut self.houses[h];
        house.contract = Some(Contract { kind, until, deal });
        if u.request {
            house.refused = None;
        }
        // Its state is now the floor's (told as it is: the contract's own message says it).
        let state = self.house_state(h);
        self.houses[h].standing.state = state;
        let label = self.house_label(h);
        let line = self.house_contract_view(h).map(|v| v.line).unwrap_or_default();
        self.house_add_deed(h, format!("Ultimatum accepted. {line}"));
        self.house_msg(h, || format!("{label}: ultimatum accepted. {line}"));
        self.tel_house(p, "ultimatum", "accepted");
        kind
    }

    /// Does what ultimatum `kind` of House `h` demands; `deal`: the resource trade made.
    fn ultimatum_carry_out(&mut self, h: HouseId, kind: UltimatumKind, deal: &mut Option<DealId>) -> Result<(), String> {
        let (p, hp) = (self.houses[h].owner, self.houses[h].player);
        let pt = self.players[p].team;
        match kind {
            UltimatumKind::GrantCity(plot) => {
                let c = self.persons_city_at(plot).ok_or("No city there.")?;
                self.player_grant_city(p, c, GrantTo::House(h)).map(|_| ()).map_err(String::from)
            }
            UltimatumKind::War(q) => {
                let qt = self.players[q].team;
                if !self.team_can_declare_war(pt, qt) {
                    return Err("War cannot be declared.".into());
                }
                self.team_declare_war(pt, qt, false, None);
                Ok(())
            }
            UltimatumKind::AdoptReligion(r) => {
                if !self.player_can_convert(p, Some(r)) {
                    return Err("The religion cannot be adopted.".into());
                }
                self.player_convert(p, Some(r));
                Ok(())
            }
            UltimatumKind::Tribute(pct) => self.player_set_house_tribute(p, h, pct),
            UltimatumKind::Peace(q) => {
                let qt = self.players[q].team;
                let peace = [item(TradeableItem::TRADE_PEACE_TREATY, 0)];
                self.implement_deal(p, q, &peace, &peace, false);
                if self.teams[pt].at_war[qt] {
                    return Err("Peace was not made.".into());
                }
                Ok(())
            }
            UltimatumKind::Resource(b) => {
                // A trade from the liege to its House (the rules' resource trade: `TRADE_RESOURCES`,
                // given for nothing); the House trades nothing of its own (rule 9).
                let d = self.deals.insert_with(|id| Deal::new(id, p, hp));
                self.deal_init(d, p, hp);
                self.deal_add_trades(d, &[item(TradeableItem::TRADE_RESOURCES, b.idx() as i32)], &[], false);
                if !self.deals.contains(d) || self.deals[d].first_trades.is_empty() {
                    if self.deals.contains(d) {
                        self.deal_kill(d, true);
                    }
                    return Err("The resource cannot be given.".into());
                }
                *deal = Some(d);
                Ok(())
            }
            UltimatumKind::Gold(gold) => {
                self.player_change_gold_per_turn_by_player(p, hp, -gold);
                self.player_change_gold_per_turn_by_player(hp, p, gold);
                Ok(())
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // The contract's end
    // -----------------------------------------------------------------------------------------

    /// House `h`'s contract ends (its turns ran out, `told`; or its record ends): its gold a turn
    /// stops, its resource trade ends.
    pub(crate) fn house_contract_end(&mut self, h: HouseId, told: bool) {
        let Some(c) = self.houses.get_mut(h).and_then(|house| house.contract.take()) else { return };
        let (p, hp) = (self.houses[h].owner, self.houses[h].player);
        match c.kind {
            UltimatumKind::Gold(gold) => {
                self.player_change_gold_per_turn_by_player(p, hp, gold);
                self.player_change_gold_per_turn_by_player(hp, p, -gold);
            }
            UltimatumKind::Resource(_) => {
                if let Some(d) = c.deal.filter(|&d| self.deals.contains(d)) {
                    self.deal_kill(d, true);
                }
            }
            _ => {}
        }
        if told {
            let label = self.house_label(h);
            self.house_add_deed(h, "Its contract ended.".into());
            self.house_msg(h, || format!("{label}: the contract ends."));
        }
    }

    /// Player `p`'s Houses' contracts whose turns ran out end (its turn processing, before the
    /// states: a House Furious again issues a new ultimatum).
    pub(crate) fn player_persons_contracts(&mut self, p: PlayerId) {
        let turn = self.game_turn();
        for h in self.player_houses(p) {
            if self.houses[h].contract.is_some_and(|c| c.until <= turn) {
                self.house_contract_end(h, true);
            }
        }
    }
}
