# Asset Contract

A CosmWasm `cw721`-compatible asset contract for the XION network that layers marketplace
functionality and a pluggable execution pipeline on top of NFT collections. The library exposes
traits that make it easy to extend vanilla `cw721` contracts with listing, reservation, and plugin
logic without rewriting the core token implementation.

## Core Types and Traits

- `AssetContract` / `DefaultAssetContract`
  - Thin wrapper around the canonical `cw721` storage helpers (`Cw721Config`) plus marketplace
    indices (`IndexedMap` for listings and plugin registry).
  - `DefaultAssetContract` picks `AssetExtensionExecuteMsg` as the extension message so you get the
    marketplace verbs (list, reserve, delist, buy) out of the box.

- `SellableAsset`
  - Trait that adds four high-level marketplace entry points: `list`, `reserve`, `delist`, and `buy`.
  - Each method wires through the shared `AssetConfig` helpers defined in `execute.rs`, handling
    ownership checks, price validation, and state transitions.
  - Implemented for `AssetContract`, so adopting the trait is as simple as embedding the contract
    struct in your project.

- `PluggableAsset`
  - Trait that wraps `cw721::Cw721Execute::execute` with a plugin pipeline (`execute_pluggable`).
  - Hooks (`on_list_plugin`, `on_buy_plugin`, etc.) run before the base action and can mutate a
    `PluginCtx` shared across plugins. The returned `Response` from plugins is merged back into the
    main execution result, allowing plugins to enqueue messages, attributes, or data.
  - `DefaultAssetContract` implements the trait using `DefaultXionAssetContext`, giving you sensible
    defaults while still allowing custom contexts if you implement the trait yourself.

## Messages and State

- `AssetExtensionExecuteMsg` provides the marketplace verbs clients call via the `cw721` execute
  route. These are automatically dispatched through `SellableAsset` when `DefaultAssetContract`
  handles `execute_extension`.
- `Reserve` captures an optional reservation window (`Expiration`) and address, used to gate buys.
- `ListingInfo` stores price, seller, reservation data, and marketplace fee settings. The contract
  fetches NFT metadata directly from the `cw721` state when needed instead of duplicating it in the
  listing storage.
- `AssetConfig` centralizes the contract's storage maps and exposes helper constructors so you can
  use custom storage keys when embedding the contract inside another crate.

## Plugin System

`plugin.rs` includes a `Plugin` enum and a default plugin module:

- Price guards (`ExactPrice`, `MinimumPrice`). `ExactPrice` takes no parameters: it means *payment must equal the listing price*, so overpayment is refused rather than absorbed by the seller.
- Temporal restrictions (`NotBefore`, `NotAfter`, `TimeLock`).
- Access control (`AllowedMarketplaces`).
- Currency allow-listing (`AllowedCurrencies`).
- Royalty payouts (`Royalty`).

The provided `default_plugins` module contains ready-to-use helpers that enforce the relevant rules
and can enqueue `BankMsg::Send` payouts (e.g., royalties) or raise errors to abort the action.
Register plugins per collection with `AssetConfig::collection_plugins` and they will be invoked by
`execute_pluggable` automatically.

### Plugin parameter validation

Saving a plugin is creator-only, and parameters are validated at save time (`Plugin::validate`).
This matters because several fields fail at *hook* time rather than at save time, and some of them
brick the whole collection rather than the one message that carries them. Rejected:

| Rule | Why |
|---|---|
| `Royalty.bps` must be below `MAX_ROYALTY_BPS` (10000) | At 100% the royalty consumes the whole payment and the seller is sent a zero coin, which the bank module rejects; above it the deduction exceeds the payment. Every buy then fails, and because a configured royalty also disables raw transfers, nothing in the collection can move until the creator removes the plugin. |
| `Royalty.recipient` and `AllowedMarketplaces.marketplaces` must be valid addresses | `Addr` carries no validation once deserialized from JSON, so a bad address would surface only as a failed bank send on the first buy. |
| `TimeLock.time` must not exceed `MAX_TIME_LOCK_SECONDS` (ten years) | The hook computes `block.time + time_lock`; with `overflow-checks` on, an extreme value traps and takes the reserve path down with it. Matches the proxy's cap on approval expiry. |
| Denoms must match `[a-zA-Z][a-zA-Z0-9/:._-]{2,127}` | They are echoed into bank messages, which reject anything the SDK's own denom rule refuses. `cosmwasm_std` has no denom validator, so the rule is mirrored here. IBC and token-factory denoms satisfy it. |
| `AllowedMarketplaces` and `AllowedCurrencies` must be non-empty | Both hooks treat an empty list as *allow everything*, so storing one leaves a restriction that silently permits. Remove the plugin instead to express no restriction. |
| `NotBefore.time` may not be `Never` | `not_before_plugin` errors while the bound has *not* passed, and `Never` never passes, so every listing is blocked forever. |
| `NotAfter.time` may not be `Never` or already elapsed | `Never` never expires, so the bound never restricts anything; a bound already in the past blocks every listing from then on. |

