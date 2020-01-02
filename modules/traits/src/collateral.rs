/// Collateral
/// - see atomic reference counter implementation in std to find out how I can store an internal counter
/// - the power of this comes from lending protocols that depend on interpreting this
/// - each step of the counter should be associated with some identifier that says what it is being used for
/// -- so measured rehypothecation is allowed and specifically permitted by certain lending protocols which take on the risk of 
/// ---leveraging collateral that could also be forfeited under other contracts