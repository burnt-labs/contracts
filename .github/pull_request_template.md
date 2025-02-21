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

### Finding Code ID and Hash

If you have a governance proposal ID, you can find the code ID by:

1. View the proposal on [XION Explorer](https://explorer.burnt.com/xion/gov)
2. Find the `store-code` or `instantiate-contract` message in the proposal details
3. Once the proposal is passed and executed:
   - The code ID will be visible in the transaction details
   - The hash can be found in the transaction logs
4. You can also query via the chain's RPC endpoint:
   ```bash
   xiond query gov proposal <proposal-id>
   xiond query wasm code-info <code-id>
   ```

### Validation

Before submitting your PR, run the validation script to check your changes:

```bash
node scripts/validate.js
```

The script checks:
- All required fields are present
- Hash is 64 characters and uppercase hex
- URLs are valid HTTPS links
- Code IDs are unique
- Contracts are ordered by code_id within active and deprecated sections
- Active contracts come before deprecated ones

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
- [ ] Entry maintains alphabetical order in the JSON array
- [ ] Ran `node validate.js` and fixed any validation errors

### Additional Notes

<!-- Add any additional context or notes about the contract deployment here -->

Note: After the PR is merged, the README.md will be automatically updated from contracts.json.