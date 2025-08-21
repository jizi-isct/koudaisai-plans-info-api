mod icon;
mod jwks;
mod jwt_verifier;
mod models;
mod util;

use crate::icon::{put_icon, PutIconError};
use crate::jwt_verifier::JwtVerifier;
use crate::models::{
    PlanCreate, PlanCreateError, PlanRead, PlanReadError, PlanTypeRead, PlanUpdate, PlanUpdateError,
};
use wasm_bindgen::JsValue;
use worker::*;

const VAR_JWKS_URL: &str = "JWKS_URL";
const KV_PLANS: &str = "PLANS";
const R2_PLAN_IMAGES: &str = "plan_icons";
const IMG_SIZES: &[u32] = &[128, 256, 512];

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    // Router設定
    let router = Router::new();

    router
        // GET /plans - 全ての企画情報を取得
        .get_async("/v1/plans", |mut req, ctx| async move {
            let url = req.url()?;
            let query_params = url.query_pairs();

            // クエリパラメータの解析
            let mut plan_type: Option<String> = None;
            let mut recommended: Option<bool> = None;
            let mut child_friendly: Option<bool> = None;
            let mut lab_tour: Option<bool> = None;

            for (key, value) in query_params {
                match key.as_ref() {
                    "type" => plan_type = Some(value.into_owned()),
                    "recommended" => recommended = value.parse().ok(),
                    "child_friendly" => child_friendly = value.parse().ok(),
                    "lab_tour" => lab_tour = value.parse().ok(),
                    _ => {}
                }
            }

            // 企画を全て取得
            let kv = ctx.env.kv(KV_PLANS)?;
            let mut plans: Vec<PlanRead> = match PlanRead::read_all(&kv).await {
                Ok(plans) => plans,
                Err(PlanReadError::NotFound) => {
                    return Ok(Response::from_json(&serde_json::json!({
                        "code": 404,
                        "message": "Plan not found."
                    }))?
                    .with_status(404));
                }
                Err(PlanReadError::KvError(e)) => {
                    console_error!("kverror: {:?}", e);
                    return Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "Internal error occurred."
                    }))?
                    .with_status(500));
                }
                Err(PlanReadError::WorkerError(e)) => {
                    console_error!("workererror: {:?}", e);
                    return Ok(Response::from_json(&serde_json::json!({
                        "code": 500,
                        "message": "Internal error occurred."
                    }))?
                    .with_status(500));
                }
            };

            // フィルター
            plans.retain(|plan| match plan.r#type {
                PlanTypeRead::Booth {} => {
                    (plan_type == Some("booth".into()) || plan_type == None)
                        && (recommended == Some(plan.is_recommended) || recommended == None)
                        && (child_friendly == Some(plan.is_child_friendly)
                            || child_friendly == None)
                }
                PlanTypeRead::General {} => {
                    (plan_type == Some("general".into()) || plan_type == None)
                        && (recommended == Some(plan.is_recommended) || recommended == None)
                        && (child_friendly == Some(plan.is_child_friendly)
                            || child_friendly == None)
                }
                PlanTypeRead::Stage {} => {
                    (plan_type == Some("stage".into()) || plan_type == None)
                        && (recommended == Some(plan.is_recommended) || recommended == None)
                        && (child_friendly == Some(plan.is_child_friendly)
                            || child_friendly == None)
                }
                PlanTypeRead::Labo { is_lab_tour } => {
                    (plan_type == Some("stage".into()) || plan_type == None)
                        && (recommended == Some(plan.is_recommended) || recommended == None)
                        && (child_friendly == Some(plan.is_child_friendly)
                            || child_friendly == None)
                        && (lab_tour == Some(is_lab_tour) || lab_tour == None)
                }
            });

            Response::from_json(&serde_json::json!({
                "plans": plans
            }))
        })
        // GET /plans/{planId} - 特定の企画情報を取得
        .get_async("/v1/plans/:plan_id", |req, ctx| async move {
            let plan_id = ctx.param("plan_id").map_or("", |v| v);

            let kv = ctx.env.kv(KV_PLANS)?;

            match PlanRead::read(kv, plan_id).await {
                Ok(plan) => Response::from_json(&plan),
                Err(PlanReadError::NotFound) => Ok(Response::from_json(&serde_json::json!({
                    "code": 404,
                    "message": "Plan not found."
                }))?
                .with_status(404)),
                Err(_) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": "Internal error occurred."
                }))?
                .with_status(500)),
            }
        })
        // PUT /plans/{planId} - 新しい企画を作成
        .put_async("/v1/plans/:plan_id", |mut req, ctx| async move {
            console_debug!("PUT /plans/:plan_id");
            let plan_id = ctx.param("plan_id").map_or("", |v| v);

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

            match req.json::<PlanCreate>().await {
                Ok(plan_create) => {
                    let kv = ctx.env.kv(KV_PLANS)?;
                    match plan_create.create(kv, plan_id).await {
                        Ok(_) => {
                            // 企画作成成功時は204 No Contentを返す
                            Ok(Response::empty()?.with_status(204))
                        }
                        Err(PlanCreateError::Conflict) => {
                            Ok(Response::from_json(&serde_json::json!({
                                "code": 409,
                                "message": "指定されたIDの企画が既に存在します"
                            }))?
                            .with_status(409))
                        }
                        Err(PlanCreateError::KvError(_)) => {
                            Ok(Response::from_json(&serde_json::json!({
                                "code": 500,
                                "message": "内部エラーが発生しました"
                            }))?
                            .with_status(500))
                        }
                    }
                }
                Err(e) => Ok(Response::from_json(&serde_json::json!({
                    "code": 400,
                    "message": e.to_string()
                }))?
                .with_status(400)),
            }
        })
        // PATCH /plans/{planId} - 企画情報を更新
        .patch_async("/v1/plans/:plan_id", |mut req, ctx| async move {
            let plan_id = ctx.param("plan_id").map_or("", |v| v);

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

            let kv = ctx.env.kv(KV_PLANS)?;

            match req.json::<PlanUpdate>().await {
                Ok(plan_update) => {
                    match plan_update.update(kv, plan_id).await {
                        Ok(_) => {
                            // 企画更新成功時は204 No Contentを返す
                            Ok(Response::empty()?.with_status(204))
                        }
                        Err(PlanUpdateError::NotFound) => {
                            Ok(Response::from_json(&serde_json::json!({
                                "code": 404,
                                "message": "企画が見つかりません"
                            }))?
                            .with_status(404))
                        }
                        Err(_) => Ok(Response::from_json(&serde_json::json!({
                            "code": 500,
                            "message": "内部エラーが発生しました"
                        }))?
                        .with_status(500)),
                    }
                }
                Err(_) => Ok(Response::from_json(&serde_json::json!({
                    "code": 400,
                    "message": "リクエストが無効です"
                }))?
                .with_status(400)),
            }
        })
        // DELETE /plans/{planId} - 企画を削除
        .delete_async("/v1/plans/:plan_id", |req, ctx| async move {
            let plan_id = ctx.param("plan_id").map_or("", |v| v);

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

            let kv = ctx.env.kv(KV_PLANS)?;

            // 企画が存在するか確認
            match PlanRead::read(kv.clone(), plan_id).await {
                Ok(_) => {
                    // 削除実行
                    match kv.delete(plan_id).await {
                        Ok(_) => Ok(Response::empty()?.with_status(204)),
                        Err(_) => Ok(Response::from_json(&serde_json::json!({
                            "code": 500,
                            "message": "内部エラーが発生しました"
                        }))?
                        .with_status(500)),
                    }
                }
                Err(PlanReadError::NotFound) => Ok(Response::from_json(&serde_json::json!({
                    "code": 404,
                    "message": "企画が見つかりません"
                }))?
                .with_status(404)),
                Err(_) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": "内部エラーが発生しました"
                }))?
                .with_status(500)),
            }
        })
        .put_async("/v1/plans/:plan_id/icon", |mut req, ctx| async move {
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
            match put_icon(bucket, plan_id, bytes, ct).await {
                Ok(_) => Ok(Response::empty()?.with_status(204)),
                Err(PutIconError::WorkerError(e)) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": format!("Internal error occurred: {}", e.to_string())
                }))?
                .with_status(500)),
                Err(PutIconError::TransformError(e)) => {
                    Ok(Response::from_json(&serde_json::json!({
                        "code": 502,
                        "message": e
                    }))?
                    .with_status(502))
                }
            }
        })
        .get_async("/v1/plans/:plan_id/icon", |req, ctx| async move {
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
                .with_status(200))
        })
        .post_async(
            "/v1/plans/:plan_id/icon:import",
            |mut req, ctx| async move {
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
                match put_icon(bucket, plan_id, bytes, ct).await {
                    Ok(_) => Ok(Response::empty()?.with_status(204)),
                    Err(PutIconError::WorkerError(e)) => {
                        Ok(Response::from_json(&serde_json::json!({
                            "code": 500,
                            "message": format!("Internal error occurred: {}", e.to_string())
                        }))?
                        .with_status(500))
                    }
                    Err(PutIconError::TransformError(e)) => {
                        Ok(Response::from_json(&serde_json::json!({
                            "code": 502,
                            "message": e
                        }))?
                        .with_status(502))
                    }
                }
            },
        )
        .run(req, env)
        .await
}
