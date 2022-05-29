// Copyright 2022 Database Mesh Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use proxy_mysql::proxy::MySQLNode;
use proxy_postgresql::proxy::PostgresqlNode;
use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};

use crate::listener::listener::Listener;

#[derive(Debug, Deserialize, Clone)]
pub struct ProxiesConfig {
    pub configs: Option<Vec<ProxyConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    pub listen_addr: String,
    pub pool_size: u32,
    pub username: String,
    pub password: String,
    pub db: String,
    pub backend_type: String,
}

pub enum BackendNodeType {
    MySQL(Vec::<MySQLNode>),
    Postgresql(Vec::<PostgresqlNode>),
}

pub struct Proxy {
    pub listener: Listener,
    pub app: ProxyConfig,
    pub backend_nodes: BackendNodeType,
    // pub postgresql_nodes: Vec<PostgresqlNode>,
}

impl Proxy {
    pub fn build_listener(&mut self) -> Result<TcpListener, std::io::Error> {
        self.listener.build_listener()
    }

    pub async fn accept(&mut self, listener: &TcpListener) -> Result<TcpStream, std::io::Error> {
        self.listener.accept(listener).await
    }
}
