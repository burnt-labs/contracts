use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, StdResult};

/// Shared context passed through the pipeline, mutated by plugins.
pub struct DefaultPluginCtx<'a> {
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,

    pub token_id: String,
    pub seller: Addr,
    pub buyer: Option<Addr>,

    pub ask_price: Option<u128>,     // if (List) or None on transfer
    pub funds_total: u128,           // sum(info.funds)
    pub funds_remaining: u128,       // decreases as plugins deduct
    pub deductions: Vec<(String, u128, Addr)>, // (reason, amount, to)

    pub primary_complete: bool,
}

/// All plugins implement these
pub trait Plugin<Context> {
    fn on_list(ctx: &mut Context) -> StdResult<()>;
    fn on_delist(ctx: &mut Context) -> StdResult<()>;
    fn on_transfer(ctx: &mut Context) -> StdResult<()>;
}