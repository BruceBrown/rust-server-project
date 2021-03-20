use super::*;

use smol::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{Shutdown, TcpStream},
};

use super_slab::SuperSlab;

// This is where machines meet the network.
pub mod net {
    // this allows us to easily use ? for error handling
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

#[derive(SmartDefault)]
enum NetCoreField {
    #[default]
    Uninitialized,
    NetSender(NetSender),
    ServiceState(ServiceState),
}

#[allow(non_upper_case_globals)]
static netcore: AtomicRefCell<NetCore> = AtomicRefCell::new(NetCore::new());

#[derive(Default)]
pub struct NetCore {
    state: NetCoreField,
    // control: NetCoreField,
    sender: NetCoreField,
}

#[allow(dead_code)]
impl NetCore {
    const fn new() -> Self {
        Self {
            state: NetCoreField::ServiceState(ServiceState::Init),
            sender: NetCoreField::Uninitialized,
        }
    }
    pub fn start() {
        if let NetCoreField::ServiceState(ref mut state) = netcore.borrow_mut().state {
            if state.can_start() {
                state.start();
                log::info!("started network");
            } else {
                return;
            }
        }
        let (sender, receiver) = smol::channel::unbounded::<NetCmd>();
        get_executor()
            .spawn(async move {
                let mut controller = NetController::default();
                while let Ok(cmd) = receiver.recv().await {
                    match cmd {
                        NetCmd::Stop => break,
                        _ => {
                            controller.handle(cmd).await.ok();
                        },
                    }
                }
                // exit on channel close
            })
            .detach();
        netcore.borrow_mut().sender = NetCoreField::NetSender(sender);
        if let NetCoreField::ServiceState(ref mut state) = netcore.borrow_mut().state {
            state.run();
            log::info!("running network");
        }
    }

    pub fn get_sender() -> NetSender {
        let network = netcore.borrow();
        if let NetCoreField::ServiceState(ref state) = network.state {
            if state.is_running() {
                if let NetCoreField::NetSender(sender) = &network.sender {
                    return sender.clone();
                }
            }
        }
        smol::channel::unbounded().0
    }

    pub fn stop() {
        if let NetCoreField::ServiceState(ref mut state) = netcore.borrow_mut().state {
            if state.can_stop() {
                log::info!("stopping network");
                state.stop();
            }
        }
        if let NetCoreField::NetSender(sender) = &netcore.borrow().sender {
            sender.close();
        }
    }
}

#[derive(Debug)]
struct Server {
    is_dead: bool,
    bind_addr: String,
    listener_task: BackgroundTask,
    key: usize,
}

#[derive(Debug)]
struct Connection {
    stream: TcpStream,
    listener_sender: NetSender,
    sender: Option<NetSender>,
    recv_task: BackgroundTask,
}

#[derive(Debug, Default)]
struct NetController {
    servers: Arc<Mutex<SuperSlab<Server>>>,
    connections: Arc<Mutex<SuperSlab<Connection>>>,
}
impl NetController {
    async fn handle(&mut self, cmd: NetCmd) -> net::Result<()> {
        match cmd {
            NetCmd::BindTcpListener(address, sender) => {
                self.bind_tcp_listener(address, sender).await.ok();
            },
            NetCmd::BindUdpListener(address, sender) => {
                self.bind_udp_listener(address, sender).await.ok();
            },
            NetCmd::BindConn(conn_id, sender) => {
                self.bind_conn(conn_id, sender).await.ok();
            },
            NetCmd::CloseConn(conn_id) => {
                self.close_conn(conn_id).await.ok();
            },
            NetCmd::SendBytes(conn_id, bytes) => {
                self.send_bytes(conn_id, bytes).await.ok();
            },
            NetCmd::SendPkt(conn_id, address, bytes) => {
                self.send_pkt(conn_id, address, bytes).await.ok();
            },
            _ => {
                self.unknown_cmd(&cmd);
            },
        };
        Ok(())
    }
    fn unknown_cmd(&mut self, _cmd: &NetCmd) {}
    async fn bind_tcp_listener(&mut self, address: String, sender: NetSender) -> net::Result<()> {
        let executor = get_executor();
        let task = {
            log::debug!("tcp_listener bound to local_addr={}", address);
            let address = address.clone();
            let connections = self.connections.clone();
            executor.spawn(async move {
                match smol::net::TcpListener::bind(address.clone()).await {
                    Ok(listener) => loop {
                        if let Ok((stream, addr)) = listener.accept().await {
                            log::debug!("tcp_listener bound to local_addr={} accepted remote_addr={}", address, addr);
                            let connection = Connection {
                                stream,
                                listener_sender: sender.clone(),
                                sender: None,
                                recv_task: BackgroundTask::default(),
                            };
                            let mut connections = connections.lock().await;
                            let entry = connections.vacant_entry();
                            let id: usize = entry.key();
                            entry.insert(connection);
                            sender.send(NetCmd::NewConn(id, address.clone(), addr.to_string())).await.ok();
                        }
                    },
                    Err(_err) => {},
                }
            })
        };

        let task = BackgroundTask::detach(task, "listener");
        let mut servers = self.servers.lock().await;
        let entry = servers.vacant_entry();
        let key = entry.key();
        let server = Server {
            is_dead: false,
            bind_addr: address,
            listener_task: task,
            key,
        };
        entry.insert(server);
        Ok(())
    }

