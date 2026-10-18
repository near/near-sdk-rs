//! Usability exercise: a small DAO (members, proposals with per-proposal votes, a config
//! record), written against `near_sdk::state` (storage v2).
#![cfg(feature = "unstable")]

use near_sdk::state::testing::{self, Harness, Receipt};
use near_sdk::state::{Map, Page, Set};
use near_sdk::{AccountId, NearToken, env, near, require};

pub type ProposalId = u64;

#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vote {
    Yes,
    No,
}

#[near(serializers = [json])]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Rejected,
}

/// Inline in the root record.
#[near(storage)]
pub struct Config {
    #[key(1)]
    quorum: u32,
    #[key(2)]
    threshold_pct: u8,
    #[key(3)]
    max_open_per_member: u32,
}

#[near(storage)]
pub struct Member {
    #[key(1)]
    joined_at: u64,
    /// Open proposals this member created.
    #[key(2)]
    proposals: Set<ProposalId>,
    /// Open proposals this member voted on.
    #[key(3)]
    votes: Set<ProposalId>,
}

#[near(storage)]
pub struct Proposal {
    #[key(1, required)]
    proposer: AccountId,
    #[key(2)]
    description: String,
    #[key(3)]
    votes: Map<AccountId, Vote>,
}

#[near(serializers = [json])]
#[derive(Debug, PartialEq)]
pub struct ProposalJson {
    pub id: ProposalId,
    pub proposer: AccountId,
    pub description: String,
    pub yes: u32,
    pub no: u32,
    pub votes: Vec<(AccountId, Vote)>,
}

#[near(contract_state(layout = v2))]
pub struct Dao {
    #[key(1, required)]
    council: AccountId,
    #[key(2)]
    config: Config,
    #[key(3)]
    members: Map<AccountId, Member>,
    #[key(4)]
    proposals: Map<ProposalId, Proposal>,
    #[key(5)]
    next_id: ProposalId,
}

#[near]
impl Dao {
    #[init]
    pub fn new(council: AccountId, quorum: u32, threshold_pct: u8) -> Self {
        Self {
            council,
            config: Config { quorum, threshold_pct, max_open_per_member: 3 },
            members: Default::default(),
            proposals: Default::default(),
            next_id: 0,
        }
    }

    fn assert_council(&self) {
        require!(env::predecessor_account_id() == self.council, "council only");
    }

    pub fn add_member(&mut self, account_id: AccountId) {
        self.assert_council();
        // T1: `try_insert` is the existence check and the insert in one index read.
        require!(
            self.members
                .try_insert(account_id, Member { joined_at: env::block_height(), ..Default::default() })
                .is_ok(),
            "already a member"
        );
    }

    /// Removes the member, every open proposal they created (with its votes, and with the
    /// matching entries in each voter's `votes` set), and every vote they cast.
    pub fn remove_member(&mut self, account_id: AccountId) {
        self.assert_council();
        let member = self.members.get(&account_id).expect("not a member");
        let own: Vec<ProposalId> = member.proposals.iter().collect();
        let voted: Vec<ProposalId> = member.votes.iter().collect();

        for pid in own {
            self.drop_proposal(pid);
        }
        for pid in voted {
            if let Some(p) = self.proposals.get_mut(&pid) {
                p.votes.discard(&account_id);
            }
        }
        self.members.remove(&account_id);
    }

    pub fn propose(&mut self, description: String) -> ProposalId {
        let proposer = env::predecessor_account_id();
        let max_open = self.config.max_open_per_member;
        let member = self.members.get_mut(&proposer).expect("members only");
        require!(member.proposals.len() < max_open, "too many open proposals");
        let id = self.next_id;
        self.next_id += 1;
        member.proposals.insert(id);
        self.proposals
            .try_insert(id, Proposal { proposer, description, votes: Default::default() })
            .expect("proposal id reused");
        id
    }

