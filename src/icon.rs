use crate::service::discord::Discord;
use thiserror::Error;
use worker::{console_error, Bucket, Data, HttpMetadata};

#[derive(Debug, Error)]
pub enum PutIconError {
    #[error(transparent)]
    WorkerError(#[from] worker::Error),
}

/// アイコンをr2 bucketに保存する
///
/// # params
/// * `bucket` - r2 bucket
/// * `
pub async fn put_icon(
    bucket: Bucket,
    plan_id: &str,
    bytes: impl Into<Data> + Into<Vec<u8>> + Clone,
    content_type: String,
    discord: Discord,
) -> Result<(), PutIconError> {
    // オリジナルを保存
    let key_original = format!("{}/original", plan_id);
    bucket
        .put(&key_original, bytes.clone())
        .http_metadata(HttpMetadata {
            content_type: Some(content_type.clone()),
            ..Default::default()
        })
        .execute()
        .await?;

    // discordに通知
    match discord
        .send_update_plan_icon(plan_id.into(), content_type, bytes)
        .await
    {
        Ok(_) => {}
        Err(err) => {
            console_error!("webhook error: {}", err);
        }
    }

    Ok(())
}
