"""LangChain tool adapters for RillCoin agent operations."""

from __future__ import annotations

try:
    from langchain_core.tools import tool
except ImportError:
    raise ImportError("Install langchain-core: pip install rill-agent-sdk[langchain]")

from .client import RillAgent

_agent = RillAgent()


@tool
def rill_create_wallet() -> dict:
    """Generate a new RillCoin testnet wallet with mnemonic and address."""
    return _agent.create_wallet()


@tool
def rill_register_agent(mnemonic: str) -> dict:
    """Register a wallet as an AI agent on RillCoin (stakes 50 RILL)."""
    return _agent.register_agent(mnemonic)


@tool
def rill_get_conduct_profile(address: str) -> dict:
    """Get Proof of Conduct profile for a RillCoin address."""
    return _agent.get_conduct_profile(address)


@tool
def rill_vouch_for_agent(mnemonic: str, target_address: str) -> dict:
    """Vouch for another agent (requires conduct score >= 700)."""
    return _agent.vouch(mnemonic, target_address)


@tool
def rill_create_contract(mnemonic: str, counterparty: str, value_rill: float) -> dict:
    """Create an agent-to-agent contract with escrow value."""
    return _agent.create_contract(mnemonic, counterparty, value_rill)


@tool
def rill_fulfil_contract(mnemonic: str, contract_id: str) -> dict:
    """Fulfil an open agent contract."""
    return _agent.fulfil_contract(mnemonic, contract_id)


@tool
def rill_submit_review(mnemonic: str, subject: str, score: int, contract_id: str) -> dict:
    """Submit peer review (1-10) for a completed contract."""
    return _agent.submit_review(mnemonic, subject, score, contract_id)
