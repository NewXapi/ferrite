//! Admin page API adapter.
//!
//! Provides typed REST API calls using `client::ApiClient` and `contract::api` DTOs
//! for tokens, channels, groups, and billing entities.

use client::{ApiClient, ApiResult};
use contract::api::admin::{ChannelDto, ChannelUpsertRequest, GroupDto, GroupUpsertRequest};
use contract::api::token::{CreateTokenRequest, TokenDto, UpdateTokenRequest};
use contract::api::billing::{AliasDto, AliasUpsertRequest, RedemptionDto, RedemptionUpsertRequest, SubscriptionDto, SubscriptionUpsertRequest};

/// List channels from the admin API.
pub async fn list_channels_api(client: &ApiClient) -> ApiResult<Vec<ChannelDto>> {
    client.get("/api/channel").await
}

/// List groups from the admin API.
pub async fn list_groups_api(client: &ApiClient) -> ApiResult<Vec<GroupDto>> {
    client.get("/api/group").await
}

/// Create a new channel.
pub async fn create_channel_api(client: &ApiClient, req: ChannelUpsertRequest) -> ApiResult<()> {
    client.post("/api/channel", &req).await
}

/// Update a channel.
pub async fn update_channel_api(client: &ApiClient, key: &str, req: ChannelUpsertRequest) -> ApiResult<()> {
    client.put(&format!("/api/channel/{key}"), &req).await
}

/// Delete a channel.
pub async fn delete_channel_api(client: &ApiClient, key: &str) -> ApiResult<()> {
    client.delete(&format!("/api/channel/{key}")).await
}

/// List tokens from the admin API.
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    client.get("/api/token").await
}

/// Create a new token.
pub async fn create_token_api(client: &ApiClient, req: CreateTokenRequest) -> ApiResult<()> {
    client.post("/api/token", &req).await
}

/// Update a token.
pub async fn update_token_api(client: &ApiClient, key: &str, req: UpdateTokenRequest) -> ApiResult<()> {
    client.put(&format!("/api/token/{key}"), &req).await
}

/// Delete a token.
pub async fn delete_token_api(client: &ApiClient, key: &str) -> ApiResult<()> {
    client.delete(&format!("/api/token/{key}")).await
}

/// List aliases from the admin API.
pub async fn list_aliases_api(client: &ApiClient) -> ApiResult<Vec<AliasDto>> {
    client.get("/api/alias").await
}

/// Create a new alias.
pub async fn create_alias_api(client: &ApiClient, req: AliasUpsertRequest) -> ApiResult<()> {
    client.post("/api/alias", &req).await
}

/// Update an alias.
pub async fn update_alias_api(client: &ApiClient, key: &str, req: AliasUpsertRequest) -> ApiResult<()> {
    client.put(&format!("/api/alias/{key}"), &req).await
}

/// Delete an alias.
pub async fn delete_alias_api(client: &ApiClient, key: &str) -> ApiResult<()> {
    client.delete(&format!("/api/alias/{key}")).await
}

/// List subscriptions from the admin API.
pub async fn list_subscriptions_api(client: &ApiClient) -> ApiResult<Vec<SubscriptionDto>> {
    client.get("/api/subscription").await
}

/// Create a new subscription.
pub async fn create_subscription_api(client: &ApiClient, req: SubscriptionUpsertRequest) -> ApiResult<()> {
    client.post("/api/subscription", &req).await
}

/// Update a subscription.
pub async fn update_subscription_api(client: &ApiClient, key: &str, req: SubscriptionUpsertRequest) -> ApiResult<()> {
    client.put(&format!("/api/subscription/{key}"), &req).await
}

/// Delete a subscription.
pub async fn delete_subscription_api(client: &ApiClient, key: &str) -> ApiResult<()> {
    client.delete(&format!("/api/subscription/{key}")).await
}

/// List redemptions from the admin API.
pub async fn list_redemptions_api(client: &ApiClient) -> ApiResult<Vec<RedemptionDto>> {
    client.get("/api/redemption").await
}

/// Create a new redemption.
pub async fn create_redemption_api(client: &ApiClient, req: RedemptionUpsertRequest) -> ApiResult<()> {
    client.post("/api/redemption", &req).await
}

/// Update a redemption.
pub async fn update_redemption_api(client: &ApiClient, key: &str, req: RedemptionUpsertRequest) -> ApiResult<()> {
    client.put(&format!("/api/redemption/{key}"), &req).await
}

/// Delete a redemption.
pub async fn delete_redemption_api(client: &ApiClient, key: &str) -> ApiResult<()> {
    client.delete(&format!("/api/redemption/{key}")).await
}