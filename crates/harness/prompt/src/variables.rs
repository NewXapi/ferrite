//! 变量展开兜底。
//!
//! **前端（Dioxus）负责** `{{user}}` / `{{char}}` / `{{time}}` / `/roll` 等
//! 所有宏 / 变量替换；Rust 端只做兜底（前端漏展开时给一个安全的回退），且只支持
//! `{{char}}` 与 `{{user}}` 两个 — 这两个是上下文无关的。
//!
//! 任何未识别的 `{{xxx}}` 模式**保持原样**，让前端能在日志中看到并修复；
//! Rust 不会假装知道怎么处理它们。

/// `expand_variables` 用的上下文。
#[derive(Debug, Clone, Default)]
pub struct VariableContext {
    /// 角色名（对应 `{{char}}`）
    pub character_name: String,
    /// 用户名（对应 `{{user}}`）
    pub user_name: String,
}

/// 在已物化的字符串上做最后一层 `{{char}}` / `{{user}}` 替换。
///
/// 规则：
/// - `{{char}}` → `VariableContext::character_name`
/// - `{{user}}` → `VariableContext::user_name`
/// - 其他 `{{xxx}}` 保持原样（前端再次物化的信号）
/// - 找不到闭合 `}}` 的 `{{` 也保持原样
pub fn expand_variables(input: &str, context: &VariableContext) -> String {
    // ponytail: O(n) 单遍扫描，规则简单到不需要 regex 依赖。
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // 找最近的 "}}"；找不到就保留 "{{"
            let mut j = i + 2;
            let mut found = None;
            while j + 1 < bytes.len() {
                if bytes[j] == b'}' && bytes[j + 1] == b'}' {
                    found = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(end) = found {
                let key = &input[i + 2..end];
                match key {
                    "char" => out.push_str(&context.character_name),
                    "user" => out.push_str(&context.user_name),
                    _ => {
                        // 未识别 → 原样保留（含 "{{" 和 "}}"），由前端兜底
                        out.push_str(&input[i..=end + 1]);
                    }
                }
                i = end + 2;
                continue;
            } else {
                // 未闭合，保留 "{{"
                out.push_str("{{");
                i += 2;
                continue;
            }
        }
        // 复制一个 UTF-8 字符；这里字节级复制在 ASCII 路径上正确，多字节 UTF-8
        // 也能保留（多字节序列不含 `{`），不必走 char 迭代以减少分支。
        out.push(input[i..].chars().next().unwrap());
        i += input[i..].chars().next().unwrap().len_utf8();
    }
    out
}
