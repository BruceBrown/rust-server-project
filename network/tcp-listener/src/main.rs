use futures::{future::FutureExt, pin_mut, select};
use log;
use smol;

/// BackgroundTask is a task wrapper allowing a task to run detached, while also allowing it to be cancelled.
///
/// Examples:
///
/// '''rust
/// let address = "127.0.0.1:1999";
///
/// // This is the task we want to rn in the background. It never finishes
/// let task = smol::spawn(async move {
///     let listener = smol::net::TcpListener::bind(address)
///         .await
///         .unwrap_or_else(|err| panic!("Unable to bind to {} error={:#?}", address, err));
///     loop {
///         log::info!("waiting for connection on {}", address);
///         if let Ok((_stream, addr)) = listener.accept().await {
///             log::info!("accepted connection on {} from {}", address, addr.to_string()
///         }
///     }
/// });
///
/// // Detach task and provide a logging label
/// let task = BackgroundTask::detach(task, "listener");
///
/// log::info!("server will run for 5 seconds");
/// std::thread::sleep(std::time::Duration::from_secs(5));
///
/// log::info!("server should shutdown listener now");
/// // Cancel the task
/// task.cancel();
/// std::thread::sleep(std::time::Duration::from_secs(5));
/// '''
struct BackgroundTask {
    sender: smol::channel::Sender<()>,
}
impl BackgroundTask {
    fn detach<T: 'static + Send>(task: smol::Task<T>, label: &str) -> Self {
        let (sender, receiver) = smol::channel::unbounded::<()>();
        let t1 = smol::spawn(async move { receiver.recv().await.unwrap_or_else(|_err| ()) }).fuse();
        let t2 = task.fuse();
        let label = label.to_string();
        smol::spawn(async move {
            pin_mut!(t1, t2);
            select! {
                () = t1 => log::debug!("{} closed", label), // completes when sender is closed
                _ = t2 => log::warn!("{} task completed", label), // never completes
            }
            log::debug!("{} completed", label);
        })
        .detach();
        Self { sender }
    }
    fn cancel(&self) {
        self.sender.close();
    }
}

use simplelog::*;
use std::str::FromStr;
fn main() {
    // simplelog setup:
    let level_filter = <log::LevelFilter as FromStr>::from_str("debug").unwrap();
    CombinedLogger::init(vec![TermLogger::new(
        level_filter,
        Config::default(),
        TerminalMode::Mixed,
    )])
    .unwrap();

    let address = "127.0.0.1:1999";
    // nc -vz 127.0.0.1 1999 should connect and drop

    let task = smol::spawn(async move {
        let listener = smol::net::TcpListener::bind(address)
            .await
            .unwrap_or_else(|err| panic!("Unable to bind to {} error={:#?}", address, err));
        loop {
            log::info!("waiting for connection on {}", address);
            if let Ok((_stream, addr)) = listener.accept().await {
                log::info!(
                    "accepted connection on {} from {}",
                    address,
                    addr.to_string()
                );
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_main() {
        main();
    }
}
