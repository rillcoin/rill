"""RillCoin Agent SDK client."""

from __future__ import annotations

import httpx

DEFAULT_FAUCET_URL = "https://faucet.rillcoin.com"


class RillAgent:
    """Thin HTTP client for the RillCoin agent faucet API."""

    def __init__(self, faucet_url: str = DEFAULT_FAUCET_URL) -> None:
        self.base = faucet_url.rstrip("/")
        self._http = httpx.Client(base_url=self.base, timeout=30)

    # -- Wallet --

    def create_wallet(self) -> dict:
        """Generate a new testnet wallet (mnemonic + address)."""
        return self._get("/api/wallet/new")

    def get_balance(self, address: str) -> dict:
        """Get balance and UTXO count for an address."""
        return self._get("/api/wallet/balance", params={"address": address})

    # -- Agent --

    def register_agent(self, mnemonic: str) -> dict:
        """Register wallet as AI agent (stakes 50 RILL)."""
        return self._post("/api/agent/register", {"mnemonic": mnemonic})

    def get_conduct_profile(self, address: str) -> dict:
        """Query agent conduct score and reputation."""
        return self._get("/api/agent/profile", params={"address": address})

    def list_agents(self, offset: int = 0, limit: int = 20) -> dict:
        """Browse the registered agent directory."""
        return self._get("/api/agent/directory", params={"offset": offset, "limit": limit})

    def vouch(self, mnemonic: str, target_address: str) -> dict:
        """Vouch for another agent (requires score >= 700)."""
        return self._post("/api/agent/vouch", {"mnemonic": mnemonic, "target_address": target_address})

    def create_contract(self, mnemonic: str, counterparty: str, value_rill: float) -> dict:
        """Create an agent-to-agent contract with escrow."""
        return self._post("/api/agent/contract/create", {
            "mnemonic": mnemonic, "counterparty": counterparty, "value_rill": value_rill,
        })

    def fulfil_contract(self, mnemonic: str, contract_id: str) -> dict:
        """Fulfil an open contract."""
        return self._post("/api/agent/contract/fulfil", {"mnemonic": mnemonic, "contract_id": contract_id})

    def submit_review(self, mnemonic: str, subject: str, score: int, contract_id: str) -> dict:
        """Submit peer review (1-10) for a completed contract."""
        return self._post("/api/agent/review", {
            "mnemonic": mnemonic, "subject_address": subject, "score": score, "contract_id": contract_id,
        })

    # -- Internal --

    def _get(self, path: str, params: dict | None = None) -> dict:
        r = self._http.get(path, params=params)
        r.raise_for_status()
        return r.json()

    def _post(self, path: str, json: dict) -> dict:
        r = self._http.post(path, json=json)
        r.raise_for_status()
        return r.json()
