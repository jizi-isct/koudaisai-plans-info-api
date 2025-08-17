export async function kv_bulk_get(kv, keys, type = "text", withMetadata = false) {
  if (!kv || typeof kv.get !== "function") throw new Error(`KV namespace not valid`);
  if (!Array.isArray(keys)) throw new Error("keys must be an array of strings");

  if (withMetadata) {
    // Map<string, { value, metadata }>
    const map = await kv.getWithMetadata(keys, {type});
    return Object.fromEntries(map); // そのまま JS Object にして返す
  } else {
    // Map<string, string|object|null>
    const map = await kv.get(keys, {type});
    return Object.fromEntries(map);
  }
}