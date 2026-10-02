# asset-proxyable

The XION [`asset`](../asset) contract plus a narrow, typed envelope that lets an explicitly
trusted proxy contract execute a closed set of user actions on behalf of the real sender.
Everything else is delegated verbatim to the base `asset` code, so this crate is a thin
wrapper and collections on the base code id have no proxy surface at all.

## What is added

- `proxy_execute { sender, action }`: accepted only from a registered trusted proxy, never
  with funds. `action` is one of `burn { token_id }` (owner-only, stricter than a direct
  burn), `approve_all { operator, expires }` (only for an operator the creator has
  admitted, see below), `revoke_all { operator }`. The action runs through the base
  contract with `sender` as the effective sender, so the base's own checks (ownership,
  operator storage, burn-while-listed guard) apply unchanged.
- `add_trusted_proxy { proxy, require_immutable? }` / `remove_trusted_proxy { proxy }`:
  cw721 **creator** only, at most 4 entries, events emitted. **Registering a proxy grants it
  burn power over every token owner in the collection**, limited to what those owners could
  do themselves, plus the ability to approve the operators named in
  `add_approval_operator`. Treat it like handing out an operator role collection-wide,
  bounded by that list.
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
- `add_approval_operator { operator }` / `remove_approval_operator { operator }`: cw721
  **creator** only, at most 8 entries, events emitted. This is the list of operators a
  proxy is allowed to approve on a holder's behalf. **It fails closed: while the list is
  empty, no proxied `approve_all` is relayed at all.** A proxy is trusted to say who the
  sender is, not to choose what that sender approves, so registering a proxy no longer
  implies it can make any address operator over the collection.
  - Proxied `revoke_all` is deliberately **not** gated. Revocation only removes authority
    and has to keep working during an incident, including for an operator that has since
    been delisted. Same reasoning as the removable-operator path on the proxy.
  - Removal is prospective. Approvals already granted to a delisted operator survive as
    ordinary cw721 approvals; delisting only stops new ones. Holders (or a proxy on their
    behalf) can still revoke them.
  - The gate is on the proxy path only. A holder signing `approve_all` themselves keeps
    the unrestricted cw721 behaviour, whatever this list says.
  - **A trusted proxy may not also be an approvable operator**, enforced in both
    directions. Otherwise a proxy could relay `approve_all { operator: itself }` for every
    holder and then act as an ordinary cw721 operator, which permits `approve`,
    `transfer_nft` and `send_nft` -- none of which are in the proxied action set. The
    closed action set only bounds a proxy that has not been made an operator.
  - While a creator transfer is live the list may only shrink, exactly as the trusted set
    does, so an outgoing creator cannot plant an operator after the incoming one has
    inspected it. Accepting the handover clears it.
- `get_trusted_proxies {}` and `get_approval_operators {}` queries.

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
token whose owner it names, and can make any owner approve one of the operators the
creator has admitted with `add_approval_operator`. It cannot choose the operator itself:
that list is the creator's, and while it is empty no proxied approval is relayed.

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

**What this means for a handover.** A previous creator could have registered a proxy they
controlled, had it create operator approvals for holders, and then handed the collection on. The
incoming creator sees an empty trusted-proxy set, which does not by itself mean no approvals are
outstanding. This follows from the forwarder pattern: nothing on chain distinguishes a
proxy-created approval from one the owner made directly, and revoking other users' approvals
from any single key would break the effective-sender model the design rests on. The checklist
below is how to inspect for them.

The approval-operator list does not change that, because the same creator controls both lists. What it does is make the setup **visible and narrow**: the address
being approved has to be named on-chain first and emits `approval_operator_added`, so the
seeding is a matter of public record rather than an invisible consequence of trusting a
proxy. It also bounds the damage from a proxy that is compromised rather than malicious
from the start, which is the more likely failure. A proxy whose key or code is taken over
can still burn, and can still approve the marketplaces the creator listed, but it cannot
direct approvals to an address of the attacker's choosing, and because the two roles are
disjoint it cannot approve itself either.

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
4. Re-list the operators you intend to allow. Accepting the handover clears
   `approval_operators` along with the trusted set, so sponsored approvals fail closed
   until you call `add_approval_operator` yourself. The `approval_operators_cleared`
   attribute on the accepting transaction says how many entries the previous creator had;
   the historical `approval_operator_added` events say which addresses they were, which is
   worth reading before deciding what to re-list.

