#!/bin/bash

echo "# XION Mainnet Contracts" > contracts.md
echo "Contract information for XION mainnet" >> contracts.md
echo "" >> contracts.md

echo "| Name | Description | Release | Author | Code ID | Hash | Governance Proposal |" >> contracts.md
echo "|:-----|:------------|:--------|:-------|:--------|:-----|:-------------------|" >> contracts.md

jq -r '.[] | "| \(.name) | \(.description // "") | \(if .release then "[\(.release.version)](\(.release.url))" else "" end) | \(if .author then "[\(.author.name)](\(.author.url))" else "" end) | `\(.code_id // "")` | `\(.hash)` | \(.governance // "") |"' contracts.json >> contracts.md