### Removed and changed plugins

- **`RequiresProof` was removed.** Nothing ever enforced it: no hook loaded it, its run arm
  was empty, and no message carried a proof to check. A creator could set it and an
  integrator could read it back from `GetCollectionPlugins`, so it advertised a precondition
  that did not exist. A collection that already stored one keeps an inert row; no hook reads
  it, `GetCollectionPlugins` skips rows it cannot deserialize rather than failing, and
  `RemoveCollectionPlugin` still deletes it by name.
- **`ExactPrice` lost its `amount` field.** It previously overwrote the listing price with
  its own stored amount, which would have priced every item in a collection identically had
  it ever run. It is now a flag checked against the listing price. The old
  `{"exact_price":{"amount":...}}` wire form still deserializes, with the amount ignored.

Not validated, and worth knowing:

- **`Royalty { bps: 0 }` still disables raw transfers.** `is_transfer_enabled_plugin` keys off the royalty being configured at all, not its rate, so a zero-rate royalty collects nothing yet forces every movement through the buy flow. That may be deliberate; it is not rejected.
- **`TimeLock { 0 }` makes explicit reservations impossible.** The hook allows only `reserved_until <= now` while `reserve` requires `> now`, so no reservation can satisfy both.
- **Contradictions between separately saved plugins are not detected**, for example a `MinimumPrice` in a denom that `AllowedCurrencies` excludes, or a `NotBefore` at or after a `NotAfter`. Validation is per plugin, at save time.
- **Existing collections are not repaired.** Validation is on the write path only, so a collection that already stored a bad value keeps its behaviour until the creator overwrites the plugin or calls `RemoveCollectionPlugin`, neither of which runs the list/buy/reserve hooks. If the creator has renounced ownership, that state is permanent.

A batch is validated in full before any of it is written, so a rejected plugin cannot leave the
collection half configured.

## Using the Library

1. **Instantiate `cw721` normally**
   ```rust
   pub type InstantiateMsg = asset::msg::InstantiateMsg<MyCollectionExtension>;
   ```
   Use the standard `cw721` instantiate flow; the asset contract reuses `Cw721InstantiateMsg`.

2. **Embed the contract**
   ```rust
   use asset::traits::{AssetContract, DefaultAssetContract};

   pub struct AppContract {
       asset: DefaultAssetContract<'static, MyNftExtension, MyNftMsg, MyCollectionExtension, MyCollectionMsg>,
   }

   impl Default for AppContract {
       fn default() -> Self {
           Self { asset: AssetContract::default() }
       }
   }
   ```

3. **Expose execute entry points**
   ```rust
   use asset::traits::{PluggableAsset, SellableAsset};

   pub fn execute(
       deps: DepsMut,
       env: Env,
       info: MessageInfo,
       msg: asset::msg::ExecuteMsg<MyNftMsg, MyCollectionMsg, asset::msg::AssetExtensionExecuteMsg>,
   ) -> Result<Response, ContractError> {
       Ok(APP_CONTRACT.asset.execute_pluggable(deps, &env, &info, msg)?)
   }
   ```
   The `PluggableAsset` trait forwards marketplace operations to the relevant hooks and finally to
   the base `cw721` implementation.

4. **Dispatch marketplace operations**
   ```rust
   use asset::msg::AssetExtensionExecuteMsg;
   use cosmwasm_std::{Coin, to_json_binary, CosmosMsg};

   let list = CosmosMsg::Wasm(WasmMsg::Execute {
       contract_addr: collection_addr.into(),
       funds: vec![],
       msg: to_json_binary(&AssetExtensionExecuteMsg::List {
           token_id: "token-1".into(),
           price: Coin::new(1_000_000u128, "uxion"),
           reservation: None,
       })?,
   });
   ```
   Similar patterns apply for `Reserve`, `Buy` (attach payment funds), and `Delist` messages.

## Feature Flags

- `asset_base` (default): ships the standard marketplace + plugin behavior.
- `crossmint`: alternative configuration for cross-minting scenarios (mutually exclusive with
  `asset_base`). Ensure only one of these is enabled at a time.

## Testing

`src/test.rs` demonstrates how to wire mocks for the `SellableAsset` entry points and validate
plugin flows. Use it as a reference when building integration tests around custom plugin behavior.
