//! model → channel 映射

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// 渠道配置 (从 PG kv_store 加载)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelConfig {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub channel_type: String,
    pub keys: Vec<String>,
    pub models: Vec<ModelRoute>,
}

/// 模型路由：alias → upstream
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelRoute {
    pub alias: String,
    pub upstream: String,
}

/// 一个渠道在某个模型上的路由信息
#[derive(Debug, Clone)]
#[allow(dead_code)] // channel_id 后续日志/计费会用
pub struct ResolvedRoute {
    pub channel_id: i64,
    pub channel_name: Arc<str>,
    pub base_url: Arc<str>,
    pub upstream_model: Arc<str>,
    pub api_key: Arc<str>,
}

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("unknown model: {0}")]
    UnknownModel(String),
}

/// 路由索引：model alias → ResolvedRoute
pub struct RouteIndex {
    inner: arc_swap::ArcSwap<HashMap<String, ResolvedRoute>>,
}

impl RouteIndex {
    pub fn new() -> Self {
        Self {
            inner: arc_swap::ArcSwap::from_pointee(HashMap::new()),
        }
    }

    /// 从渠道配置列表构建索引并原子替换
    pub fn build_from_channels(&self, channels: &[ChannelConfig]) {
        let mut index = HashMap::new();
        for ch in channels {
            let channel_name: Arc<str> = Arc::from(ch.name.as_str());
            let base_url: Arc<str> = Arc::from(ch.base_url.as_str());
            // ponytail: 取第一个 key，多 key 轮转以后再做
            let api_key: Arc<str> = ch
                .keys
                .first()
                .map(|k| Arc::from(k.as_str()))
                .unwrap_or_else(|| Arc::from(""));

            for m in &ch.models {
                index.insert(
                    m.alias.clone(),
                    ResolvedRoute {
                        channel_id: ch.id,
                        channel_name: channel_name.clone(),
                        base_url: base_url.clone(),
                        upstream_model: Arc::from(m.upstream.as_str()),
                        api_key: api_key.clone(),
                    },
                );
            }
        }
        self.inner.store(Arc::new(index));
    }

    /// 查 model 对应的路由
    pub fn resolve(&self, model: &str) -> Result<ResolvedRoute, DispatchError> {
        self.inner
            .load()
            .get(model)
            .cloned()
            .ok_or_else(|| DispatchError::UnknownModel(model.to_string()))
    }

    /// 列出所有可用模型 alias（字典序，输出稳定）
    pub fn list_models(&self) -> Vec<String> {
        let mut models: Vec<String> = self.inner.load().keys().cloned().collect();
        models.sort();
        models
    }
}

impl Default for RouteIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(name: &str, aliases: &[&str]) -> ChannelConfig {
        ChannelConfig {
            id: 1,
            name: name.into(),
            base_url: "http://up/v1".into(),
            channel_type: "openai".into(),
            keys: vec!["k".into()],
            models: aliases
                .iter()
                .map(|a| ModelRoute {
                    alias: (*a).into(),
                    upstream: (*a).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn list_models_sorted_and_complete() {
        let idx = RouteIndex::new();
        idx.build_from_channels(&[channel("c1", &["zeta", "alpha"]), channel("c2", &["mid"])]);
        assert_eq!(idx.list_models(), vec!["alpha", "mid", "zeta"]);
    }
}
