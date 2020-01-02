/// dynamically sized currency
/// - currency can be different types and all the sizes are comparable
/// - operate everything as a (debit, credit) situation (look at Preston's old tweets on accounting and look up double accounting)
/// - TryFrom and TryInto for the different currency types
/// - upon overflow, automatically switch into a different one
/// - (eventually) if the largest type overflows
/// ---- add (_) to the tuple and associate it's values with one of the previous columns, thereby requireing metadato to do so explicitly