    /// Revision 3: `at_mut` reaches `votes` without reading the proposal record (or the
    /// member's), and `try_insert` is the double-vote check (T1: it writes nothing when the
    /// voter already voted; `require!(votes.insert(..))` had already overwritten).
    pub fn vote(&mut self, id: ProposalId, vote: Vote) {
        let voter = env::predecessor_account_id();
        require!(self.members.contains_key(&voter), "members only");
        // D7: `at_mut` returns `Option`, so the existence check is the `expect`.
        let p = self.proposals.at_mut(&id).expect("no such proposal");
        require!(p.votes.try_insert(voter.clone(), vote).is_ok(), "already voted");
        self.members.at_mut(&voter).expect("members only").votes.insert(id);
    }

    /// Counts the votes, removes the proposal, and reports the outcome.
    pub fn finalize(&mut self, id: ProposalId) -> Outcome {
        let p = self.proposals.get(&id).expect("no such proposal");
        let (yes, no) = tally(&p.votes);
        let members = self.members.len();
        let passed = yes + no >= self.config.quorum
            && (yes as u64) * 100 >= (self.config.threshold_pct as u64) * (members as u64);
        self.drop_proposal(id);
        if passed { Outcome::Passed } else { Outcome::Rejected }
    }

    /// Removes a proposal and every back-reference to it.
    fn drop_proposal(&mut self, id: ProposalId) {
        let p = self.proposals.get(&id).expect("no such proposal");
        let proposer = p.proposer.clone();
        let voters: Vec<AccountId> = p.votes.keys().collect();
        for v in voters {
            if let Some(m) = self.members.get_mut(&v) {
                m.votes.remove(&id);
            }
        }
        if let Some(m) = self.members.get_mut(&proposer) {
            m.proposals.remove(&id);
        }
        self.proposals.remove(&id);
    }

    pub fn get_proposal(&self, id: ProposalId) -> Option<ProposalJson> {
        self.proposals.get(&id).map(|p| proposal_json(id, p))
    }

    /// Paginated proposals, as views. Revision 3: `page_with` maps each record.
    /// D8: a removed cursor is `Err(CursorGone)`; this view maps it to a panic with its message.
    pub fn proposals(&self, after: Option<ProposalId>, limit: u32) -> Page<ProposalId, ProposalJson> {
        self.proposals
            .page_with(after.as_ref(), limit, |id, p| proposal_json(*id, p))
            .unwrap_or_else(|e| env::panic_str(e.as_ref()))
    }

    pub fn is_member(&self, account_id: AccountId) -> bool {
        self.members.contains_key(&account_id)
    }

    pub fn member_open_proposals(&self, account_id: AccountId) -> u32 {
        self.members.at(&account_id).proposals.len()
    }
}

fn proposal_json(id: ProposalId, p: &Proposal) -> ProposalJson {
    let votes: Vec<(AccountId, Vote)> = p.votes.iter().map(|(k, v)| (k, *v)).collect();
    let (yes, no) = tally(&p.votes);
    ProposalJson { id, proposer: p.proposer.clone(), description: p.description.clone(), yes, no, votes }
}

fn tally(votes: &Map<AccountId, Vote>) -> (u32, u32) {
    votes.iter().fold((0, 0), |(y, n), (_, v)| match v {
        Vote::Yes => (y + 1, n),
        Vote::No => (y, n + 1),
    })
}

// ---------------------------------------------------------------------------------------
// harness (revision 3: `state::testing::Harness` instead of copied boilerplate)
// ---------------------------------------------------------------------------------------

fn id(s: &str) -> AccountId {
    s.parse().unwrap()
}

fn init() -> Receipt {
    Harness::init("council.near", || Dao::new(id("council.near"), 2, 50)).1
}

fn call<R>(who: &str, f: impl FnOnce(&mut Dao) -> R) -> (R, Receipt) {
    Harness::<Dao>::attach().call(who, NearToken::from_yoctonear(0), f)
}

fn view<R>(f: impl FnOnce(&Dao) -> R) -> (R, Receipt) {
    Harness::<Dao>::attach().view(f)
}

fn setup(members: &[&str]) {
    init();
    for m in members {
        call("council.near", |c| c.add_member(id(m)));
    }
}

fn keys() -> std::collections::BTreeSet<Vec<u8>> {
    testing::store().into_keys().collect()
}

// ---------------------------------------------------------------------------------------

#[test]
fn add_and_remove_member() {
    setup(&["a.near"]);
    assert!(view(|c| c.is_member(id("a.near"))).0);
    call("council.near", |c| c.remove_member(id("a.near")));
    assert!(!view(|c| c.is_member(id("a.near"))).0);
}

