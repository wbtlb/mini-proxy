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

use std::str::FromStr;

use config::config::MiniProxyConfig;
use proxy::factory::Factory;
use server::server::SimpleFactory;
use tokio::runtime::{Builder, Runtime};
use tracing::{error, info, Level};

fn main() {
    let config = MiniProxyConfig::load_config();
    tracing_subscriber::fmt()
        .with_max_level(Level::from_str(config.admin.log_level.as_str()).ok())
        .init();

    let mut servers = Vec::with_capacity(config.get_proxies().len());

    build_runtime().block_on(async move {
        for proxy_config in config.get_proxies() {
            let cfg = proxy_config.clone();
            let factory = SimpleFactory::new(cfg, config.clone());
            match proxy_config.backend_type.as_str() {
                "mysql" => servers.push(tokio::spawn(server::server::new_proxy_server(
                    factory.make_proxy(proxy::factory::ProxyKind::MySQL),
                ))),
                &_ => {}
            }
        }
        for server in servers {
            if let Err(err) = server.await {
                error!("{:?}", err)
            }
        }
    });
}

pub fn build_runtime() -> Runtime {
    let num_cpus = num_cpus::get();
    match num_cpus {
        0 | 1 => {
            info!("pisa-proxy running on current thread");
            Builder::new_current_thread()
                .thread_name("pisa-proxy")
                .enable_all()
                .build()
                .expect("failed to build runtime")
        }
        num_cpus => {
            info!("pisa-proxy running on multi thread");
            Builder::new_multi_thread()
                .thread_name("pisa-proxy")
                .worker_threads(num_cpus)
                .max_blocking_threads(num_cpus)
                .enable_all()
                .build()
                .expect("failed to build runtime")
        }
    }
}
