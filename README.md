# mini-proxy
`mini-proxy` is an minimal implementation of Database proxy server.

The intent of this project is to provide a example of writing a Database protocol application. MayBe it includes: MySQL,Postgresql,redis etc.

## futures
- [x] simple toml config
- [x] mysql protocol support
- [x] mysql proxy
- [x] simple connection pool

## usage
### 1. build
```
cargo build --release
```
### 2. startup
```
./target/release/mini-proxy --config etc/config.toml
```

### 3. configration
```
[admin]
log_level = "INFO"

[proxy]
[[proxy.configs]]
listen_addr = "0.0.0.0:9088"
pool_size = 10
username = "root"
password = "12345678"
db = "test"
backend_type = "mysql"

[mysql]
[[mysql.nodes]]
name = "node001"
user = "root"
password = "12345678"
addr = "127.0.0.1:3306"
db = "test"
```

### 3. test connct proxy
```
#mysql -h 192.168.33.10 -P9088 -uroot -p12345678
Welcome to the MariaDB monitor.  Commands end with ; or \g.
Your MySQL connection id is 1
Server version: 5.7.37 mini-proxy 0.1.0 MariaDB Server

Copyright (c) 2000, 2018, Oracle, MariaDB Corporation Ab and others.

Type 'help;' or '\h' for help. Type '\c' to clear the current input statement.

MySQL [(none)]>
```