#[test]
#[should_panic(expected = "already a member")]
fn add_member_twice_panics() {
    setup(&["a.near"]);
    call("council.near", |c| c.add_member(id("a.near")));
}

#[test]
#[should_panic(expected = "council only")]
fn only_council_adds_members() {
    setup(&[]);
    call("a.near", |c| c.add_member(id("a.near")));
}

#[test]
fn propose_vote_and_view_tally() {
    setup(&["a.near", "b.near", "c.near"]);
    let (pid, _) = call("a.near", |c| c.propose("fund the thing".into()));
    call("a.near", |c| c.vote(pid, Vote::Yes));
    call("b.near", |c| c.vote(pid, Vote::No));
    call("c.near", |c| c.vote(pid, Vote::Yes));
    let (v, _) = view(|c| c.get_proposal(pid));
    let v = v.unwrap();
    assert_eq!((v.yes, v.no), (2, 1));
    assert_eq!(v.proposer, id("a.near"));
    let json = near_sdk::serde_json::to_value(&v).unwrap();
    assert_eq!(json["yes"], 2);
    assert_eq!(json["votes"][1], near_sdk::serde_json::json!(["b.near", "No"]));
}

#[test]
#[should_panic(expected = "already voted")]
fn double_vote_panics() {
    setup(&["a.near"]);
    let (pid, _) = call("a.near", |c| c.propose("x".into()));
    call("a.near", |c| c.vote(pid, Vote::Yes));
    call("a.near", |c| c.vote(pid, Vote::No));
}

#[test]
#[should_panic(expected = "members only")]
fn non_member_cannot_vote() {
    setup(&["a.near"]);
    let (pid, _) = call("a.near", |c| c.propose("x".into()));
    call("z.near", |c| c.vote(pid, Vote::Yes));
}

#[test]
#[should_panic(expected = "too many open proposals")]
fn open_proposal_limit_from_config() {
    setup(&["a.near"]);
    for _ in 0..4 {
        call("a.near", |c| c.propose("x".into()));
    }
}

#[test]
fn finalize_counts_and_removes() {
    setup(&["a.near", "b.near", "c.near"]);
    let before = keys();
    let (pid, _) = call("a.near", |c| c.propose("x".into()));
    call("a.near", |c| c.vote(pid, Vote::Yes));
    call("b.near", |c| c.vote(pid, Vote::Yes));
    let (out, _) = call("council.near", |c| c.finalize(pid));
    assert_eq!(out, Outcome::Passed);
    assert!(view(|c| c.get_proposal(pid)).0.is_none());
    // Everything the proposal created is gone, except the root (next_id changed).
    let after = keys();
    assert_eq!(after, before, "leaked: {:?}", after.difference(&before).collect::<Vec<_>>());
    assert_eq!(view(|c| c.member_open_proposals(id("a.near"))).0, 0);
}

#[test]
fn finalize_below_quorum_rejects() {
    setup(&["a.near", "b.near", "c.near"]);
    let (pid, _) = call("a.near", |c| c.propose("x".into()));
    call("a.near", |c| c.vote(pid, Vote::Yes));
    assert_eq!(call("council.near", |c| c.finalize(pid)).0, Outcome::Rejected);
}

#[test]
fn remove_member_cleans_up_proposals_and_votes() {
    setup(&["a.near", "b.near", "c.near"]);
    let before = keys();
    // b proposes; a and c vote on it. a proposes; b votes on it.
    let (pb, _) = call("b.near", |c| c.propose("b's".into()));
    call("a.near", |c| c.vote(pb, Vote::Yes));
    call("c.near", |c| c.vote(pb, Vote::No));
    let (pa, _) = call("a.near", |c| c.propose("a's".into()));
    call("b.near", |c| c.vote(pa, Vote::Yes));

    call("council.near", |c| c.remove_member(id("b.near")));

    // b's proposal is gone, and a and c no longer list it among their votes.
    assert!(view(|c| c.get_proposal(pb)).0.is_none());
    // b's vote on a's proposal is gone.
    let v = view(|c| c.get_proposal(pa)).0.unwrap();
    assert_eq!((v.yes, v.no), (0, 0));
    assert!(v.votes.is_empty());
    // a can still vote on its own proposal; c's votes set is empty again.
    call("a.near", |c| c.vote(pa, Vote::Yes));
    call("council.near", |c| c.finalize(pa));
    call("council.near", |c| c.add_member(id("b.near")));
    // Back to the starting key set: nothing orphaned anywhere.
    let after = keys();
    assert_eq!(after, before, "leaked: {:?}", after.difference(&before).collect::<Vec<_>>());
}

