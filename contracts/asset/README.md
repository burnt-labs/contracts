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

- Price guards (`ExactPrice`, `MinimumPrice`).
- Temporal restrictions (`NotBefore`, `NotAfter`, `TimeLock`).
- Access control (`AllowedMarketplaces`, `RequiresProof`).
- Currency allow-listing (`AllowedCurrencies`).
- Royalty payouts (`Royalty`).

The provided `default_plugins` module contains ready-to-use helpers that enforce the relevant rules
and can enqueue `BankMsg::Send` payouts (e.g., royalties) or raise errors to abort the action.
Register plugins per collection with `AssetConfig::collection_plugins` and they will be invoked by
`execute_pluggable` automatically.

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
