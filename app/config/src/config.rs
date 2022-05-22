// Copyright 2022 Database Mesh Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.ro
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{fs::File, io::prelude::*};

use clap::{Arg, Command};
use proxy::proxy::{ProxiesConfig, ProxyConfig};
use proxy_mysql::proxy::{MySQLNode, MySQLNodes};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub admin: Admin,
    pub mysql: Option<MySQLNodes>,
    pub proxy: Option<ProxiesConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Admin {
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MiniProxyConfig {
    pub admin: Admin,
    pub proxies: Vec<ProxyConfig>,
    pub mysql_nodes: Vec<MySQLNode>,
}

impl MiniProxyConfig {
    pub fn get_proxies(&self) -> &Vec<ProxyConfig> {
        &self.proxies
    }

    pub fn load_config() -> Self {
        let matches = Command::new("Pisa-Proxy")
            .arg(Arg::new("port").short('p').long("port").help("Http port").takes_value(true))
            .arg(Arg::new("config").short('c').long("config").help("Config path").takes_value(true))
            .arg(Arg::new("loglevel").long("log-level").help("Log level").takes_value(true))
            .get_matches();
        let config: Config;

        let mut config_path = "etc/config.toml";

        if let Some(path) = matches.value_of("config") {
            config_path = path;
        }

        let mut file = match File::open(config_path) {
            Err(e) => {
                eprintln!("{:?}", e);
                std::process::exit(-1);
            }
            Ok(file) => file,
        };

        let mut config_str = String::new();
        file.read_to_string(&mut config_str).unwrap();
        config = toml::from_str(&config_str).unwrap();

        let mut mini_proxy_config =
            MiniProxyConfig { admin: config.admin, proxies: vec![], mysql_nodes: vec![] };

        if let Some(config) = config.proxy {
            if let Some(app_config) = config.configs {
                for app in app_config {
                    mini_proxy_config.proxies.push(app);
                }
            }
        }

        if let Some(mysql) = config.mysql {
            if let Some(mysql_nodes) = mysql.nodes {
                mini_proxy_config.mysql_nodes = mysql_nodes;
            }
        }

        mini_proxy_config
    }
}
