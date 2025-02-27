/**
 * verify-code-ids.js
 * 
 * This utility verifies that the code IDs and their corresponding hashes in the local
 * contracts.json file match those deployed on the Xion mainnet.
 * 
 * Usage:
 *   1. Install dependencies: npm install
 *   2. Run the script: npm run verify
 * 
 * The script will:
 *   - Read the local contracts.json file
 *   - Fetch current contract data from Xion mainnet
 *   - Compare code IDs and hashes
 *   - Report any mismatches or discrepancies
 */

const fs = require('fs');
const path = require('path');

async function verifyCodeIds() {
    try {
        // Read local contracts.json
        const contractsPath = path.join(__dirname, '..', 'contracts.json');
        const localContracts = JSON.parse(fs.readFileSync(contractsPath, 'utf8'));

        // Create a map of local code_ids
        const localCodeIds = new Map();
        localContracts.forEach(contract => {
            localCodeIds.set(contract.code_id, {
                name: contract.name,
                hash: contract.hash
            });
        });

        // Fetch remote data
        console.log('Fetching data from Xion mainnet...');
        const response = await fetch('https://api.xion-mainnet-1.burnt.com/cosmwasm/wasm/v1/code');
        const remoteData = await response.json();

        // Compare code_ids
        console.log('\nVerifying code IDs...\n');
        let mismatches = [];

        remoteData.code_infos.forEach(info => {
            const codeId = info.code_id;
            const localInfo = localCodeIds.get(codeId);

            if (!localInfo) {
                mismatches.push(`Code ID ${codeId} exists on chain but not in contracts.json`);
            } else if (localInfo.hash !== info.data_hash.toUpperCase()) {
                mismatches.push(`Hash mismatch for Code ID ${codeId} (${localInfo.name}):
                    Local:  ${localInfo.hash}
                    Remote: ${info.data_hash.toUpperCase()}`);
            }
        });

        // Check for local codes that don't exist on chain
        localCodeIds.forEach((info, codeId) => {
            const exists = remoteData.code_infos.some(remote => remote.code_id === codeId);
            if (!exists) {
                mismatches.push(`Code ID ${codeId} (${info.name}) exists in contracts.json but not on chain`);
            }
        });

        // Report results
        if (mismatches.length === 0) {
            console.log('✅ All code IDs match successfully!');
        } else {
            console.log('❌ Found the following mismatches:');
            mismatches.forEach(mismatch => console.log(`- ${mismatch}`));
        }

    } catch (error) {
        console.error('Error during verification:', error);
        process.exit(1);
    }
}

verifyCodeIds();
