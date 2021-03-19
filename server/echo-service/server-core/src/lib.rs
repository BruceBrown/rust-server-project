use async_trait::async_trait;
use crossbeam::atomic::AtomicCell;
use num_cpus;
use once_cell::sync::Lazy;
use smol::{self};
use std::{
    fmt,
    panic::catch_unwind,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};
use uuid::Uuid;

mod background_task;
mod machine_adpter;
mod machine_builder;

pub use background_task::BackgroundTask;
pub use machine_builder::MachineBuilder;

/// The server-core library is the lowest layer. It is dependent upon external
/// crates and the core library. If you get a circular dependency error, it is
/// becasue you violated this rule.

/// The MachineImpl binds an adapter and instruction set together via an enum variant.
pub trait MachineImpl: 'static + Send + Sync {
    type Adapter;
    type InstructionSet: Send + Sync + Clone;
}

/// The Machine trait must be implemented by each machine model for each instruction set that
/// it supports.
pub trait Machine<T>: Send + Sync
where
    T: 'static + Send + Sync,
{
    fn receive(&self, cmd: T, sender: &mut MachineSender);
    fn disconnected(&self) {}
    fn connected(&self, _uuid: uuid::Uuid) {}
}

/// The AsyncSender trait exposes an async fn for sending an instruction to a sender.
#[async_trait]
trait AsyncSender: Send + Sync {
    async fn do_send(&mut self);
}

/// The SharedMachine wraps a machine
pub type SharedMachine<T> = Arc<T>;

/// The MachineSender object is opaque, exposing a single send method. It is used by the receiver to send
/// instructions to other machines.
#[derive(Default)]
pub struct MachineSender {
    queue: Vec<Box<dyn AsyncSender>>,
}
impl MachineSender {
    /// Send an instruction to another machine.
    pub fn send<T: MachineImpl>(&mut self, sender: smol::channel::Sender<T>, cmd: T) {
        let sender = Box::new(SendContext(sender, Some(cmd))) as Box<dyn AsyncSender>;
        self.queue.push(sender);
    }
}

// The SendContext contains a Sender and Instruction. Its used by the MachineSender.
struct SendContext<T: MachineImpl>(::smol::channel::Sender<T>, Option<T>);

// The implementation of SendContext, which erases the generic type T.
#[async_trait]
impl<T> AsyncSender for SendContext<T>
where
    T: MachineImpl,
{
    async fn do_send(&mut self) { self.0.send(self.1.take().unwrap()).await.ok(); }
}

// Seed for dispersing machines across executors.
static EXECUTOR_SEED: AtomicUsize = AtomicUsize::new(0);

#[allow(non_upper_case_globals)]
// The default number of threads to use. If 0, it will default to the number of CPUs available.
static default_num_threads: AtomicCell<usize> = AtomicCell::new(0);

/// The executors, as a tupple of: executors, join handles, and a sender.
/// When the sender is closed the executors will terminate.
static EXECUTOR: Lazy<(
    Vec<Arc<::smol::Executor<'_>>>,
    Vec<thread::JoinHandle<()>>,
    smol::channel::Sender<()>,
)> = Lazy::new(|| {
    let handles: Vec<thread::JoinHandle<()>> = Vec::new();
    let (s, r) = ::smol::channel::unbounded::<()>();
    let mut executors: Vec<Arc<::smol::Executor<'_>>> = Vec::new();
    let mut num_threads = default_num_threads.load();
    if num_threads == 0 {
        num_threads = num_cpus::get();
    }

    for n in 1 ..= num_threads {
        let e = Arc::new(::smol::Executor::new());
        let r = r.clone();
        executors.push(e.clone());
        thread::Builder::new()
            .name(format!("executor-{}", n))
            .spawn(move || loop {
                catch_unwind(|| ::smol::future::block_on(e.run(async { r.recv().await }))).ok();
            })
            .expect("cannot spawn executor thread");
    }
    (executors.clone(), handles, s)
});

// core functions begin here

/// Set the default number of threads to use, returning the previous value. If 0, the framework will default to the
/// number of CPUs available.
pub fn set_default_num_threads(num_threads: usize) -> usize {
    let res = get_default_num_threads();
    default_num_threads.store(num_threads);
    res
}
/// Get the default number of threads to use. If 0, the framework will default to the
/// number of CPUs available.
pub fn get_default_num_threads() -> usize { default_num_threads.load() }

/// Get an executor, selecting one of the executors in the pool of executors.
pub fn get_executor() -> Arc<smol::Executor<'static>> {
    let next = EXECUTOR_SEED.fetch_add(1, Ordering::SeqCst);
    let idx = next % EXECUTOR.0.len();
    EXECUTOR.0[idx].clone()
}

pub fn stop_executors() { EXECUTOR.2.close(); }

#[cfg(test)]
mod tests {
    // While its unlikely there will be any tests, it doesn't hurt to leave this here.
}
