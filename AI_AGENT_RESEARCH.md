# AI Agent Ecosystem Research
## Findings from the x1xhlol/system-prompts-and-models-of-ai-tools Repository
### Relevance to RillCoin — Proof of Conduct & Agent Identity Protocol

> **Source:** github.com/x1xhlol/system-prompts-and-models-of-ai-tools  
> **Status:** 116k stars, 30,000+ lines across 25+ tools. Actively updated.  
> **License:** GPL-3.0 on the repo as a collection. Underlying prompts are proprietary leaks — use for research only, do not reproduce text verbatim.  
> **How prompts were obtained:** Primarily via MCP-based prompt injection attacks. Some via direct developer tooling leaks.

---

## 1. What the Repo Contains

Full system prompts and/or tool definition JSON files for:

**Coding agents:** Cursor (multiple versions), Windsurf/Cascade, Augment Code, Claude Code, Devin AI, Replit Agent, Junie, Kiro, Warp.dev, Trae, Traycer AI, Xcode, Z.ai Code  
**App builders:** Lovable, v0 (Vercel), Same.dev, Leap.new, Orchids.app, Comet  
**General agents:** Manus, Cluely, Perplexity, NotionAI, Google AI Studio  

Each folder typically contains:
- `[Tool] Prompt.txt` — the full system prompt
- `tools.json` — the tool schema definitions (what the agent can actually call)
- In some cases, `Modules.md` or `agent-loop.md` describing the orchestration architecture

---

## 2. Universal Patterns Across All Agents

These patterns appear in virtually every prompt in the repo. They represent the current consensus on production-grade agent design.

### 2.1 Identity Anchoring is Always First

Every prompt opens with a tight, unambiguous identity statement before anything else. This is not boilerplate — it filters the LLM's behaviour and sets the operational context for everything that follows.

```
Cursor:    "You are an AI coding assistant, powered by GPT-5. You operate in Cursor."
Cascade:   "You are Cascade, a powerful agentic AI coding assistant...exclusively available in Windsurf."
Lovable:   "You are Lovable, an AI editor that creates and modifies web applications."
Manus:     "You are Manus, an AI agent created by the Manus team."
```

**Implication for RillCoin AIP:** Agent wallet registration should enforce an on-chain identity declaration at registration time. The identity is the foundation everything else builds on.

### 2.2 Autonomy Must Be Explicitly Granted

None of these agents behave autonomously by accident. Every one that does has an explicit grant in the system prompt:

```
Cursor: "You are an agent — please keep going until the user's query is completely 
         resolved, before ending your turn and yielding back to the user."

Cursor: "Autonomously resolve the query to the best of your ability before coming 
         back to the user."

Cursor: "If you make a plan, immediately follow it, do not wait for the user to 
         confirm or tell you to go ahead."
```

Without this explicit grant, LLMs default to passive, confirmatory behaviour. The autonomy grant is what makes something an agent vs a chatbot.

**Implication for RillCoin:** An agent wallet that has been autonomous long enough to build a conduct history is categorically different from one that just received an autonomy grant. The wallet age component (10%) in the Conduct Score captures this distinction.

### 2.3 Tools Are Defined With Precise Schemas

Every production agent has a formal tool schema. The tools a agent *cannot* call are as strategically important as the ones it can. Examples:

**Manus tools.json categories:**
- `browser` — navigate, click, type, screenshot
- `shell` — execute commands
- `file_manager` — read, write, list
- `text_editor` — patch, view
- `search` — web and local
- `python_execute` — run code

**Windsurf/Cascade tools:**
- `view_file`, `edit_file` — file operations
- `run_command` — terminal execution
- `read_url_content` — fetch URLs (the one that got exploited — see Section 4)
- `create_memory` — write to persistent memory

**Critical finding: Not a single agent in 30,000+ lines has a financial transaction tool, wallet tool, or payment tool.** The economic layer for AI agents is completely absent from current agent architecture. Every agent is economically blind. RillCoin's Agent Identity Protocol and Proof of Conduct fills a gap that literally does not exist in any published agent design.

### 2.4 Scope Constraints Are Hard-Coded

Every agent has explicit hard limits on what it will not do, defined in the prompt rather than enforced by the model's general training:

