//! 第三方登录（OIDC / OAuth 2.0）——「用 Google / GitHub / 微软账号登录」。
//!
//! 职责边界：本 crate 只管**协议层**——授权跳转 URL 构造、callback 的
//! code 换 token、ID token 验签与 claims 提取、state/nonce 防重放。
//! 「外部身份 → 本站用户」的解析（找号 / 绑号 / 建号）在
//! [`auth::identity`]，铸 JWT 在 [`auth::jwt`]，都不在本 crate 重复。
//!
//! Provider 配置存 PG `identity_providers` 表（admin 可管理），不写死在代码里：
//! 任何标准 OIDC provider 用同一套代码，靠 discovery document 自适应。
//!
//! 模块划分：
//! - [`provider`] — provider 行结构 + 注册表（DB 加载 + 内存缓存）
//! - [`store`] — state/nonce/PKCE 一次性存储（防重放）
//! - [`flow`] — authorize URL 构造 + callback 换 token 验 claims
//!
//! axum 端点与 `apps/api` 装配不在本 crate（下一个 PR）。

pub mod flow;
pub mod provider;
pub mod store;
