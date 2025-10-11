use std::collections::VecDeque;
use std::sync::Arc;
use chrono::Local;
use colored::*;
use tokio::sync::Mutex;
use chrono::DateTime;

#[derive(Clone)]
pub struct App {
    pub input: String,
    pub logs: VecDeque<String>,
    pub console: VecDeque<String>,
    pub connections: Vec<Connection>,
    pub metrics: Vec<String>,
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
    pub fn new() -> Self {
        Self {
            input: String::new(),
            logs: VecDeque::with_capacity(100),
            console: VecDeque::with_capacity(100),
            connections: vec![],
            metrics: vec!["Balance: 0".to_string(), "TPS: 0".to_string()],
        }
    }

    pub fn log(&mut self, msg: String) {
        if self.logs.len() == 100 {
            self.logs.pop_front();
        }
        self.logs.push_back(msg.trim_end().to_string());
    }

    pub fn console(&mut self, msg: String) {
        if self.console.len() == 100 {
            self.console.pop_front();
        }
        self.console.push_back(msg.trim_end().to_string());
    }

    pub fn add_connection(&mut self, conn: Connection) {
        self.connections.push(conn);
    }

    pub fn remove_connection(&mut self, ip: &str, port: u16) {
        self.connections.retain(|c| !(c.ip == ip && c.port == port));
    }

    pub fn update_connection_data_received(&mut self, ip: &str, port: u16, bytes: usize) {
        if let Some(conn) = self.connections.iter_mut()
            .find(|c| c.ip == ip && c.port == port)
        {
            conn.data_received += bytes as u64;
        }
    }

    pub fn update_connection_data_sent(&mut self, ip: &str, port: u16, bytes: usize) {
        if let Some(conn) = self.connections.iter_mut()
            .find(|c| c.ip == ip && c.port == port)
        {
            conn.data_sent += bytes as u64;
        }
    }


    pub fn add_metric(&mut self, metric: String) {
        self.metrics.push(metric);
    }

    pub fn clear_metrics(&mut self) {
        self.metrics.clear();
    }
}