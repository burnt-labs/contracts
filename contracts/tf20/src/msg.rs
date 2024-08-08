use cosmos_sdk_proto::cosmos::bank::v1beta1::Metadata;
use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Binary, Coin, Decimal, Uint128};
use cw20::Expiration;
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};
pub use cw_controllers::ClaimsResponse;

#[cw_serde]
pub struct InstantiateMsg {
    /// name of the derivative token
    pub creator: String,
    /// symbol / ticker of the derivative token
    pub subdenom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Admin functionality to mirror TokenFactory
    /// Creates more tokens to the specified account
    Mint { recipient: String, amount: Uint128 },
    
    /// The following are admin overrides of their matching named commands
    ForceTransfer { owner: String, recipient: String, amount: Uint128 },
    ForceBurn { owner: String, amount: Uint128 },
    ForceSend {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },

    /// Allows current admin of the contract to select a new admin for the contract, or set it to empty.
    /// if the admin is set to empty, no admin commands can be called again
    UpdateContractAdmin { new_admin: String },
    /// Allows the current admin to select a new admin of the TokenFactory denom, or set it to empty.
    /// If a new admin is selected for the denom, this contract will no longer be a valid admin of the denom
    /// and all allowances and cw20 utility will no longer be functional
    UpdateTokenFactoryAdmin { new_admin: String },
    /// Allows the admin to modify the token denom metadata
    ModifyMetadata { metadata: Metadata },

    /// Implements CW20. Transfer is a base message to move tokens to another account without triggering actions
    Transfer { recipient: String, amount: Uint128 },
    /// Implements CW20. Burn is a base message to destroy tokens forever
    Burn { amount: Uint128 },
    /// Implements CW20.  Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Implements CW20 "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Implements CW20 "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Implements CW20 "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `env.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Implements CW20 "approval" extension. Sends amount tokens from owner -> contract
    /// if `env.sender` has sufficient pre-approval.
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Implements CW20 "approval" extension. Destroys tokens forever
    BurnFrom { owner: String, amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Implements CW20. Returns the current balance of the given address, 0 if unset.
    #[returns(BalanceResponse)]
    Balance { address: String },
    /// Implements CW20. Returns metadata on the contract - name, decimals, supply, etc.
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    /// Implements CW20 "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },
}