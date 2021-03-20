// This could be made a lot simpler, however, we're going to illustrate running an instruction set.
use components::{NetCmd, NetConnId, NetCore, NetSender, ServerService, ServiceResult, ServiceState};
use machine_foundation::{get_executor, machine, Machine, MachineSender};

// piggy-back on the config-service example
use config_service::{Service, ServiceConfig, Settings};

use smol::lock::Mutex;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct EchoService {
    controller: Arc<Mutex<Controller>>,
    config: Service,
    state: Arc<Mutex<ServiceState>>,
}

impl ServerService for EchoService {
    fn get_name(&self) -> &str { "echo-service" }
    fn get_drain_count(&self) -> usize { smol::block_on(async { self.controller.lock().await.get_connection_count() }) }
    fn start(&mut self) -> ServiceResult<()> {
        log::debug!("echo service preparing to start");
        let address = format!("127.0.0.1:{}", self.config.server.port);
        let state = self.state.clone();
        let executor = get_executor();
        let controller = self.controller.clone();
        executor
            .spawn(async move {
                let net_sender = NetCore::get_sender();
                let (sender, receiver) = smol::channel::unbounded::<NetCmd>();
                net_sender.send(NetCmd::BindTcpListener(address, sender)).await.ok();
                while let Ok(cmd) = receiver.recv().await {
                    let state = state.lock().await;
                    if *state == ServiceState::Stopped {
                        break;
                    }
                    controller.lock().await.handle_cmd(cmd, &*state).await;
                }
            })
            .detach();
        smol::block_on(async {
            let mut state = self.state.lock().await;
            state.start()
        })
    }

    fn run(&mut self) -> ServiceResult<()> {
        log::debug!("echo service preparing to run");
        smol::block_on(async { self.state.lock().await.run() })
    }

    fn drain(&mut self) -> ServiceResult<()> {
        log::debug!("echo service preparing to drain, connection_count={}", self.get_drain_count());
        smol::block_on(async { self.state.lock().await.drain() })
    }

    fn stop(&mut self) -> ServiceResult<()> {
        log::debug!("echo service preparing to stop");
        smol::block_on(async { self.state.lock().await.stop() })
    }
}

#[allow(dead_code)]
impl EchoService {
    /// Create the service. The config parameter is configuration for the service, while the settings
    /// parameter is settings for the server. Generally, it can be ignored, however there may be
    /// services which need to know features, the envionment, or other settings.
    pub fn create(config: &ServiceConfig, _settings: &Settings) -> Option<Box<dyn ServerService>> {
        NetCore::start();
        if let ServiceConfig::EchoService(config) = config {
            let net_sender = NetCore::get_sender();
            let controller = Arc::new(Mutex::new(Controller::new(net_sender)));
            let res = Self {
                controller,
                config: config.clone(),
                state: Arc::new(Mutex::new(ServiceState::default())),
            };
            let res = Box::new(res) as Box<dyn ServerService>;
            Some(res)
        } else {
            None
        }
    }
}

#[derive(Debug)]
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

    fn get_connection_count(&self) -> usize { self.connections.len() }

    async fn handle_cmd(&mut self, cmd: NetCmd, state: &ServiceState) {
        match cmd {
            NetCmd::NewConn(conn_id, local_addr, remote_addr) if state.is_running() => {
                log::debug!(
                    "new connection conn_id={}, local_addr={}, remote_addr={}",
                    conn_id,
                    local_addr,
                    remote_addr
                );
                let connection = EchoConnection::new(self.net_sender.clone());
                let (_, sender) = machine::create(connection);
                self.connections.insert(conn_id, sender.clone());
                log::info!("connection_count={}", self.connections.len());
                match self.net_sender.send(NetCmd::BindConn(conn_id, sender)).await {
                    Ok(()) => (),
                    Err(err) => log::warn!("failed to send to net_sender error={}", err),
                }
            },
            NetCmd::NewConn(conn_id, _, _) => {
                log::debug!("closing conn_id={} state={:#?}", conn_id, state);
                self.net_sender.send(NetCmd::CloseConn(conn_id)).await.ok();
            },
            NetCmd::CloseConn(conn_id) => {
                log::debug!("removing connection conn_id={}", conn_id,);
                self.connections.remove(&conn_id);
                log::info!("connection_count={}", self.connections.len());
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
