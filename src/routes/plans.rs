pub mod details;
pub mod icon;

use crate::models::plan::{PlanRead, PlanReadError};
use crate::models::plan_type::PlanTypeRead;
use crate::KV_PLANS;
use std::str::FromStr;
use worker::{console_error, Cache, Cors, Error, Method, Request, Response, RouteContext};

pub async fn get_plans(req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    // cacheからの復元
    let cache_key = Request::new(req.url()?.as_str(), Method::Get)?;
    let cache = Cache::default();
    if let Some(response) = cache.get(&cache_key, false).await? {
        return Ok(response);
    }

    let url = req.url()?;
    let query_params = url.query_pairs();

    // クエリパラメータの解析
    let mut plan_types: Option<Vec<String>> = None;
    let mut recommended: Option<bool> = None;
    let mut child_friendly: Option<bool> = None;
    let mut lab_tour: Option<bool> = None;
    let mut combine_schedule: bool = true;

    for (key, value) in query_params {
        match key.as_ref() {
            "type" => plan_types = Some(value.split(",").map(|s| s.into()).collect()),
            "recommended" => recommended = value.parse().ok(),
            "child_friendly" => child_friendly = value.parse().ok(),
            "lab_tour" => lab_tour = value.parse().ok(),
            "combine_schedule" => combine_schedule = value.parse().ok().unwrap_or(true),
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

    // combine
    if combine_schedule {
        plans = plans
            .iter()
            .map(|plan| PlanRead {
                schedule: plan.schedule.combine(),
                ..plan.clone()
            })
            .collect();
    } else {
        plans = plans
            .iter()
            .map(|plan| PlanRead {
                schedule: plan.schedule.uncombine(),
                ..plan.clone()
            })
            .collect();
    }

    let mut response = Response::from_json(&serde_json::json!({
        "plans": plans
    }))?;

    response = response.with_cors(&Cors::new().with_origins(vec!["*"]))?;

    if 200 <= response.status_code() && response.status_code() < 300 {
        let headers = response.headers_mut();
        headers.set("Cache-Control", "public, max-age=3600, s-maxage=3600")?;
    }

    cache.put(&cache_key, response.cloned()?).await?;

    Ok(response)
}

pub async fn get_plan(req: Request, ctx: RouteContext<()>) -> Result<Response, Error> {
    // cacheからの復元
    let cache_key = Request::new(req.url()?.as_str(), Method::Get)?;
    let cache = Cache::default();
    if let Some(response) = cache.get(&cache_key, false).await? {
        return Ok(response);
    }

    let plan_id = ctx.param("plan_id").map_or("", |v| v);

    let url = req.url()?;
    let query_params = url.query_pairs();

    // クエリパラメータの解析
    let mut combine_schedule: bool = true;

    for (key, value) in query_params {
        match key.as_ref() {
            "combine_schedule" => combine_schedule = value.parse().ok().unwrap_or(true),
            _ => {}
        }
    }

    let kv = ctx.env.kv(KV_PLANS)?;

    let mut response = match PlanRead::read(kv, plan_id).await {
        Ok(mut plan) => {
            if combine_schedule {
                plan = PlanRead {
                    schedule: plan.schedule.combine(),
                    ..plan.clone()
                }
            } else {
                plan = PlanRead {
                    schedule: plan.schedule.uncombine(),
                    ..plan.clone()
                }
            }
            Response::from_json(&plan)?
        }
        Err(PlanReadError::NotFound) => Response::from_json(&serde_json::json!({
            "code": 404,
            "message": "Plan not found."
        }))?
        .with_status(404),
        Err(_) => Response::from_json(&serde_json::json!({
            "code": 500,
            "message": "Internal error occurred."
        }))?
        .with_cors(&Cors::new().with_origins(vec!["*"]))?
        .with_status(500),
    };

    response = response.with_cors(&Cors::new().with_origins(vec!["*"]))?;

    if 200 <= response.status_code() && response.status_code() < 300 {
        let headers = response.headers_mut();
        headers.set("Cache-Control", "public, max-age=3600, s-maxage=3600")?;
    }

    cache.put(&cache_key, response.cloned()?).await?;

    Ok(response)
}
