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
