#![recursion_limit="128"]
#![cfg_attr(not(feature = "std"), no_std)]

use runtime_primitives::{
    RuntimeDebug, 
    traits::{Hash, SimpleArithmetic, Dispatchable, MaybeSerializeDeserialize, EnsureOrigin},
};
use support::{
    decl_event, decl_module, decl_storage,
    ensure, StorageMap, StorageValue, Parameter,
    traits::{Currency, ReservableCurrency, Get, ChangeMembers, InitializeMembers},
};
use parity_scale_codec::{Encode, Decode, FullCodec};
use rstd::{prelude::*, fmt::Debug};
use system::{self, ensure_signed};

pub trait Trait: system::Trait {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// The balances type
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    /// Proposal type, dispatchable from runtime methods
    type Proposal: Parameter + Dispatchable<Origin=Self::Origin>;
    /// Signal type, for representing each member's voting power
    type Signal: SimpleArithmetic + Clone + Copy + Default + FullCodec + MaybeSerializeDeserialize + Debug;
    /// Origin for decisions that only require a majority vote
    type MajorityCarries: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    /// Origin for members to cancel proposals in emergency scenarios
    type CancellationOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    /// Origin for setting a member's relative voting weight (meta, path to QV)
    type WeightOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    /// The period for which votes are counted
    type VotePeriod: Get<Self::BlockNumber>;
}

// TODO
// - create a few election structures
// - create traits for them (abstract shared behavior and move everything to trait objects)

#[derive(Encode, Decode, RuntimeDebug)]
pub struct SimpleTally;

decl_storage! {
    trait Store for Module<T: Trait> as Vote {
        /// Track user debt (to prevent excessive requests beyond means)
        TallyVotes get(fn tally_votes): map T::AccountId => Option<SimpleTally>;
    }
}

decl_event!(
    pub enum Event<T>
    where 
        <T as system::Trait>::AccountId
    {
        NullEvent(AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_finalize(n: T::BlockNumber) {
            let fake = 6u64;
        }
    }
}

impl<T: Trait> Module<T> {
    // use origins
}