use color_eyre::eyre::Context;
use poem::{listener::TcpListener, Server};
use poem_grpc::{Response, RouteGrpc, Status};

poem_grpc::include_proto!("notifier");

struct NotificationService;

#[poem::async_trait]
impl Notifier for NotificationService {
    async fn send_notification(
        &self,
        request: poem_grpc::Request<Notification>,
    ) -> Result<Response<()>, Status> {
        let notification = request.into_inner();

        // TODO: In the future, actually send notification to user
        println!("Received notification request: {:?}", notification);

        Ok(Response::new(()))
    }
}

#[derive(serde::Deserialize, Debug)]
struct Env {
    port: String,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Envars
    dotenvy::dotenv().ok();
    let env = envy::from_env::<Env>().context("Failed to parse environment variables")?;

    // Setup tracing/logging
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=debug,info");
    }
    tracing_subscriber::fmt::init();

    Server::new(TcpListener::bind(format!("127.0.0.1:{}", env.port)))
        .run(RouteGrpc::new().add_service(NotifierServer::new(NotificationService)))
        .await?;

    Ok(())
}
