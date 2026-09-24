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

## Trust model, design decisions and constraints

Read this before registering a proxy, and before acquiring or inheriting a collection that
runs this code.

### Wasm admin

The collection's x/wasm admin decides which code the collection runs and on what terms it
migrates. It is therefore the strongest authority over the contract, and everything below
describes guarantees about proxies, creators and users rather than about that account.

A collection cannot give it up and stay upgradeable, since the admin is what allows moving
onto this code and onward to later releases. Hold it in a multisig. Clearing it is a
reasonable alternative and freezes the collection's code for good, because a cleared admin
can never be set again.

### What registering a proxy actually grants

`add_trusted_proxy` tells this collection: *this address may assert who the sender is.*
Every proxied action then runs with whatever sender the proxy names, checked only against
that claimed identity. Within the closed action set that means the proxy can burn any
token whose owner it names, and can create operator approvals on behalf of any owner.

That is custodial-equivalent authority over the collection, limited to what each owner
could do themselves. It is a deliberate design decision, not an oversight: the whole point
of the pattern is that the collection stops asking "did the caller sign this?" and starts
asking "do I trust the caller to tell me who did?". Treat registering a proxy as handing
out a collection-wide operator role, and register only proxies whose code you have
reviewed.

The grant is public. Anyone can read `GetTrustedProxies`, and every registration emits
`trusted_proxy_added` with the proxy's kind, code id, admin and whether the strict check
was applied. Users of a collection inherit the creator's judgement here, so make it
visible in your own interfaces.

### Constraint: proxy-created approvals outlive the proxy and the creator

Clearing trust does not clear its consequences. When a proxy creates an operator approval,
cw721 stores it in its own map, keyed by owner and operator. Nothing records that the
approval came through a proxy. So:

- removing a proxy stops it acting, but leaves every approval it already created;
- a creator handover (accepted transfer, or a migration that rotates the creator) clears
  the trusted-proxy set, and still leaves those approvals;
- the approvals persist until they expire or each owner revokes them individually.

**Accepted residual (2026-09-24).** A creator can register a proxy they control, seed
operator approvals from every holder to an address they control, and then hand the
collection over. The new creator sees an empty trusted-proxy set and may reasonably
conclude the collection is clean while those approvals are still live. This is inherent to
the forwarder pattern: no on-chain mechanism can distinguish a proxy-created approval from
one the owner made directly, and revoking other users' approvals from any single key would
break the effective-sender model the design rests on.

The elimination path, deliberately out of scope, is to stop trusting the proxy's word:
have each user sign the intended payload and verify that signature here against the
claimed sender, so a proxy can only relay actions users actually authorized. That is a
meta-transaction design and needs a signed payload format, per-user nonces for replay
protection, and client support for the extra signature.

### Checklist when acquiring or inheriting a collection

An empty `trusted_proxies` set is not by itself evidence that a collection is clean.
Before treating a handover as complete:

1. Enumerate live operator approvals for holders you care about, either with per-holder
   `AllOperators` queries or by reconstructing them from historical
   `trusted_proxy_added` and `proxied_by` event attributes. Approvals with no expiry
   deserve particular attention.
2. Check `withdraw_address`. It is creator-set, survives handover, and is not cleared by
   any of the handover paths, so stray-fund recovery keeps routing to the previous
   creator's address until you overwrite it.
3. Re-register only the proxies you intend to trust, preferably with `require_immutable`
   set, and confirm the resulting `trusted_proxy_added` events show a contract with no
   wasm admin.

### Why the immutability check is opt-in

`require_immutable` is off by default because a proxy kept under a wasm admin is a
legitimate setup while the proxy itself is still being iterated on: a hard rule would
block it. Turn it on once the proxy deployment is final. It does not defend against a
malicious creator, who can register a hostile immutable contract just as easily, but for
an honest deployment it converts "trust this address" into "trust this audited code that
cannot change". Recommended for production.

## Migration

`migrate` accepts an explicit table of cw2 `(name, version)` sources: base `asset` 0.1.0 and
0.2.0, and prior `asset-proxyable` versions. Coming from `asset`, the trusted-proxy map is
cleared, so dormant entries from an earlier proxyable life cannot wake up. Use
`{ "with_update": { "minter": null, "creator": null } }` unless you deliberately rotate
roles. Requires a wasm admin on the collection.

Fresh instantiations record cw2 name `asset-proxyable`, never `asset`.

A migration that rotates the creator (`with_update { creator: Some(..) }`) clears the
trusted-proxy set exactly like an accepted handover does, so a new creator always starts
with no proxies regardless of the route by which they took over.

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
