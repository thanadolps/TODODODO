use std::io::Result;

use poem_grpc_build::Config;

fn main() -> Result<()> {
    Config::new().compile(&["./proto/performance.proto"], &["./proto"])
}
