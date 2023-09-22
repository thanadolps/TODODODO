use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new()
        .build_server(false)
        .build_client(true)
        .compile(&["./proto/notification.proto"], &["./proto"])
}
