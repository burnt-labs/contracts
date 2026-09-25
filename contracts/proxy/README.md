# xion-asset-proxy

A fixed-address sponsorship proxy. Treasury `authz` and `feegrant` configurations name this
one contract once; it relays a closed set of user actions to allowlisted
[`asset-proxyable`](../asset-proxyable) collections whose addresses are not known when
users authorize their session keys.

The effective sender forwarded to a collection is always this contract's own
`info.sender`. There is no sender parameter anywhere in the API.

## Messages

| Message | Who | Effect |
|---|---|---|
| `sponsored_burn { collection, token_id }` | any user | Burn `token_id` as the caller. The collection enforces owner-only. |
| `sponsored_approval { collection, action }` | any user | `approve_all` a currently allowed operator, or `revoke_all` any operator, as the caller. |
| `add_collection { collection }` | admin | Allowlist a collection as a target for sponsored calls. |
| `remove_collection { collection }` | admin | Remove it. Nothing is relayed there afterwards, revocation included; users revoke directly on the collection. |
| `remove_allowed_operator { operator }` | admin | Remove an operator. There is no way to add one after instantiation. Prospective only: existing approvals persist until expiry or revocation, and `revoke_all` keeps working for the removed operator. |
| `update_admin { admin }` | admin | Hand over administration (never to the proxy itself). |

Burn and approval are separate top-level keys on purpose: an authz
`ContractExecutionAuthorization` filters on the outer key, so the irreversible action can
get its own small `MaxCallsLimit` budget.

Queries: `config`, `collections { start_after, limit }`, and
`is_allowed { collection, action }`, which evaluates the proxy-side policy without
executing. It cannot tell you whether the collection trusts this proxy; only the
collection knows that (`get_trusted_proxies` on the collection).

## Policy enforced here

- Every message is non-payable. Forwarded calls attach no funds.
- The collection must be allowlisted and must not be this contract.
- `approve_all.operator` must be one of the operators currently allowed.
  `revoke_all.operator` may be any operator that is or ever was allowed here, so a user can
  withdraw authority through the sponsored path even after the admin removed that operator,
  while a stolen session cannot disturb approvals the user granted to unrelated operators.
  Revocation is always available directly on the collection, unsponsored, which is the
  path users take once a collection leaves the allowlist.
- With `max_approval_seconds` set, `approve_all.expires` must be a timestamp within
  `(now, now + cap]`. `never`, height-based and missing expiries are rejected because cw721
  treats a missing expiry as permanent.

The collection then applies the user's own authorization, so a relayed action can never
exceed what the caller could do directly, and a relayed burn is stricter (owner-only).

## Deployment rules

- **No wasm admin.** Instantiate with `admin: None` at the x/wasm level. This crate has no
  `migrate` entrypoint, but that alone does not prevent migration: wasmd runs the
  destination code's migrate. Only the absence of a wasm admin makes the sender
  derivation immutable.
- **What the allowlist is for.** It bounds where Treasury money can be spent, not who may
  act. Authorization is decided by each collection, which honours this proxy only if its
  creator registered it, so the allowlist adds nothing there and removing an entry revokes
  nothing. Its one job is to stop a sponsored call *succeeding* against a contract the admin
  never chose: a failing call costs only the fee, but a successful one also consumes the
  grant's call budget, and a contract written to burn gas would drain both. Keep the list to
  collections you operate.
- The proxy does not inspect what an allowlisted collection runs. Whether a genuine
  `asset-proxyable` collection honours forwarded actions is decided by its creator with
  `add_trusted_proxy`; a base-code collection rejects the envelope outright. An arbitrary
  contract the admin allowlists could accept the envelope and run its own logic, but it
  gains no authority over the caller: the forwarded message carries no funds and the
  target can only act with the authority it already has. The allowlist therefore bounds
  where Treasury gas can be spent, and the admin should only add collections they operate.
