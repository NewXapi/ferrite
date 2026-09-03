//! `state` —— gate 2：状态闸（enabled / 过期 / IP 白名单 / 用户禁用 / auth_version）

use async_trait::async_trait;
use std::sync::Arc;
use arc_swap::ArcSwap;
use gateway_pipeline::IpPolicy;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;
use super::UserInfo;
use super::snapshot::UserSnapshot;

pub struct StateGate {
    users: Arc<ArcSwap<UserSnapshot>>,
    allow_ips: Arc<ArcSwap<IpPolicy>>,
}

impl StateGate {
    pub fn new(users: Arc<ArcSwap<UserSnapshot>>, allow_ips: Arc<ArcSwap<IpPolicy>>) -> Self {
        Self { users, allow_ips }
    }
}

#[async_trait]
impl Gate for StateGate {
    fn name(&self) -> &'static str { "state" }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let token = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;

        // 1. 查 user
        let user_record = self.users.load().lookup(token.user_id)
            .ok_or(Rejection::UserNotFound)?;

        // 2. user 状态
        if !user_record.enabled { return Err(Rejection::UserDisabled); }

        // 3. auth_version 单调性：写安全相关字段后必须 bump
        if user_record.auth_version < token.auth_version {
            return Err(Rejection::TokenAuthVersionMismatch);
        }

        // 4. token 过期
        if let Some(exp) = token.expires_at {
            if now_unix() >= exp { return Err(Rejection::TokenExpired); }
        }

        // 5. IP 白名单
        if !self.allow_ips.load().allows(&ctx.request_meta.client_ip) {
            return Err(Rejection::IpNotAllowed);
        }

        // 6. 填充 user / group
        ctx.user = Some(UserInfo {
            id: user_record.id,
            enabled: user_record.enabled,
            group: user_record.group.clone(),
            auth_version: user_record.auth_version,
        });
        ctx.group = if !token.group.is_empty() {
            Some(token.group.clone())
        } else {
            Some(user_record.group)
        };
        Ok(())
    }
}

fn now_unix() -> i64 {
    // TODO: std::time::SystemTime::now() 转换
    unimplemented!("now_unix")
}
