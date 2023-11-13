use tracing::info;

use sqlx::{PgPool};
use std::time::Duration;
use tokio::time::MissedTickBehavior;
// TODO: come up with better name and design for this
/// Perodically check for deadline from database to send them notifier service.
pub async fn refresh_routine_task(pool: PgPool, check_period: Duration) {
    let mut interval = tokio::time::interval(check_period);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    // type conversion to postgres interval

    info!("checking routine");

    loop {
        interval.tick().await;
        info!("checking routine!");

        let _result = sqlx::query!(
            "UPDATE routine SET completed = false, checktime = NOW() WHERE 
            typena = 'daily' AND completed = true AND 
            DATE_TRUNC('day', checktime) <= now() - interval '1 day'"
        )
        .fetch_all(&pool)
        .await;

        let _result = sqlx::query!(
            "UPDATE routine SET completed = false, checktime = NOW() WHERE 
            typena = 'weekly' AND completed = true AND 
            DATE_TRUNC('week', checktime) <= now() - interval '1 week'"
        )
        .fetch_all(&pool)
        .await;

        let _result = sqlx::query!(
            "UPDATE routine SET completed = false, checktime = NOW() WHERE 
            typena = 'monthly' AND completed = true AND 
            DATE_TRUNC('month', checktime) <= now() - interval '1 month'"
        )
        .fetch_all(&pool)
        .await;
    }
}
