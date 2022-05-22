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

use bytes::{Buf, BufMut, BytesMut};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

pub struct MySqlServer {
    // pub client: Connection,
    pub buf: BytesMut,
    is_quit: bool,
}

impl MySqlServer {
    pub async fn new(client: TcpStream) -> MySqlServer {
        MySqlServer { buf: BytesMut::with_capacity(8192), is_quit: false }
    }
}
