use worker::{Bucket, Data, HttpMetadata};

pub enum PutIconError {
    WorkerError(worker::Error),
    TransformError(String),
}

impl From<worker::Error> for PutIconError {
    fn from(err: worker::Error) -> Self {
        PutIconError::WorkerError(err)
    }
}

/// アイコンをr2 bucketに保存する
///
/// # params
/// * `bucket` - r2 bucket
/// * `
pub async fn put_icon(
    bucket: Bucket,
    plan_id: &str,
    bytes: impl Into<Data> + Clone,
    content_type: String,
) -> Result<(), PutIconError> {
    // オリジナルを保存
    let key_original = format!("{}/original", plan_id);
    bucket
        .put(&key_original, bytes.clone())
        .http_metadata(HttpMetadata {
            content_type: Some(content_type),
            ..Default::default()
        })
        .execute()
        .await?;

    Ok(())
}
