// This could be made a lot simpler, however, we're going to illustrate running an instruction set.
use components::{NetCmd, NetConnId, NetCore, NetSender, ServerService, ServiceState};
use machine_foundation::{get_executor, machine, Machine, MachineSender};

// piggy-back on the config-service example
use config_service::{Service, ServiceConfig, Settings};

use smol::lock::Mutex;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Default)]
pub struct EchoService {
    connection_count: usize,
    config: Service,
    state: Arc<Mutex<ServiceState>>,
}

impl ServerService for EchoService {
    fn get_name(&self) -> &str { "echo-service" }
    fn get_drain_count(&self) -> usize { self.connection_count }
    fn start(&mut self) -> Result<(), std::io::Error> {
        NetCore::start();
        let address = format!("127.0.0.1:{}", self.config.server.port);
        let state = self.state.clone();
        let executor = get_executor();
        executor
            .spawn(async move {
                let net_sender = NetCore::get_sender();
                let (sender, receiver) = smol::channel::unbounded::<NetCmd>();
                let mut controller = Controller::new(net_sender.clone());
                net_sender.send(NetCmd::BindTcpListener(address, sender)).await.ok();
                loop {
                    match receiver.recv().await {
                        Ok(cmd) => {
                            let state = *state.lock().await;
                            match state {
                                ServiceState::Init => {
                                    log::debug!("echo state=ServiceState::Init");
                                },
                                ServiceState::Started => {
                                    log::debug!("echo state=ServiceState::Started");
                                },
                                ServiceState::Running => {
                                    log::debug!("echo state=ServiceState::Running");
                                    controller.handle_cmd(cmd).await;
                                },
                                ServiceState::Draining => {
                                    log::debug!("echo state=ServiceState::Draining");
                                },
                                ServiceState::Stopped => {
                                    log::debug!("echo state=ServiceState::Stopped");
                                    break;
                                },
                            }
                        },
                        Err(_err) => break,
                    }
                }
                // we get here when stopped or on error
            })
            .detach();
        let state = self.state.clone();
        executor
            .spawn(async move {
                let mut guard = state.lock().await;
                (*guard).start();
            })
            .detach();
        Ok(())
    }

    fn run(&mut self) -> Result<(), std::io::Error> {
        log::debug!("echo service preparing to run");
        let state = self.state.clone();
        get_executor()
            .spawn(async move {
                let mut guard = state.lock().await;
                (*guard).run();
                log::debug!("echo service running");
            })
            .detach();
        Ok(())
    }

    fn drain(&mut self) -> Result<(), std::io::Error> {
        let state = self.state.clone();
        get_executor()
            .spawn(async move {
                let mut guard = state.lock().await;
                (*guard).drain();
            })
            .detach();
        Ok(())
    }

    fn stop(&mut self) -> Result<(), std::io::Error> {
        let state = self.state.clone();
        get_executor()
            .spawn(async move {
                let mut guard = state.lock().await;
                (*guard).stop();
            })
            .detach();
        Ok(())
    }
}

#[allow(dead_code)]
impl EchoService {
    /// Create the service. The config parameter is configuration for the service, while the settings
    /// parameter is settings for the server. Generally, it can be ignored, however there may be
    /// services which need to know features, the envionment, or other settings.
    pub fn create(config: &ServiceConfig, _settings: &Settings) -> Option<Box<dyn ServerService>> {
        if let ServiceConfig::EchoService(config) = config {
            let mut res = Self::default();
            res.config = config.clone();
            let res = Box::new(res) as Box<dyn ServerService>;
            Some(res)
        } else {
            None
        }
    }
}

struct Controller {
    net_sender: NetSender,
    connections: HashMap<NetConnId, NetSender>,
}
impl Controller {
    fn new(net_sender: NetSender) -> Self {
        Self {
            net_sender,
            connections: HashMap::new(),
        }
    }

    async fn handle_cmd(&mut self, cmd: NetCmd) {
        match cmd {
            NetCmd::NewConn(conn_id, local_addr, remote_addr) => {
                log::debug!(
                    "new connection conn_id={}, local_addr={}, remote_addr={}",
                    conn_id,
                    local_addr,
                    remote_addr
                );
                let connection = EchoConnection::new(self.net_sender.clone());
                let (_, sender) = machine::create(connection);
                self.connections.insert(conn_id, sender.clone());
                self.net_sender.send(NetCmd::BindConn(conn_id, sender)).await.ok();
            },
            NetCmd::CloseConn(conn_id) => {
                log::debug!("removing connection conn_id={}", conn_id,);
                self.connections.remove(&conn_id);
            },
            _ => (),
        }
    }
}

struct EchoConnection {
    net_sender: NetSender,
}
impl EchoConnection {
    fn new(net_sender: NetSender) -> Self { Self { net_sender } }
}
impl Machine<NetCmd> for EchoConnection {
    fn receive(&self, cmd: NetCmd, sender: &mut MachineSender) {
        match cmd {
            NetCmd::RecvBytes(conn_id, buf) => {
                log::debug!("conn_id={} received bytes={}", conn_id, buf.len());
                sender.send(self.net_sender.clone(), NetCmd::SendBytes(conn_id, buf));
            },
            NetCmd::CloseConn(conn_id) => {
                log::debug!("remote close conn_id={}", conn_id,);
            },
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {}
