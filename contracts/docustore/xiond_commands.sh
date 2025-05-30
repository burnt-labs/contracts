#!/bin/bash
# Xiond contract interaction commands
# Contract address: xion1svpts9q2ml4ahgc4tuu95w8cqzv988s6mf5mupt5kt56gvdnklks9hzar4
# Node: https://rpc.xion-testnet-2.burnt.com:443
# Chain ID: xion-testnet-2
# Broadcast mode: sync

CONTRACT=xion1svpts9q2ml4ahgc4tuu95w8cqzv988s6mf5mupt5kt56gvdnklks9hzar4
NODE=https://rpc.xion-testnet-2.burnt.com:443
CHAIN_ID=xion-testnet-2
KEY=<your-key>
USER=<user-xion-address>

# Set Document
xiond tx wasm execute $CONTRACT '{"Set":{"collection":"mycol","document":"doc1","data":"{\"foo\":\"bar\"}"}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Update Document
xiond tx wasm execute $CONTRACT '{"Update":{"collection":"mycol","document":"doc1","data":"{\"foo\":\"baz\"}"}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Delete Document
xiond tx wasm execute $CONTRACT '{"Delete":{"collection":"mycol","document":"doc1"}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Set Collection Permissions
xiond tx wasm execute $CONTRACT '{"SetCollectionPermissions":{"collection":"mycol","permissions":{"create":{"Anyone":{}},"update":{"Anyone":{}},"delete":{"AdminOnly":{}},"read":{"Anyone":{}}}}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Grant Role
xiond tx wasm execute $CONTRACT '{"GrantRole":{"user":"'$USER'","role":"editor"}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Revoke Role
xiond tx wasm execute $CONTRACT '{"RevokeRole":{"user":"'$USER'","role":"editor"}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Transfer Admin
xiond tx wasm execute $CONTRACT '{"TransferAdmin":{"new_admin":"'$USER'"}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Batch Write
xiond tx wasm execute $CONTRACT '{"BatchWrite":{"operations":[{"collection":"mycol","document":"doc2","operation":{"Set":{"data":"{\"foo\":\"bar2\"}"}}}]}}' \
  --from $KEY --gas auto --gas-adjustment 1.3 --gas-prices 0.025uxion --broadcast-mode sync --chain-id $CHAIN_ID --node $NODE

# Queries

# Get Document
xiond query wasm contract-state smart $CONTRACT '{"Get":{"collection":"mycol","document":"doc1"}}' --node $NODE

# List Collection
xiond query wasm contract-state smart $CONTRACT '{"Collection":{"collection":"mycol","limit":10}}' --node $NODE

# User Documents
xiond query wasm contract-state smart $CONTRACT '{"UserDocuments":{"owner":"'$USER'","collection":"mycol","limit":10}}' --node $NODE

# Get Collection Permissions
xiond query wasm contract-state smart $CONTRACT '{"GetCollectionPermissions":{"collection":"mycol"}}' --node $NODE

# Get User Roles
xiond query wasm contract-state smart $CONTRACT '{"GetUserRoles":{"user":"'$USER'"}}' --node $NODE

# Check Permission
xiond query wasm contract-state smart $CONTRACT '{"CheckPermission":{"collection":"mycol","user":"'$USER'","action":"create"}}' --node $NODE 