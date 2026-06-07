use sqlx::PgPool;
use crate::error::AppError;

const ADVISORY_LOCK_KEY: i64 = 0x4841_5250; // "HARP" in hex

/// 运行所有数据库迁移，advisory lock 防并发 race
pub async fn migrate(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(ADVISORY_LOCK_KEY)
        .execute(pool)
        .await?;

    let result = sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Internal(e.into()));

    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(ADVISORY_LOCK_KEY)
        .execute(pool)
        .await?;

    result
}
