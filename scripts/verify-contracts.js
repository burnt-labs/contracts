/**
 * verify-contracts.js
 * 
 * This utility verifies:
 * 1. Code IDs and their corresponding hashes in the local contracts.json file 
 *    match those deployed on the Xion mainnet
 * 2. All governance proposals with store code messages have corresponding entries
 *    in contracts.json
 * 3. Cross-references missing code IDs with their originating proposals
 * 4. Verifies Genesis contracts were not deployed through proposals
 * 
 * Usage:
 *   node scripts/verify-contracts.js
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const zlib = require('zlib');

// Function to calculate SHA256 hash of wasm byte code
function calculateWasmHash(base64WasmCode) {
    try {
        const wasmBuffer = Buffer.from(base64WasmCode, 'base64');
        
        // Try to gunzip the buffer
        try {
            const unzippedBuffer = zlib.gunzipSync(wasmBuffer);
            // If successful, calculate hash from unzipped data
            const hash = crypto.createHash('sha256')
                .update(unzippedBuffer)
                .digest('hex')
                .toUpperCase();
            return hash;
        } catch (gzipError) {
            // If gunzip fails, assume it's regular base64 encoded wasm
            const hash = crypto.createHash('sha256')
                .update(wasmBuffer)
                .digest('hex')
                .toUpperCase();
            return hash;
        }
    } catch (error) {
        console.error('Error calculating wasm hash:', error);
        return null;
    }
}

// Helper function to get human readable status
function getStatusString(status) {
    const statusMap = {
        'PROPOSAL_STATUS_UNSPECIFIED': 'Unspecified',
        'PROPOSAL_STATUS_DEPOSIT_PERIOD': 'Deposit Period',
        'PROPOSAL_STATUS_VOTING_PERIOD': 'Voting Period',
        'PROPOSAL_STATUS_PASSED': 'Passed',
        'PROPOSAL_STATUS_REJECTED': 'Rejected',
        'PROPOSAL_STATUS_FAILED': 'Failed'
    };
    return statusMap[status] || status;
}

async function fetchAllProposals() {
    console.log('Fetching proposals from Xion mainnet...');
    const response = await fetch('https://api.xion-mainnet-1.burnt.com/cosmos/gov/v1/proposals?proposal_status=0');
    if (!response.ok) {
        console.error('Failed to fetch proposals:', response.status, response.statusText);
        return [];
    }
    const proposalsData = await response.json();
    return proposalsData.proposals || [];
}

async function verifyContracts() {
    try {
        // Read local contracts.json
        const contractsPath = path.join(__dirname, '..', 'contracts.json');
        const localContracts = JSON.parse(fs.readFileSync(contractsPath, 'utf8'));

        // Create maps for local contracts
        const localCodeIds = new Map();
        const localHashes = new Map();
        const genesisContracts = new Map(); // Track Genesis contracts
        localContracts.forEach(contract => {
            const isGenesis = contract.governance === 'Genesis';
            localCodeIds.set(contract.code_id, {
                name: contract.name,
                hash: contract.hash,
                governance: contract.governance,
                isGenesis
            });
            localHashes.set(contract.hash, {
                name: contract.name,
                code_id: contract.code_id,
                governance: contract.governance,
                isGenesis
            });
            if (isGenesis) {
                genesisContracts.set(contract.code_id, {
                    name: contract.name,
                    governance: contract.governance
                });
            }
        });

        // Fetch all proposals
        const proposals = await fetchAllProposals();

        // Create a map of proposal hashes to their details
        const proposalHashMap = new Map();
        let analyzedProposals = 0;
        let storeCodeProposals = 0;
        let totalStoreCodeMessages = 0;

        // Track which contracts were uploaded by which proposals
        const uploadedByProposal = new Map(); // code_id -> proposal info

        proposals.forEach(proposal => {
            analyzedProposals++;
            if (proposal.messages) {
                const storeCodeMessages = proposal.messages.filter((msg, idx) => {
                    if (msg['@type'] === '/cosmwasm.wasm.v1.MsgStoreCode') {
                        const hash = calculateWasmHash(msg.wasm_byte_code);
                        if (hash) {
                            const proposalInfo = {
                                proposalId: proposal.id,
                                proposalTitle: proposal.title,
                                status: getStatusString(proposal.status),
                                messageIndex: idx + 1,
                                totalMessages: proposal.messages.length,
                                hash
                            };
                            proposalHashMap.set(hash, proposalInfo);

                            // Find if this hash matches any code ID in contracts.json
                            for (const [codeId, info] of localCodeIds) {
                                if (info.hash === hash) {
                                    uploadedByProposal.set(codeId, proposalInfo);
                                    break;
                                }
                            }

                            totalStoreCodeMessages++;
                            return true;
                        }
                    }
                    return false;
                });
                if (storeCodeMessages.length > 0) {
                    storeCodeProposals++;
                }
            }
        });

        // Fetch chain data
        console.log('Fetching code data from Xion mainnet...');
        const response = await fetch('https://api.xion-mainnet-1.burnt.com/cosmwasm/wasm/v1/code');
        const chainData = await response.json();

        // Analyze discrepancies
        const discrepancies = {
            missingFromJson: [], // Exists on chain but not in contracts.json
            missingFromChain: [], // Exists in contracts.json but not on chain
            hashMismatches: [],   // Hash mismatches between chain and contracts.json
            missingFromBoth: [],   // Found in proposals but not in chain or contracts.json
            genesisWithProposal: [] // Genesis contracts that have an associated proposal
        };

        // Check each contract in contracts.json
        localCodeIds.forEach((info, codeId) => {
            // If this is a Genesis contract (based on governance field)
            if (info.isGenesis) {
                const proposalInfo = uploadedByProposal.get(codeId);
                if (proposalInfo) {
                    discrepancies.genesisWithProposal.push({
                        codeId,
                        name: info.name,
                        governance: info.governance,
                        hash: info.hash,
                        proposal: proposalInfo
                    });
                }
            }
        });

        // Check chain codes against contracts.json
        chainData.code_infos.forEach(info => {
            const codeId = info.code_id;
            const chainHash = info.data_hash.toUpperCase();
            const localInfo = localCodeIds.get(codeId);
            const proposalInfo = proposalHashMap.get(chainHash);

            if (!localInfo) {
                discrepancies.missingFromJson.push({
                    codeId,
                    chainHash,
                    proposal: proposalInfo
                });
            } else if (localInfo.hash !== chainHash) {
                discrepancies.hashMismatches.push({
                    codeId,
                    name: localInfo.name,
                    localHash: localInfo.hash,
                    chainHash,
                    proposal: proposalInfo
                });
            }
        });

        // Check contracts.json codes against chain
        localCodeIds.forEach((info, codeId) => {
            const exists = chainData.code_infos.some(remote => remote.code_id === codeId);
            if (!exists) {
                discrepancies.missingFromChain.push({
                    codeId,
                    name: info.name,
                    hash: info.hash,
                    proposal: proposalHashMap.get(info.hash)
                });
            }
        });

        // Check for proposals with hashes missing from both chain and contracts.json
        proposalHashMap.forEach((info, hash) => {
            const existsInChain = chainData.code_infos.some(code => code.data_hash.toUpperCase() === hash);
            const existsInJson = localHashes.has(hash);
            
            if (!existsInChain && !existsInJson) {
                discrepancies.missingFromBoth.push({
                    hash,
                    proposal: info
                });
            }
        });

        // Print unified report
        console.log('\n📊 Analysis Summary:');
        console.log(`   Total contracts in contracts.json: ${localContracts.length}`);
        console.log(`   Genesis contracts: ${genesisContracts.size}`);
        console.log(`   Total code IDs on chain: ${chainData.code_infos.length}`);
        console.log(`   Total proposals analyzed: ${analyzedProposals}`);
        console.log(`   Proposals with store code: ${storeCodeProposals}`);
        console.log(`   Total store code messages: ${totalStoreCodeMessages}\n`);

        const hasDiscrepancies = Object.values(discrepancies).some(arr => arr.length > 0);
        
        if (!hasDiscrepancies) {
            console.log('✅ All verifications passed successfully!');
            return false;
        }

        console.log('❌ Found the following discrepancies:\n');

        if (discrepancies.missingFromJson.length > 0) {
            console.log('📝 Codes that exist on chain but not in contracts.json:');
            discrepancies.missingFromJson.forEach(({codeId, chainHash, proposal}) => {
                console.log(`   Code ID ${codeId}:`);
                console.log(`   Hash: ${chainHash}`);
                if (proposal) {
                    console.log(`   Found in Proposal ${proposal.proposalId}: ${proposal.proposalTitle}`);
                    console.log(`   Status: ${proposal.status}`);
                    console.log(`   Message ${proposal.messageIndex} of ${proposal.totalMessages}`);
                } else {
                    console.log('   No matching proposal found');
                }
                console.log('');
            });
        }

        if (discrepancies.missingFromChain.length > 0) {
            console.log('🔍 Codes that exist in contracts.json but not on chain:');
            discrepancies.missingFromChain.forEach(({codeId, name, hash, proposal}) => {
                console.log(`   Code ID ${codeId} (${name})`);
                console.log(`   Hash: ${hash}`);
                if (proposal) {
                    console.log(`   Found in Proposal ${proposal.proposalId}: ${proposal.proposalTitle}`);
                    console.log(`   Status: ${proposal.status}`);
                    console.log(`   Message ${proposal.messageIndex} of ${proposal.totalMessages}`);
                }
                console.log('');
            });
        }

        if (discrepancies.hashMismatches.length > 0) {
            console.log('⚠️  Hash mismatches between chain and contracts.json:');
            discrepancies.hashMismatches.forEach(({codeId, name, localHash, chainHash, proposal}) => {
                console.log(`   Code ID ${codeId} (${name}):`);
                console.log(`   contracts.json: ${localHash}`);
                console.log(`   chain:         ${chainHash}`);
                if (proposal) {
                    console.log(`   Found in Proposal ${proposal.proposalId}: ${proposal.proposalTitle}`);
                    console.log(`   Status: ${proposal.status}`);
                    console.log(`   Message ${proposal.messageIndex} of ${proposal.totalMessages}`);
                }
                console.log('');
            });
        }

        if (discrepancies.missingFromBoth.length > 0) {
            console.log('❗ Store code messages found in proposals but missing from both chain and contracts.json:');
            discrepancies.missingFromBoth.forEach(({hash, proposal}) => {
                console.log(`   Hash: ${hash}`);
                console.log(`   Found in Proposal ${proposal.proposalId}: ${proposal.proposalTitle}`);
                console.log(`   Status: ${proposal.status}`);
                console.log(`   Message ${proposal.messageIndex} of ${proposal.totalMessages}`);
                console.log('');
            });
        }

        if (discrepancies.genesisWithProposal.length > 0) {
            console.log('⚠️  Contracts marked as "Genesis" but were uploaded via governance proposal:');
            discrepancies.genesisWithProposal.forEach(({codeId, name, governance, hash, proposal}) => {
                console.log(`   Code ID ${codeId} (${name}):`);
                console.log(`   Governance: ${governance}`);
                console.log(`   Hash: ${hash}`);
                console.log(`   Found in Proposal ${proposal.proposalId}: ${proposal.proposalTitle}`);
                console.log(`   Status: ${proposal.status}`);
                console.log(`   Message ${proposal.messageIndex} of ${proposal.totalMessages}`);
                console.log('');
            });
        }

        return true;
    } catch (error) {
        console.error('Error during verification:', error);
        return true;
    }
}

// Main function
async function main() {
    try {
        const hasErrors = await verifyContracts();
        if (hasErrors) {
            process.exit(1);
        }
    } catch (error) {
        console.error('Fatal error:', error);
        process.exit(1);
    }
}

main(); 