#[test]
fn proposals_paginate_and_stop_at_the_end() {
    setup(&["a.near", "b.near"]);
    for i in 0..5 {
        let who = if i % 2 == 0 { "a.near" } else { "b.near" };
        call(who, |c| c.propose(format!("p{i}")));
    }
    let (p1, _) = view(|c| c.proposals(None, 2));
    assert_eq!(p1.items.iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![0, 1]);
    let (p2, _) = view(|c| c.proposals(p1.next, 2));
    let (p3, _) = view(|c| c.proposals(p2.next, 2));
    assert_eq!(p3.items.len(), 1);
    assert_eq!(p3.next, None);
    assert_eq!(p3.items[0].1.description, "p4");
    // limit 0, and a page that starts exactly at the end
    assert!(view(|c| c.proposals(None, 0)).0.items.is_empty());
    let (p, _) = view(|c| c.proposals(Some(4), 10));
    assert!(p.items.is_empty() && p.next.is_none());
}

#[test]
fn vote_op_counts() {
    setup(&["a.near", "b.near"]);
    let (pid, _) = call("a.near", |c| c.propose("x".repeat(2_000)));
    let (_, r) = call("b.near", |c| c.vote(pid, Vote::Yes));
    // Revision 3, with `at_mut` and `insert -> bool`. reads: root, votes index + header
    // (insert), member votes-set index + header. has_key: member and proposal existence
    // (`at_mut` reuses them from the cache). writes: 4 for the votes Map entry, 4 for the
    // member's Set entry. Before revision 3: (7, 8, 0) + 1 has_key, reading both records.
    //
    // API round 2 (D9, flag key): a record being cleared lives at its flag key with its record
    // key absent, so `at_mut` on a present record stays 1 `storage_has_key` and reads nothing.
    // (The in-record `00 00` flag had made this (7, 8, 0) + 1 has_key, rereading the 2,000-byte
    // description.) Back to the revision-3 numbers.
    assert_eq!(r.rwd(), (5, 8, 0));
    assert_eq!(r.has_keys(), 2);
    assert!(r.read_bytes() < 2_000, "read {} bytes: the proposal is not read", r.read_bytes());
}

/// D7: `at_mut` on an absent record is `None` instead of creating orphan children.
#[test]
fn at_mut_on_absent_record_is_none() {
    setup(&["a.near"]);
    let (none, r) = call("a.near", |c| c.proposals.at_mut(&99).is_none());
    assert!(none);
    assert!(r.keys_written().iter().all(|k| k.as_slice() == [0xF0]), "nothing created");
}

#[test]
#[should_panic(expected = "no such proposal")]
fn vote_on_absent_proposal_panics() {
    setup(&["a.near"]);
    call("a.near", |c| c.vote(99, Vote::Yes));
}

#[test]
fn finalize_and_remove_member_op_counts() {
    let names: Vec<String> = (0..5).map(|i| format!("m{i}.near")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    setup(&refs);
    let (pid, _) = call("m0.near", |c| c.propose("x".into()));
    for n in &refs {
        call(n, |c| c.vote(pid, Vote::Yes));
    }
    let (_, r) = call("council.near", |c| c.finalize(pid));
    eprintln!("finalize with 5 votes: rwd={:?} has_key={}", r.rwd(), r.has_keys());

    let (pid, _) = call("m0.near", |c| c.propose("y".into()));
    for n in &refs[1..] {
        call(n, |c| c.vote(pid, Vote::No));
    }
    let (_, r) = call("council.near", |c| c.remove_member(id("m0.near")));
    eprintln!("remove_member owning 1 proposal with 4 votes: rwd={:?}", r.rwd());
}
