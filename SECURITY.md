# Security Policy

This policy covers the **core protocol contracts** in this repository — the
account contract and the treasury contract. These are governance-deployed and
form the foundation of XION's account abstraction and fee infrastructure.

It supplements the
[organization-wide policy](https://github.com/burnt-labs/.github/blob/main/SECURITY.md),
which governs anything not addressed here.

## Reporting a Vulnerability

**Do not open a public GitHub issue for a security vulnerability.**

| Type of finding                  | How to report                                         |
| -------------------------------- | ----------------------------------------------------- |
| Security vulnerability           | Email [security@burnt.com](mailto:security@burnt.com)  |
| Non-sensitive or operational bug | Open a GitHub issue on this repository                 |

Include the type of vulnerability, affected contract and deployed version, steps
to reproduce, impact, how an attacker would exploit it, and any known
mitigations.

We acknowledge receipt within **5 business days** and provide a triage decision
within **14 days**. Active exploitation, or confirmed attacker awareness of an
unpatched vulnerability, escalates the issue to Critical handling regardless of
its original classification.

## Covered Contracts

| Contract | Path                                                                                    |
| -------- | ---------------------------------------------------------------------------------------- |
| Account  | [`contracts/account`](https://github.com/burnt-labs/contracts/tree/main/contracts/account)   |
| Treasury | [`contracts/treasury`](https://github.com/burnt-labs/contracts/tree/main/contracts/treasury) |

**Scope under this contract-specific policy is limited exclusively to the two
contracts above.** All other contracts in this repository — `contracts/asset`,
`contracts/marketplace`, and `contracts/user_map` — fall back to the
organization-wide policy, as do example and demo contracts; they are not part
of this repository-specific bounty scope.

Scope applies to contracts deployed on the current mainnet. Findings affecting
only deprecated deployments, or already remediated in the currently deployed
bytecode, are not eligible regardless of whether the fix was publicly announced.
Reporters are responsible for verifying exploitability against the current
deployed contract version before submission.

### Treasury Scope Note

Vulnerabilities in the treasury's fee grant issuance functions are covered only
where the attack results in **direct, uncapped extraction of funds from the
treasury contract's XION balance to an attacker-controlled address, without
requiring any privileged setup**.

Findings targeting bounded fee grant operations — grants issued with explicit
allowance limits and expiration enforced at the contract level — are not
eligible. The grant-issuance design intentionally delegates authorization to the
calling application layer.

## Proof of Concept Requirements

**Reports must include an end-to-end proof of concept.** Severity is assessed on
demonstrated impact under real-world constraints, not theoretical worst-case
scenarios.

Tests that mock contract state or bypass CosmWasm message routing — including
`cw-multi-test` environments and harnesses that stub the bank, staking, or IBC
modules — do not demonstrate exploitability on their own.

The proof of concept should run against a **locally running XION node configured
with mainnet parameters**, using the governance-deployed contract bytecode, the
XION ante handler chain, and module configuration matching mainnet. The attack
should be executed via standard transaction broadcast against that node.

## Permissioned Chain Policy

XION mainnet operates with `code_upload_access: Nobody`. New contracts require
governance approval to deploy.

**Any attack vector requiring an attacker to deploy a malicious contract on
mainnet is out of scope, regardless of technical validity.** A finding must be
exploitable using only contracts already deployed on mainnet.

## Privileged Actor Policy

Attacks requiring a contract admin, governance, or another privileged party to
take self-destructive or colluding action are classified at **Medium at most**,
regardless of downstream impact. The threat model assumes privileged actors
behave according to their role.

## Out of Scope

**Assets**

- `contracts/asset`, `contracts/marketplace`, and `contracts/user_map`
- Example and demo contracts
- Third-party contracts deployed on XION by external teams
- Chain node modules — see [`burnt-labs/xion`](https://github.com/burnt-labs/xion/blob/main/SECURITY.md)
- Frontend applications and web properties
- Upstream dependencies — vulnerabilities in CosmWasm or the Cosmos SDK are not
  eligible here; only code originating in this repository is covered

**Vulnerability classes**

- Attacks requiring malicious contract deployment on mainnet
- Denial of service requiring sustained attacker resource expenditure
  proportional to the harm caused
- Treasury fee grant issuance bounded by explicit allowance limits and
  expiration, absent demonstrated direct uncapped extraction of funds
- Governance attacks requiring a malicious proposal to pass
- Theoretical vulnerabilities without a working end-to-end proof of concept
- Attacks where the attacker's cost to execute exceeds the demonstrable harm to
  the protocol or its users
- Best practices, gas optimizations, missing events, and informational findings

## Severity Characterization

| Severity     | Description                                                                                                       |
| ------------ | ------------------------------------------------------------------------------------------------------------------- |
| **CRITICAL** | Direct, permanent, irrecoverable theft or loss of funds held in or routed through covered contracts at meaningful scale. Complete bypass of account authentication where the proof of concept demonstrates actual movement of funds from a pre-existing victim account to an attacker-controlled address using only attacker-controlled keys. Permanent state corruption with no recovery path |
| **HIGH**     | Theft or freezing of funds affecting individual accounts. Authentication bypass with demonstrated exploitability against an existing account. Permanent disruption of core contract functionality |
| **MEDIUM**   | Limited fund loss requiring specific preconditions. Attacks requiring privileged-party cooperation. Temporary disruption recoverable by governance |
| **LOW**      | Valid, reproducible code-level issue with no direct risk to funds, representing a meaningful hardening opportunity. Must include a specific code reference |

Severity is assessed by Burnt Labs based on demonstrated impact. Reports
submitted at a severity that does not match the definitions above are assessed
as written; we do not reclassify or negotiate severity on a reporter's behalf.

## Responsible Disclosure

- Do not exploit a vulnerability beyond what is necessary to confirm it exists
- **Do not test against XION mainnet.** Testing that targets live production
  systems will disqualify the report
- Do not access, modify, or exfiltrate user data
- Do not disclose publicly before a fix is confirmed and deployed

## Safe Harbor

Burnt Labs will not pursue legal action against researchers who report
vulnerabilities in good faith under this policy, do not exploit beyond what is
necessary to confirm the finding, do not access or disclose user data, and do
not disrupt production systems.

Authorization to actively test extends only to assets named in a published Burnt
Labs bug bounty program. Testing systems outside that scope is not authorized.
Reporting a vulnerability you encountered incidentally is always welcome.
