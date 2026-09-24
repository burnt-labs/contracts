# asset-proxyable

The XION [`asset`](../asset) contract plus a narrow, typed envelope that lets an explicitly
trusted proxy contract execute a closed set of user actions on behalf of the real sender.
Everything else is delegated verbatim to the base `asset` code, so this crate is a thin
wrapper and collections on the base code id have no proxy surface at all.

## What is added

- `proxy_execute { sender, action }`: accepted only from a registered trusted proxy, never
  with funds. `action` is one of `burn { token_id }` (owner-only, stricter than a direct
  burn), `approve_all { operator, expires }`, `revoke_all { operator }`. The action runs
  through the base contract with `sender` as the effective sender, so the base's own
  checks (ownership, operator storage, burn-while-listed guard) apply unchanged.
- `add_trusted_proxy { proxy, require_immutable? }` / `remove_trusted_proxy { proxy }`:
  cw721 **creator** only, at most 4 entries, events emitted. **Registering a proxy grants it
  burn and operator approval power over every token owner in the collection**, limited to
  what those owners could do themselves. Treat it like handing out an operator role
  collection-wide.
  - `require_immutable` (optional, default `false`) makes the collection check x/wasm at
    registration and refuse anything that is not a contract or that still has a wasm
    admin. A cleared admin can never be set again, so a proxy that passes stays immutable.
    Turn it on once the proxy deployment is final; leave it off while the proxy is still
    upgraded under an admin.
  - The `trusted_proxy_added` event always carries `proxy_kind` (`contract` or `account`),
    `proxy_code_id`, `proxy_admin` and `require_immutable`, so indexers can flag a
    collection that trusts a mutable proxy or a plain wallet.
- The creator cannot renounce ownership while any proxy is registered, so trust always
  stays revocable.
- Creator handover resets trust: while a creator transfer is live (pending, unexpired, and
  to someone other than the current creator), proxies can be removed but not added, and
  accepting ownership from a different creator clears every registered proxy. The new
  creator re-registers the proxy they trust with one `add_trusted_proxy` call. cw-ownable
  has no cancel action; proposing a transfer to yourself is the cancel idiom and lifts the
  add restriction immediately without clearing anything.
- What handover does **not** undo: operator approvals that were created through a proxy
  before the handover are ordinary cw721 approvals and persist until they expire or the
  owner revokes them. They are visible on-chain (`proxied_by` attributes) and users can
  revoke them directly or through the proxy at any time.
- `get_trusted_proxies {}` query.

All base messages and queries keep their exact JSON shape.

## Migration

`migrate` accepts an explicit table of cw2 `(name, version)` sources: base `asset` 0.1.0 and
0.2.0, and prior `asset-proxyable` versions. Coming from `asset`, the trusted-proxy map is
cleared, so dormant entries from an earlier proxyable life cannot wake up. Use
`{ "with_update": { "minter": null, "creator": null } }` unless you deliberately rotate
roles. Requires a wasm admin on the collection.

Fresh instantiations record cw2 name `asset-proxyable`, never `asset`.

Rollback is supported: base `asset` 0.2.0 accepts `asset-proxyable` 0.1.0 as a migration
source. After rolling back, the proxy messages and queries no longer exist (base code does
not know them) and the dormant `trusted_proxies` storage is never read; moving forward to
the variant again clears it.

## Relationship to the base crate

This crate depends on `asset` with its `library` feature and calls its entrypoint functions
directly. It is coupled to the base storage layout and message enums; any base or cw721
bump must re-run this crate's differential tests, and the two release together with pinned
versions. Build deployable artifacts per package (`cargo wasm -p asset-proxyable`), not
workspace-wide, or feature unification strips the base contract's entrypoints.
