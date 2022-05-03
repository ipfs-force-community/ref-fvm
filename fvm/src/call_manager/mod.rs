use std::time::Duration;

use fvm_ipld_encoding::RawBytes;
use fvm_shared::address::Address;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::{ActorID, MethodNum};

use crate::gas::{GasCharge, GasTracker, PriceList};
use crate::kernel::Result;
use crate::machine::{Machine, MachineContext};
use crate::state_tree::StateTree;
use crate::Kernel;

pub mod backtrace;

pub use backtrace::Backtrace;

mod default;

pub use default::DefaultCallManager;

use crate::trace::ExecutionTrace;

/// BlockID representing nil parameters or return data.
pub const NO_DATA_BLOCK_ID: u32 = 0;

/// The `CallManager` manages a single call stack.
///
/// When a top-level message is executed:
///
/// 1. The [`crate::executor::Executor`] creates a [`CallManager`] for that message, giving itself
///    to the [`CallManager`].
/// 2. The [`crate::executor::Executor`] calls the specified actor/method using
///    [`CallManager::send()`].
/// 3. The [`CallManager`] then constructs a [`Kernel`] and executes the actual actor code on that
///    kernel.
/// 4. If an actor calls another actor, the [`Kernel`] will:
///    1. Detach the [`CallManager`] from itself.
///    2. Call [`CallManager::send()`] to execute the new message.
///    3. Re-attach the [`CallManager`].
///    4. Return.
pub trait CallManager: 'static {
    /// The underlying [`Machine`] on top of which this [`CallManager`] executes.
    type Machine: Machine;

    /// Construct a new call manager.
    fn new(machine: Self::Machine, gas_limit: i64, origin: Address, nonce: u64) -> Self;

    /// Send a message. The type parameter `K` specifies the the _kernel_ on top of which the target
    /// actor should execute.
    fn send<K: Kernel<CallManager = Self>>(
        &mut self,
        from: ActorID,
        to: Address,
        method: MethodNum,
        params: &RawBytes,
        value: &TokenAmount,
    ) -> Result<InvocationResult>;

    /// Execute some operation (usually a send) within a transaction.
    fn with_transaction(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<InvocationResult>,
    ) -> Result<InvocationResult>;

    /// Finishes execution, returning the gas used, machine, and exec trace if requested.
    fn finish(self) -> (FinishRet, Self::Machine);

    /// Returns a reference to the machine.
    fn machine(&self) -> &Self::Machine;
    /// Returns a mutable reference to the machine.
    fn machine_mut(&mut self) -> &mut Self::Machine;

    /// Returns reference to the gas tracker.
    fn gas_tracker(&self) -> &GasTracker;
    /// Returns a mutable reference to the gas tracker.
    fn gas_tracker_mut(&mut self) -> &mut GasTracker;

    /// Getter for origin actor.
    fn origin(&self) -> Address;

    /// Getter for message nonce.
    fn nonce(&self) -> u64;

    /// Gets and increment the call-stack actor creation index.
    fn next_actor_idx(&mut self) -> u64;

    /// Returns the current price list.
    fn price_list(&self) -> &PriceList {
        self.machine().context().price_list
    }

    /// Returns the machine context.
    fn context(&self) -> &MachineContext {
        self.machine().context()
    }

    /// Returns the blockstore.
    fn blockstore(&self) -> &<Self::Machine as Machine>::Blockstore {
        self.machine().blockstore()
    }

    /// Returns the externs.
    fn externs(&self) -> &<Self::Machine as Machine>::Externs {
        self.machine().externs()
    }

    /// Returns the state tree.
    fn state_tree(&self) -> &StateTree<<Self::Machine as Machine>::Blockstore> {
        self.machine().state_tree()
    }

    /// Returns a mutable state-tree.
    fn state_tree_mut(&mut self) -> &mut StateTree<<Self::Machine as Machine>::Blockstore> {
        self.machine_mut().state_tree_mut()
    }

    /// Charge gas.
    fn charge_gas(&mut self, charge: GasCharge) -> Result<()> {
        self.gas_tracker_mut().charge_gas(charge)?;
        Ok(())
    }

    /// Returns a mutable reference to the execution stats to allow other
    /// components to update them.
    fn exec_stats_mut(&mut self) -> &mut ExecutionStats;

    /// Record a gas trace.
    #[cfg(feature = "tracing")]
    fn record_trace(
        &mut self,
        context: crate::gas::tracer::Context,
        point: crate::gas::tracer::Point,
        consumption: crate::gas::tracer::Consumption,
    );
}

/// The result of a method invocation.
#[derive(Clone, Debug)]
pub enum InvocationResult {
    /// Indicates that the actor successfully returned. The value may be empty.
    Return(RawBytes),
    /// Indicates that the actor aborted with the given exit code.
    Failure(ExitCode),
}

impl Default for InvocationResult {
    fn default() -> Self {
        Self::Return(Default::default())
    }
}

impl InvocationResult {
    /// Get the exit code for the invocation result. [`ExitCode::Ok`] on success, or the exit code
    /// from the [`Failure`](InvocationResult::Failure) variant otherwise.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Return(_) => ExitCode::OK,
            Self::Failure(e) => *e,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct ExecutionStats {
    /// Wasm fuel used over the course of the message execution.
    pub fuel_used: u64,
    /// Time spent inside wasm code.
    pub wasm_duration: Duration,
    /// Time spent setting up and tearing down wasm calls.
    pub call_overhead: Duration,
    /// Total number of actor calls (that invoke wasm).
    pub call_count: u64,
    /// Compute gas actually used.
    pub compute_gas: u64,
    /// Number of syscalls invoked.
    pub num_syscalls: u64,
    /// Number of externs invoked.
    pub num_externs: u64,
    /// Number of bytes read from state via ipld::block_open.
    pub block_bytes_read: u64,
    /// Number of bytes staged/written via ipld::block_write.
    pub block_bytes_written: u64,
}

/// The returned values upon finishing a call manager.
pub struct FinishRet {
    pub gas_used: i64,
    pub backtrace: Backtrace,
    pub exec_trace: ExecutionTrace,
    pub exec_stats: ExecutionStats,
}
