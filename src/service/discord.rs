use crate::models::base::Location;
use crate::models::plan::{PlanCreate, PlanUpdate};
use crate::models::plan_type::{PlanTypeCreate, PlanTypeUpdate};
use crate::models::schedule::{DaySchedule, Time};
use crate::util::extension_from_content_type;
use anyhow::Result;
use serde_json::{json, Value};
use thiserror::Error;
use worker::{Env, Fetch, Method, Request, RequestInit};

pub struct Discord {
    webhook_url: String,
    base_url: String,
}

#[derive(Error, Debug)]
pub enum DiscordError {
    #[error("HTTP request failed: {0}")]
    HttpError(String),
    #[error("JSON serialization failed: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Worker error: {0}")]
    WorkerError(#[from] worker::Error),
}

impl Discord {
    pub fn new<T: Into<String>>(webhook_url: T) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            base_url: "https://api2025.jizi.jp".into(),
        }
    }

    fn time_to_string(t: &Time) -> String {
        // Time implements Serialize to HH:mm; use serde to avoid accessing internals
        serde_json::to_string(t)
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_default()
    }

    fn combine_range(day: &Vec<DaySchedule>) -> Option<(Time, Time)> {
        if day.is_empty() {
            return None;
        }
        let mut start = day[0].start_time;
        let mut end = day[0].end_time;
        for s in day.iter().skip(1) {
            if s.start_time < start {
                start = s.start_time;
            }
            if s.end_time > end {
                end = s.end_time;
            }
        }
        Some((start, end))
    }

    fn format_range(day: &Vec<DaySchedule>) -> String {
        match Self::combine_range(day) {
            Some((start, end)) => format!(
                "{} - {}",
                Self::time_to_string(&start),
                Self::time_to_string(&end)
            ),
            None => "なし".to_string(),
        }
    }

    pub fn new_from_env(env: &Env) -> Self {
        let webhook_url = env
            .secret("DISCORD_WEBHOOK_URL")
            .expect("DISCORD_WEBHOOK_URL is not set")
            .to_string();
        Self::new(webhook_url)
    }

    fn create_embed_field(name: &str, value: String, inline: bool) -> Value {
        json!({
            "name": name,
            "value": value,
            "inline": inline
        })
    }

    async fn send_webhook(&self, payload: Value) -> Result<(), DiscordError> {
        loop {
            let mut init = RequestInit::new();
            init.with_method(Method::Post);
            init.with_body(Some(payload.to_string().into()));

            let headers = worker::Headers::new();
            headers.set("Content-Type", "application/json")?;
            init.with_headers(headers);

            let request = Request::new_with_init(&self.webhook_url, &init)?;
            let mut response = Fetch::Request(request).send().await?;

            let status = response.status_code();

            if (200..300).contains(&status) {
                return Ok(());
            }

            // Handle rate limiting (429 status)
            if status == 429 {
                // Try to get Retry-After header
                if let Ok(retry_after_str) = response.headers().get("Retry-After") {
                    if let Some(retry_after_str) = retry_after_str {
                        if let Ok(retry_seconds) = retry_after_str.parse::<u64>() {
                            // Sleep for the specified number of seconds
                            worker::console_log!(
                                "Rate limited. Retrying after {} seconds",
                                retry_seconds
                            );
                            self.sleep_ms(retry_seconds * 1000).await;
                            continue; // Retry the request
                        }
                    }
                }
                // Fallback: sleep for 1 second if no valid Retry-After header
                worker::console_log!("Rate limited. Retrying after 1 second (fallback)");
                self.sleep_ms(1000).await;
                continue;
            }

            // For other errors, return immediately
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DiscordError::HttpError(format!(
                "Discord webhook failed with status {}: {}",
                status, error_text
            )));
        }
    }

    async fn sleep_ms(&self, ms: u64) {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::JsFuture;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_name = setTimeout)]
            fn set_timeout(closure: &Closure<dyn FnMut()>, timeout: u32) -> f64;
        }

        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let resolve_clone = resolve.clone();
            let closure = Closure::wrap(Box::new(move || {
                resolve_clone.call0(&wasm_bindgen::JsValue::NULL).unwrap();
            }) as Box<dyn FnMut()>);

            set_timeout(&closure, ms as u32);
            closure.forget();
        });

        let _ = JsFuture::from(promise).await;
    }

    pub async fn send_create_plan(
        &self,
        id: String,
        plan_create: &PlanCreate,
    ) -> Result<(), DiscordError> {
        let mut fields = Vec::new();

        // type
        match plan_create.r#type {
            PlanTypeCreate::Booth { .. } => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "模擬店企画".to_string(),
                    false,
                ));
            }
            PlanTypeCreate::General { .. } => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "一般企画".to_string(),
                    false,
                ));
            }
            PlanTypeCreate::Stage { .. } => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "ステージ企画".to_string(),
                    false,
                ));
            }
            PlanTypeCreate::Labo { is_lab_tour } => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "研究室企画".to_string(),
                    false,
                ));
                fields.push(Self::create_embed_field(
                    "研究室ツアー企画課どうか",
                    is_lab_tour.to_string(),
                    false,
                ));
            }
        }

        fields.push(Self::create_embed_field(
            "団体名",
            plan_create.organization_name.clone(),
            true,
        ));
        fields.push(Self::create_embed_field(
            "企画名",
            plan_create.plan_name.clone(),
            true,
        ));
        fields.push(Self::create_embed_field(
            "企画内容紹介文",
            plan_create.description.clone(),
            false,
        ));
        fields.push(Self::create_embed_field(
            "子供向け企画かどうか",
            plan_create.is_child_friendly.to_string(),
            true,
        ));
        fields.push(Self::create_embed_field(
            "おすすめ企画かどうか",
            plan_create.is_recommended.to_string(),
            true,
        ));

        // schedule
        let day1_str = if plan_create.schedule.day1.is_empty() {
            "なし".to_string()
        } else {
            Self::format_range(&plan_create.schedule.day1)
        };
        fields.push(Self::create_embed_field(
            "1日目企画実施時間",
            day1_str,
            true,
        ));

        let day2_str = if plan_create.schedule.day2.is_empty() {
            "なし".to_string()
        } else {
            Self::format_range(&plan_create.schedule.day2)
        };
        fields.push(Self::create_embed_field(
            "2日目企画実施時間",
            day2_str,
            true,
        ));

        // location
        let mut locations = String::new();
        for location in plan_create.location.iter() {
            match location {
                Location::IndoorLocation { building, room } => {
                    locations.push_str(&format!("- {} {}\n", building, room));
                }
                Location::OutdoorLocation { name } => {
                    locations.push_str(&format!("- {}\n", name));
                }
            }
        }
        if !locations.is_empty() {
            fields.push(Self::create_embed_field("実施場所", locations, false));
        }

        let embed = json!({
            "title": "企画情報が新たに作成されました",
            "fields": fields
        });

        let payload = json!({
            "username": id,
            "avatar_url": format!("{}/v1/plans/{}/icon", self.base_url, id),
            "embeds": [embed]
        });

        self.send_webhook(payload).await
    }

    pub async fn send_bulk_create_plan(&self) -> Result<(), DiscordError> {
        let embed = json!({
            "title": "複数の企画情報が新たに作成されました"
        });

        let payload = json!({
            "username": "Bulk Create",
            "embeds": [embed]
        });

        self.send_webhook(payload).await
    }

    pub async fn get_update_plan_embed(
        &self,
        id: String,
        plan_update: &PlanUpdate,
    ) -> Result<Value, DiscordError> {
        let mut fields = Vec::new();

        fields.push(Self::create_embed_field("企画ID", id, false));

        // type
        match &plan_update.r#type {
            Some(PlanTypeUpdate::Booth { .. }) => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "模擬店企画".to_string(),
                    false,
                ));
            }
            Some(PlanTypeUpdate::General { .. }) => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "一般企画".to_string(),
                    false,
                ));
            }
            Some(PlanTypeUpdate::Stage { .. }) => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "ステージ企画".to_string(),
                    false,
                ));
            }
            Some(PlanTypeUpdate::Labo { is_lab_tour }) => {
                fields.push(Self::create_embed_field(
                    "種類",
                    "研究室企画".to_string(),
                    false,
                ));
                if let Some(is_lab_tour) = is_lab_tour {
                    fields.push(Self::create_embed_field(
                        "研究室ツアー企画課どうか",
                        is_lab_tour.to_string(),
                        false,
                    ));
                }
            }
            None => {}
        }

        if let Some(organization_name) = &plan_update.organization_name {
            fields.push(Self::create_embed_field(
                "団体名",
                organization_name.clone(),
                true,
            ));
        }

        if let Some(plan_name) = &plan_update.plan_name {
            fields.push(Self::create_embed_field("企画名", plan_name.clone(), true));
        }

        if let Some(description) = &plan_update.description {
            fields.push(Self::create_embed_field(
                "企画内容紹介文",
                description.clone(),
                false,
            ));
        }

        if let Some(is_child_friendly) = plan_update.is_child_friendly {
            fields.push(Self::create_embed_field(
                "子供向け企画かどうか",
                is_child_friendly.to_string(),
                true,
            ));
        }

        if let Some(is_recommended) = plan_update.is_recommended {
            fields.push(Self::create_embed_field(
                "おすすめ企画かどうか",
                is_recommended.to_string(),
                true,
            ));
        }

        // schedule
        if let Some(schedule) = &plan_update.schedule {
            if let Some(day1) = &schedule.day1 {
                if let Some(day1) = day1 {
                    fields.push(Self::create_embed_field(
                        "1日目企画実施時間",
                        Self::format_range(day1),
                        true,
                    ));
                } else {
                    fields.push(Self::create_embed_field(
                        "1日目企画実施時間",
                        "なし".to_string(),
                        true,
                    ));
                }
            }
            if let Some(day2) = &schedule.day2 {
                if let Some(day2) = day2 {
                    fields.push(Self::create_embed_field(
                        "2日目企画実施時間",
                        Self::format_range(day2),
                        true,
                    ));
                } else {
                    fields.push(Self::create_embed_field(
                        "2日目企画実施時間",
                        "なし".to_string(),
                        true,
                    ));
                }
            }
        }

        // location
        if let Some(location) = &plan_update.location {
            let mut locations = String::new();
            for location in location {
                match location {
                    Location::IndoorLocation { building, room } => {
                        locations.push_str(&format!("- {} {}\n", building, room));
                    }
                    Location::OutdoorLocation { name } => {
                        locations.push_str(&format!("- {}\n", name));
                    }
                }
            }
            if !locations.is_empty() {
                fields.push(Self::create_embed_field("実施場所", locations, false));
            }
        }

        let embed = json!({
            "title": "企画情報が編集されました",
            "fields": fields
        });

        Ok(embed)
    }

    pub async fn send_bulk_update_plan(
        &self,
        plans: Vec<(String, PlanUpdate)>,
    ) -> Result<(), DiscordError> {
        for chunk in plans.chunks(10) {
            let mut embeds = vec![];
            for (id, plan_update) in chunk {
                let embed = self.get_update_plan_embed(id.clone(), plan_update).await?;
                embeds.push(embed);
            }

            let payload = json!({
                "username": "複数の企画情報更新",
                "embeds": embeds
            });

            self.send_webhook(payload).await?;
        }
        Ok(())
    }

    pub async fn send_update_plan(&self, id: String, plan_update: &PlanUpdate) -> Result<()> {
        let embed = self.get_update_plan_embed(id.clone(), plan_update).await?;

        let payload = json!({
            "username": id,
            "avatar_url": format!("{}/v1/plans/{}/icon", self.base_url, id),
            "embeds": [embed]
        });

        self.send_webhook(payload).await.map_err(Into::into)
    }

    pub async fn send_delete_plan(&self, id: String) -> Result<()> {
        let embed = json!({
            "title": "企画情報が削除されました"
        });

        let payload = json!({
            "username": id,
            "embeds": [embed]
        });

        self.send_webhook(payload).await.map_err(Into::into)
    }

    pub async fn send_update_plan_icon(
        &self,
        id: String,
        content_type: String,
        data: impl Into<Vec<u8>>,
    ) -> Result<()> {
        let data_bytes = data.into();
        let extension = extension_from_content_type(&content_type);
        let filename = format!("{}.{}", id, extension);

        // Create embed for the message
        let embed = json!({
            "title": "企画アイコンが変更されました",
            "image": {
                "url": format!("attachment://{}", filename)
            }
        });

        // Create multipart/form-data payload
        let boundary = "----formdata-boundary-1234567890";
        let mut body = Vec::new();

        // Add payload_json part
        let payload_json = json!({
            "username": id,
            "embeds": [embed]
        });

        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"payload_json\"\r\n");
        body.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
        body.extend_from_slice(payload_json.to_string().as_bytes());
        body.extend_from_slice(b"\r\n");

        // Add file part
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"files[0]\"; filename=\"{}\"\r\n",
                filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
        body.extend_from_slice(&data_bytes);
        body.extend_from_slice(b"\r\n");

        // Close boundary
        body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        // Send multipart request with rate limiting retry
        loop {
            let mut init = RequestInit::new();
            init.with_method(Method::Post);
            init.with_body(Some(body.clone().into()));

            let headers = worker::Headers::new();
            headers.set(
                "Content-Type",
                &format!("multipart/form-data; boundary={}", boundary),
            )?;
            init.with_headers(headers);

            let request = Request::new_with_init(&self.webhook_url, &init)?;
            let mut response = Fetch::Request(request).send().await?;

            let status = response.status_code();

            if (200..300).contains(&status) {
                return Ok(());
            }

            // Handle rate limiting (429 status)
            if status == 429 {
                // Try to get Retry-After header
                if let Ok(retry_after_str) = response.headers().get("Retry-After") {
                    if let Some(retry_after_str) = retry_after_str {
                        if let Ok(retry_seconds) = retry_after_str.parse::<u64>() {
                            // Sleep for the specified number of seconds
                            worker::console_log!(
                                "Rate limited (multipart). Retrying after {} seconds",
                                retry_seconds
                            );
                            self.sleep_ms(retry_seconds * 1000).await;
                            continue; // Retry the request
                        }
                    }
                }
                // Fallback: sleep for 1 second if no valid Retry-After header
                worker::console_log!(
                    "Rate limited (multipart). Retrying after 1 second (fallback)"
                );
                self.sleep_ms(1000).await;
                continue;
            }

            // For other errors, return immediately
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Discord webhook failed with status {}: {}",
                status,
                error_text
            ));
        }
    }
}
