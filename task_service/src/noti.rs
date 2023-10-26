use lapin::{options::*, BasicProperties, Channel};

use serde_json::json;
use tracing::info;

use sqlx::{postgres::types::PgInterval, PgPool};
use std::time::Duration;
use tokio::time::MissedTickBehavior;
// TODO: come up with better name and design for this
/// Perodically check for deadline from database to send them notifier service.
pub async fn watch_notification_task(
    pool: PgPool,
    check_period: Duration,
    lead_time: Duration,
    channel: Channel,
) {
    tracing::info!("watching for deadline");

    let mut interval = tokio::time::interval(check_period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    // type conversion to postgres interval
    let check_period = PgInterval::try_from(check_period).unwrap();
    let lead_time = PgInterval::try_from(lead_time).unwrap();

    loop {
        interval.tick().await;
        tracing::trace!("checking for deadline");

        let result = sqlx::query!(
            r#"
        SELECT id, title, description, deadline as "deadline!"
        FROM task
        WHERE deadline IS NOT NULL
        "#
        )
        .fetch_all(&pool)
        .await;

        //test by removing AND

        match result {
            Err(err) => {
                tracing::error!("failed to fetch tasks: {}", err);
            }
            Ok(tasks) => {
                for task in tasks {
                    // Send to notifier service
                    // tracing::debug!("sending notification for task: {:?}", task);
                    // let result = notifier
                    //     .send_notification(Request::new(NotificationDetail {
                    //         task_id: task.id.to_string(),
                    //         title: task.title,
                    //         description: task.description,
                    //         deadline: task.deadline.map(|d| SystemTime::from(d).into()),
                    //     }))
                    //     .await;
                    // Serialize the Request to JSON.

                    let task = json!({
                        "task_id": task.id.to_string(),
                        "title": task.title,
                        "description": task.description,
                        "deadline": task.deadline,
                    });
                    let payload: Vec<u8> =
                        serde_json::to_vec(&task).expect("Failed to serialize Task to JSON");

                    channel
                        .basic_publish(
                            "",
                            "task_queue",
                            BasicPublishOptions::default(),
                            &payload,
                            BasicProperties::default(),
                        )
                        .await
                        .unwrap();

                    info!("Sent task: {:?}", task);

                    //set payload into rabbit mq
                }
            }
        }
    }
}
