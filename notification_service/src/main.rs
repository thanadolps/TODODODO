use futures_lite::stream::StreamExt;
use lapin::{
    options::*, publisher_confirm::Confirmation, types::FieldTable, BasicProperties, Connection,
    ConnectionProperties, Result,
};
use tracing::info;

fn main() -> Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    let addr = std::env::var("AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

    async_global_executor::block_on(async {
        let conn = Connection::connect(&addr, ConnectionProperties::default()).await?;

        info!("Notification Service CONNECTED!");

        // Create a channel.
        let channel = conn.create_channel().await?;

        // Declare the same queue for receiving tasks.
        let task_queue = channel
            .queue_declare(
                "task_queue",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;
        info!(?task_queue, "Declared task queue");

        // Set up a consumer to receive and process tasks.
        let mut consumer = channel
            .basic_consume(
                "task_queue",
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        async_global_executor::spawn(async move {
            info!("Notification Service is ready to receive tasks.");
            while let Some(delivery_result) = consumer.next().await {
                match delivery_result {
                    Ok(delivery) => {
                        let task = String::from_utf8_lossy(&delivery.data);
                        info!("Received task: {}", task);

                        // Acknowledge the task to remove it from the queue.
                        channel
                            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                            .await
                            .expect("Failed to acknowledge the task");
                    }
                    Err(err) => {
                        info!("Error in consumer: {:?}", err);
                    }
                }
            }
        })
        .await;

        Ok(())
    })
}
