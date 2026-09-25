//! Proxy-side argument policy. Pure functions so the `IsAllowed` query and the execute
//! path cannot drift apart. Everything here is about bounding what a compromised session
//! key can do; the collection still applies the user's own authorization afterwards.

use cosmwasm_std::{Addr, Env, Storage};
use cw721::Expiration;

use crate::{
    error::ContractError,
    msg::ProxyAction,
    state::{ALLOWED_OPERATORS, COLLECTIONS, Config, FORMER_OPERATORS},
};

/// Check a resolved (validated) collection and action against the stored policy.
pub fn check(
    storage: &dyn Storage,
    env: &Env,
    config: &Config,
    collection: &Addr,
    action: &ProxyAction,
) -> Result<(), ContractError> {
    if *collection == env.contract.address {
        return Err(ContractError::SelfTarget {});
    }
    if !COLLECTIONS.has(storage, collection) {
        return Err(ContractError::CollectionNotAllowed {
            collection: collection.to_string(),
        });
    }
    match action {
        ProxyAction::Burn { .. } => Ok(()),
        // Revocation only removes authority the effective sender granted, so it stays
        // available for an operator the admin has since removed. It is still bounded to
        // operators this proxy ever allowed, so a stolen session cannot disturb approvals
        // the user granted to unrelated operators.
        ProxyAction::RevokeAll { operator } => check_operator_current_or_former(storage, operator),
        ProxyAction::ApproveAll { operator, expires } => {
            check_operator(storage, operator)?;
            check_expiry(env, config, expires)
        }
    }
}

fn check_operator(storage: &dyn Storage, operator: &str) -> Result<(), ContractError> {
    // Operators are stored validated; an unvalidated string can only match if it is the
    // exact canonical form, which is what we want.
    let key = Addr::unchecked(operator);
    if ALLOWED_OPERATORS.has(storage, &key) {
        Ok(())
    } else {
        Err(ContractError::OperatorNotAllowed {
            operator: operator.to_string(),
        })
    }
}

fn check_operator_current_or_former(
    storage: &dyn Storage,
    operator: &str,
) -> Result<(), ContractError> {
    let key = Addr::unchecked(operator);
    if ALLOWED_OPERATORS.has(storage, &key) || FORMER_OPERATORS.has(storage, &key) {
        Ok(())
    } else {
        Err(ContractError::OperatorNotAllowed {
            operator: operator.to_string(),
        })
    }
}

/// With a cap configured, an approval must expire at a concrete timestamp no later than
/// `now + cap`. `Never`, height-based and missing expiries are rejected because cw721
/// defaults a missing expiry to `Never`, which would outlive the sponsored session.
fn check_expiry(
    env: &Env,
    config: &Config,
    expires: &Option<Expiration>,
) -> Result<(), ContractError> {
    let Some(max_seconds) = config.max_approval_seconds else {
        return Ok(());
    };
    let err = ContractError::ApprovalExpiryOutOfBounds { max_seconds };
    match expires {
        Some(Expiration::AtTime(t)) => {
            // u128 nanosecond arithmetic: cannot overflow for any u64 cap
            let now = env.block.time.nanos() as u128;
            let t = t.nanos() as u128;
            let window = (max_seconds as u128) * 1_000_000_000;
            if t <= now || t - now > window {
                return Err(err);
            }
            Ok(())
        }
        _ => Err(err),
    }
}
