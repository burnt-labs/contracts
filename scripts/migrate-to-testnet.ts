require('dotenv').config();
const { DirectSecp256k1HdWallet } = require("@cosmjs/proto-signing");
const { SigningCosmWasmClient } = require("@cosmjs/cosmwasm-stargate");
const { GasPrice } = require("@cosmjs/stargate");
const fs = require('fs');
const axios = require('axios');
const path = require('path');

// Configuration
const MAINNET_API = "https://api.xion-mainnet-1.burnt.com";
const MAINNET_RPC = "https://rpc.xion-mainnet-1.burnt.com:443";
const TESTNET_RPC = "https://rpc.xion-testnet-2.burnt.com:443";
const CONTRACTS_FILE = path.join(__dirname, '../contracts.json');

interface ContractInfo {
    name: string;
    description: string;
    code_id: string;
    hash: string;
    release: {
        url: string;
        version: string;
    };
    author: {
        name: string;
        url: string;
    };
    governance: string;
    deprecated: boolean;
    testnet?: {
        code_id: string;
        hash: string;
        network: string;
        deployed_by: string;
        deployed_at: string;
    };
}

async function downloadWasm(codeId: number, contractName: string): Promise<Uint8Array> {
    try {
        const response = await axios.get(
            `${MAINNET_API}/cosmwasm/wasm/v1/code/${codeId}`,
            { 
                headers: {
                    'Accept': 'application/json'
                }
            }
        );
        
        console.log(`Retrieved code info for ${contractName} (ID: ${codeId})`);
        
        // The data field contains the base64-encoded WASM
        const base64Wasm = response.data.data;
        if (!base64Wasm) {
            throw new Error(`No WASM data found for code ID ${codeId}`);
        }

        // Convert base64 to Uint8Array
        const binaryString = Buffer.from(base64Wasm, 'base64');
        return new Uint8Array(binaryString);

    } catch (error) {
        console.error('Error downloading WASM:', error.response?.data || error.message);
        throw error;
    }
}

async function main() {
    // Read contracts.json
    const contractsData: ContractInfo[] = JSON.parse(fs.readFileSync(CONTRACTS_FILE, 'utf8'));
    
    // Process only non-deprecated contracts that don't have testnet information
    const contractsToProcess = contractsData.filter(contract => !contract.testnet && !contract.deprecated);
    console.log(`Processing non-deprecated contracts without testnet deployments: ${contractsToProcess.map(c => c.name).join(', ')}`);
    
    if (contractsToProcess.length === 0) {
        console.log('All contracts have already been migrated to testnet.');
        return;
    }

    // Get wallet from mnemonic
    const mnemonic = process.env.MNEMONIC;
    if (!mnemonic) {
        throw new Error("Please set MNEMONIC environment variable");
    }

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
        prefix: "xion",
    });
    const [account] = await wallet.getAccounts();

    // Connect to mainnet to get code IDs
    const mainnetClient = await SigningCosmWasmClient.connect(MAINNET_RPC);
    
    // Connect to testnet for deployment with gas price
    const gasPrice = GasPrice.fromString('0.025uxion');
    const testnetClient = await SigningCosmWasmClient.connectWithSigner(
        TESTNET_RPC,
        wallet,
        { gasPrice }
    );

    console.log(`Starting migration with address: ${account.address}`);

    // Process each contract
    for (const contract of contractsToProcess) {
        try {
            console.log(`Processing contract: ${contract.name}`);
            const codeId = parseInt(contract.code_id);
            
            console.log(`Downloading WASM for code ID: ${codeId}`);
            const wasmBinary = await downloadWasm(codeId, contract.name);

            // Store code on testnet
            console.log('Storing code on testnet...');
            const storeResult = await testnetClient.upload(
                account.address,
                wasmBinary,
                'auto',
                `Upload ${contract.name}`
            );
            
            // Update contract with testnet information
            contract.testnet = {
                code_id: storeResult.codeId.toString(),
                hash: storeResult.transactionHash,
                network: "xion-testnet-2",
                deployed_by: account.address,
                deployed_at: new Date().toISOString()
            };

            // Save updated contracts
            fs.writeFileSync(
                CONTRACTS_FILE,
                JSON.stringify(contractsData, null, 2)
            );
            
            console.log(`Code stored on testnet with ID: ${storeResult.codeId}`);
            console.log(`Transaction hash: ${storeResult.transactionHash}`);
            console.log(`Migration completed for ${contract.name}`);
            console.log('------------------------');

        } catch (error) {
            console.error(`Error processing contract ${contract.name}:`, error);
        }
    }
}

main().catch(console.error); 