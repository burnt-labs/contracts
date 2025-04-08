#!/bin/bash

# Check required dependencies
if ! command -v jq >/dev/null 2>&1; then
    echo "❌ Error: jq is not installed. Please install it first:"
    echo "  - On macOS: brew install jq"
    echo "  - On Ubuntu/Debian: sudo apt-get install jq"
    echo "  - On Windows with chocolatey: choco install jq"
    exit 1
fi

if ! command -v node >/dev/null 2>&1; then
    echo "❌ Error: Node.js is not installed. Please install it first:"
    echo "  - On macOS: brew install node"
    echo "  - On Ubuntu/Debian: curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash - && sudo apt-get install -y nodejs"
    echo "  - On Windows: Download from https://nodejs.org/"
    exit 1
fi

# Validate contracts.json
if [ -n "$CI" ]; then
    node scripts/validate.js || exit 1
    OLD_HASH=$(if command -v md5sum >/dev/null 2>&1; then md5sum README.md | cut -d' ' -f1; else md5 -q README.md; fi)
fi

# Generate README
echo "<!-- AUTOGENERATED FILE DO NOT EDIT -->" > README.md
echo "" >> README.md
echo "# XION Mainnet Contracts" >> README.md
echo "" >> README.md
echo "Contract information for XION mainnet" >> README.md
echo "" >> README.md
echo "## Development" >> README.md
echo "" >> README.md
echo "### Updating Documentation" >> README.md
echo "" >> README.md
echo "The README is automatically generated from \`contracts.json\`. To update it:" >> README.md
echo "" >> README.md
echo "1. Ensure you have the required dependencies:" >> README.md
echo "   - Node.js: https://nodejs.org/" >> README.md
echo "   - jq: \`brew install jq\` (macOS) or \`apt-get install jq\` (Ubuntu/Debian)" >> README.md
echo "" >> README.md
echo "2. Modify \`contracts.json\` with your changes" >> README.md
echo "   - The script will validate the JSON format and required fields" >> README.md
echo "   - Each contract must include: name, description, code_id, hash, release info, and author details" >> README.md
echo "" >> README.md
echo "3. Run the convert script:" >> README.md
echo "\`\`\`bash" >> README.md
echo "./convert.sh" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md
echo "4. Commit both the \`contracts.json\` and generated \`README.md\` changes" >> README.md
echo "   - The CI will validate both files are in sync during pull requests" >> README.md
echo "   - Pull requests with manual README edits will be rejected" >> README.md
echo "" >> README.md
echo "### Compiling" >> README.md
echo "" >> README.md
echo "\`\`\`bash" >> README.md
echo "" >> README.md
echo "docker run --rm -v \"\$(pwd)\":/code \\" >> README.md
echo "  --mount type=volume,source=\"\$(basename \"\$(pwd)\")_cache\",target=/target \\" >> README.md
echo "  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \\" >> README.md
echo "  cosmwasm/optimizer:0.16.1" >> README.md
echo "" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md

echo "## Active Contracts" >> README.md
echo "| Name | Description | Release | Author | Code ID | Code ID (Testnet) | Hash | Governance Proposal |" >> README.md
echo "|:-----|:------------|:--------|:-------|:--------|:------------------|:-----|:-------------------|" >> README.md
jq -r '.[] | select(.deprecated != true) | "| \(.name) | \(.description // "") | \(if .release then "[\(.release.version)](\(.release.url))" else "" end) | \(if .author then "[\(.author.name)](\(.author.url))" else "" end) | `\(.code_id // "")` | \(if .testnet then "`\(.testnet.code_id)`" else "-" end) | `\(.hash)` | \(if .governance and (.governance | test("^[0-9]+$")) then "[\(.governance)](https://www.mintscan.io/xion/proposals/\(.governance))" else .governance // "" end) |"' contracts.json >> README.md

echo "" >> README.md
echo "## Deprecated Contracts" >> README.md
echo "| Name | Description | Release | Author | Code ID | Hash | Governance Proposal |" >> README.md
echo "|:-----|:------------|:--------|:-------|:--------|:-----|:-------------------|" >> README.md
jq -r '.[] | select(.deprecated == true) | "| \(.name) | \(.description // "") | \(if .release then "[\(.release.version)](\(.release.url))" else "" end) | \(if .author then "[\(.author.name)](\(.author.url))" else "" end) | `\(.code_id // "")` | `\(.hash)` | \(if .governance and (.governance | test("^[0-9]+$")) then "[\(.governance)](https://www.mintscan.io/xion/proposals/\(.governance))" else .governance // "" end) |"' contracts.json >> README.md

echo "" >> README.md
echo "## Utilities" >> README.md
echo "" >> README.md
echo "### Code ID Verification" >> README.md
echo "" >> README.md
echo "The repository includes a utility to verify that the code IDs and their corresponding hashes in the local \`contracts.json\` file match those deployed on the Xion mainnet." >> README.md
echo "" >> README.md
echo "#### Prerequisites" >> README.md
echo "- Node.js 18 or higher: https://nodejs.org/" >> README.md
echo "" >> README.md
echo "#### Usage" >> README.md
echo "To verify code IDs:" >> README.md
echo "\`\`\`bash" >> README.md
echo "node scripts/verify-contracts.js" >> README.md
echo "\`\`\`" >> README.md
echo "" >> README.md
echo "The utility will:" >> README.md
echo "1. Read the local contracts.json file" >> README.md
echo "2. Fetch current contract data from Xion mainnet" >> README.md
echo "3. Compare code IDs and hashes" >> README.md
echo "4. Report any mismatches or discrepancies" >> README.md
echo "" >> README.md
echo "This helps ensure that the contract information in this repository accurately reflects what's deployed on the Xion mainnet." >> README.md

# In CI, verify README hasn't changed
if [ -n "$CI" ]; then
    NEW_HASH=$(if command -v md5sum >/dev/null 2>&1; then md5sum README.md | cut -d' ' -f1; else md5 -q README.md; fi)
    if [ "$OLD_HASH" != "$NEW_HASH" ]; then
        echo "❌ README.md is out of sync with contracts.json. Please run ./convert.sh locally and commit the changes."
        exit 1
    fi
    echo "✅ README.md is up to date with contracts.json"
fi
