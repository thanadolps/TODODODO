use color_eyre::eyre::Context;
use poem::{listener::TcpListener, Server};
use poem_grpc::{Response, RouteGrpc, Status};

use gengrpc::notification::{NotificationDetail, Notifier, NotifierServer};
use webhook::client::WebhookClient;

struct NotificationService;

#[poem::async_trait]
impl Notifier for NotificationService {
    async fn send_notification(
        &self,
        request: poem_grpc::Request<NotificationDetail>,
    ) -> Result<Response<()>, Status> {
        let notification: NotificationDetail = request.into_inner();

        // TODO: In the future, actually send notification to user
        let url: &str = "https://discord.com/api/webhooks/1166039146989629563/Rylu9HS5c34vNSDMVY9LyhukJLtvV09-3MlN_QmsrGKQ-KFbIQd6E_aFZDqMSdlAqOgC";
        let msg = format!(
            "This is a notification for your task {} ({}). Description: {}. Deadline: {:?}",
            notification.title,
            notification.task_id,
            notification.description,
            notification.deadline
        );
        let client: WebhookClient = WebhookClient::new(url);
        client.send(|message| message.content(&msg)).await.unwrap();

        Ok(Response::new(()))
    }
}

#[derive(serde::Deserialize, Debug)]
struct Env {
    port: u16,
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
