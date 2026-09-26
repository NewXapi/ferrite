/// 从一段文本中解析出 (URL, Key) 二元组;两者缺一即返回 `None`。
///
/// 支持三种行形状,按行扫描、首个命中即锁定(不覆盖已取值):
/// - `key=value`:键名大小写不敏感,URL 键为 `url` / `base_url` / `endpoint`
///   / `api_base`,Key 键为 `key` / `api_key` / `apikey` / `token`;
/// - 空白/`|`/`,`/`;` 分隔的多段行:首段以 `http(s)://` 开头视为 URL,
///   其后各段中 `sk-` / `rk-` 前缀或长度 ≥ 20 的视为 Key;
/// - 裸行:`sk-`/`rk-` 前缀且长度 ≥ 20 的视为 Key,`http(s)://` 开头的视为 URL。
///
/// 纯函数:无副作用、不发网络、不改入参。订阅 tab 的「粘贴 URL + Key」解析用。
pub fn parse_url_key(text: &str) -> Option<(String, String)> {
    let mut url = None::<String>;
    let mut key = None::<String>;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // key=value 形式
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            if k == "url" || k == "base_url" || k == "endpoint" || k == "api_base" {
                url = Some(v.to_string());
                continue;
            }
            if k == "key" || k == "api_key" || k == "apikey" || k == "token" {
                key = Some(v.to_string());
                continue;
            }
            continue;
        }
        let parts: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || c == '|' || c == ',' || c == ';')
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            let first = parts[0];
            if first.starts_with("http://") || first.starts_with("https://") {
                url.get_or_insert_with(|| first.to_string());
                for p in &parts[1..] {
                    if p.starts_with("sk-") || p.starts_with("rk-") || p.len() >= 20 {
                        key.get_or_insert_with(|| p.to_string());
                        break;
                    }
                }
                continue;
            }
        }
        // 裸 key 行:sk-/rk- 前缀 + 至少 20 字符,降低误匹配短串的概率
        if (line.starts_with("sk-") || line.starts_with("rk-")) && line.len() >= 20 {
            key.get_or_insert_with(|| line.to_string());
            continue;
        }
        // 裸 URL 行
        if line.starts_with("http://") || line.starts_with("https://") {
            url.get_or_insert_with(|| line.to_string());
        }
    }
    match (url, key) {
        (Some(u), Some(k)) => Some((u, k)),
        _ => None,
    }
}
