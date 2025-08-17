mod jwks;
mod jwt_verifier;
mod models;
mod util;

use crate::jwt_verifier::JwtVerifier;
use crate::models::{
    PlanCreate, PlanCreateError, PlanRead, PlanReadError, PlanTypeRead, PlanUpdate, PlanUpdateError,
};
use worker::*;

const VAR_JWKS_URL: &str = "JWKS_URL";
const KV_PLANS: &str = "PLANS";

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
        .run(req, env)
        .await
}
