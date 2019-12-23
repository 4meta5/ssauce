#![cfg_attr(not(feature = "std"), no_std)]
//! Scheduling Execution
use sp_std::{prelude::*, cmp, result, mem, fmt::Debug};
use sp_runtime::{ traits::Zero, DispatchResult };
use support::{
    decl_event, decl_module, decl_storage, decl_error,
    ensure,
    traits::Get,
    StorageDoubleMap, StorageMap, StorageValue,
};
use system::ensure_signed;
// import task
mod task;
use task::Task;

pub type TaskId = Vec<u8>;
pub type PriorityScore = u32;
pub type RoundIndex = u32;

pub trait Trait: system::Trait {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// Quota for members to signal task priority every ExecutionFrequency
    type SignalQuota: Get<PriorityScore>;

    /// The frequency of batch executions for tasks (in `on_finalize`)
    type ExecutionFrequency: Get<Self::BlockNumber>;

    /// The maximum number of tasks that can be approved in an `ExecutionFrequency` period
    type TaskLimit: Get<PriorityScore>;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        BlockNumber = <T as system::Trait>::BlockNumber,
    {
        /// Signal is refreshed for all members at this block number
        SignalRefreshed(BlockNumber),
        /// Task is scheduled by the proposer with `TaskId` and expected_execution_time
        TaskScheduled(AccountId, TaskId, BlockNumber),
        /// Signal support for a task
        SignalSupport(TaskId, PriorityScore),
        /// Task is executed with this identifier at this block number
        TaskExecuted(TaskId, BlockNumber),
        /// New expected execution time for tasks not completed within first *opportunity*
        UpdatedTaskSchedule(TaskId, BlockNumber),
    }
);

