use super::*;
use smart_default::*;

use futures::{future::FutureExt, pin_mut, select};
use log;
use smol;

/// BackgroundTask is a task wrapper allowing a task to run detached, while also allowing it to be cancelled.
///
/// Examples:
///
/// ```rust
/// use server_core::BackgroundTask;
///
/// let address = "127.0.0.1:1999";
///
/// // This is the task we want to run in the background. It never finishes
/// let task = smol::spawn(async move {
///     let listener = smol::net::TcpListener::bind(address)
///         .await
///         .unwrap_or_else(|err| panic!("Unable to bind to {} error={:#?}", address, err));
///     loop {
///         log::info!("waiting for connection on {}", address);
///         if let Ok((_stream, addr)) = listener.accept().await {
///             log::info!("accepted connection on {} from {}", address, addr.to_string());
///         }
///     }
/// });
///
/// // Detach task and provide a logging label
/// let task = BackgroundTask::detach(task, "listener");
///
/// log::info!("server will run for 1 seconds");
/// std::thread::sleep(std::time::Duration::from_secs(1));
///
/// log::info!("server should shutdown listener now");
///
/// // Cancel the task
/// task.cancel();
///
/// std::thread::sleep(std::time::Duration::from_secs(1));
/// ```
#[derive(Debug, SmartDefault)]
pub struct BackgroundTask {
    #[default(smol::channel::unbounded::<()>().0)]
    sender: smol::channel::Sender<()>,
}
#[allow(unused)]
impl BackgroundTask {
    /// Detach the task and provide a logging label.
    pub fn detach<T: 'static + Send>(task: smol::Task<T>, label: &str) -> Self {
        let (sender, receiver) = smol::channel::unbounded::<()>();
        let executor = get_executor();
        let t1 = executor
            .spawn(async move { receiver.recv().await.unwrap_or_else(|_err| ()) })
            .fuse();
        let t2 = task.fuse();
        let label = label.to_string();
        executor
            .spawn(async move {
                pin_mut!(t1, t2);
                select! {
                    () = t1 => log::trace!("{} closed", label), // completes when sender is closed
                    _ = t2 => log::trace!("{} task completed", label), // never completes
                }
                log::debug!("{} completed", label);
            })
            .detach();
        Self { sender }
    }

    /// Cancel the detached task.
    pub fn cancel(&self) { self.sender.close(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore]
    fn test_main() {
        use simplelog::*;
        use std::str::FromStr;

        // simplelog setup:
        let level_filter = <log::LevelFilter as FromStr>::from_str("debug").unwrap();
        CombinedLogger::init(vec![TermLogger::new(level_filter, Config::default(), TerminalMode::Mixed)]).unwrap();

        let address = "127.0.0.1:1999";
        // nc -vz 127.0.0.1 1999 should connect and drop

        let task = smol::spawn(async move {
            let listener = smol::net::TcpListener::bind(address)
                .await
                .unwrap_or_else(|err| panic!("Unable to bind to {} error={:#?}", address, err));
            loop {
                log::info!("waiting for connection on {}", address);
                if let Ok((_stream, addr)) = listener.accept().await {
                    log::info!("accepted connection on {} from {}", address, addr.to_string());
                }
            }
        });
        let task = BackgroundTask::detach(task, "listener");

        log::info!("server will run for 5 seconds");
        std::thread::sleep(std::time::Duration::from_secs(5));

        log::info!("server should shutdown listener now");
        task.cancel();
        // should no longer be able to connect
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
