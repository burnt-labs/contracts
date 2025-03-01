## Description
<!-- Provide a brief description of the changes in this PR -->

## Contract Details
Please append all the required information below for the contract(s) being added to `contracts.json`.

Required fields and their descriptions:
- `name`: Contract name (required)
- `description`: Brief description of the contract's purpose
- `code_id`: Contract code ID on mainnet
- `hash`: Contract hash in UPPERCASE
- `release`:
  - `url`: URL to the release/commit (e.g., https://github.com/org/repo/releases/tag/v1.0.0)
  - `version`: Version tag or first 7 chars of commit hash
- `author`:
  - `name`: Organization name
  - `url`: Organization website URL
- `governance`: "Genesis" or proposal number
- `deprecated`: true if contract is deprecated (mixed inline with active contracts)

Example JSON structure:
```json
{
  "name": "",
  "description": "",
  "code_id": "",
  "hash": "",
  "release": {
    "url": "",
    "version": ""
  },
  "author": {
    "name": "",
    "url": ""
  },
  "governance": "",
  "deprecated": false
}
```

### Finding Code ID and Hash
To find the latest code ID and hash:
1. Run the verification tool which will show all code IDs on chain:
   ```bash
   node scripts/verify-code-ids.js
   ```
2. The new code ID will be shown in the mismatches as "exists on chain but not in contracts.json"
3. You can also query the code hash via the chain's RPC endpoint:
   ```bash
   xiond query wasm code-info <code-id> --node https://rpc.xion-mainnet-1.burnt.com
   ```

### Documentation Updates
The README.md is automatically generated from `contracts.json`. After making changes:

1. Ensure you have the required dependencies:
   - Node.js: https://nodejs.org/
   - jq: `brew install jq` (macOS) or `apt-get install jq` (Ubuntu/Debian)

2. Run the convert script to validate and update the README:
   ```bash
   ./convert.sh
   ```

3. Commit both the `contracts.json` and generated `README.md` changes

⚠️ Important Notes:
- Do not edit README.md manually. All changes must be made through `contracts.json`
- Pull requests with manual README edits will be automatically rejected by CI
- If you forget to run `./convert.sh` locally, the CI will fail with a "README out of sync" error

### Validation
The `convert.sh` script automatically performs these validations:
- All required fields are present and properly formatted
- Hash is 64 characters and uppercase hex
- URLs are valid HTTPS links
- Code IDs are unique
- Contracts are ordered by code_id (both active and deprecated contracts follow the same ordering)
- README.md stays in sync with contracts.json

If any validation fails, the script will show specific error messages to help you fix the issues.

### Checklist
- [ ] Added entry to `contracts.json` with all required fields
- [ ] Contract name is clear and descriptive
- [ ] Description explains the contract's purpose
- [ ] Code ID matches the mainnet deployed code
- [ ] Hash is in uppercase and matches the stored code
- [ ] Release URL points to the correct tag/commit
- [ ] Version matches the release tag or commit hash
- [ ] Author information is correct with valid URL
- [ ] Governance field correctly references proposal or "Genesis"
- [ ] Deprecated flag is set appropriately
- [ ] Entry is placed in code_id order (regardless of deprecated status)
- [ ] Ran `./convert.sh` and fixed any validation errors
- [ ] Both `contracts.json` and generated `README.md` are included in the commit

### Additional Notes
<!-- Add any additional context or notes about the contract deployment here -->
