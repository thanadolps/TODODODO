use lapin::{options::*, BasicProperties, Channel};

use serde::Serialize;

use time::OffsetDateTime;
use tracing::{error, info, trace};

use sqlx::{postgres::types::PgInterval, PgPool};
use std::time::Duration;
use tokio::time::MissedTickBehavior;
use uuid::Uuid;



#[derive(Serialize, Debug)]
struct NotificationDetail {
    user_id: Uuid,
    task_id: String,
    title: String,
    description: String,
    deadline: OffsetDateTime,
}

// TODO: come up with better name and design for this
/// Perodically check for deadline from database to send them notifier service.
#[tracing::instrument(skip(pool, channel))]
pub async fn watch_notification_task(
    pool: PgPool,
    check_period: Duration,
    lead_time: Duration,
    channel: Channel,
) {
    let mut interval = tokio::time::interval(check_period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    // type conversion to postgres interval

    let Ok(check_period) = PgInterval::try_from(check_period) else {
        error!(
            ?check_period,
            "failed to convert check_period to PgInterval"
        );
        error!("Watch notification task failed to start");
        panic!("Watch notification task failed to start");
    };
    let Ok(lead_time) = PgInterval::try_from(lead_time) else {
        error!(?lead_time, "failed to convert lead_time to PgInterval");
        error!("Watch notification task failed to start");
        panic!("Watch notification task failed to start");
    };

    loop {
        interval.tick().await;
        trace!("checking for deadline");

        let result = sqlx::query_as!(
            NotificationDetail,
            r#"
        SELECT id as task_id, user_id, title, description, deadline as "deadline!"
        FROM task
        WHERE deadline IS NOT NULL
        AND deadline BETWEEN NOW() + $1 AND NOW() + $1 + $2
        "#,
            Some(&lead_time),
            Some(&check_period)
        )
        .fetch_all(&pool)
        .await;

        //test by removing AND

        let tasks = match result {
            Ok(tasks) => tasks,
            Err(err) => {
                error!(error = ?err, "failed to fetch tasks");
                continue;
            }
        };

        for task in tasks {
            send_task(&channel, task).await;
        }
    }
}

#[tracing::instrument(skip(channel))]
async fn send_task(channel: &Channel, content: NotificationDetail) {
    // Serialize the Request to JSON.
    let Ok(payload) = serde_json::to_vec(&content)
        .map_err(|err| error!(error = ?err, "Failed to serialize Task to JSON"))
    else {
        return;
    };

    //set payload into rabbit mq
    if let Err(err) = channel
        .basic_publish(
            "",
            "task_queue",
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await
    {
        error!(error = ?err, "Failed to send task");
    } else {
        info!("Sent content: {:?}", content);
    }
}