### Why the immutability check is opt-in

`require_immutable` is off by default because a proxy kept under a wasm admin is a
legitimate setup while the proxy itself is still being iterated on: a hard rule would
block it. Turn it on once the proxy deployment is final. It does not defend against a
malicious creator, who can register a hostile immutable contract just as easily, but for
an honest deployment it converts "trust this address" into "trust this reviewed code that
cannot change". Recommended for production.

## Migration

`migrate` accepts an explicit table of cw2 `(name, version)` sources: base `asset` 0.1.0 and
0.2.0, and `asset-proxyable` at its own current version, which allows redeploying a patched
build over an existing one. Coming from `asset`, both the trusted-proxy map
and the approval-operator map are cleared, so dormant entries from an earlier proxyable life
cannot wake up. Use `{ "with_update": { "minter": null, "creator": null } }` unless you
deliberately rotate roles. Requires a wasm admin on the collection.

Fresh instantiations record cw2 name `asset-proxyable`, never `asset`.

**This crate keeps version parity with `asset`.** Both are at `0.2.0`, and when one bumps the
other bumps with it, even if its own code did not change. The variant is a thin wrapper whose
behaviour is mostly the base's, so a shared number is the only honest way for a deployed
collection's cw2 record to say which base behaviour it carries. Keep the two
`MIGRATABLE_FROM` tables in step when bumping: this crate's accepted sources, and the base's
entry for `("asset-proxyable", <version>)` that makes rollback work.

A migration that rotates the creator (`with_update { creator: Some(..) }`) clears both sets
exactly like an accepted handover does, so a new creator always starts with no trusted
proxies and no approvable operators regardless of the route by which they took over. Both
counts are reported as `trusted_proxies_cleared` and `approval_operators_cleared`
attributes.

### A collection arriving on this build starts with no approvable operators

Every route onto this code — a fresh instantiation, or a migration from base `asset` — leaves
the approval-operator list **empty**, and an empty list denies. Sponsored `approve_all` relays
nothing until the creator calls `add_approval_operator`. Plan for it:

- **Nothing is left open in the meantime.** The list denies while empty, so there is no
  window in which the collection is more permissive than it ends up being — a proxy can relay
  no approvals at all until the creator names an operator. Burns, revocations, user-signed
  `approve_all`, and every approval already stored keep working throughout. What waits on
  configuration is *new sponsored approvals*.
- **Close the gap in one transaction if you care about continuity.** A Cosmos transaction
  carries several messages, so `MsgMigrateContract` and `MsgExecuteContract` with
  `add_approval_operator` can travel together: trivially when the wasm admin and the creator
  are the same key, and otherwise with both signing the one transaction. There is no migrate
  parameter for this on purpose. Trust configuration is always a separate creator-signed
  call here, exactly as it is after instantiate, which keeps every entry attributable to a
  creator through its `approval_operator_added` event.
- **Remember it is per collection.** Each variant collection has its own list, so a fleet
  migration needs one `add_approval_operator` per collection, per operator.

Rollback is supported: base `asset` 0.2.0 accepts `asset-proxyable` 0.2.0 as a migration
source. After rolling back, the proxy messages and queries no longer exist (base code does
not know them) and the dormant `trusted_proxies` and `approval_operators` storage is never
read; moving forward to the variant again clears both.

## Relationship to the base crate

This crate depends on `asset` with its `library` feature and calls its entrypoint functions
directly. It is coupled to the base storage layout and message enums; any base or cw721
bump must re-run this crate's differential tests, and the two release together with pinned
versions. Build deployable artifacts per package (`cargo wasm -p asset-proxyable`), not
workspace-wide, or feature unification strips the base contract's entrypoints.
