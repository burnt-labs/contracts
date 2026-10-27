use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub in_denom: String,
    pub out_denom: String,
    pub rate: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Attach exactly one coin of `in_denom`; receive `floor(amount * rate)` of `out_denom`.
    /// Fails if the output is below `min_out`, guarding against a rate change before inclusion.
    Swap {
        min_out: Option<Uint128>,
    },
    UpdateRate {
        rate: Decimal,
    },
    /// Admin only. Nominates `admin`; takes effect when that address calls AcceptAdmin.
    ProposeAdmin {
        admin: String,
    },
    /// Pending admin only.
    AcceptAdmin {},
    /// Admin only. `amount: None` withdraws the contract's entire balance of `denom`. Sent to the admin.
    Withdraw {
        denom: String,
        amount: Option<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    Config {},
    /// Output amount for a hypothetical swap of `amount` in_denom; errors with SwapTooSmall if it floors to zero.
    #[returns(Uint128)]
    Simulate { amount: Uint128 },
}

#[cw_serde]
pub struct MigrateMsg {}
