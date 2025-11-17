use chrono::Local;
use std::borrow::Cow;
use std::sync::Arc;
use chrono::DateTime;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use crate::network::firewall::Firewall;

pub struct App {
    pub input: Mutex<String>,
    pub log_tx: broadcast::Sender<Cow<'static, str>>,
    pub console_tx: broadcast::Sender<Cow<'static, str>>,
    pub conn_tx: broadcast::Sender<ConnectionEvent>,
    pub metrics_tx: broadcast::Sender<(f32, f32)>,
    pub firewall: Mutex<Firewall>
}

#[derive(Clone)]
pub enum ConnectionEvent {
    Add(Connection),
    Remove(String, u16),
    UpdateDataSent(String, u16, u64),
    UpdateDataReceived(String, u16, u64),
}

#[derive(Clone, Debug)]
pub struct Connection {
    pub name : String,
    pub ip: String,
    pub port: u16,
    pub status: bool,
    pub data_sent: u64,
    pub data_received: u64,
    pub uptime: DateTime<Local>,
}

impl App {
    pub fn new() -> Arc<Self>  {
        let (log_tx, _) = broadcast::channel(100);
        let (console_tx, _) = broadcast::channel(100);
        let (conn_tx, _) = broadcast::channel(100);
        let (metrics_tx, _) = broadcast::channel(100);

        Arc::new(Self { 
            input: Mutex::new(String::new()),
            firewall: Mutex::new(Firewall::new("eth0").unwrap()),
            log_tx, 
            console_tx,
            conn_tx,
            metrics_tx
        })
    }

    pub fn log(&self, msg: impl Into<Cow<'static, str>>) {
        let _ = self.log_tx.send(msg.into());
    }

    pub fn console(&self, msg: impl Into<Cow<'static, str>>) {
        let _ = self.console_tx.send(msg.into());
    }

    pub fn add_connection(&self, conn: Connection) {
        let _ = self.conn_tx.send(ConnectionEvent::Add(conn));
    }

    pub fn remove_connection(&self, ip: &str, port: u16) {
        let _ = self.conn_tx.send(ConnectionEvent::Remove(ip.to_string(), port));
    }

    pub fn update_connection_data_received(&self, ip: &str, port: u16, bytes: u64) {
        let _ = self.conn_tx.send(ConnectionEvent::UpdateDataReceived(ip.to_string(), port, bytes));
    }

    pub fn update_connection_data_sent(&self, ip: &str, port: u16, bytes: u64) {
        let _ = self.conn_tx.send(ConnectionEvent::UpdateDataSent(ip.to_string(), port, bytes));
    }
}