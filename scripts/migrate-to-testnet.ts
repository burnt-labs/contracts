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
const WASM_DIR = path.join(__dirname, '../wasm');

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

// Create wasm directory if it doesn't exist
if (!fs.existsSync(WASM_DIR)) {
    fs.mkdirSync(WASM_DIR, { recursive: true });
}

async function downloadWasm(codeId: number, contractName: string): Promise<Uint8Array> {
    const wasmPath = path.join(WASM_DIR, `${contractName}_${codeId}.wasm`);

    // Check if we already have the file
    if (fs.existsSync(wasmPath)) {
        console.log(`Using cached WASM file for ${contractName} (${codeId})`);
        return new Uint8Array(fs.readFileSync(wasmPath));
    }

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
        const wasmBinary = new Uint8Array(binaryString);

        // Save to file
        fs.writeFileSync(wasmPath, wasmBinary);
        console.log(`Saved WASM file to ${wasmPath}`);

        return wasmBinary;

    } catch (error) {
        console.error('Error downloading WASM:', error.response?.data || error.message);
        throw error;
    }
}

async function main() {
    // Read contracts.json
    const contractsData: ContractInfo[] = JSON.parse(fs.readFileSync(CONTRACTS_FILE, 'utf8'));
    
    // Process all contracts
    const contractsToProcess = contractsData;
    console.log(`Processing all contracts: ${contractsToProcess.map(c => c.name).join(', ')}`);

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

function updateReadme(contractsData: ContractInfo[]) {
    const readmePath = path.join(__dirname, '../README.md');
    let readmeContent = fs.readFileSync(readmePath, 'utf8');
    
    // Find the table in the README
    const tableRegex = /\|.*Name.*\|.*Code ID.*\|/;
    const tableStart = readmeContent.search(tableRegex);
    
    if (tableStart === -1) {
        console.log('Could not find contracts table in README');
        return;
    }

    // Get the existing table header line
    const headerEndIndex = readmeContent.indexOf('\n', tableStart);
    const existingHeader = readmeContent.slice(tableStart, headerEndIndex);
    
    // Insert the new column while preserving existing ones
    const headers = existingHeader.split('|');
    // Find the "Code ID" column index
    const codeIdIndex = headers.findIndex(h => h.includes('Code ID'));
    
    // Create new headers by inserting the testnet column after the Code ID column
    headers.splice(codeIdIndex + 1, 0, ' Code ID (Testnet) ');
    const newHeader = headers.join('|');

    // Create the separator line with the correct number of columns
    const separator = '|' + headers.map(() => '---').join('|') + '|';

    // Get existing rows
    const tableEndRegex = /\n\n/;
    const tableEnd = readmeContent.slice(tableStart).search(tableEndRegex) + tableStart;
    const existingRows = readmeContent
        .slice(headerEndIndex + 1, tableEnd)
        .trim()
        .split('\n');

    // Create new rows while preserving existing data
    const newRows = existingRows.map(row => {
        const columns = row.split('|');
        const contractName = columns[1].trim();
        const contract = contractsData.find(c => c.name === contractName);
        const testnetCodeId = contract?.testnet ? `\`${contract.testnet.code_id}\`` : '-';
        
        // Insert testnet code ID after mainnet code ID
        columns.splice(codeIdIndex + 1, 0, ` ${testnetCodeId} `);
        return columns.join('|');
    });

    // Combine all parts
    const newTableContent = [newHeader, separator, ...newRows].join('\n');

    // Replace old table with new one
    const newContent = readmeContent.slice(0, tableStart) + 
                      newTableContent + 
                      readmeContent.slice(tableEnd);
    
    fs.writeFileSync(readmePath, newContent);
    console.log('Updated README.md with testnet code IDs');
}

main()
    .then(() => {
        const contractsData = JSON.parse(fs.readFileSync(CONTRACTS_FILE, 'utf8'));
        updateReadme(contractsData);
    })
    .catch(console.error); 