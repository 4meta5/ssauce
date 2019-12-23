# traits

## wishlist
- [ ] new currency trait with internal counter for collateralization norms; should consider how this will be used in the proposal quota, voting signal, and other member action quota (paid for in and delegated by `txphi` proc-macro (TBW))

- [ ] proposal trait `=>` should be easy to track it's path through the state machine as well as its relation to each part of the configuration (might need to be a module type as well)

- [ ] a new `EnsureSignedBy` and/or `EnsureProportionAtLeast` but for specific origins that incorporate signal-based weighting

* trait for the voting algorithm chosen which converts a `SignalProfile` to weight in a vote based on the proposal.

## references

* `RustCrypto/traits`
* `frame-support/traits`