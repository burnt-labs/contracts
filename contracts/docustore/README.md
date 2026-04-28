# **CW Counter Starter Contract**

This is a basic CosmWasm smart contract that allows you to set a counter and then either **increment** or **reset** it. You can also query the current counter value.

---

## **Prerequisites**

Before deploying the contract, ensure you have the following:

1. **XION Daemon (`xiond`)**  
   Follow the official guide to install `xiond`:  
   [Interact with XION Chain: Setup XION Daemon](https://docs.burnt.com/xion/developers/featured-guides/setup-local-environment/interact-with-xion-chain-setup-xion-daemon)

2. **Docker**  
   Install and run [Docker](https://www.docker.com/get-started), as it is required to compile the contract.

---

## **Deploy and Interact with the Contract**

### **Step 1: Clone the Repository**
```sh
git clone https://github.com/burnt-labs/cw-counter
cd cw-counter
```

---

### **Step 2: Compile and Optimize the Wasm Bytecode**
Run the following command to compile and optimize the contract:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.0
```

> **Note:**  
> This step uses **CosmWasm's Optimizing Compiler**, which reduces the contract's binary size, making it more efficient for deployment.  
> Learn more [here](https://github.com/CosmWasm/optimizer).

The optimized contract will be stored as:
```
cw-counter/artifacts/cw_counter.wasm
```

---

### **Step 3: Upload the Bytecode to the Blockchain**
First, set your wallet address:
```sh
WALLET="your-wallet-address-here"
```

Now, upload the contract to the blockchain:
```sh
RES=$(xiond tx wasm store ./artifacts/cw_counter.wasm \
      --chain-id xion-testnet-1 \
      --gas-adjustment 1.3 \
      --gas-prices 0.1uxion \
      --gas auto \
      -y --output json \
      --node https://rpc.xion-testnet-1.burnt.com:443 \
      --from $WALLET)
```

After running the command, **extract the transaction hash**:
```sh
echo $RES
```

Example output:
```json
{
  "height": "0",
  "txhash": "B557242F3BBF2E68D228EBF6A792C3C617C8C8C984440405A578FBBB8A385035",
  ...
}
```

Copy the transaction hash for the next step.

---

### **Step 4: Retrieve the Code ID**
Set your transaction hash:
```sh
TXHASH="your-txhash-here"
```

Query the blockchain to get the **Code ID**:
```sh
CODE_ID=$(xiond query tx $TXHASH \
  --node https://rpc.xion-testnet-1.burnt.com:443 \
  --output json | jq -r '.events[-1].attributes[1].value')
```

Now, display the retrieved Code ID:
```sh
echo $CODE_ID
```

Example output:
```
1213
```

---

### **Step 5: Instantiate the Contract**
Set the contract's initialization message:
```sh
MSG='{ "count": 1 }'
```

Instantiate the contract with the **Code ID** from the previous step:
```sh
xiond tx wasm instantiate $CODE_ID "$MSG" \
  --from $WALLET \
  --label "cw-counter" \
  --gas-prices 0.025uxion \
  --gas auto \
  --gas-adjustment 1.3 \
  -y --no-admin \
  --chain-id xion-testnet-1 \
  --node https://rpc.xion-testnet-1.burnt.com:443
```

Example output:
```
gas estimate: 217976
code: 0
txhash: 09D48FE11BE8D8BD4FCE11D236D80D180E7ED7707186B1659F5BADC4EC116F30
```

Copy the new transaction hash for the next step.

---

### **Step 6: Retrieve the Contract Address**
Set the new transaction hash:
```sh
TXHASH="your-txhash-here"
```

Query the blockchain to get the **contract address**:
```sh
CONTRACT=$(xiond query tx $TXHASH \
  --node https://rpc.xion-testnet-1.burnt.com:443 \
  --output json | jq -r '.events[] | select(.type == "instantiate") | .attributes[] | select(.key == "_contract_address") | .value')
```

Display the contract address:
```sh
echo $CONTRACT
```

Example output:
```
xion1v6476wrjmw8fhsh20rl4h6jadeh5sdvlhrt8jyk2szrl3pdj4musyxj6gl
```
