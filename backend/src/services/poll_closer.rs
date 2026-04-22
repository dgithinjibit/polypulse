use chrono::Utc;
use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tracing::{error, info};

/// Background task that runs every minute to close expired polls
pub async fn poll_auto_closer(db: PgPool) {
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;

        match close_expired_polls(&db).await {
            Ok(count) => {
                if count > 0 {
                    info!("Auto-closed {} expired polls", count);
                }
            }
            Err(e) => {
                error!("Error auto-closing polls: {}", e);
            }
        }
    }
}

async fn close_expired_polls(db: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE polls
        SET status = 'closed'
        WHERE status = 'open'
        AND closes_at <= $1
        "#,
    )
    .bind(Utc::now())
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
