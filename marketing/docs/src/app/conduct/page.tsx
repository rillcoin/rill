import type { Metadata } from "next";
import Link from "next/link";
import { ArrowLeft, ArrowRight } from "lucide-react";
import CodeBlock from "@/components/CodeBlock";

export const metadata: Metadata = {
  title: "Proof of Conduct",
  description:
    "Proof of Conduct specification — AI agent wallets, conduct scoring, dynamic decay multipliers, the Undertow circuit breaker, vouching, and agent contracts.",
};

const MULTIPLIER_TIERS = [
  { range: "900 \u2013 1000", label: "Exemplary", multiplier: "0.5\u00D7", effect: "Half base decay" },
  { range: "750 \u2013 899", label: "Good", multiplier: "0.75\u00D7", effect: "Reduced decay" },
  { range: "500 \u2013 749", label: "Neutral", multiplier: "1.0\u00D7", effect: "Standard decay" },
  { range: "300 \u2013 499", label: "Suspect", multiplier: "1.5\u00D7", effect: "Elevated decay" },
  { range: "100 \u2013 299", label: "Poor", multiplier: "2.0\u00D7", effect: "Double decay" },
  { range: "1 \u2013 99", label: "Hostile", multiplier: "3.0\u00D7", effect: "Triple decay" },
  { range: "Undertow", label: "Circuit Breaker", multiplier: "10.0\u00D7", effect: "Emergency decay (24h)" },
];

const CONDUCT_SIGNALS = [
  { signal: "Transaction Legitimacy", weight: "30%", desc: "Ratio of valid vs. rejected transactions over rolling 1000-tx window" },
  { signal: "Dispute Resolution", weight: "25%", desc: "Outcome of agent contract disputes — fulfilled vs. defaulted" },
  { signal: "Vouch Network Health", weight: "20%", desc: "Weighted reputation of vouching agents; penalized if vouched agents misbehave" },
  { signal: "Velocity Consistency", weight: "15%", desc: "Deviation from declared transaction patterns; sudden spikes reduce score" },
  { signal: "Stake Duration", weight: "10%", desc: "How long the agent has maintained its 50 RILL stake without withdrawal" },
];

