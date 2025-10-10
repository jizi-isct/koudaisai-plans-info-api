use crate::icon::{write_icon, WriteIconError};
use crate::jwt_verifier::JwtVerifier;
use crate::service::discord::Discord;
use crate::{R2_PLAN_IMAGES, VAR_JWKS_URL};
use wasm_bindgen::JsValue;
use worker::{console_error, Cors, Headers, Request, Response};

pub async fn put_icon(
    mut req: Request,
    ctx: worker::RouteContext<()>,
) -> Result<Response, worker::Error> {
    let plan_id = ctx.param("plan_id").unwrap();
    let bucket = ctx.env.bucket(R2_PLAN_IMAGES)?;

    // ヘッダー検証
    let ct = req.headers().get("content-type")?.unwrap_or_default();
    if !ct.starts_with("image/") {
        return Response::error("content-type must be image/*", 415);
    }
    let bytes = req.bytes().await?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Response::error("payload too large", 413);
    }

    // 保存
    let discord = Discord::new_from_env(&ctx.env);
    match write_icon(bucket, plan_id, bytes, ct, discord).await {
        Ok(_) => Ok(Response::empty()?.with_status(204)),
        Err(WriteIconError::WorkerError(e)) => Ok(Response::from_json(&serde_json::json!({
            "code": 500,
            "message": format!("Internal error occurred: {}", e.to_string())
        }))?
        .with_status(500)),
    }
}

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

pub async fn post_icon_import(
    mut req: Request,
    ctx: worker::RouteContext<()>,
) -> Result<Response, worker::Error> {
    let plan_id = ctx.param("plan_id").unwrap();
    let bucket = ctx.env.bucket(R2_PLAN_IMAGES)?;

    // JWT認証チェック
    let jwt_verifier = JwtVerifier::new(&*ctx.env.var(VAR_JWKS_URL)?.to_string())
        .await
        .unwrap();
    if jwt_verifier
        .verify_token_in_headers(&req.headers())
        .is_err()
    {
        return Ok(Response::from_bytes("Unauthorized".into())?.with_status(401));
    }

    // リクエストボディからURLを取得
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(_) => {
            return Ok(Response::from_json(&serde_json::json!({
                "code": 400,
                "message": "Invalid JSON body"
            }))?
            .with_status(400));
        }
    };

    let url = match body.get("url").and_then(|u| u.as_str()) {
        Some(url) => url,
        None => {
            return Ok(Response::from_json(&serde_json::json!({
                "code": 400,
                "message": "Missing 'url' field in request body"
            }))?
            .with_status(400));
        }
    };

    // URLからファイルをダウンロード
    let download_req = match Request::new(url, worker::Method::Get) {
        Ok(req) => req,
        Err(_) => {
            return Ok(Response::from_json(&serde_json::json!({
                "code": 400,
                "message": "Invalid URL"
            }))?
            .with_status(400));
        }
    };

    let mut download_resp = match worker::Fetch::Request(download_req).send().await {
        Ok(resp) => resp,
        Err(_) => {
            return Ok(Response::from_json(&serde_json::json!({
                "code": 502,
                "message": "Failed to download image from URL"
            }))?
            .with_status(502));
        }
    };

    if download_resp.status_code() < 200 || download_resp.status_code() >= 300 {
        console_error!("Failed to download image from URL: {:?}", download_resp);
        return Ok(Response::from_json(&serde_json::json!({
            "code": 502,
            "message": "Failed to download image: HTTP error"
        }))?
        .with_status(502));
    }

    // Content-Typeをチェック
    let ct = download_resp
        .headers()
        .get("content-type")?
        .unwrap_or_default();
    if !ct.starts_with("image/") {
        return Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": "Downloaded content is not an image"
        }))?
        .with_status(400));
    }

    let bytes = match download_resp.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(Response::from_json(&serde_json::json!({
                "code": 502,
                "message": "Failed to read image data"
            }))?
            .with_status(502));
        }
    };

    // アイコンを保存
    let discord = Discord::new_from_env(&ctx.env);
    match write_icon(bucket, plan_id, bytes, ct, discord).await {
        Ok(_) => Ok(Response::empty()?.with_status(204)),
        Err(WriteIconError::WorkerError(e)) => Ok(Response::from_json(&serde_json::json!({
            "code": 500,
            "message": format!("Internal error occurred: {}", e.to_string())
        }))?
        .with_status(500)),
    }
}
