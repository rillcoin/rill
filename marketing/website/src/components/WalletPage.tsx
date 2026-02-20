"use client";

import { useState, useEffect, useCallback } from "react";
import { Copy, ExternalLink, LogOut, Send, Droplets, Loader2, AlertTriangle, Check, Wallet, KeyRound } from "lucide-react";

const FAUCET_API = "https://faucet.rillcoin.com";
const EXPLORER_URL = "https://explorer.rillcoin.com";

type WalletState = "none" | "loaded";

interface BalanceInfo {
  balance_rill: number;
  balance_rills: number;
  utxo_count: number;
}

interface TxResult {
  txid: string;
  amount_rill: number;
  fee_rill?: number;
}

export default function WalletPage() {
  const [walletState, setWalletState] = useState<WalletState>("none");
  const [mnemonic, setMnemonic] = useState("");
  const [address, setAddress] = useState("");
  const [balance, setBalance] = useState<BalanceInfo | null>(null);
  const [loading, setLoading] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [lastTx, setLastTx] = useState<TxResult | null>(null);
  const [copied, setCopied] = useState(false);

  // Send form
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");

  // Restore form
  const [restoreInput, setRestoreInput] = useState("");
  const [showRestore, setShowRestore] = useState(false);

  // Load from localStorage on mount.
  useEffect(() => {
    const saved = localStorage.getItem("rill_wallet");
    if (saved) {
      try {
        const data = JSON.parse(saved);
        if (data.mnemonic && data.address) {
          setMnemonic(data.mnemonic);
          setAddress(data.address);
          setWalletState("loaded");
        }
      } catch {
        // Corrupted data, ignore.
      }
    }
  }, []);

  // Fetch balance when wallet is loaded.
  const fetchBalance = useCallback(async () => {
    if (!address) return;
    try {
      const res = await fetch(`${FAUCET_API}/api/wallet/balance?address=${encodeURIComponent(address)}`);
      if (res.ok) {
        const data = await res.json();
        setBalance(data);
      }
    } catch {
      // Silent fail — balance will retry.
    }
  }, [address]);

  useEffect(() => {
    if (walletState !== "loaded") return;
    fetchBalance();
    const interval = setInterval(fetchBalance, 15000);
    return () => clearInterval(interval);
  }, [walletState, fetchBalance]);

  const saveWallet = (m: string, a: string) => {
    localStorage.setItem("rill_wallet", JSON.stringify({ mnemonic: m, address: a }));
  };

  const clearMessages = () => {
    setError("");
    setSuccess("");
    setLastTx(null);
  };

  const handleCreate = async () => {
    clearMessages();
    setLoading("Creating wallet...");
    try {
      const res = await fetch(`${FAUCET_API}/api/wallet/new`);
      if (!res.ok) throw new Error("Failed to create wallet");
      const data = await res.json();
      setMnemonic(data.mnemonic);
      setAddress(data.address);
      saveWallet(data.mnemonic, data.address);
      setWalletState("loaded");
      setSuccess("Wallet created! Save your mnemonic phrase — it's the only way to restore your wallet.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create wallet");
    } finally {
      setLoading("");
    }
  };

  const handleRestore = async () => {
    clearMessages();
    const phrase = restoreInput.trim();
    const wordCount = phrase.split(/\s+/).length;
    if (wordCount !== 24) {
      setError(`Mnemonic must be 24 words (got ${wordCount})`);
      return;
    }
    setLoading("Restoring wallet...");
    try {
      const res = await fetch(`${FAUCET_API}/api/wallet/derive`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ mnemonic: phrase }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Failed to derive address");
      setMnemonic(phrase);
      setAddress(data.address);
      saveWallet(phrase, data.address);
      setRestoreInput("");
      setShowRestore(false);
      setWalletState("loaded");
      setSuccess("Wallet restored successfully.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to restore wallet");
    } finally {
      setLoading("");
    }
  };

  const handleFaucet = async () => {
    clearMessages();
    setLoading("Requesting from faucet...");
    try {
      const res = await fetch(`${FAUCET_API}/api/faucet`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ address }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Faucet request failed");
      setLastTx({ txid: data.txid, amount_rill: data.amount_rill });
      setSuccess(`Received ${data.amount_rill} RILL from faucet`);
      setTimeout(fetchBalance, 3000);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Faucet request failed");
    } finally {
      setLoading("");
    }
  };

  const handleSend = async () => {
    clearMessages();
    if (!sendTo.trim().startsWith("trill1")) {
      setError("Recipient must be a testnet address (trill1...)");
      return;
    }
    const amount = parseFloat(sendAmount);
    if (isNaN(amount) || amount <= 0) {
      setError("Enter a valid amount");
      return;
    }
    setLoading("Sending transaction...");
    try {
      const res = await fetch(`${FAUCET_API}/api/wallet/send`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ mnemonic, to: sendTo.trim(), amount_rill: amount }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Send failed");
      setLastTx({ txid: data.txid, amount_rill: data.amount_rill, fee_rill: data.fee_rill });
      setSuccess(`Sent ${data.amount_rill} RILL`);
      setSendTo("");
      setSendAmount("");
      setTimeout(fetchBalance, 3000);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Send failed");
    } finally {
      setLoading("");
    }
  };

  const handleLogout = () => {
    localStorage.removeItem("rill_wallet");
    setMnemonic("");
    setAddress("");
    setBalance(null);
    setWalletState("none");
    clearMessages();
    setSendTo("");
    setSendAmount("");
  };

  const copyAddress = async () => {
    await navigator.clipboard.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const truncateAddress = (a: string) =>
    a.length > 20 ? `${a.slice(0, 12)}...${a.slice(-8)}` : a;

  return (
    <>
      {/* ---- Page hero ---- */}
      <section
        className="relative flex flex-col items-center gap-6 px-5 pt-20 pb-8 lg:px-20 lg:pt-28 lg:pb-12 overflow-hidden"
        style={{
          background:
            "linear-gradient(180deg, #020408 0%, #040B16 50%, #020408 100%)",
        }}
      >
        {/* Radial glow — matches Hero section */}
        <div
          className="absolute top-0 left-1/2 -translate-x-1/2 pointer-events-none"
          style={{
            width: 900,
            height: 500,
            background:
              "radial-gradient(ellipse at 50% 0%, #0C2040 0%, transparent 70%)",
          }}
        />

        {/* Badge — same pattern as Hero */}
        <div
          className="relative inline-flex items-center gap-2 self-center rounded px-3 py-1"
          style={{
            backgroundColor: "rgba(34,211,238,0.055)",
            border: "1px solid rgba(34,211,238,0.188)",
          }}
        >
          <span
            className="block rounded-full flex-shrink-0"
            style={{ width: 5, height: 5, backgroundColor: "var(--cyan-400)" }}
          />
          <span
            className="font-mono font-semibold text-[10px] tracking-[2.5px]"
            style={{ color: "var(--cyan-400)" }}
          >
            TESTNET WALLET
          </span>
        </div>

        {/* Heading — Instrument Serif, matching site hierarchy */}
        <h1
          className="relative font-serif text-center leading-none"
          style={{
            fontSize: "clamp(48px, 6vw, 80px)",
            color: "var(--text-primary)",
          }}
        >
          <span className="text-gradient-hero">
            Send RILL.
          </span>
        </h1>

        {/* Sub-heading */}
        <p
          className="relative font-sans text-[16px] lg:text-[18px] leading-[1.65] text-center max-w-md"
          style={{ color: "var(--text-muted)" }}
        >
          Create a wallet, get testnet RILL from the faucet, and send payments — all in your browser.
        </p>

        {/* Divider — matches Hero/Stats section dividers */}
        <div
          className="relative w-full max-w-[640px] mt-4"
          style={{
            height: 1,
            background:
              "linear-gradient(90deg, transparent 0%, rgba(148,163,184,0.082) 50%, transparent 100%)",
          }}
        />
      </section>

      {/* ---- Wallet body ---- */}
      <section
        className="flex flex-col items-center gap-6 px-5 py-10 lg:px-20 lg:py-14"
        style={{ backgroundColor: "var(--void)" }}
      >
        {/* Warning banner */}
        <div
          className="flex items-center gap-3 rounded-lg px-5 py-3 w-full max-w-[560px]"
          style={{
            backgroundColor: "rgba(249,115,22,0.06)",
            border: "1px solid rgba(249,115,22,0.15)",
          }}
        >
          <AlertTriangle size={16} color="var(--orange-400)" className="flex-shrink-0" />
          <p className="font-sans text-[12px] leading-[1.5]" style={{ color: "var(--orange-400)" }}>
            Testnet only. Do not store real value. Mnemonic is saved in this browser.
          </p>
        </div>

        {/* Error/success messages */}
        {error && (
          <div
            className="flex items-center gap-3 rounded-lg px-5 py-3 w-full max-w-[560px]"
            style={{ backgroundColor: "rgba(239,68,68,0.06)", border: "1px solid rgba(239,68,68,0.15)" }}
          >
            <p className="font-sans text-[13px]" style={{ color: "#EF4444" }}>{error}</p>
          </div>
        )}
        {success && (
          <div
            className="flex items-center gap-3 rounded-lg px-5 py-3 w-full max-w-[560px]"
            style={{ backgroundColor: "rgba(16,185,129,0.06)", border: "1px solid rgba(16,185,129,0.15)" }}
          >
            <Check size={14} color="#10B981" className="flex-shrink-0" />
            <p className="font-sans text-[13px]" style={{ color: "#10B981" }}>{success}</p>
          </div>
        )}

        {/* Loading */}
        {loading && (
          <div className="flex items-center gap-2">
            <Loader2 size={16} color="var(--cyan-400)" className="animate-spin" />
            <span className="font-sans text-[13px]" style={{ color: "var(--text-secondary)" }}>{loading}</span>
          </div>
        )}

        {walletState === "none" ? (
          /* ---- Create / Restore ---- */
          <div className="flex flex-col items-center gap-6 w-full max-w-[560px]">
            <div className="flex flex-col sm:flex-row gap-4 w-full">
              {/* Create button — primary CTA, orange gradient like Hero */}
              <button
                onClick={handleCreate}
                disabled={!!loading}
                className="flex-1 flex items-center justify-center gap-2.5 rounded-lg px-6 py-3.5 font-sans font-semibold text-[14px] transition-opacity hover:opacity-90 disabled:opacity-50"
                style={{
                  color: "#0A0F1A",
                  background: "linear-gradient(135deg, #F97316 0%, #FB923C 100%)",
                  boxShadow: "0 4px 20px rgba(249,115,22,0.2)",
                }}
              >
                <Wallet size={16} />
                Create New Wallet
              </button>
              {/* Restore — secondary, outline */}
              <button
                onClick={() => { setShowRestore(!showRestore); clearMessages(); }}
                disabled={!!loading}
                className="flex-1 flex items-center justify-center gap-2.5 rounded-lg px-6 py-3.5 font-sans font-medium text-[14px] transition-opacity hover:opacity-90 disabled:opacity-50"
                style={{
                  color: "var(--blue-500)",
                  border: "1px solid rgba(59,130,246,0.22)",
                }}
              >
                <KeyRound size={16} />
                Restore from Mnemonic
              </button>
            </div>

            {showRestore && (
              <div
                className="flex flex-col gap-4 rounded-xl p-6 w-full"
                style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
              >
                <label className="font-mono text-[10px] tracking-[2px]" style={{ color: "var(--text-faint)" }}>
                  RECOVERY PHRASE
                </label>
                <textarea
                  value={restoreInput}
                  onChange={e => setRestoreInput(e.target.value)}
                  placeholder="Enter your 24 words separated by spaces..."
                  rows={3}
                  className="rounded-lg px-4 py-3 font-mono text-[13px] resize-none focus:outline-none placeholder:text-[var(--text-faint)]"
                  style={{
                    backgroundColor: "var(--base)",
                    border: "1px solid var(--border-subtle)",
                    color: "var(--text-primary)",
                  }}
                />
                <button
                  onClick={handleRestore}
                  disabled={!!loading || !restoreInput.trim()}
                  className="rounded-lg px-6 py-3 font-sans font-medium text-[14px] transition-opacity hover:opacity-90 disabled:opacity-50"
                  style={{
                    color: "var(--cyan-400)",
                    backgroundColor: "rgba(34,211,238,0.071)",
                    border: "1px solid rgba(34,211,238,0.271)",
                  }}
                >
                  Restore Wallet
                </button>
              </div>
            )}

            {/* Trust line — matches Hero/CTA pattern */}
            <p
              className="font-mono text-[11px] text-center"
              style={{ color: "#1E2A38" }}
            >
              Ed25519&nbsp;&nbsp;&middot;&nbsp;&nbsp;HD Derivation&nbsp;&nbsp;&middot;&nbsp;&nbsp;Client-side keys
            </p>
          </div>
        ) : (
          /* ---- Wallet loaded ---- */
          <div className="flex flex-col gap-5 w-full max-w-[560px]">

            {/* Balance card — featured, gradient bg like Bento decay card */}
            <div
              className="flex flex-col gap-5 rounded-xl p-6 lg:p-8"
              style={{
                background: "radial-gradient(ellipse at 90% 10%, #0C2448 0%, #060C18 60%)",
                border: "1px solid var(--border-blue)",
              }}
            >
              {/* Balance — large, gradient text like Stats section */}
              <div className="flex flex-col gap-1">
                <span className="font-mono font-medium text-[10px] tracking-[2px]" style={{ color: "var(--text-faint)" }}>
                  BALANCE
                </span>
                <div className="flex items-baseline gap-3">
                  <span
                    className="font-mono font-bold text-gradient-blue-cyan leading-none"
                    style={{ fontSize: "clamp(36px, 4vw, 56px)" }}
                  >
                    {balance ? balance.balance_rill.toFixed(2) : "—"}
                  </span>
                  <span className="font-mono font-medium text-[14px]" style={{ color: "var(--text-dim)" }}>RILL</span>
                </div>
                {balance && (
                  <span className="font-mono text-[11px] mt-1" style={{ color: "var(--text-dim)" }}>
                    {balance.utxo_count} UTXO{balance.utxo_count !== 1 ? "s" : ""}
                  </span>
                )}
              </div>

              {/* Divider */}
              <div style={{ height: 1, backgroundColor: "rgba(59,130,246,0.094)" }} />

              {/* Address */}
              <div className="flex flex-col gap-1.5">
                <span className="font-mono font-medium text-[10px] tracking-[2px]" style={{ color: "var(--text-faint)" }}>
                  YOUR ADDRESS
                </span>
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[13px] break-all" style={{ color: "var(--text-secondary)" }}>
                    <span className="hidden sm:inline">{address}</span>
                    <span className="sm:hidden">{truncateAddress(address)}</span>
                  </span>
                  <button onClick={copyAddress} className="flex-shrink-0 p-1 transition-opacity hover:opacity-70">
                    {copied
                      ? <Check size={14} color="#10B981" />
                      : <Copy size={14} color="var(--text-dim)" />
                    }
                  </button>
                </div>
              </div>

              {/* Faucet button — cyan outline, consistent with site CTAs */}
              <button
                onClick={handleFaucet}
                disabled={!!loading}
                className="flex items-center justify-center gap-2 rounded-lg px-5 py-3 font-sans font-medium text-[13px] transition-opacity hover:opacity-90 disabled:opacity-50"
                style={{
                  color: "var(--cyan-400)",
                  backgroundColor: "rgba(34,211,238,0.071)",
                  border: "1px solid rgba(34,211,238,0.271)",
                }}
              >
                <Droplets size={14} />
                Request from Faucet
              </button>
            </div>

            {/* Send card — raised surface like Bento PoW card */}
            <div
              className="flex flex-col gap-4 rounded-xl p-6 lg:p-8"
              style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
            >
              <span className="font-mono font-medium text-[10px] tracking-[2px]" style={{ color: "var(--text-faint)" }}>
                SEND RILL
              </span>

              <div className="flex flex-col gap-3">
                <input
                  type="text"
                  value={sendTo}
                  onChange={e => setSendTo(e.target.value)}
                  placeholder="Recipient address (trill1...)"
                  className="rounded-lg px-4 py-3 font-mono text-[13px] focus:outline-none placeholder:text-[var(--text-faint)]"
                  style={{
                    backgroundColor: "var(--base)",
                    border: "1px solid var(--border-subtle)",
                    color: "var(--text-primary)",
                  }}
                />
                <input
                  type="text"
                  inputMode="decimal"
                  value={sendAmount}
                  onChange={e => setSendAmount(e.target.value)}
                  placeholder="Amount (RILL)"
                  className="rounded-lg px-4 py-3 font-mono text-[13px] focus:outline-none placeholder:text-[var(--text-faint)]"
                  style={{
                    backgroundColor: "var(--base)",
                    border: "1px solid var(--border-subtle)",
                    color: "var(--text-primary)",
                  }}
                />
              </div>

              {/* Send button — solid blue, primary action */}
              <button
                onClick={handleSend}
                disabled={!!loading || !sendTo || !sendAmount}
                className="flex items-center justify-center gap-2 rounded-lg px-5 py-3 font-sans font-semibold text-[14px] transition-opacity hover:opacity-90 disabled:opacity-50"
                style={{
                  color: "#F1F5F9",
                  backgroundColor: "var(--blue-500)",
                  boxShadow: "0 4px 16px rgba(59,130,246,0.2)",
                }}
              >
                <Send size={14} />
                Send
              </button>
            </div>

            {/* Last transaction */}
            {lastTx && (
              <div
                className="flex flex-col gap-2 rounded-xl p-5"
                style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
              >
                <span className="font-mono font-medium text-[10px] tracking-[2px]" style={{ color: "var(--text-faint)" }}>
                  LAST TRANSACTION
                </span>
                <a
                  href={`${EXPLORER_URL}/tx/${lastTx.txid}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 font-mono text-[12px] break-all transition-opacity hover:opacity-70"
                  style={{ color: "var(--cyan-400)" }}
                >
                  {lastTx.txid}
                  <ExternalLink size={12} className="flex-shrink-0" />
                </a>
                <div className="flex gap-4">
                  <span className="font-mono text-[12px]" style={{ color: "var(--text-dim)" }}>
                    {lastTx.amount_rill} RILL
                  </span>
                  {lastTx.fee_rill !== undefined && (
                    <span className="font-mono text-[12px]" style={{ color: "var(--text-dim)" }}>
                      fee: {lastTx.fee_rill} RILL
                    </span>
                  )}
                </div>
              </div>
            )}

            {/* Mnemonic + disconnect */}
            <div className="flex flex-col gap-4">
              <MnemonicReveal mnemonic={mnemonic} />
              <button
                onClick={handleLogout}
                className="flex items-center justify-center gap-2 rounded-lg px-5 py-2.5 font-sans text-[13px] transition-opacity hover:opacity-70"
                style={{ color: "var(--text-dim)" }}
              >
                <LogOut size={14} />
                Disconnect Wallet
              </button>
            </div>
          </div>
        )}
      </section>
    </>
  );
}

function MnemonicReveal({ mnemonic }: { mnemonic: string }) {
  const [visible, setVisible] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(mnemonic);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className="flex flex-col gap-3 rounded-xl p-5"
      style={{ backgroundColor: "var(--raised)", border: "1px solid var(--border-subtle)" }}
    >
      <div className="flex items-center justify-between">
        <span className="font-mono font-medium text-[10px] tracking-[2px]" style={{ color: "var(--text-faint)" }}>
          RECOVERY PHRASE
        </span>
        <button
          onClick={() => setVisible(!visible)}
          className="font-mono text-[11px] tracking-[1px] transition-opacity hover:opacity-70"
          style={{ color: "var(--text-dim)" }}
        >
          {visible ? "HIDE" : "REVEAL"}
        </button>
      </div>
      {visible && (
        <>
          <div
            className="grid grid-cols-3 sm:grid-cols-4 gap-x-4 gap-y-2 rounded-lg p-4"
            style={{ backgroundColor: "var(--base)" }}
          >
            {mnemonic.split(" ").map((word, i) => (
              <span key={i} className="font-mono text-[12px]" style={{ color: "var(--text-secondary)" }}>
                <span style={{ color: "var(--text-faint)" }}>{i + 1}.</span> {word}
              </span>
            ))}
          </div>
          <button
            onClick={handleCopy}
            className="flex items-center gap-2 font-mono text-[11px] tracking-[0.5px] transition-opacity hover:opacity-70"
            style={{ color: "var(--text-dim)" }}
          >
            {copied ? <Check size={12} color="#10B981" /> : <Copy size={12} />}
            {copied ? "Copied" : "Copy mnemonic"}
          </button>
        </>
      )}
    </div>
  );
}
