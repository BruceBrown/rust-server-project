use crossbeam::atomic::AtomicCell;
use num_cpus;
use once_cell::sync::Lazy;
use std::{
    panic::catch_unwind,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

#[allow(non_upper_case_globals)]
/// The default number of threads to use. If 0, it will default to the
/// number of CPUs available.
pub static default_num_threads: AtomicCell<usize> = AtomicCell::new(0);

// Seed for dispersing machines across executors.
static EXECUTOR_SEED: AtomicUsize = AtomicUsize::new(0);

/// The executors, as a tupple of: executors, join handles, and a sender.
/// When the sender is closed the executors will terminate.
pub static EXECUTOR: Lazy<(Vec<Arc<::smol::Executor<'_>>>, Vec<thread::JoinHandle<()>>, smol::channel::Sender<()>)> = Lazy::new(|| {
    let handles: Vec<thread::JoinHandle<()>> = Vec::new();
    let (s, r) = ::smol::channel::unbounded::<()>();
    let mut executors: Vec<Arc<::smol::Executor<'_>>> = Vec::new();
    let mut num_threads = default_num_threads.load();
    if num_threads == 0 {
        num_threads = log_and_get_cpus();
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

fn log_and_get_cpus() -> usize {
    let logical_cpus = num_cpus::get();
    let physical_cpus = num_cpus::get_physical();
    log::info!("logical_cpus={} physical_cpus={}", logical_cpus, physical_cpus);
    logical_cpus
}

/// Get an executor, selecting one of the executors in the pool of executors.
pub fn get_executor() -> Arc<smol::Executor<'static>> {
    let next = EXECUTOR_SEED.fetch_add(1, Ordering::SeqCst);
    let idx = next % EXECUTOR.0.len();
    EXECUTOR.0[idx].clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    // use simplelog::*;

    #[test]
    fn executors() {
        // if only the executors test it run, you'll get some logging
        // CombinedLogger::init(vec![TermLogger::new(LevelFilter::Trace, Config::default(), TerminalMode::Mixed)]).unwrap();
        let _ex = get_executor();
    }
}
