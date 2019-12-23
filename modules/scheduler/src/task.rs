use codec::{Encode, Decode, FullCodec};
use sp_std::{fmt::Debug, cmp::Ordering};
use sp_runtime::{RuntimeDebug, traits::{SimpleArithmetic, MaybeSerializeDeserialize}};
use crate::{TaskId, PriorityScore};

#[derive(Encode, Decode, RuntimeDebug, Clone, Eq)]
pub struct Task<BlockNumber: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default> {
    /// A vec of bytes which could be an identifier or a hash corresponding to associated data in IPFS or something
    pub id: TaskId,
    /// The priority of the task relative to other tasks
    pub score: PriorityScore,
    /// The block number at which the task is initially queued
    pub proposed_at: BlockNumber,
}

// task_id is _supposed_ to be a unique identifier
impl<BlockNumber: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default> PartialEq for Task<BlockNumber> {
    fn eq(&self, other: &Task<BlockNumber>) -> bool {
        self.id == other.id
    }
}

// for sorting based on `score` in `OnFinalize`
impl<BlockNumber: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default> Ord for Task<BlockNumber> {
    fn cmp(&self, other: &Task<BlockNumber>) -> Ordering {
        self.score.cmp(&other.score)
    }
}
impl<BlockNumber: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default> PartialOrd for Task<BlockNumber> {
    fn partial_cmp(&self, other: &Task<BlockNumber>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
