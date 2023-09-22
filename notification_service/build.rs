use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .build_server(true)
        .build_client(false)
        .compile(&["./proto/notification.proto"], &["./proto"])
}
