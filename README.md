<!-- AUTOGENERATED FILE DO NOT EDIT -->

# XION Mainnet Contracts

Contract information for XION mainnet

## Development

### Updating Documentation

The README is automatically generated from `contracts.json`. To update it:

1. Ensure you have the required dependencies:
   - Node.js: https://nodejs.org/
   - jq: `brew install jq` (macOS) or `apt-get install jq` (Ubuntu/Debian)

2. Modify `contracts.json` with your changes
   - The script will validate the JSON format and required fields
   - Each contract must include: name, description, code_id, hash, release info, and author details

3. Run the convert script:
```bash
./convert.sh
```

4. Commit both the `contracts.json` and generated `README.md` changes
   - The CI will validate both files are in sync during pull requests
   - Pull requests with manual README edits will be rejected

### Compiling

```bash

docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.1

```

## Active Contracts
| Name | Description | Release | Author | Code ID | Hash | Governance Proposal |
|:-----|:------------|:--------|:-------|:--------|:-----|:-------------------|
| Polytone Proxy | ICA Proxy, allows controlled execution of messages. | [v1.1.0](https://github.com/DA0-DA0/polytone/releases/tag/v1.1.0) | [DAO DAO](https://daodao.zone/) | `2` | `54E909B7F9AB191A0A0DB2040E09C8CFAB45DB75CA22852098531EC301878FC2` | Genesis |
| Polytone Voice | Receiver of messages over IBC, executes on the destination chain. Maintains access control through the proxy (see above). | [v1.1.0](https://github.com/DA0-DA0/polytone/releases/tag/v1.1.0) | [DAO DAO](https://daodao.zone/) | `3` | `3AA8F962BADEB899DB4BC6E5931C852473B5719DBA5AFF5DC26C66CDE1ED250E` | Genesis |
| Polytone Note | Sends messages to be executed on other chains over IBC. Handles channel management and packet routing. | [v1.1.0](https://github.com/DA0-DA0/polytone/releases/tag/v1.1.0) | [DAO DAO](https://daodao.zone/) | `4` | `CD13C487B820CE79BC7932F41497274635477845C2DCAF5CD4B06332175F53EC` | Genesis |
| MetaAccount (v2) | Second version of Xion's MetaAccount implementation | [pr40](https://github.com/burnt-labs/contracts/pull/40) | [Burnt Labs](https://burnt.com) | `5` | `FEFA4D0C57F6CA47A5D89C6F077A176D26027DB4EEFA758A929DD4C4AAF17D1B` | Genesis |
| cw1 Subkeys | A Proxy contract that extends the functionality of cw1-whitelist. Allows admins to grant allowances and set permissions to 'subkeys' | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `6` | `0DEE80010FB15A7A03FC1153389DC1EEC36482B8D872B0640B8762C14E5C3CF8` | Genesis |
| cw1 Whitelist | Proxy contract maintaining a list of admin addresses that can execute messages through it. Admin list defined at contract creation. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `7` | `86C0008909BEB14147FA99F66CA1AFB674FDCD737CCAD89C47EA2C95966F747E` | Genesis |
| cw3 Fixed Multisig | Implements a multisig wallet with a fixed set of voters defined at instantiation, each voter can have different voting weights. Allows voters to create, vote on, and execute proposals containing arbitrary messages. Supports different voting thresholds & configurable voting periods. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `8` | `734A8B5B958D9F3A9D97CAAEA93AAE409BD7FF21648B35B3F9A40F6DF0C39C00` | Genesis |
| cw3 Flex Multisig | An advanced multisig using a separate cw4 (group) contract to manage its voter set, allowing multiple multisigs to share the same group of voters with different voting thresholds. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `9` | `8047BC30ED7129F24D4A89E7527C4926D3363A6BA038830A592A2041301553CF` | Genesis |
| cw4 Group | Manages group membership with weighted voting power. Maintains a list of members, controlled by an admin with rights to add or remove members. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `10` | `4604A284E209C2FE320F223B9FD29805A0E8F2CF8EA7B01FAC28C3EFC4EE63F0` | Genesis |
| cw4 Stake | Determines group membership and voting weights based on the amount of tokens (native or cw20) that users have staked, with configurable parameters like minimum bond amount and tokens-per-weight ratio. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `12` | `DCA8257AD67CCB15B4A61A882131B9D3FDD0DD178B121BB51BBDA35B682C6653` | Genesis |
| cw20 Base | Implementation of the CW20 token standard in CosmWasm. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `13` | `4D8E90DD340993033F1B9E8E3A3EE7F8673C582CA9BCDD8C8CF3C7470D6537D5` | Genesis |
| cw20 ics20 | Enables CW20 tokens to be sent over IBC using the ICS20 protocol, allowing custom CW20 tokens from one chain to be used like native tokens on other chains. | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `14` | `A63BA1554095B6AC04D2F08246ABCCFA7F1C2276BF19F52A943EE5B85FD7749B` | Genesis |
| Treasury | Treasury | [v0.1.0](https://github.com/burnt-labs/contracts/commit/8224140b66da51fcdef25227a195d2dee16cc422) | [Burnt Labs](https://burnt.com) | `15` | `6A30325831651208E07F9A9F6FE5B29ADD99D6EDBDF5601C4AF9856D287E56E6` | Genesis |
| Abstract Account | Abstract Account | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `16` | `D3D18E16A185FD5D82A510D2D51E8849E1135A1EF23090738ED90CE1F2E377DA` | Genesis |
| Abstract Acccount (XION) | Abstract Acccount (XION) | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `17` | `A46BECDFECDECF94837B3D424826E78A483AF4F1E248EA378BFD5D702C5761AD` | Genesis |
| Abstract ANS Host | Abstract ANS Host | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `18` | `B34F0DF05BAC1D769A87389B7856554751B5608D485943E98BC526A4C3322ADB` | Genesis |
| Abstract IBC Client | Abstract IBC Client | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `19` | `919A489D744E4384FDC9E3706AA7C37E80A25D39083FF028956BA300AD9AC2E8` | Genesis |
| Abstract IBC Host | Abstract IBC Host | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `20` | `1FA61DFAE0CF886FEB9EA6A5AFFAA84F478781B243D57B2CBFBDB01F9395AF5B` | Genesis |
| Abstract ICA Client | Abstract ICA Client | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `21` | `E23F28815FB7673BA3C78AA81E2C738F648A7A610111341E60A1B29E2306B8E3` | Genesis |
| Abstract Module Factory | Abstract Module Factory | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `22` | `9B287AFE1380CB886B161C66582255BF03DAD5EACFD27DF24FE89ADB81D2436D` | Genesis |
| Abstract Registry | Abstract Registry | [v0.25.0](https://github.com/AbstractSDK/abstract/releases/tag/v0.25.0) | [Abstract Money](https://abstract.money/) | `23` | `647047E79FEAF28D36A49372877703555C80F5B45B18C9ADB8BBBCFBCA421CD5` | Genesis |
| Multiquery | Multiquery | [ae6b422](https://github.com/AbstractSDK/multiquery/commit/ae6b4225c9a3086a4f353522f5b03343138b16e1) | [Abstract Money](https://abstract.money/) | `24` | `C3282C016874B7FE7F4127F0695D42003C92EBA1C1BB10CC16BC584BAB186205` | Genesis |
| cw721 Base | cw721 Base | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `25` | `E13AA30E0D70EA895B294AD1BC809950E60FE081B322B1657F75B67BE6021B1C` | Genesis |
| cw721 Expiration | cw721 Expiration | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `26` | `EC8FE99C35618D786C6DC5F83293FC37CD98C4A297CF6AA9D150F64941E6442D` | Genesis |
| cw721 Fixed Price | cw721 Fixed Price | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `27` | `A58EE79215200778768FE3862F7C995B1BE35FBF3AB34C2DE715E5B9D77DCCBB` | Genesis |
| cw721 Metadata Onchain | cw721 Metadata Onchain | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `28` | `51A70227FF5DC29C38DC514B0F32BB474ECB82FFFA3C029C6789578A55925143` | Genesis |
| cw721 Non-Transferable | cw721 Non-Transferable | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `29` | `68D5DB29833B0C25A1DD4C8D837038528E521EF3622D9945FFCB0B70676FCABE` | Genesis |
| cw721 Receiver Tester | cw721 Receiver Tester | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `30` | `FEBB507E5FDA85C8C450CF28DCFBCBFB1BF17DECE43B7B7ECAD14D2FAD20C828` | Genesis |
| cw2981 Royalties | cw2981 Royalties | [v0.19.0](https://github.com/public-awesome/cw-nfts/releases/tag/v0.19.0) | [CosmWasm](https://cosmwasm.com/) | `31` | `5BC7CE4A04A747FAFD1A139F2DB73E7EAC094C6D3882AF8E055D15FFD3EE67E8` | Genesis |
| Mercle Mint with Claim | Mercle Mint with Claim | [18ceaf7](https://github.com/mercledao/MercleCosmwasmContracts/commit/18ceaf7e1a57a1dbf189da6e3a173618d4ea64fa) | [Mercle](https://mercle.xyz/) | `32` | `E1472FCB9275B908A931A1EA789AA8232EDF275D2EFEA05736BB786180CA91A1` | Genesis |
| Mercle NFT Membership | Mercle NFT Membership | [18ceaf7](https://github.com/mercledao/MercleCosmwasmContracts/commit/18ceaf7e1a57a1dbf189da6e3a173618d4ea64fa) | [Mercle](https://mercle.xyz/) | `33` | `B8998FEF98FBC7DE80437E41D4F2372CC471237F2D3F0A94F151B195C3418A33` | Genesis |
| BonusBlock Badge Minter | BonusBlock Badge Minter | [202538d](https://github.com/BBlockLabs/BonusBlock-Minter-SC-Rust/commit/202538de73d52f0ff66a8e2abb9baaad4ee98053) | [BonusBlock](https://www.bonusblock.io/) | `34` | `933AF6AB10A1024CBC0627C4E31DD87FC37F4C70A76C6C4DE9DB06FBFE229DEF` | Genesis |
| Talis Collection Offer | Talis Collection Offer | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `35` | `8524BEE102B7E1B6C85D0ED1DE7C47EA9B7AA2B51845D5DFDAB1EA645599B4DD` | Genesis |
| Talis English Auction | Talis English Auction | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `36` | `36FABE3758D19F4285C0B503579FABB06D702B09BCE74CD0FF8AB8987152EE36` | Genesis |
| Talis Marketplace | Talis Marketplace | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `37` | `0B9D0EF7D459A86062A77924EE440ECAC9A0BF21F6A201126032A353C1E19CCF` | Genesis |
| Talis Multi-Flavor | Talis Multi-Flavor | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `38` | `5D6EB094E88C8BE613570C612D74951BA9D5BDBD0D772B8987E68AE62D30B9DD` | Genesis |
| Talis PoC Candy Mint | Talis PoC Candy Mint | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `40` | `811E37F714B7229BB6731A98EDA48CF4A2438E3CEA4B8BFC31B7F7CAF277B0FD` | Genesis |
| Talis Xion Proxy | Talis Xion Proxy | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `41` | `21C147C2EC45190437367407B22D18717E137722EA3C8C4F410C05E55C403A57` | Genesis |
| Astroport Factory | Astroport Factory | [v5.7.0](https://github.com/astroport-fi/astroport-core/releases/tag/v5.7.0) | [Astroport](https://astroport.fi/) | `42` | `56EA99FB759B2DF28D18A0B1CFCCD4A0CACBADA3E7254DC2842D188277727CFB` | Genesis |
| Astroport Maker | Astroport Maker | [v5.7.0](https://github.com/astroport-fi/astroport-core/releases/tag/v5.7.0) | [Astroport](https://astroport.fi/) | `43` | `ADEFC4EE995B783BC45B0C338A6299A03FDEADB1F69CED4C2B6F22AF07B9EC1A` | Genesis |
| Astroport Native Coin Registry | Astroport Native Coin Registry | [v5.7.0](https://github.com/astroport-fi/astroport-core/releases/tag/v5.7.0) | [Astroport](https://astroport.fi/) | `44` | `2958D95914D24E4856D10877C38740B955C760F86D2B082EDCF19691809D378E` | Genesis |
| Astroport Pair | Astroport Pair | [v5.7.0](https://github.com/astroport-fi/astroport-core/releases/tag/v5.7.0) | [Astroport](https://astroport.fi/) | `45` | `88C14F95C3BCBB0B8AABC433DC28F49373FD25EAB7141A881AC310BE4B04979D` | Genesis |
| Astroport Router | Astroport Router | [v5.7.0](https://github.com/astroport-fi/astroport-core/releases/tag/v5.7.0) | [Astroport](https://astroport.fi/) | `46` | `6FEF673B1318C84AC8AB2CA81B2CDCA96B8C8C9D0995B8038D919F539AE7C3CC` | Genesis |
| Astroport TokenFactory Tracker | Astroport TokenFactory Tracker | [v5.7.0](https://github.com/astroport-fi/astroport-core/releases/tag/v5.7.0) | [Astroport](https://astroport.fi/) | `47` | `B0C14C860F1473B007A734DCC4ADBA1D3B52CECC660465670033F6E875014318` | Genesis |
| Talis Whitelist | Talis Whitelist | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `48` | `37360FDE0EE1384AA56781B0D05D4B0187843AE96335158DFEAD7806106DE779` | [18](https://www.mintscan.io/xion/proposals/18) |
| Talis Staking | Talis Staking | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `49` | `9C9047420A5B870D490585C753FFB46C97E310A55E9FCF50EA784BAFC2A701FD` | [18](https://www.mintscan.io/xion/proposals/18) |
| Talis Frens Proxy | Talis Frens Proxy | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `50` | `919FF797B7F35A40B5D32B506C2C05918DAA1C42B89C6864425F2047B5BC19F7` | [18](https://www.mintscan.io/xion/proposals/18) |
| Talis Trading Escrow | Talis Trading Escrow | [f084f5b](https://github.com/Talis-Art/talis_contracts_v2/commit/f084f5b7950f911b16090dfd38e1c06a177a1da8) | [Talis](https://talis.art/) | `51` | `01AA4D93B63871DE8E94B35FECAA0E586C8B4824A8B0EE833416303796B256E2` | [18](https://www.mintscan.io/xion/proposals/18) |
| Fractit | A protocol for the fractional ownership of NFTs | [b62e2bc8aa7646e73bceb07af67137c48c5a7488](https://github.com/Fractit/fractible_xion) | [Fractit](https://fractit.com) | `52` | `F6D5ADDC062B5B45BCA207EAF49B8D2736A2D7F86956FCE4D5176E3D07C91980` | [21](https://www.mintscan.io/xion/proposals/21) |
| Fractit Inception Pass | Protocol for managing Fractit Inception Pass NFTs | [2eb952db4592ea3965070c1199117927319f433b](https://github.com/Fractit/fractible_xion) | [Fractit](https://fractit.com) | `53` | `D45A22411A5C430A2C74248A20391618F5E0ECDD1BDC579C87493823E242663F` | [22](https://www.mintscan.io/xion/proposals/22) |
| Thrive Protocol | First Thrive Protocol contract implementation on XION, enabling to distribute rewards achieved in Thrive XION | [cd3f0e36d50f06b4f6a04511ff67bfb7b515829b](https://github.com/ThriveCoin/tp-xion-reward-contract-rs) | [Thrive Protocol](https://thriveprotocol.com) | `54` | `BB2FAC1091B93026A0CD57AE40E814A916D9CFC4A7B4F23F1E49A28D7ABEF286` | [24](https://www.mintscan.io/xion/proposals/24) |
| MetaAccount (v3) | Third version of Xion's MetaAccount implementation | [98fb64c9c4d917ba9e4b223b64558a2fd4c09ac7](https://github.com/burnt-labs/contracts) | [Burnt Labs](https://burnt.com) | `55` | `6FD7AA76AA9ED8E6F55D16093EE64611CCFB9743AC5A07B71AD4ACB342AF0EBF` | [26](https://www.mintscan.io/xion/proposals/26) |

## Deprecated Contracts
| Name | Description | Release | Author | Code ID | Hash | Governance Proposal |
|:-----|:------------|:--------|:-------|:--------|:-----|:-------------------|
| MetaAccount (v1.0.0) | Initial version of MetaAccount implementation, superseded by v2 | [v1.0.0](https://github.com/burnt-labs/contracts/releases/tag/v1.0.0) | [Burnt Labs](https://burnt.com) | `1` | `5E0F49F9686FAD66C132031EC6A43EC63AD84A2B6C8A35C555542AC84FC03708` | Genesis |
| cw4 Stake | cw4 Stake | [v2.0.0](https://github.com/CosmWasm/cw-plus/releases/tag/v2.0.0) | [CosmWasm](https://cosmwasm.com/) | `11` | `DCA8257AD67CCB15B4A61A882131B9D3FDD0DD178B121BB51BBDA35B682C6653` | Genesis |

## Utilities

### Code ID Verification

The repository includes a utility to verify that the code IDs and their corresponding hashes in the local `contracts.json` file match those deployed on the Xion mainnet.

#### Prerequisites
- Node.js 18 or higher: https://nodejs.org/

#### Usage
To verify code IDs:
```bash
node scripts/verify-contracts.js
```

The utility will:
1. Read the local contracts.json file
2. Fetch current contract data from Xion mainnet
3. Compare code IDs and hashes
4. Report any mismatches or discrepancies

This helps ensure that the contract information in this repository accurately reflects what's deployed on the Xion mainnet.
