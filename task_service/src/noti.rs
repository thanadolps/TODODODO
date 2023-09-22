use poem_grpc::Request;
use sqlx::{postgres::types::PgInterval, PgPool};
use std::time::{Duration, SystemTime};
use tokio::time::MissedTickBehavior;

poem_grpc::include_proto!("notifier");

// TODO: come up with better name and design for this
/// Perodically check for deadline from database to send them notifier service.
pub async fn watch_notification_task(
    pool: PgPool,
    notifier: NotifierClient,
    check_period: Duration,
    lead_time: Duration,
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
        SELECT id, title, description, deadline
        FROM task
        WHERE deadline IS NOT NULL
        AND deadline BETWEEN NOW() + $1 AND NOW() + $1 + $2
        "#,
            Some(&lead_time),
            Some(&check_period)
        )
        .fetch_all(&pool)
        .await;

        match result {
            Err(err) => {
                tracing::error!("failed to fetch tasks: {}", err);
            }
            Ok(tasks) => {
                for task in tasks {
                    // Send to notifier service
                    tracing::debug!("sending notification for task: {:?}", task);
                    let result = notifier
                        .send_notification(Request::new(Notification {
                            task_id: task.id.to_string(),
                            title: task.title,
                            description: task.description,
                            deadline: task.deadline.map(|d| SystemTime::from(d).into()),
                        }))
                        .await;
                    if let Err(err) = result {
                        tracing::error!("failed to send notification: {:?}", err);
                    }
                }
            }
        }
    }
}
