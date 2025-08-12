# XION Contracts (Move)

Smart contract implementations for XION blockchain.

## Contract Registry Notice

**The XION deployed contracts registry has been moved to a dedicated repository:**

🔗 **[burnt-labs/deployed-contract-listings](https://github.com/burnt-labs/deployed-contract-listings)**

The new repository provides:
- Complete registry of all deployed XION contracts (mainnet and testnet)
- Interactive web interface at [https://burnt-labs.github.io/deployed-contract-listings/](https://burnt-labs.github.io/deployed-contract-listings/)
- Contract verification utilities
- Automated GitHub Pages deployment

Please update your bookmarks and references to use the new location.

## About This Repository

This repository contains the source code for smart contracts that can be deployed on XION, including:

### Contracts

- **Account**: MetaAccount implementation
- **Treasury**: Treasury management contract
- **User Map**: User mapping functionality

### Compiling

To compile the contracts, use the CosmWasm optimizer:

```bash
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.1
```

## Development

For contract development and deployment information, please refer to the individual contract directories.

## License

See LICENSE file for details.