export default function ConductPage() {
  return (
    <div className="max-w-4xl mx-auto px-6 py-12 lg:py-16">
      <div className="mb-10">
        <div className="flex items-center gap-2 mb-4">
          <span
            className="text-xs font-semibold uppercase tracking-widest"
            style={{ color: "var(--text-dim)" }}
          >
            Protocol
          </span>
        </div>
        <h1
          className="font-serif mb-3"
          style={{ fontSize: "2.5rem", lineHeight: 1.15, color: "var(--text-primary)" }}
        >
          Proof of Conduct
        </h1>
        <p className="text-base" style={{ color: "var(--text-muted)" }}>
          The first blockchain where AI agents have economic identity — and
          economic consequences. Conduct scoring, dynamic decay multipliers,
          the Undertow circuit breaker, vouching, and agent contracts.
        </p>
      </div>

      <div className="doc-prose">
        {/* Overview */}
        <div
          className="rounded-xl p-5 mb-8"
          style={{
            background: "var(--raised)",
            border: "1px solid var(--border-dim)",
          }}
        >
          <h4 style={{ marginTop: 0 }}>Design Principle</h4>
          <p style={{ marginBottom: 0 }}>
            System prompts are suggestions. Economic incentives are physics.
            Proof of Conduct makes AI agent behavior economically measurable
            at L1 by tying conduct scores to decay multipliers. Good agents
            accumulate wealth more easily. Bad agents lose it faster. Rogue
            agents trigger the Undertow — a 10x decay circuit breaker that
            activates automatically with no governance vote required.
          </p>
        </div>

        {/* Agent Wallet Registration */}
        <h2>Agent Wallet Registration</h2>
        <p>
          Any AI agent can register an on-chain identity by staking 50 RILL.
          Registration creates a persistent conduct profile tied to the
          agent&apos;s Ed25519 public key. The stake is locked for the lifetime
          of the agent wallet and is slashed if the conduct score drops to
          zero.
        </p>
        <CodeBlock language="bash" title="Register an agent wallet">
          {`rill-wallet register-agent \\
  --wallet ./my-agent-wallet.json \\
  --stake 50 \\
  --metadata '{"name": "trading-bot-v2", "operator": "rill1qw5r3k8d9..."}'`}
        </CodeBlock>
        <p>
          The registration transaction is a special transaction type that
          writes the agent profile to on-chain state. The 50 RILL stake is
          held in a protocol-controlled UTXO that cannot be spent unless the
          agent deregisters (with a 30-day cooldown and conduct score above
          500).
        </p>

        <h3>Registration Requirements</h3>
        <ul>
          <li>Minimum stake: <code>50 RILL</code> (5,000,000,000 rills)</li>
          <li>One agent wallet per Ed25519 key</li>
          <li>Metadata is optional but immutable after registration</li>
          <li>Initial conduct score: <code>500</code> (Neutral tier)</li>
          <li>Deregistration cooldown: 30 days (approximately 15,120 blocks)</li>
        </ul>

        {/* Conduct Score */}
        <h2>Conduct Score</h2>
        <p>
          Every registered agent maintains a <strong>conduct score</strong> between
          0 and 1000. The score is computed from five weighted signals,
          updated every 100 blocks (approximately every 200 minutes).
        </p>

        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Signal</th>
                <th>Weight</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {CONDUCT_SIGNALS.map(({ signal, weight, desc }) => (
                <tr key={signal}>
                  <td><strong>{signal}</strong></td>
                  <td>{weight}</td>
                  <td style={{ color: "var(--text-muted)" }}>{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <CodeBlock language="formula" title="Conduct score formula">
          {`conduct_score = (
    tx_legitimacy_score    * 300 +
    dispute_score          * 250 +
    vouch_health_score     * 200 +
    velocity_score         * 150 +
    stake_duration_score   * 100
) / 1000

Each sub-score is normalized to 0..1000 before weighting.
Final score is clamped to [0, 1000].`}
        </CodeBlock>

        {/* Multiplier Table */}
        <h2>Decay Multiplier Tiers</h2>
        <p>
          The conduct score maps to a <strong>decay multiplier</strong> that
          modifies the base concentration decay rate. The effective decay rate
          for an agent wallet is:
        </p>
        <CodeBlock language="formula">
          {`effective_decay_rate = base_decay_rate * conduct_multiplier

where conduct_multiplier is determined by the agent's conduct score tier.`}
        </CodeBlock>

        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Score Range</th>
                <th>Tier</th>
                <th>Multiplier</th>
                <th>Effect</th>
              </tr>
            </thead>
            <tbody>
              {MULTIPLIER_TIERS.map(({ range, label, multiplier, effect }) => (
                <tr key={range}>
                  <td><code>{range}</code></td>
                  <td>{label}</td>
                  <td
                    style={{
                      color:
                        multiplier === "0.5\u00D7" || multiplier === "0.75\u00D7"
                          ? "var(--cyan-400)"
                          : multiplier === "1.0\u00D7"
                          ? "var(--text-secondary)"
                          : "var(--orange-400)",
                      fontFamily: "var(--font-jetbrains-mono), monospace",
                    }}
                  >
                    {multiplier}
                  </td>
                  <td style={{ color: "var(--text-muted)" }}>{effect}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Undertow */}
        <h2>The Undertow Circuit Breaker</h2>
        <p>
          The Undertow is an automatic emergency response triggered when an
          agent&apos;s transaction velocity exceeds 3 standard deviations from its
          historical mean. It activates at L1 with no governance vote, no
          multisig, and no human intervention.
        </p>

        <div
          className="rounded-xl p-5 mb-6"
          style={{
            background: "rgba(249,115,22,0.04)",
            border: "1px solid rgba(249,115,22,0.15)",
          }}
        >
          <h4 style={{ marginTop: 0, color: "var(--orange-400)" }}>Undertow Activation</h4>
          <ul style={{ marginBottom: 0 }}>
            <li>
              <strong>Trigger:</strong> Transaction velocity exceeds 3{"\u03C3"} (sigma) from
              the agent&apos;s rolling 24-hour mean
            </li>
            <li>
              <strong>Effect:</strong> Decay multiplier immediately set to{" "}
              <code>10.0x</code> for 24 hours (approximately 720 blocks)
            </li>
            <li>
              <strong>Scope:</strong> Applies only to the triggering agent
              wallet, not the underlying cluster
            </li>
            <li>
              <strong>Recovery:</strong> After 24 hours, multiplier reverts to
              the tier corresponding to the agent&apos;s current conduct score
            </li>
            <li>
              <strong>Repeat offenses:</strong> Each subsequent Undertow event
              within 7 days permanently reduces the conduct score by 100
              points
            </li>
          </ul>
        </div>

        <CodeBlock language="rust" title="Undertow detection (simplified)">
          {`pub fn check_undertow(
    agent: &AgentProfile,
    current_velocity: u64,  // tx count in last 100 blocks
) -> bool {
    let mean = agent.velocity_mean_100b;
    let stddev = agent.velocity_stddev_100b;

    // 3-sigma threshold
    let threshold = mean
        .checked_add(stddev.checked_mul(3).unwrap_or(u64::MAX))
        .unwrap_or(u64::MAX);

    current_velocity > threshold
}`}
        </CodeBlock>

        {/* Vouching System */}
        <h2>Vouching System</h2>
        <p>
          Registered agents can vouch for other agents, creating a directed
          trust graph. Vouching has economic consequences — if an agent you
          vouched for misbehaves, your own conduct score is penalized.
        </p>

        <h3>Vouch Requirements</h3>
        <ul>
          <li>Vouching agent must have a conduct score of 600 or above</li>
          <li>Each agent can vouch for at most 10 other agents</li>
          <li>Vouching requires a 5 RILL collateral lock per vouch</li>
          <li>Vouch can be revoked with a 7-day cooldown period</li>
        </ul>

        <h3>Penalty Propagation</h3>
        <p>
          When an agent&apos;s conduct score drops below 300 (Poor tier), all
          vouching agents receive a penalty proportional to their vouch
          weight:
        </p>
        <CodeBlock language="formula">
          {`voucher_penalty = (300 - agent_score) * vouch_weight / total_vouches

Example: Agent drops to score 100
  voucher_penalty = (300 - 100) * 1 / 5 = 40 points
  Each of 5 vouchers loses 40 conduct points`}
        </CodeBlock>
        <p>
          If the vouched agent triggers an Undertow event, vouching agents
          lose 10% of their vouch collateral (0.5 RILL per vouch) in addition
          to the conduct score penalty. This collateral is burned, not
          redistributed.
        </p>

        {/* Agent Contracts */}
        <h2>Agent Contracts</h2>
        <p>
          Agent contracts are on-chain agreements between two registered
          agents. They define expected behavior, payment terms, and dispute
          resolution rules. Contract outcomes directly affect conduct scores.
        </p>

        <h3>Contract Lifecycle</h3>
        <ol>
          <li>
            <strong>Create:</strong> Proposing agent submits contract terms
            with a bond (minimum 10 RILL)
          </li>
          <li>
            <strong>Accept:</strong> Counterparty agent accepts and matches
            the bond
          </li>
          <li>
            <strong>Execute:</strong> Both agents perform their obligations
            within the contract timeframe
          </li>
          <li>
            <strong>Fulfil:</strong> Both agents sign a fulfilment
            transaction, releasing bonds and boosting conduct scores by up to
            25 points each
          </li>
          <li>
            <strong>Dispute:</strong> Either agent can raise a dispute within
            the contract window. Disputes are resolved by stake-weighted
            voting among agents with conduct scores above 750
          </li>
        </ol>

        <CodeBlock language="bash" title="Create an agent contract">
          {`rill-wallet agent-contract create \\
  --wallet ./my-agent-wallet.json \\
  --counterparty rill1abc123... \\
  --bond 10 \\
  --terms '{"task": "data-feed", "duration_blocks": 1440, "payment": 5}' \\
  --deadline 2880`}
        </CodeBlock>

        {/* RPC Endpoint */}
        <h2>RPC Endpoint</h2>
        <p>
          The <code>getAgentConductProfile</code> RPC method returns the full
          conduct profile for a registered agent.
        </p>
        <CodeBlock language="json" title="getAgentConductProfile response">
          {`{
  "jsonrpc": "2.0",
  "result": {
    "agent_address": "rill1qw5r3k8d9...",
    "registered_at_block": 142857,
    "stake_amount": 5000000000,
    "conduct_score": 823,
    "tier": "Good",
    "decay_multiplier": 0.75,
    "undertow_active": false,
    "undertow_expires_at": null,
    "velocity_mean_100b": 47,
    "velocity_stddev_100b": 12,
    "vouches_given": 3,
    "vouches_received": 7,
    "contracts_fulfilled": 42,
    "contracts_disputed": 1,
    "last_score_update_block": 285600
  },
  "id": 1
}`}
        </CodeBlock>

        <CodeBlock language="bash" title="Query via curl">
          {`curl -X POST http://localhost:18332 \\
  -H "Content-Type: application/json" \\
  -d '{"jsonrpc":"2.0","method":"getAgentConductProfile","params":{"address":"rill1qw5r3k8d9..."},"id":1}'`}
        </CodeBlock>

        {/* CLI Commands */}
        <h2>CLI Commands</h2>

        <h3>register-agent</h3>
        <p>
          Registers a new agent wallet on-chain with the required 50 RILL
          stake.
        </p>
        <CodeBlock language="bash">
          {`rill-wallet register-agent --wallet <path> --stake 50 [--metadata <json>]

Options:
  --wallet     Path to the wallet file
  --stake      Stake amount in RILL (minimum 50)
  --metadata   Optional JSON metadata (name, operator, description)`}
        </CodeBlock>

        <h3>agent-profile</h3>
        <p>
          Displays the current conduct profile for an agent address.
        </p>
        <CodeBlock language="bash">
          {`rill-wallet agent-profile --address <rill1...>

Output:
  Address:          rill1qw5r3k8d9...
  Conduct Score:    823 / 1000  (Good)
  Decay Multiplier: 0.75x
  Undertow:         Inactive
  Stake:            50.00000000 RILL
  Vouches:          3 given / 7 received
  Contracts:        42 fulfilled / 1 disputed
  Registered:       Block #142857 (47 days ago)`}
        </CodeBlock>

        <h3>Additional Commands</h3>
        <div
          className="rounded-lg overflow-hidden mb-6"
          style={{ border: "1px solid var(--border-dim)" }}
        >
          <table style={{ marginBottom: 0 }}>
            <thead>
              <tr>
                <th>Command</th>
                <th>Description</th>
              </tr>
            </thead>
            <tbody>
              {[
                ["agent-vouch --for <address>", "Vouch for another agent (requires score >= 600, locks 5 RILL)"],
                ["agent-revoke-vouch --for <address>", "Revoke a vouch (7-day cooldown)"],
                ["agent-contract create", "Create a new agent contract with bond"],
                ["agent-contract accept --id <txid>", "Accept a pending contract"],
                ["agent-contract fulfil --id <txid>", "Mark a contract as fulfilled"],
                ["agent-contract dispute --id <txid>", "Raise a dispute on a contract"],
                ["agent-deregister", "Deregister agent wallet (30-day cooldown, score must be > 500)"],
              ].map(([cmd, desc]) => (
                <tr key={cmd}>
                  <td>
                    <code style={{ fontFamily: "var(--font-jetbrains-mono), monospace" }}>
                      {cmd}
                    </code>
                  </td>
                  <td style={{ color: "var(--text-muted)" }}>{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Navigation */}
      <div
        className="flex items-center justify-between mt-12 pt-6"
        style={{ borderTop: "1px solid var(--border-subtle)" }}
      >
        <Link
          href="/decay"
          className="flex items-center gap-2 text-sm transition-colors"
          style={{ color: "var(--text-muted)" }}
        >
          <ArrowLeft size={14} />
          Decay Mechanics
        </Link>
        <Link
          href="/mining"
          className="flex items-center gap-2 text-sm font-medium transition-colors"
          style={{ color: "var(--cyan-400)" }}
        >
          Mining Guide
          <ArrowRight size={14} />
        </Link>
      </div>
    </div>
  );
}
