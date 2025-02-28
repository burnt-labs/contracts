/**
 * verify-code-ids.js
 * 
 * This utility verifies that the code IDs and their corresponding hashes in the local
 * contracts.json file match those deployed on the Xion mainnet.
 * 
 * Usage:
 *   node scripts/verify-code-ids.js
 * 
 * The script will:
 *   - Read the local contracts.json file
 *   - Fetch current contract data from Xion mainnet
 *   - Compare code IDs and data hashes
 *   - Report any mismatches categorized as:
 *     1. Codes that exist on chain but not in contracts.json
 *     2. Codes that exist in contracts.json but not on chain
 *     3. Hash mismatches between local and remote data
 *   - Show a summary of total mismatches found
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
        console.log('\nVerifying code IDs and hashes...\n');
        let missingOnChain = [];
        let missingInJson = [];
        let hashMismatches = [];

        remoteData.code_infos.forEach(info => {
            const codeId = info.code_id;
            const localInfo = localCodeIds.get(codeId);

            if (!localInfo) {
                missingInJson.push({
                    codeId,
                    hash: info.data_hash.toUpperCase()
                });
            } else if (localInfo.hash !== info.data_hash.toUpperCase()) {
                hashMismatches.push({
                    codeId,
                    name: localInfo.name,
                    localHash: localInfo.hash,
                    remoteHash: info.data_hash.toUpperCase()
                });
            }
        });

        // Check for local codes that don't exist on chain
        localCodeIds.forEach((info, codeId) => {
            const exists = remoteData.code_infos.some(remote => remote.code_id === codeId);
            if (!exists) {
                missingOnChain.push({
                    codeId,
                    name: info.name
                });
            }
        });

        // Report results
        const totalMismatches = missingOnChain.length + missingInJson.length + hashMismatches.length;

        if (totalMismatches === 0) {
            console.log('✅ All code IDs and hashes match successfully!');
        } else {
            console.log('❌ Found the following mismatches:');

            if (missingInJson.length > 0) {
                console.log('\n📝 Codes that exist on chain but not in contracts.json:');
                missingInJson.forEach(({codeId, hash}) => {
                    console.log(`   Code ID ${codeId}:`);
                    console.log(`   Hash: ${hash}`);
                });
            }

            if (missingOnChain.length > 0) {
                console.log('\n🔍 Codes that exist in contracts.json but not on chain:');
                missingOnChain.forEach(({codeId, name}) => {
                    console.log(`   Code ID ${codeId} (${name})`);
                });
            }

            if (hashMismatches.length > 0) {
                console.log('\n⚠️  Hash mismatches:');
                hashMismatches.forEach(({codeId, name, localHash, remoteHash}) => {
                    console.log(`   Code ID ${codeId} (${name}):`);
                    console.log(`   Local:  ${localHash}`);
                    console.log(`   Remote: ${remoteHash}`);
                });
            }

            console.log(`\nTotal mismatches found: ${totalMismatches}`);
            process.exit(1);
        }

    } catch (error) {
        console.error('Error during verification:', error);
        process.exit(1);
    }
}

verifyCodeIds();