decl_error! {
    /// Errors that can occur in my module.
    pub enum Error for Module<T: Trait> {
        /// The task does not exist in runtime storage
        TaskDNEStorage,
        /// The task score overflowed in the signalling priority method
        TaskScoreOverflow,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as TaskScheduler {
        /// Outstanding tasks getter
        PendingTasks get(fn pending_tasks): map TaskId => Option<Task<T::BlockNumber>>;
        /// Dispatch queue for task execution
        /// TODO: only store TaskId here, sort and reorder
        ExecutionQueue get(fn execution_queue): Vec<Task<T::BlockNumber>>;
        /// The signalling quota left in terms of `PriorityScore` for all members of the council (until it is killed `on_initialize` on `ExecutionFrequency` blocks)
        SignalBank get(fn signal_bank): double_map RoundIndex, twox_128(T::AccountId) => PriorityScore;
        /// The (closed and static) council of members (anyone can submit tasks but only members can signal priority)
        Council get(fn council): Vec<T::AccountId>;
        /// The nonce that increments every `ExecutionFrequency` for a new `SignalBank` instantiation
        Era get(fn era): RoundIndex;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        const SignalQuota: PriorityScore = T::SignalQuota::get();
        const ExecutionFrequency: T::BlockNumber = T::ExecutionFrequency::get();
        const TaskLimit: PriorityScore = T::TaskLimit::get();

        fn deposit_event() = default;

        /// On Initialize
        ///
        /// After the last block's on_finalize, the logic expressed in this method
        /// is executed before the logic in the next block.
        /// - This allows us to start from 0 for all tasks
        fn on_initialize(n: T::BlockNumber) {
            let batch_frequency = T::ExecutionFrequency::get();
            if ((n - 1.into()) % batch_frequency).is_zero() {
                let last_era = <Era>::get();
                // clean up the previous double_map with this last_era group index
                <SignalBank<T>>::remove_prefix(&last_era);
                // unlikely to overflow so no checked_add
                let next_era: RoundIndex = last_era + (1u32 as RoundIndex);
                <Era>::put(next_era);

                // get the SignalQuota for each `ExecutionFrequency` period
                let signal_quota = T::SignalQuota::get();
                // instantiate next mapping for SignalQuota with new Era
                <Council<T>>::get().into_iter().for_each(|member| {
                    // refresh signal quota for all members for the next era
                    <SignalBank<T>>::insert(next_era, &member, signal_quota);
                });
                Self::deposit_event(RawEvent::SignalRefreshed(n));
            }
        }

        /// Schedule Task for Batch Execution
        ///
        /// - the task initially has no priority
        /// - only council members can schedule tasks
        fn schedule_task(origin, data: Vec<u8>) {
            let proposer = ensure_signed(origin)?;
            ensure!(Self::is_on_council(&proposer), "only members of the council can schedule tasks");

            // current block number
            let proposed_at = <system::Module<T>>::block_number();
            // use current time to estimate the expected `BlockNumber` for execution
            let expected_execution = Self::execution_estimate(proposed_at);

            let task_to_schedule = Task {
                id: data.clone(),
                score: 0u32,
                proposed_at,
            };
            // add tasks as values to map with `TaskId` as the key
            // note: by default overwrites any value stored at the `data.clone()` key
            <PendingTasks<T>>::insert(data.clone(), task_to_schedule.clone());
            // add to TaskQ for scheduled execution
            <ExecutionQueue<T>>::append(&[task_to_schedule]);

            Self::deposit_event(RawEvent::TaskScheduled(proposer, data, expected_execution));
        }

        /// Increase Priority for the Task
        ///
        /// - members of the council have limited voting power to increase the priority
        /// of tasks
        fn signal_priority(origin, id: TaskId, signal: PriorityScore) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            Self::do_signal(voter, id, signal)
        }

        fn on_finalize(n: T::BlockNumber) {
            if (n % T::ExecutionFrequency::get()).is_zero() {
                Self::execute_tasks(n);
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// Execute Tasks
    ///
    /// - exhaustively executes tasks in the order of their priority
    pub fn execute_tasks(n: T::BlockNumber) {
        // task limit in terms of priority allowed to be executed every period
        let mut task_allowance = T::TaskLimit::get();
        let remove_queue = 6; // vec limited by task_allowance size
        let mut execution_q = <ExecutionQueue<T>>::get().clone();
        execution_q.sort_unstable();
        execution_q.into_iter().for_each(|t| {
            let task_id = t.id;
            if let Some(task) = <PendingTasks<T>>::get(&task_id) {
                if task.score <= task_allowance {
                    // execute task (could have more expressive computation here)
                    // or in off-chain worker running after this block
                    task_allowance -= task.score;
                    // could also add a field `cost` instead of score
                    Self::deposit_event(RawEvent::TaskExecuted(task_id.clone(), n));
                } else {
                    // need to explicitly end the loop when a single priority_score > task_allowance (prevent exhaustive execution)
                    return;
                }
            }
            <PendingTasks<T>>::remove(&task_id);
        });
    }
}

// Private Methods
impl<T: Trait> Module<T> {
    /// Checks whether the input member is in the council governance body
    fn is_on_council(who: &T::AccountId) -> bool {
        Self::council().contains(who)
    }
    /// Efficient Execution Estimate
    fn execution_estimate(n: T::BlockNumber) -> T::BlockNumber {
        let batch_frequency = T::ExecutionFrequency::get();
        let miss = n % batch_frequency;
        n + (batch_frequency - miss)
    }

    fn do_signal(voter: T::AccountId, id: TaskId, signal: PriorityScore) -> DispatchResult {
        ensure!(Self::is_on_council(&voter), "The voting member must be on the council");

        let current_era = <Era>::get();
        // get voter's remaining signal in this voting era
        let voters_signal = <SignalBank<T>>::get(current_era, &voter);
        ensure!(voters_signal >= signal, "The voter cannot signal more than their remaining signal");
        let mut task = Self::pending_tasks(id.clone()).ok_or(Error::<T>::TaskDNEStorage)?;
        task.score = task.score.checked_add(signal).ok_or(Error::<T>::TaskScoreOverflow)?;
        // explicitly write to storage (required!)
        let _ = <PendingTasks<T>>::insert(id.clone(), task);
        // no need to checked_sub because verified earlier above that voters_signal >= signal with an `ensure` check
        let remaining_signal = voters_signal - signal;
        <SignalBank<T>>::insert(current_era, &voter, remaining_signal);
        Self::deposit_event(RawEvent::SignalSupport(id, signal));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::*; //{Module, Trait, RawEvent, Task, GenesisConfig};
    use primitives::H256;
    use sp_io;
    use sp_runtime::{
        testing::Header,
        traits::{BlakeTwo256, IdentityLookup, OnFinalize, OnInitialize, SimpleArithmetic},
        Perbill,
    };

    // Helper Imports and Methods For Testing Purposes
    //
    // rand imports
    use rand::{rngs::OsRng, thread_rng, Rng, RngCore};
    use std::cell::RefCell;
    use support::{impl_outer_event, impl_outer_origin, parameter_types, traits::Get};
    use system::ensure_signed;

    // to compare expected storage items with storage items after method calls
    impl<BlockNumber: SimpleArithmetic + Copy> PartialEq for Task<BlockNumber> {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
    impl<BlockNumber: Copy + SimpleArithmetic> Eq for Task<BlockNumber> {}

    // bc I couldn't get the `add_extra_genesis` to work
    impl<T: Trait> Module<T> {
        fn add_member_to_council(who: T::AccountId) {
            <Council<T>>::mutate(|members| members.push(who));
        }

        fn add_member(who: T::AccountId) {
            Self::add_member_to_council(who.clone());
            let current_era = <Era>::get();
            // intialize with 0, filled full at beginning of next_era
            <SignalBank<T>>::insert(current_era, who, 0u32);
        }

        // Naive Execution Estimate
        //
        // emits an event parameter in `schedule_task` to tell users when
        // (which block number), the task is expected to be executed based on when it was submitted
        // - iteration makes it quite naive
        fn naive_execution_estimate(now: T::BlockNumber) -> T::BlockNumber {
            // the frequency with which tasks are batch executed
            let batch_frequency = T::ExecutionFrequency::get();
            let mut expected_execution_time = now;
            loop {
                // the expected execution time is the next block number divisible by `ExecutionFrequency`
                if (expected_execution_time % batch_frequency).is_zero() {
                    break;
                } else {
                    expected_execution_time += 1.into();
                }
            }
            expected_execution_time
        }
    }

    // Random Task Generation for (Future) Testing Purposes
    impl<BlockNumber: std::convert::From<u64>> Task<BlockNumber> {
        fn random() -> Self {
            let mut rng = thread_rng();
            let random_score: u32 = rng.gen();
            let random_block: u64 = rng.gen();
            Self {
                id: id_generate(),
                score: random_score.into(),
                proposed_at: random_block.into(),
            }
        }
    }

    // helper method fo task id generation (see above `random` method)
    pub fn id_generate() -> TaskId {
        let mut buf = vec![0u8; 32];
        OsRng.fill_bytes(&mut buf);
        buf.into()
    }

    impl_outer_origin! {
        pub enum Origin for TestRuntime {}
    }

    thread_local! {
        static SIGNAL_QUOTA: RefCell<u32> = RefCell::new(0);
        static EXECUTION_FREQUENCY: RefCell<u64> = RefCell::new(0);
        static TASK_LIMIT: RefCell<u32> = RefCell::new(0);
    }

    pub struct SignalQuota;
    impl Get<u32> for SignalQuota {
        fn get() -> u32 {
            SIGNAL_QUOTA.with(|v| *v.borrow())
        }
    }

    pub struct ExecutionFrequency;
    impl Get<u64> for ExecutionFrequency {
        fn get() -> u64 {
            EXECUTION_FREQUENCY.with(|v| *v.borrow())
        }
    }

    pub struct TaskLimit;
    impl Get<u32> for TaskLimit {
        fn get() -> u32 {
            TASK_LIMIT.with(|v| *v.borrow())
        }
    }

    // Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestRuntime;
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: u32 = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::one();
    }
    impl system::Trait for TestRuntime {
        type Origin = Origin;
        type Index = u64;
        type Call = ();
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = TestEvent;
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
        type ModuleToIndex = ();
    }

    mod task_scheduler {
        pub use crate::Event;
    }

    impl_outer_event! {
        pub enum TestEvent for TestRuntime {
            task_scheduler<T>,
        }
    }

    impl Trait for TestRuntime {
        type Event = TestEvent;
        type SignalQuota = SignalQuota;
        type ExecutionFrequency = ExecutionFrequency;
        type TaskLimit = TaskLimit;
    }

    pub type System = system::Module<TestRuntime>;
    pub type TaskScheduler = Module<TestRuntime>;

    pub struct ExtBuilder {
        signal_quota: u32,
        execution_frequency: u64,
        task_limit: u32,
    }
    impl Default for ExtBuilder {
        fn default() -> Self {
            Self {
                signal_quota: 100u32,
                execution_frequency: 5u64,
                task_limit: 10u32,
            }
        }
    }

    impl ExtBuilder {
        pub fn signal_quota(mut self, signal_quota: u32) -> Self {
            self.signal_quota = signal_quota;
            self
        }
        pub fn execution_frequency(mut self, execution_frequency: u64) -> Self {
            self.execution_frequency = execution_frequency;
            self
        }
        pub fn task_limit(mut self, task_limit: u32) -> Self {
            self.task_limit = task_limit;
            self
        }
        pub fn set_associated_consts(&self) {
            SIGNAL_QUOTA.with(|v| *v.borrow_mut() = self.signal_quota);
            EXECUTION_FREQUENCY.with(|v| *v.borrow_mut() = self.execution_frequency);
            TASK_LIMIT.with(|v| *v.borrow_mut() = self.task_limit);
        }
        pub fn build(self) -> sp_io::TestExternalities {
            self.set_associated_consts();
            let t = system::GenesisConfig::default()
                .build_storage::<TestRuntime>()
                .unwrap();
            // GenesisConfig::<TestRuntime> {
            //     council_members: vec![1, 2, 3, 4, 5, 6],
            // }.assimilate_storage(&mut t).unwrap();
            t.into()
        }
    }

    /// Auxiliary method for simulating block time passing
    fn run_to_block(n: u64) {
        while System::block_number() < n {
            TaskScheduler::on_finalize(System::block_number());
            System::set_block_number(System::block_number() + 1);
            TaskScheduler::on_initialize(System::block_number() + 1);
        }
    }

    #[test]
    fn eras_change_correctly() {
        ExtBuilder::default()
            .execution_frequency(2)
            .build()
            .execute_with(|| {
                System::set_block_number(1);
                run_to_block(13);
                assert_eq!(TaskScheduler::era(), 6);
                run_to_block(32);
                assert_eq!(TaskScheduler::era(), 16);
            })
    }

    #[test]
    fn naive_estimator_works() {
        ExtBuilder::default()
            .execution_frequency(8)
            .build()
            .execute_with(|| {
                let current_block = 5u64;
                assert_eq!(
                    TaskScheduler::naive_execution_estimate(current_block.into()),
                    8u64.into()
                );
                assert_eq!(
                    TaskScheduler::execution_estimate(current_block.into()),
                    8u64.into()
                );
                let next_block = 67u64;
                assert_eq!(
                    TaskScheduler::naive_execution_estimate(next_block.into()),
                    72u64.into()
                );
                assert_eq!(
                    TaskScheduler::execution_estimate(next_block.into()),
                    72u64.into()
                );
            })
    }

    #[test]
    fn estimator_works() {
        ExtBuilder::default()
            .execution_frequency(8)
            .build()
            .execute_with(|| {
                let current_block = 5u64;
                assert_eq!(
                    TaskScheduler::execution_estimate(current_block.into()),
                    8u64.into()
                );
                let next_block = 67u64;
                assert_eq!(
                    TaskScheduler::execution_estimate(next_block.into()),
                    72u64.into()
                );
            })
    }

    #[test]
    fn schedule_task_behaves() {
        ExtBuilder::default()
            .execution_frequency(10)
            .build()
            .execute_with(|| {
                // this is unsatisfying/unsafe and should be changed
                let first_account = ensure_signed(Origin::signed(0)).unwrap_or(0);
                TaskScheduler::add_member(first_account.clone());
                assert!(TaskScheduler::is_on_council(&first_account));
                System::set_block_number(2);
                let new_id = id_generate();
                let _ = TaskScheduler::schedule_task(Origin::signed(1), new_id.clone());

                // check storage changes
                let expected_task: Task<u64> = Task {
                    id: new_id.clone(),
                    score: 0u32,
                    proposed_at: 2u64,
                };
                let task = TaskScheduler::pending_tasks(new_id.clone());
                assert_eq!(
                    // unwrap is necessary for comparison, but this doesn't check existence (it assumes that)
                    task,
                    Some(expected_task)
                );
                assert_eq!(TaskScheduler::execution_queue(), vec![new_id.clone()]);

                // check event behavior
                let expected_event =
                    TestEvent::task_scheduler(RawEvent::TaskScheduled(first_account, new_id, 10));
                assert!(System::events().iter().any(|a| a.event == expected_event));
            })
    }

    #[test]
    fn priority_signalling_behaves() {
        ExtBuilder::default()
            .execution_frequency(5)
            .signal_quota(10)
            .task_limit(1)
            .build()
            .execute_with(|| {
                System::set_block_number(2u64);
                let first_account = ensure_signed(Origin::signed(1)).unwrap_or(1);
                let second_account = ensure_signed(Origin::signed(2)).unwrap_or(2);
                let new_task = id_generate();
                let _ = TaskScheduler::add_member(first_account);
                let _ = TaskScheduler::add_member(second_account);

                // refresh signal_quota
                run_to_block(7u64);

                let _ = TaskScheduler::schedule_task(Origin::signed(2), new_task.clone());

                let _ = TaskScheduler::signal_priority(Origin::signed(1), new_task.clone(), 2u32.into());

                // check that banked signal has decreased
                assert_eq!(
                    TaskScheduler::signal_bank(1u32, &first_account),
                    8u32.into()
                );

                // check that task priority has increased
                assert_eq!(
                    TaskScheduler::pending_tasks(new_task.clone())
                        .unwrap()
                        .score,
                    2u32.into()
                );
            })
    }
}