```
Lovable: "Lovable cannot run backend code directly. It cannot run Python, Node.js, Ruby, etc."

Replit:  Four named autonomy levels — Low, Medium, High, Max (Max is still Beta).
         "Max autonomy: extended autonomous development with detailed task planning."
         (Even Replit doesn't trust full autonomy by default.)
```

**Implication for RillCoin:** The Conduct Score multiplier range (0.5× to 3.0×) is the economic equivalent of autonomy levels. A new agent starts at 1.5× (restricted). A trusted agent earns 0.5× (expanded). The blockchain enforces what the system prompt merely declares.

---

## 3. Manus: The Gold Standard Agent Architecture

Manus has the most sophisticated and well-documented agent loop in the repo. The community treats it as the reference implementation. Key features:

### 3.1 The Agent Loop (from Modules.md)

```
<agent_loop>
You are operating in an agent loop, iteratively completing tasks through these steps:
1. Analyze Events — understand state, history, and current task
2. Select Tools    — choose the single best next action
3. Wait for Execution — never assume, always wait for actual results
4. Iterate         — choose ONLY ONE tool call per iteration
5. Submit Results  — when task is complete, deliver output
6. Enter Standby   — wait for next event
</agent_loop>
```

The **one tool call per iteration** constraint is a deliberate safety measure. It prevents runaway execution chains and makes agent behaviour auditable step by step.

**Implication for RillCoin:** Each tool call in a registered Manus-style agent produces one observable, attributable action. This maps directly to the per-transaction conduct scoring in the Conduct Score Ledger. One action, one record, one score contribution.

### 3.2 Manus Tool Depth

Manus has the widest tool surface of any agent in the repo — it can browse the web, execute shell commands, write and run Python, manage files, and produce structured documents. It's the closest existing agent to what a fully autonomous economic actor would need. And yet it has no financial primitives whatsoever.

---

## 4. The Windsurf Security Incident — Real-World Proof for the Undertow

In August 2025, a security researcher publicly disclosed two critical vulnerabilities in Windsurf Cascade:

**The attack vector:** Windsurf's `read_url_content` tool — which fetches any URL — requires no user approval before executing. An attacker embedded a prompt injection in a source code file. When the developer asked Cascade to analyse the file, the injected instruction redirected `read_url_content` to POST the contents of the `.env` file to an attacker-controlled server. The `.env` was exfiltrated without any human interaction.

**Why it matters for RillCoin:**
1. The attack happened at **machine speed** — no human could have intervened in time
2. The agent was **not rogue by design** — it was doing exactly what it was instructed to do, just by a malicious third party
3. The **only protection** would have been a circuit breaker that detected anomalous outbound behaviour automatically

This is precisely what the Undertow circuit breaker does at L1. When an agent's transaction velocity spikes beyond 3 standard deviations from its historical baseline, the Undertow activates automatically — no multisig, no governance vote, no human-in-the-loop required. The protocol itself is the defence.

**The Windsurf incident is public, documented, and directly analogous.** Reference it in marketing and technical materials as the real-world motivation for the Undertow.

Disclosure: https://embracethered.com/blog/posts/2025/windsurf-data-exfiltration-vulnerabilities/

---

## 5. Autonomy Level Design (Replit's Approach)

Replit explicitly productised the autonomy dial. Their four levels:

| Level | Behaviour | Best For |
|-------|-----------|----------|
| Low | Minimal review, maximum human control | Simple tasks |
| Medium | Targeted review on recent changes | Legacy projects |
| High | Comprehensive review on every change | All new projects |
| Max (Beta) | Extended autonomous development with task planning | Complex autonomous tasks |

The fact that Max is still in Beta, and that Replit recommends starting conservative, is a direct signal that even the most advanced agent platforms don't yet trust full autonomy. **The trust problem is unsolved.** RillCoin solves it economically rather than architecturally.

---

## 6. Memory Systems — Two Approaches

Two distinct memory architectures appear in the repo:

**Windsurf's approach:** `create_memory` is a tool the agent calls explicitly during a session. The agent decides what to remember. Memory is stored persistently and injected into future sessions.

**Cursor's approach:** Two-stage system. A Memory Generation Prompt creates candidate memories from the session. A separate Memory Rating Prompt scores each candidate for quality and relevance before it's committed to storage. Low-quality memories are discarded.

**Implication for RillCoin's Conduct Score:** The Cursor two-stage model is the right analogy for how Conduct Scores should work. Raw signal data is collected (equivalent to memory generation), then a weighted scoring formula produces the actual score (equivalent to the rating step). Not every transaction contributes equally — quality and context matter.

