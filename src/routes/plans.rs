pub mod details;
pub mod icon;

use crate::jwt_verifier::JwtVerifier;
use crate::models::{
    put_keys, PlanCreate, PlanCreateError, PlanRead, PlanReadError, PlanTypeRead, PlanUpdate,
    PlanUpdateError,
};
use crate::service::discord::Discord;
use crate::{KV_PLANS, VAR_JWKS_URL};
use worker::{console_error, Cors, Error, Request, Response, RouteContext};

pub async fn get_plans(req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let url = req.url()?;
    let query_params = url.query_pairs();

    // クエリパラメータの解析
    let mut plan_types: Option<Vec<String>> = None;
    let mut recommended: Option<bool> = None;
    let mut child_friendly: Option<bool> = None;
    let mut lab_tour: Option<bool> = None;

    for (key, value) in query_params {
        match key.as_ref() {
            "type" => plan_types = Some(value.split(",").map(|s| s.into()).collect()),
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
            .with_cors(&Cors::new().with_origins(vec!["*"]))?
            .with_status(404));
        }
        Err(PlanReadError::KvError(e)) => {
            console_error!("kverror: {:?}", e);
            return Ok(Response::from_json(&serde_json::json!({
                "code": 500,
                "message": "Internal error occurred."
            }))?
            .with_cors(&Cors::new().with_origins(vec!["*"]))?
            .with_status(500));
        }
        Err(PlanReadError::WorkerError(e)) => {
            console_error!("workererror: {:?}", e);
            return Ok(Response::from_json(&serde_json::json!({
                "code": 500,
                "message": "Internal error occurred."
            }))?
            .with_cors(&Cors::new().with_origins(vec!["*"]))?
            .with_status(500));
        }
        Err(PlanReadError::GetKeysError(e)) => {
            console_error!("error occurred while retrieving keys: {:?}", e);
            return Ok(Response::from_json(&serde_json::json!({
                "code": 500,
                "message": "Internal error occurred."
            }))?
            .with_cors(&Cors::new().with_origins(vec!["*"]))?
            .with_status(500));
        }
    };

    // フィルター
    plans.retain(|plan| {
        let mut flag = (recommended == Some(plan.is_recommended) || recommended == None)
            && (child_friendly == Some(plan.is_child_friendly) || child_friendly == None);

        if let PlanTypeRead::Labo { is_lab_tour } = plan.r#type {
            flag = flag && (lab_tour == Some(is_lab_tour) || lab_tour == None);
        }

        if plan_types.is_none() {
            return flag;
        }
        let plan_types = plan_types.as_ref().unwrap();
        flag = flag
            && match plan.r#type {
                PlanTypeRead::Booth { .. } => plan_types.contains(&"booth".into()),
                PlanTypeRead::General { .. } => plan_types.contains(&"general".into()),
                PlanTypeRead::Stage {} => plan_types.contains(&"stage".into()),
                PlanTypeRead::Labo { .. } => plan_types.contains(&"labo".into()),
            };
        flag
    });

    Ok(Response::from_json(&serde_json::json!({
        "plans": plans
    }))?
    .with_cors(&Cors::new().with_origins(vec!["*"]))?)
}

