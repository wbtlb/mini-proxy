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

use std::{str, sync::Arc};

use byteorder::{ByteOrder, LittleEndian};
use bytes::{Buf, BufMut, BytesMut};
use conn_pool::conn_pool::{Pool, PoolConn};
use futures::StreamExt;
use mysql_protocol::{
    client::{codec::ResultsetStream, conn::ClientConn},
    err::ProtocolError,
    mysql_const::*,
    server::{conn::Connection, err::MySQLError},
    util::*,
};
use proxy::proxy::ProxyConfig;
use proxy_mysql::proxy::MySQLNode;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};
use tracing::{debug, error};

pub struct MySqlServer {
    pub client: Connection,
    pub buf: BytesMut,
    pub client_conn: Arc<Mutex<PoolConn<ClientConn>>>,
    is_quit: bool,
}

impl MySqlServer {
    pub async fn new(
        client: TcpStream,
        mut pool: Pool<ClientConn>,
        proxy_config: ProxyConfig,
        mysql_nodes: Vec<MySQLNode>,
    ) -> MySqlServer {
        let factory = ClientConn::with_opts(
            mysql_nodes[0].user.to_string(),
            mysql_nodes[0].password.to_string(),
            mysql_nodes[0].addr.to_string(),
        );

        pool.set_factory(factory);

        let client_conn = pool.get_conn().await.unwrap();

        let client_conn = Arc::new(Mutex::new(client_conn));

        MySqlServer {
            client: Connection::new(
                client,
                proxy_config.username,
                proxy_config.password,
                proxy_config.db,
            ),
            client_conn,
            buf: BytesMut::with_capacity(8192),
            is_quit: false,
        }
    }

