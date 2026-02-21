# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in RillCoin, **please do not open a public issue.**

Instead, report it responsibly:

1. **Email**: Send details to **security@rillcoin.com**
2. **Discord**: DM a core team member directly (do not post in public channels)

Please include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix or mitigation**: Depends on severity, but we aim for:
  - Critical: 24-72 hours
  - High: 1-2 weeks
  - Medium/Low: Next release cycle

## Scope

The following are in scope:

- **Consensus bugs**: Double-spend, invalid block acceptance, supply inflation
- **Decay algorithm errors**: Incorrect decay calculations, bypass methods
- **Cryptographic issues**: Signature forgery, hash collisions, key leakage
- **Network attacks**: Eclipse attacks, Sybil attacks, message manipulation
- **Wallet vulnerabilities**: Key extraction, unauthorized spending, seed leakage
- **RPC vulnerabilities**: Unauthorized access, injection, information disclosure
- **Denial of service**: Crash bugs, resource exhaustion, amplification attacks

## Out of Scope

- Attacks requiring physical access to a machine
- Social engineering of team members
- Issues in dependencies (report these upstream, but let us know)
- Testnet-only issues with no mainnet impact

## Disclosure Policy

- We follow coordinated disclosure — please give us reasonable time to fix issues before public disclosure
- We will credit reporters in release notes (unless you prefer anonymity)
- We do not pursue legal action against good-faith security researchers

## Bug Bounty

We do not currently have a formal bug bounty program. Confirmed vulnerability reporters will receive the **Bug Hunter** Discord role and recognition in the changelog. A formal bounty program will be announced before mainnet launch.

## Supported Versions

| Version | Supported |
|---------|-----------|
| main branch | Yes |
| Testnet releases | Yes |
| Older commits | No |