    async fn bind_udp_listener(&mut self, _address: String, _sender: NetSender) -> net::Result<()> { Ok(()) }

    async fn bind_conn(&mut self, conn_id: NetConnId, sender: NetSender) -> net::Result<()> {
        let mut connections = self.connections.lock().await;
        if let Some(conn) = connections.get_mut(conn_id) {
            let mut stream = conn.stream.clone();
            let listener_sender = conn.listener_sender.clone();
            let recv_task = get_executor().spawn(async move {
                loop {
                    let mut buf = vec![0u8; 1024];
                    match stream.read(&mut buf).await {
                        Ok(0) => {
                            sender.send(NetCmd::CloseConn(conn_id)).await.ok();
                            listener_sender.send(NetCmd::CloseConn(conn_id)).await.ok();
                            break;
                        },
                        Ok(bytes_read) => {
                            unsafe {
                                buf.set_len(bytes_read);
                            }
                            sender.send(NetCmd::RecvBytes(conn_id, buf)).await.ok();
                        },
                        Err(_err) => {
                            sender.send(NetCmd::CloseConn(conn_id)).await.ok();
                            listener_sender.send(NetCmd::CloseConn(conn_id)).await.ok();
                            break;
                        },
                    }
                }
            });
            let label = format!("connection id={}", conn_id);
            let recv_task = BackgroundTask::detach(recv_task, &label);
            conn.recv_task = recv_task;
        }
        Ok(())
    }

    async fn close_conn(&mut self, conn_id: NetConnId) -> net::Result<()> {
        let mut connections = self.connections.lock().await;
        if let Some(conn) = connections.get_mut(conn_id) {
            conn.recv_task.cancel();
            conn.stream.shutdown(Shutdown::Both).ok();
        }
        Ok(())
    }

    async fn send_bytes(&mut self, conn_id: NetConnId, mut bytes: Vec<u8>) -> net::Result<()> {
        let mut connections = self.connections.lock().await;
        log::debug!("preparing to send conn_id={}, bytes={}", conn_id, bytes.len());
        if let Some(conn) = connections.get_mut(conn_id) {
            let mut remaining = bytes.len();
            while remaining > 0 {
                if let Some(written) = conn.stream.write(&bytes).await.ok() {
                    log::debug!("sent conn_id={}, bytes={}", conn_id, written);
                    remaining -= written;
                    if remaining > 0 {
                        bytes.drain(0 .. written);
                    }
                }
            }
        }
        Ok(())
    }

    async fn send_pkt(&mut self, _conn_id: NetConnId, _address: String, _bytes: Vec<u8>) -> net::Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_start() { NetCore::start(); }
}
