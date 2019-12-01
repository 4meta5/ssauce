#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode, FullCodec};
use rstd::{fmt::Debug, prelude::*};
use runtime_primitives::{
    traits::{EnsureOrigin, Hash, MaybeSerializeDeserialize, SimpleArithmetic},
    RuntimeDebug,
};
use support::{
    decl_event, decl_module, decl_storage,
    dispatch::{Dispatchable, Parameter},
    ensure,
    traits::{ChangeMembers, Currency, Get, InitializeMembers, ReservableCurrency},
    StorageMap, StorageValue,
};
use system::{self, ensure_signed, RawOrigin};

pub trait Trait<I = DefaultInstance>: system::Trait {
    /// The outer call dispatch type. (TODO: see collective)
    // type Proposal: Parameter + Dispatchable<Origin=Self::Origin>;
    /// Overarching event type
    type Event: From<Event<Self, I>> + Into<<Self as system::Trait>::Event>;

    /// The balances type
    type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
    /// Vote signal type, for representing each member's voting power
    type Signal: SimpleArithmetic
        + Clone
        + Copy
        + Default
        + FullCodec
        + MaybeSerializeDeserialize
        + Debug;

    /// Origin for decisions that only require a majority vote
    type MajorityOrigin: EnsureOrigin<Self::Origin>;
    /// Origin for members to cancel proposals in emergency scenarios
    type CancellationOrigin: EnsureOrigin<Self::Origin>;
    /// Origin for setting a member's relative voting weight (meta, path to QV)
    type WeightOrigin: EnsureOrigin<Self::Origin>;
    /// The period for which votes are counted
    type VotePeriod: Get<Self::BlockNumber>;
}
// first, worry about instancing and using membership
// then worry about the origin stuff from collective, democracy, and council

#[derive(Encode, Decode, RuntimeDebug)]
pub struct SimpleTally;

decl_storage! {
    trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Vote {
        /// Track user debt (to prevent excessive requests beyond means)
        TallyVotes get(fn tally_votes): map T::AccountId => Option<SimpleTally>;
    }
}

decl_event!(
    pub enum Event<T, I = DefaultInstance>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        NullEvent(AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        fn on_finalize(n: T::BlockNumber) {
            let fake = 6u64;
        }
    }
}