    pub async fn handshake(&mut self) -> Result<(), ProtocolError> {
        if let Err(err) = self.client.handshake().await {
            if let ProtocolError::AuthFailed(err) = err {
                return self.client.pkt.write_buf(&err).await.map_err(ProtocolError::Io);
            }
            return Err(err);
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), ProtocolError> {
        let db = self.client.db.clone();
        if !db.is_empty() {
            self.handle_init_db(db.as_bytes(), false).await?
        }

        let mut buf = BytesMut::with_capacity(4096);

        loop {
            self.client.pkt.sequence = 0;

            let length = match self.client.pkt.read_packet_buf(&mut buf).await {
                Err(err) => return Err(ProtocolError::Io(err)),
                Ok(length) => length,
            };

            if self.is_quit {
                return Ok(());
            }

            if length == 0 {
                return Ok(());
            }

            if let Err(err) = self.handle_command(&mut buf).await {
                error!("exec command err: {:?}", err);
            };
        }
    }

    pub async fn handle_command(&mut self, data: &mut BytesMut) -> Result<(), ProtocolError> {
        let cmd = data.get_u8();
        let payload = data.split();

        match cmd {
            COM_INIT_DB => self.handle_init_db(&payload, true).await,
            COM_QUERY => self.handle_query(&payload).await,
            COM_FIELD_LIST => self.handle_field_list(&payload).await,
            COM_QUIT => self.handle_quit().await,
            COM_PING => self.handle_ok().await,
            COM_STMT_PREPARE => self.handle_prepare(&payload).await,
            COM_STMT_EXECUTE => self.handle_execute(&payload).await,
            COM_STMT_CLOSE => self.handle_stmt_close(&payload).await,
            COM_STMT_RESET => self.handle_ok().await,
            _ => self.handle_err(format!("command {} not support", cmd)).await,
        }
    }

    pub async fn handle_init_db(
        &mut self,
        payload: &[u8],
        is_send_ok: bool,
    ) -> Result<(), ProtocolError> {
        let sql = str::from_utf8(payload).unwrap().trim_matches(char::from(0));
        let conn = self.client_conn.clone();
        let mut client_conn = conn.lock().await;
        let res = client_conn.send_use_db(sql).await?;

        if res.1 {
            if is_send_ok {
                self.client.pkt.write_ok().await.map_err(ProtocolError::Io)
            } else {
                Ok(())
            }
        } else {
            let err_info = self.client.pkt.make_err_packet(MySQLError::new(
                1049,
                "42000".as_bytes().to_vec(),
                String::from_utf8_lossy(&res.0[4..]).to_string(),
            ));
            self.client.pkt.write_buf(&err_info).await.map_err(ProtocolError::Io)
        }
    }

    pub async fn handle_field_list(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let conn = self.client_conn.clone();
        let mut client_conn = conn.lock().await;
        let mut stream = client_conn.send_common_command(COM_FIELD_LIST, payload).await?;

        let mut buf = BytesMut::with_capacity(128);

        loop {
            let mut data = match stream.next().await {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            self.client.pkt.construct_packet_buf(&mut data, &mut buf).await;

            if is_eof(&data) {
                break;
            }
        }

        self.client.pkt.write_buf(&buf).await?;
        Ok(())
    }

    pub async fn handle_prepare(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let conn = self.client_conn.clone();
        let mut client_conn = conn.lock().await;
        let stmt = client_conn.send_prepare(payload).await?;

        let mut data = BytesMut::from(&vec![0; 4][..]);
        data.put_u8(0);
        data.extend_from_slice(&u32::to_le_bytes(stmt.stmt_id));
        data.extend_from_slice(&u16::to_le_bytes(stmt.cols_count));
        data.extend_from_slice(&u16::to_le_bytes(stmt.params_count));

        data.extend_from_slice(&[0, 0, 0]);

        self.client.pkt.make_packet_header(data.len() - 4, &mut data);

        if !stmt.params_data.is_empty() {
            for mut param_data in stmt.params_data {
                self.client.pkt.make_packet_header(param_data.len() - 4, &mut param_data);
                data.extend_from_slice(&param_data);
            }

            data.extend_from_slice(&self.client.pkt.make_eof_packet());
        }

        if !stmt.cols_data.is_empty() {
            for mut col_data in stmt.cols_data {
                self.client.pkt.make_packet_header(col_data.len() - 4, &mut col_data);
                data.extend_from_slice(&col_data);
            }

            data.extend_from_slice(&self.client.pkt.make_eof_packet());
        }

        self.client.pkt.write_buf(&data).await?;
        Ok(())
    }

    pub async fn handle_query(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let conn = self.client_conn.clone();
        let mut client_conn = conn.lock().await;
        let stream = client_conn.send_query(payload).await?;

        self.handle_query_resultset(stream).await
    }

    pub async fn handle_query_resultset<'b>(
        &mut self,
        mut stream: ResultsetStream<'b>,
    ) -> Result<(), ProtocolError> {
        let data = stream.next().await;

        let mut header = match data {
            Some(Ok(data)) => data,
            Some(Err(e)) => return Err(e),
            None => return Ok(()),
        };

        let ok_or_err = header[4];

        if ok_or_err == OK_HEADER || ok_or_err == ERR_HEADER {
            self.client.pkt.write_buf(&header).await?;
            return Ok(());
        }

        let (cols, ..) = length_encode_int(&header[4..]);
        self.buf.clear();

        self.client.pkt.construct_packet_buf(&mut header, &mut self.buf).await;

        for _ in 0..cols {
            let data = stream.next().await;
            let mut data = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            self.client.pkt.construct_packet_buf(&mut data, &mut self.buf).await;
        }

        let _ = stream.next().await;

        self.buf.extend_from_slice(&self.client.pkt.make_eof_packet());

        loop {
            let data = stream.next().await;

            let mut row = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            if is_eof(&row) {
                break;
            }

            self.client.pkt.construct_packet_buf(&mut row, &mut self.buf).await;
        }

        self.buf.extend_from_slice(&self.client.pkt.make_eof_packet());
        self.client.pkt.write_buf(&self.buf).await.map_err(ProtocolError::Io)?;

        Ok(())
    }

    pub async fn handle_execute(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let conn = self.client_conn.clone();
        let mut client_conn = conn.lock().await;
        let stream = client_conn.send_execute(payload).await?;
        self.handle_query_resultset(stream).await?;
        Ok(())
    }

    pub async fn handle_ok(&mut self) -> Result<(), ProtocolError> {
        self.client.pkt.write_ok().await.map_err(ProtocolError::Io)
    }

    pub async fn handle_err(&mut self, msg: String) -> Result<(), ProtocolError> {
        let err_info = self.client.pkt.make_err_packet(MySQLError::new(
            1047,
            "08S01".as_bytes().to_vec(),
            msg,
        ));

        self.client.pkt.write_buf(&err_info).await.map_err(ProtocolError::Io)
    }

    pub async fn handle_stmt_close(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let stmt_id = LittleEndian::read_u32(payload);
        debug!("stmt close {:?}", stmt_id);

        Ok(())
    }

    pub async fn handle_quit(&mut self) -> Result<(), ProtocolError> {
        self.is_quit = true;
        self.client.pkt.conn.shutdown().await.map_err(ProtocolError::Io)
    }
}
