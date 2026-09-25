use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::Addr;
use cw721::Expiration;

pub use asset_proxyable::msg::ProxyAction;

#[cw_serde]
pub struct InstantiateMsg {
    /// Manages the collection allowlist and may remove operators. Multisig recommended.
    pub admin: String,
    /// Operators that `ApproveAll` may name (the marketplace). Fixed at instantiation;
    /// removal-only afterwards. Max 4.
    ///
    /// `RevokeAll` accepts any operator that is or ever was in this set, so a user can
    /// still withdraw authority from one the admin has since removed. It does not accept
    /// an operator this proxy never configured: those approvals are none of its business
    /// and a stolen session must not be able to disturb them.
    pub allowed_operators: Vec<String>,
    /// If set, `ApproveAll.expires` must be a concrete timestamp no further than this many
    /// seconds in the future (`Never` and height-based expiries are rejected).
    pub max_approval_seconds: Option<u64>,
    /// Initial collection allowlist. Whether a collection honours forwarded actions is the
    /// collection's own decision (`add_trusted_proxy` on the collection); this list only
    /// bounds where sponsored calls may be sent.
    pub collections: Vec<String>,
}

/// Approval actions a sponsored session may relay. Deliberately separate from
/// `ProxyAction` so `Burn` can never be smuggled in under the approval authz key.
#[cw_serde]
#[serde(deny_unknown_fields)]
pub enum ApprovalAction {
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    RevokeAll {
        operator: String,
    },
}

impl From<ApprovalAction> for ProxyAction {
    fn from(value: ApprovalAction) -> Self {
        match value {
            ApprovalAction::ApproveAll { operator, expires } => {
                ProxyAction::ApproveAll { operator, expires }
            }
            ApprovalAction::RevokeAll { operator } => ProxyAction::RevokeAll { operator },
        }
    }
}

#[cw_serde]
#[serde(deny_unknown_fields)]
pub enum ExecuteMsg {
    /// Burn `token_id` on `collection` as `info.sender`. Owner-only at the collection.
    SponsoredBurn {
        collection: String,
        token_id: String,
    },
    /// Approve a currently allowed operator, or revoke one that is or ever was allowed
    /// here, on `collection` as `info.sender`.
    SponsoredApproval {
        collection: String,
        action: ApprovalAction,
    },
    /// Admin: allowlist a collection as a target for sponsored calls.
    AddCollection { collection: String },
    /// Admin: remove a collection from the allowlist. Prospective only: it stops sponsored
    /// calls being relayed there and leaves approvals already stored on that collection
    /// untouched. Users revoke those directly on the collection.
    RemoveCollection { collection: String },
    /// Admin: remove an operator. There is deliberately no way to add one. Removal is
    /// prospective: it stops new sponsored approvals for that operator and leaves approvals
    /// already stored on collections in place until they expire or are revoked. Sponsored
    /// `RevokeAll` stays available for every operator that is or ever was allowed.
    RemoveAllowedOperator { operator: String },
    /// Admin: hand over administration.
    UpdateAdmin { admin: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub allowed_operators: Vec<Addr>,
    /// Operators removed from the allowlist. Sponsored `RevokeAll` still works for them.
    pub former_operators: Vec<Addr>,
    pub max_approval_seconds: Option<u64>,
}

#[cw_serde]
pub struct IsAllowedResponse {
    pub allowed: bool,
    /// Why not, when `allowed` is false.
    pub reason: Option<String>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    /// Allowlisted collections.
    #[returns(Vec<Addr>)]
    Collections {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Evaluate the proxy-side policy for an action without executing it. Does not check
    /// whether the collection trusts this proxy, which only the collection knows.
    #[returns(IsAllowedResponse)]
    IsAllowed {
        collection: String,
        action: ProxyAction,
    },
}
