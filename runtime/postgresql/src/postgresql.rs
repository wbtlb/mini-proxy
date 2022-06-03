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

use std::io::Error;

use conn_pool::conn_pool::Pool;
use proxy::{
    listener::listener::Listener,
    proxy::{Proxy, ProxyConfig},
};
use proxy::proxy::BackendNodeType;

use proxy_postgresql::proxy::PostgresqlNode;
use tracing::error;
use crate::postgresql::server::PostgresqlServer;

// use crate::server::server::MySqlServer;

pub struct PostgresqlProxy {
    pub proxy_config: ProxyConfig,
    pub postgresql_nodes: Vec<PostgresqlNode>,
}

#[async_trait::async_trait]
impl proxy::factory::Proxy for PostgresqlProxy {
    async fn start(&mut self) -> Result<(), Error> {
        let listener = Listener {
            backend_type: "postgresql".to_string(),
            listen_addr: self.proxy_config.listen_addr.clone(),
        };

        let mut proxy = Proxy {
            listener,
            app: self.proxy_config.clone(),
            backend_nodes: BackendNodeType::Postgresql(self.postgresql_nodes.clone()),
        };

        let listener = proxy.build_listener().unwrap();

        loop {
            let socket = proxy.accept(&listener).await.unwrap();
            // let pcfg = self.proxy_config.clone();

            // let mut mysql_server =
            //     MySqlServer::new(socket, pool, pcfg, self.mysql_nodes.clone()).await;

            let mut postgresql_server = PostgresqlServer::new(socket);
            // if let Err(err) = mysql_server.handshake().await {
            //     error!("{:?}", err);
            //     continue;
            // }

            // tokio::spawn(async move {
            //     if let Err(err) = mysql_server.run().await {
            //         error!("{:?}", err);
            //     }
            // });
        }
    }
}