- `allowed_operators` is the marketplace address. Before naming it, check the
  marketplace's own x/wasm admin posture: a marketplace migrated to hostile code could
  exercise every approval users granted it, proxy or not. Removal is permanent for this
  deployment; approvals already stored on collections are unaffected and users can still
  revoke them directly or through the proxy.
- **Set `max_approval_seconds` in production.** Without a cap, `approve_all` with no expiry
  stores a permanent approval that outlives the sponsored session and any later operator
  removal. The cap bounds the worst case after an operator incident.
- Operator removal or a marketplace redeploy to a new address cannot be undone here.
  Recovery is a new proxy deployment: new address, `add_trusted_proxy` on every collection,
  and every user re-authorizing their session key. That cost is the price of the
  removal-only guarantee users authorized against.
- Coins sent to this address by a plain bank transfer are unrecoverable. It has no
  funds-accepting path and no withdraw.
- Each new collection needs two admin transactions: `add_trusted_proxy` on the collection
  by its creator, and `add_collection` here.

## Trust model and constraints

This contract is one specific instance of the authority described in
[`asset-proxyable`'s trust model](../asset-proxyable/README.md#trust-model-design-decisions-and-constraints).
A collection that registers it is granting it the power to assert who the sender is, so
the guarantees below are what make that grant defensible. They are properties of this
deployment, not of the pattern.

- **The effective sender is never a parameter.** It is always this contract's own
  `info.sender`, derived in one place. No message accepts a sender field.
- **The action set is closed.** Burn, approve-all, revoke-all. New actions require a
  release on both sides, so the authority cannot widen by configuration.
- **Proxied burn is owner-only**, stricter than a direct burn, which honours operator and
  approval authority.
- **Approvals are bounded by policy.** Only configured operators, and with a cap set, only
  finite expiries. This is what distinguishes the canonical proxy from an arbitrary
  trusted address: a collection that registers some other proxy gets none of it.
- **No funds.** Every message rejects them and forwarded calls attach none.
- **Immutable in production.** Deployed without a wasm admin, so the sender derivation
  cannot be changed afterwards. See the deployment rules above.

Constraints that follow from the design, and are not defects:

- Removing an operator, or removing a collection, is prospective. It stops this contract
  relaying, but cannot revoke approvals already stored on collections. Users withdraw those
  themselves, through this contract while the collection is still allowlisted, or directly
  on the collection at any time.
- The operator set can only shrink. Adding one later would widen every grant users already
  signed, so recovery from a marketplace redeploy is a proxy redeployment, not a config
  change.
- Allowlisting a collection bounds only where sponsored gas may be spent. It makes no
  claim about what that collection's code does; see the redemption note below.

## For redemption backends

A sponsored burn is confirmed by the token no longer existing, not by events. The proxy
relays one message and does not verify what the target did; an allowlisted contract that
is not a genuine `asset-proxyable` collection could emit burn-like attributes and return
success without burning. Before treating a redemption as final, query `owner_of` (or
`nft_info`) on the collection after finality and require a not-found result.

## Treasury shape (summary)

- authz: **one** `MsgGrant` carrying **one** `ContractExecutionAuthorization` with several
  `ContractGrant` entries: this contract with `AcceptedMessageKeys ["sponsored_approval"]`
  and a generous `MaxCallsLimit`; this contract again with `["sponsored_burn"]` and a small
  `MaxCallsLimit`; and the marketplace's own entries. Do not issue separate grants: authz
  stores one authorization per granter, grantee and message type, so a second `MsgGrant`
  overwrites the first. wasmd matches entries by contract address and uses the first whose
  limit and filter both accept, so each entry's counter is independent.
- Budget note: approvals and revocations share the `sponsored_approval` counter. A stolen
  session can exhaust it with no-op revocations; that denies sponsorship, not authority,
  and users keep the direct unsponsored path.
- feegrant: an allowance bound to the session grantee that wraps a contract-scoped
  allowance naming this proxy and the marketplace.

See `.agent-docs/trusted-proxy-plan.md` in the maintainers' working notes for the full
design record and threat model.