pub async fn get_plan(req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    let kv = ctx.env.kv(KV_PLANS)?;

    match PlanRead::read(kv, plan_id).await {
        Ok(plan) => Response::from_json(&plan),
        Err(PlanReadError::NotFound) => Ok(Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "Plan not found."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(404)),
        Err(_) => Ok(Response::from_json(&serde_json::json!({
            "code": 500,
            "message": "Internal error occurred."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(500)),
    }
}

pub async fn put_plan(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
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
            match plan_create.clone().create(kv, plan_id).await {
                Ok(_) => {
                    // Discord通知
                    let discord = Discord::new_from_env(&ctx.env);
                    match discord.send_create_plan(plan_id.into(), &plan_create).await {
                        Ok(_) => {}
                        Err(err) => {
                            console_error!("Discord webhook error: {}", err)
                        }
                    }

                    // Update keys cache
                    let kv_cache = ctx.env.kv(KV_PLANS)?;
                    if let Err(err) = put_keys(&kv_cache).await {
                        console_error!("Failed to update keys cache: {:?}", err);
                    }

                    // 企画作成成功時は204 No Contentを返す
                    Ok(Response::empty()?.with_status(204))
                }
                Err(PlanCreateError::Conflict) => Ok(Response::from_json(&serde_json::json!({
                    "code": 409,
                    "message": "指定されたIDの企画が既に存在します"
                }))?
                .with_status(409)),
                Err(PlanCreateError::KvError(_)) => Ok(Response::from_json(&serde_json::json!({
                    "code": 500,
                    "message": "内部エラーが発生しました"
                }))?
                .with_status(500)),
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}

pub async fn patch_plan(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
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
            match plan_update.clone().update(kv, plan_id).await {
                Ok(_) => {
                    // discord通知
                    let discord = Discord::new_from_env(&ctx.env);
                    match discord.send_update_plan(plan_id.into(), &plan_update).await {
                        Ok(_) => {}
                        Err(err) => {
                            console_error!("Discord webhook error: {}", err)
                        }
                    }
                    // 企画更新成功時は204 No Contentを返す
                    Ok(Response::empty()?.with_status(204))
                }
                Err(PlanUpdateError::NotFound) => Ok(Response::from_json(&serde_json::json!({
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
        }
        Err(_) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": "リクエストが無効です"
        }))?
        .with_status(400)),
    }
}

pub async fn delete_plan(req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
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
                Ok(_) => {
                    // discord通知
                    let discord = Discord::new_from_env(&ctx.env);
                    match discord.send_delete_plan(plan_id.into()).await {
                        Ok(_) => {}
                        Err(err) => {
                            console_error!("Discord webhook error: {}", err)
                        }
                    }

                    // Update keys cache
                    let kv_cache = ctx.env.kv(KV_PLANS)?;
                    if let Err(err) = put_keys(&kv_cache).await {
                        console_error!("Failed to update keys cache: {:?}", err);
                    }

                    Ok(Response::empty()?.with_status(204))
                }
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
}

pub async fn post_plans_bulk(mut req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
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

    match req
        .json::<std::collections::HashMap<String, PlanCreate>>()
        .await
    {
        Ok(plans_map) => {
            let kv = ctx.env.kv(KV_PLANS)?;
            let mut errors = Vec::new();

            // すべてのエントリーに対して作成を試行
            for (id, plan_create) in plans_map {
                match plan_create.create(kv.clone(), &id).await {
                    Ok(_) => {
                        // 企画作成成功
                    }
                    Err(PlanCreateError::Conflict) => {
                        errors.push(serde_json::json!({
                            "plan_id": id,
                            "code": 409,
                            "message": format!("指定されたID「{}」の企画が既に存在します", id)
                        }));
                    }
                    Err(PlanCreateError::KvError(_)) => {
                        errors.push(serde_json::json!({
                            "plan_id": id,
                            "code": 500,
                            "message": format!("ID「{}」の企画作成中に内部エラーが発生しました", id)
                        }));
                    }
                }
            }

            if errors.is_empty() {
                // discord
                let discord = Discord::new_from_env(&ctx.env);
                match discord.send_bulk_create_plan().await {
                    Ok(_) => {}
                    Err(err) => {
                        console_error!("Discord webhook error: {}", err)
                    }
                }

                // Update keys cache
                let kv_cache = ctx.env.kv(KV_PLANS)?;
                if let Err(err) = put_keys(&kv_cache).await {
                    console_error!("Failed to update keys cache: {:?}", err);
                }

                // 全て成功した場合は201 Createdで空のレスポンスを返す
                Ok(Response::empty()?.with_status(201))
            } else {
                // 失敗したエントリーがある場合は207 Multi-Statusでエラー一覧を返す
                Ok(Response::from_json(&serde_json::json!({
                    "errors": errors
                }))?
                .with_status(207))
            }
        }
        Err(e) => Ok(Response::from_json(&serde_json::json!({
            "code": 400,
            "message": e.to_string()
        }))?
        .with_status(400)),
    }
}
