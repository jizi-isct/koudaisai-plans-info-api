use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::HashMap;
use worker::js_sys::{Array, Promise};
use worker::wasm_bindgen::prelude::wasm_bindgen;
use worker::wasm_bindgen::JsValue;
use worker::wasm_bindgen_futures::JsFuture;
use worker::{kv::KvStore, Error};

#[wasm_bindgen(module = "/src/js/kv_bulk.js")]
extern "C" {
    #[wasm_bindgen(js_name = kv_bulk_get)]
    pub fn kv_bulk_get_js(kv: &JsValue, keys: Array, ty: &str, with_metadata: bool) -> Promise;
}

pub async fn kv_bulk_get_values<T: DeserializeOwned>(
    kv: &KvStore,
    keys: &[String],
    ty: &str, // "text" or "json"
) -> worker::Result<HashMap<String, Option<T>>> {
    let js_kv: &JsValue = unsafe { std::mem::transmute(kv) }; // KvStoreをJsValueとして扱う
    let arr = Array::new();
    for k in keys {
        arr.push(&JsValue::from_str(k));
    }

    let p = kv_bulk_get_js(js_kv, arr, ty, false);
    let js_val = JsFuture::from(p)
        .await
        .map_err(|e| Error::RustError(format!("{e:?}")))?;
    // JS Object -> HashMap<String, Option<T>>
    let map: HashMap<String, Option<T>> = serde_wasm_bindgen::from_value(js_val)
        .map_err(|e| Error::RustError(format!("serde_wasm_bindgen: {e}")))?;
    Ok(map)
}

#[derive(serde::Deserialize)]
struct WithMeta<V, M> {
    value: Option<V>,
    metadata: Option<M>,
}

pub async fn kv_bulk_get_with_meta<V: DeserializeOwned, M: DeserializeOwned>(
    kv: &KvStore,
    keys: &[String],
) -> worker::Result<HashMap<String, WithMeta<V, M>>> {
    let js_kv: &JsValue = unsafe { std::mem::transmute(kv) }; // KvStoreをJsValueとして扱う
    let arr = Array::new();
    for k in keys {
        arr.push(&JsValue::from_str(k));
    }

    let p = kv_bulk_get_js(js_kv, arr, "json", true);
    let js_val = JsFuture::from(p)
        .await
        .map_err(|e| Error::RustError(format!("{e:?}")))?;
    let map: HashMap<String, WithMeta<V, M>> = serde_wasm_bindgen::from_value(js_val)
        .map_err(|e| Error::RustError(format!("serde_wasm_bindgen: {e}")))?;
    Ok(map)
}

/// a に b をマージする（a が更新される）
pub fn deep_merge(a: &mut Value, b: Value) {
    match (a, b) {
        (Value::Object(a_map), Value::Object(b_map)) => {
            for (k, v_b) in b_map {
                deep_merge(a_map.entry(k).or_insert(Value::Null), v_b);
            }
        }
        // 配列は置き換え
        (a, v_b) => {
            *a = v_b;
        }
    }
}

pub fn extension_from_content_type(ct: &str) -> &'static str {
    match ct {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "bin",
    }
}
