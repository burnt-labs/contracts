use asset::msg::{AssetExtensionExecuteMsg, AssetExtensionQueryMsg};
use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::Addr;
use cw721::{
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, Expiration,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The base asset execute message, exactly as the `asset` contract accepts it.
pub type BaseExecuteMsg = asset::msg::ExecuteMsg<
    DefaultOptionalNftExtensionMsg,
    DefaultOptionalCollectionExtensionMsg,
    AssetExtensionExecuteMsg,
>;

/// The base asset query message, exactly as the `asset` contract accepts it.
pub type BaseQueryMsg = asset::msg::QueryMsg<
    DefaultOptionalNftExtension,
    DefaultOptionalCollectionExtension,
    AssetExtensionQueryMsg,
>;

pub type InstantiateMsg = asset::msg::InstantiateMsg<DefaultOptionalCollectionExtensionMsg>;
pub type MigrateMsg = cw721::msg::Cw721MigrateMsg;

/// Top-level execute message. Untagged so every base message keeps its exact JSON shape and
/// the proxy messages sit next to them at the top level.
///
/// Disambiguation rests on the externally tagged variant names being disjoint: no cw721 or
/// asset-extension variant is called `proxy_execute`, `add_trusted_proxy` or
/// `remove_trusted_proxy`, and no proxy variant shares a name with a base one (see the
/// `variant_names_are_disjoint` test). The `Proxy` arm is tried first and fails fast on any
/// base message with a cheap tag mismatch; serde buffers the input once.
// The base variant dwarfs the proxy one; boxing it would add an allocation to every
// message parse for a value that lives only for the duration of one entrypoint call.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum ProxyableExecuteMsg {
    Proxy(ProxyMsg),
    Base(BaseExecuteMsg),
}

/// Messages that only exist on this variant.
#[cw_serde]
#[serde(deny_unknown_fields)]
pub enum ProxyMsg {
    /// Execute `action` as `sender`. Accepted only when `info.sender` is a trusted proxy.
    /// Never payable.
    ProxyExecute { sender: String, action: ProxyAction },
    /// Register a proxy contract. cw721 `CREATOR` only. Granting this is equivalent to
    /// giving the proxy the power to burn and to set operators for every token owner in
    /// the collection, limited to what those owners could do themselves.
    ///
    /// `require_immutable` (default false) additionally requires `proxy` to be a contract
    /// with no x/wasm admin, checked on-chain at registration. Because a cleared admin can
    /// never be set again, that property is durable. Leave it off while the proxy is still
    /// being upgraded under an admin.
    AddTrustedProxy {
        proxy: String,
        #[serde(default)]
        require_immutable: bool,
    },
    /// Remove a proxy contract. cw721 `CREATOR` only.
    RemoveTrustedProxy { proxy: String },
}

/// The closed set of actions a trusted proxy may relay. Anything else must be sent by the
/// user directly.
#[cw_serde]
#[serde(deny_unknown_fields)]
pub enum ProxyAction {
    /// Owner-only. Unlike a direct `Burn`, operator or approval authority is not honoured.
    Burn {
        token_id: String,
    },
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    RevokeAll {
        operator: String,
    },
}

/// Top-level query message, same untagged shape as the execute message.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
#[serde(untagged)]
#[query_responses(nested)]
pub enum ProxyableQueryMsg {
    Proxy(ProxyQueryMsg),
    Base(BaseQueryMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
#[serde(deny_unknown_fields)]
pub enum ProxyQueryMsg {
    #[returns(Vec<Addr>)]
    GetTrustedProxies {},
}
