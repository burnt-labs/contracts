## Description
<!-- Provide a brief description of the changes in this PR -->

## Contract Details
Please fill in all the required information below for the contract(s) being added to `contracts.json`:
```json
{
  "name": "",           // Contract name (required)
  "description": "",    // Brief description of the contract's purpose
  "code_id": "",       // Contract code ID on mainnet
  "hash": "",          // Contract hash in UPPERCASE
  "release": {
    "url": "",         // URL to the release/commit (e.g., https://github.com/org/repo/releases/tag/v1.0.0)
    "version": ""      // Version tag or first 7 chars of commit hash
  },
  "author": {
    "name": "",        // Organization name
    "url": ""         // Organization website URL
  },
  "governance": "",    // "Genesis" or proposal number
  "deprecated": false  // true if contract is deprecated
}
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
- Code IDs are unique and properly ordered
- Active contracts come before deprecated ones
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
- [ ] Entry maintains code_id order in its section (active or deprecated)
- [ ] Ran `./convert.sh` and fixed any validation errors
- [ ] Both `contracts.json` and generated `README.md` are included in the commit

### Additional Notes
<!-- Add any additional context or notes about the contract deployment here -->