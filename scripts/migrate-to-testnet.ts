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
    
    // Process only contracts that don't have testnet information
    const contractsToProcess = contractsData.filter(contract => !contract.testnet);
    console.log(`Processing contracts without testnet deployments: ${contractsToProcess.map(c => c.name).join(', ')}`);
    
    if (contractsToProcess.length === 0) {
        console.log('All contracts have already been migrated to testnet.');
        return 0;
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
    return contractsToProcess.length;
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
    
    // Check if testnet column already exists
    if (existingHeader.includes('Code ID (Testnet)')) {
        console.log('Testnet column already exists, updating values only');
        // Update existing rows without adding new column
        const tableEndRegex = /\n\n/;
        const tableEnd = readmeContent.slice(tableStart).search(tableEndRegex) + tableStart;
        const existingRows = readmeContent
            .slice(headerEndIndex + 1, tableEnd)
            .trim()
            .split('\n');

        const newRows = existingRows.map(row => {
            const columns = row.split('|');
            const contractName = columns[1].trim();
            const contract = contractsData.find(c => c.name === contractName);
            const testnetCodeId = contract?.testnet ? `\`${contract.testnet.code_id}\`` : '-';
            
            // Find and update the testnet code ID column
            const testnetColumnIndex = columns.findIndex(col => col.includes('Code ID (Testnet)'));
            if (testnetColumnIndex !== -1) {
                columns[testnetColumnIndex] = ` ${testnetCodeId} `;
            }
            return columns.join('|');
        });

        // Combine all parts
        const newTableContent = [existingHeader, existingRows[0], ...newRows.slice(1)].join('\n');

        // Replace old table with new one
        const newContent = readmeContent.slice(0, tableStart) + 
                          newTableContent + 
                          readmeContent.slice(tableEnd);
        
        fs.writeFileSync(readmePath, newContent);
        console.log('Updated README.md with new testnet code IDs');
        return;
    }

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
    console.log('Added testnet column and updated README.md with testnet code IDs');
}

main()
    .then((processedCount) => {
        const contractsData = JSON.parse(fs.readFileSync(CONTRACTS_FILE, 'utf8'));
        // Only update README if new migrations happened
        if (processedCount > 0) {
            updateReadme(contractsData);
        }
    })
    .catch(console.error); 