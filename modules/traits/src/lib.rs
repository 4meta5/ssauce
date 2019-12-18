// start with structs and then later convert to trait objects
use std::boxed;

pub enum ColoredProposals {
    Grant,
    Membership,
    Meta,
}

pub struct Grant {
    proposer: u32,
    grantee: u32,
    amount: u32,
}

pub struct Membership {
    proposer: u32,
    grantee: u32,
    amount: u32,
}

pub struct Meta {
    proposer: u32,
    grantee: u32,
    amount: u32,
}

// add trait to the `prepoll` => `negosh` module
pub trait Edit {
    type ProposalForm;
    type Change;

    // proposes delta and returns new proposal
    fn propose_change(delta: Self::Change) -> Self::ProposalForm;

    // supports delta and returns new proposal
    fn support_change(delta: Self::Change) -> Self::ProposalForm;
}