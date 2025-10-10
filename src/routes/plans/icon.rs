use crate::R2_PLAN_IMAGES;
use wasm_bindgen::JsValue;
use worker::{Cors, Headers, Request, Response};

pub async fn get_icon(
    request: Request,
    ctx: worker::RouteContext<()>,
) -> Result<Response, worker::Error> {
    let plan_id = ctx.param("plan_id").unwrap();
    let bucket = ctx.env.bucket(R2_PLAN_IMAGES)?;

    let object = bucket
        .get(format!("{}/original", plan_id))
        .execute()
        .await?;

    if object.is_none() {
        return Ok(Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "Icon not found."
        }))?
        .with_status(404));
    }

    // レスポンス
    let object = object.unwrap();
    let headers = Headers::new();
    object.write_http_metadata(headers.clone())?;
    headers.set("etag", &*object.http_etag())?;
    let Some(body) = object.body() else {
        return Err(worker::Error::Internal(JsValue::from_str("body is none")));
    };
    Ok(Response::from_bytes(body.bytes().await?)?
        .with_headers(headers)
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(200))
}
