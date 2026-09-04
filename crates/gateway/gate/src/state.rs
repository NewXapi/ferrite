//! `state` —— gate 2：状态闸（enabled / 过期 / IP 白名单 / 用户禁用 / auth_version）

use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;

use super::UserInfo;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;
use super::snapshot::adapt::{UserView, now_unix};
use super::snapshot::{IpPolicy, UserSnapshot};

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
    fn name(&self) -> &'static str {
        "state"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let token = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;

        // 0. token 自身必须 enabled
        if !token.enabled {
            return Err(Rejection::InvalidApiKey);
        }

        // 1. 查 user
        let user_key = ctx.user_key.as_deref().ok_or(Rejection::AuthSkipped)?;
        let user_record = self
            .users
            .load()
            .lookup(user_key)
            .ok_or(Rejection::UserNotFound)?;

        // 2. user 状态
        if !user_record.is_enabled() {
            return Err(Rejection::UserDisabled);
        }

        // 3. auth_version 单调性
        if user_record.auth_version() < token.auth_version {
            return Err(Rejection::TokenAuthVersionMismatch);
        }

        // 4. token 过期
        if let Some(exp) = token.expires_at
            && now_unix() >= exp
        {
            return Err(Rejection::TokenExpired);
        }

        // 5. IP 白名单
        if !self.allow_ips.load().allows(&ctx.request_meta.client_ip) {
            return Err(Rejection::IpNotAllowed);
        }

        // 6. 填充 user / group
        let user_group = user_record.group().to_string();
        let effective_group = if !token.group.is_empty() {
            token.group.clone()
        } else {
            user_group.clone()
        };

        ctx.user = Some(UserInfo {
            id: token.user_id,
            enabled: user_record.is_enabled(),
            group: user_group,
            auth_version: user_record.auth_version(),
        });
        ctx.group = Some(effective_group);

        Ok(())
    }
}