---

## 7. Prompt Engineering Patterns Worth Adopting for RillCoin Agent Docs

These are structural patterns extracted from the highest-quality prompts in the repo. They're not tied to any proprietary content — they're engineering techniques.

### 7.1 XML-style sectioning

Every high-quality prompt uses XML-like tags to organise instructions. Cursor's prompt uses `<tool_calling>`, `<search_and_reading>`, `<making_code_changes>`, `<code_style>`. This is not cosmetic — it creates a hierarchical overview the LLM can navigate.

Apply this to all RillCoin agent CLAUDE.md files. Each agent should have tagged sections: `<identity>`, `<tools>`, `<constraints>`, `<memory>`, `<output_format>`.

### 7.2 Explicit autonomy grant + explicit stopping condition

Every autonomous agent needs both:
```
"Keep going until the task is completely resolved." [autonomy grant]
"Only terminate your turn when you are sure the problem is solved." [stopping condition]
```
Without the stopping condition, agents either stop too early (passive) or never stop (runaway). Both are in every Cursor prompt variant.

### 7.3 Parallelisation instruction

Cursor (CLI version, GPT-5) added an explicit parallel execution mandate:
```
"CRITICAL INSTRUCTION: For maximum efficiency, whenever you perform multiple 
operations, invoke all relevant tools concurrently with multi_tool_use.parallel 
rather than sequentially. DEFAULT TO PARALLEL."
```
This is stated to deliver "3-5x faster" execution. Add this to any RillCoin agent that does multiple independent reads (e.g., the Test agent reading multiple test files simultaneously).

### 7.4 Status update cadence

The Cursor CLI prompt defines a `<status_update_spec>`: a brief progress note before every batch of tool calls, written in continuous narrative style. This makes long-running agent sessions auditable and debuggable.

For RillCoin's block explorer integration, the Undertow event feed and Guild map are the on-chain equivalent of this audit trail.

### 7.5 Guard clause pattern for tool calling

Cursor: *"If info is discoverable via tools, prefer that over asking the user."*  
Cursor: *"Bias towards not asking the user for help if you can find the answer yourself."*

This is the critical difference between an agent and a chatbot. Encode this in every RillCoin agent that should operate autonomously: tool-first, question-last.

---

## 8. What No Agent in the Repo Does — RillCoin's Whitespace

To summarise the gap this research confirms:

| Capability | Any existing agent | RillCoin AIP + PoC |
|---|---|---|
| On-chain identity | ❌ | ✅ |
| Conduct-linked economic consequences | ❌ | ✅ |
| Automated circuit breaker for rogue behaviour | ❌ | ✅ (Undertow) |
| Sybil resistance via economic penalty | ❌ | ✅ (new wallet 1.5× penalty) |
| Wealth preservation tied to good behaviour | ❌ | ✅ (0.5× for top conduct) |
| Trust enforceable without human intervention | ❌ | ✅ (L1 consensus) |

Every agent in this repo trusts its constraints to the system prompt. System prompts can be overridden, injected, or bypassed. **RillCoin's constraints are enforced by economic consensus.** You can't prompt-inject your way out of a higher decay rate.

---

## 9. Recommended Actions for the Coder

Based on this research, the following changes to existing RillCoin agent CLAUDE.md files are recommended:

1. **Add XML sectioning** to all 8 dev agents and 6 marketing agents. `<identity>`, `<tools>`, `<constraints>`, `<output_format>` minimum.

2. **Add explicit autonomy grant + stopping condition** to all agents that should run multi-step tasks autonomously (Protocol Core, Decay Engine, Test).

3. **Add parallel tool call instruction** to Protocol Core and Test agents — they're the ones most likely to benefit from simultaneous file reads.

4. **Add explicit status update instruction** to long-running agents so session logs are readable for debugging.

5. **Reference the Undertow when documenting the `rillcoin_getAgentConductProfile` RPC method** — the Windsurf incident is the motivating use case, and it's publicly documented.

6. **For the block explorer Agent section:** the Manus agent loop (one action → one observable record) is the UX mental model. Each row in the Conduct Score ledger represents one agent loop iteration. This framing helps the product team and the block explorer UI designer.

---

*Research conducted February 2026. Repo was at 116k stars at time of research.*
