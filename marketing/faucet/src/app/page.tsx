"use client";

import { useState, useEffect } from "react";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface FaucetStatus {
  balance_rill: number;
  height: number;
  network: string;
  amount_per_claim_rill: number;
  cooldown_secs: number;
}

type SubmitState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "success"; txid: string; amount_rill: number; address: string }
  | { kind: "error"; message: string }
  | { kind: "rate_limit" };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncateTxid(txid: string): string {
  if (txid.length <= 32) return txid;
  return `${txid.slice(0, 16)}...${txid.slice(-16)}`;
}

function formatNumber(n: number): string {
  return n.toLocaleString("en-US");
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function SkeletonBar({ width = "100%", height = 16 }: { width?: string | number; height?: number }) {
  return (
    <div
      className="animate-pulse rounded"
      style={{
        width,
        height,
        backgroundColor: "var(--text-faint)",
        opacity: 0.4,
      }}
    />
  );
}

function StatusBar({ status }: { status: FaucetStatus | null; }) {
  if (!status) {
    return (
      <div className="flex flex-wrap items-center justify-center gap-4 mt-6">
        <SkeletonBar width={110} height={32} />
        <SkeletonBar width={4} height={4} />
        <SkeletonBar width={140} height={32} />
        <SkeletonBar width={4} height={4} />
        <SkeletonBar width={120} height={32} />
      </div>
    );
  }

  const Chip = ({ label, value }: { label: string; value: string }) => (
    <div className="flex flex-col items-center gap-0.5">
      <span
        className="font-mono uppercase tracking-widest"
        style={{ fontSize: 9, color: "var(--text-faint)" }}
      >
        {label}
      </span>
      <span
        className="font-mono font-medium"
        style={{ fontSize: 14, color: "var(--blue-400)" }}
      >
        {value}
      </span>
    </div>
  );

  const Dot = () => (
    <span
      className="font-mono"
      style={{ fontSize: 14, color: "var(--text-faint)", lineHeight: 1 }}
    >
      ·
    </span>
  );

  return (
    <div className="flex flex-wrap items-center justify-center gap-4 mt-6">
      <Chip label="HEIGHT" value={formatNumber(status.height)} />
      <Dot />
      <Chip label="BALANCE" value={`${formatNumber(status.balance_rill)} RILL`} />
      <Dot />
      <Chip label="DRIP" value={`${status.amount_per_claim_rill} RILL / 24h`} />
    </div>
  );
}

function ResultCard({ state }: { state: SubmitState }) {
  if (state.kind === "idle" || state.kind === "loading") return null;

  if (state.kind === "success") {
    return (
      <div
        className="mt-4 rounded-lg px-5 py-4 text-center"
        style={{
          backgroundColor: "#060E1C",
          border: "1px solid rgba(16,185,129,0.2)",
        }}
      >
        <div
          className="font-serif mb-1"
          style={{ fontSize: 28, color: "#10B981" }}
        >
          Sent {state.amount_rill} RILL
        </div>
        <div
          className="font-mono"
          style={{ fontSize: 13, color: "#10B981", opacity: 0.75 }}
        >
          {state.address}
        </div>
        <div
          className="font-mono mt-1"
          style={{ fontSize: 13, color: "#10B981", opacity: 0.5 }}
        >
          TxID: {truncateTxid(state.txid)}
        </div>
      </div>
    );
  }

  if (state.kind === "rate_limit") {
    return (
      <div
        className="mt-4 rounded-lg px-5 py-4 text-center font-sans"
        style={{
          backgroundColor: "#060E1C",
          border: "1px solid rgba(245,158,11,0.2)",
          fontSize: 15,
          color: "#F59E0B",
        }}
      >
        Already claimed — try again in 24h
      </div>
    );
  }

  // error
  return (
    <div
      className="mt-4 rounded-lg px-5 py-4 text-center font-sans"
      style={{
        backgroundColor: "#060E1C",
        border: "1px solid rgba(239,68,68,0.2)",
        fontSize: 15,
        color: "#EF4444",
      }}
    >
      {state.message}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function FaucetPage() {
  const [address, setAddress] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [submitState, setSubmitState] = useState<SubmitState>({ kind: "idle" });
  const [status, setStatus] = useState<FaucetStatus | null>(null);
  const [buttonHovered, setButtonHovered] = useState(false);

  // Fetch network status on mount
  useEffect(() => {
    fetch("https://faucet.rillcoin.com/api/status")
      .then((r) => r.json())
      .then((data: FaucetStatus) => setStatus(data))
      .catch(() => {
        // Leave status as null — skeletons stay visible
      });
  }, []);

  function validateAddress(value: string): boolean {
    if (!value.startsWith("trill1")) {
      setValidationError("Address must start with trill1");
      return false;
    }
    setValidationError(null);
    return true;
  }

  function handleAddressChange(e: React.ChangeEvent<HTMLInputElement>) {
    const value = e.target.value;
    setAddress(value);
    // Clear validation error while typing if it now passes
    if (validationError && value.startsWith("trill1")) {
      setValidationError(null);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validateAddress(address)) return;

    setSubmitState({ kind: "loading" });

    try {
      const res = await fetch("https://faucet.rillcoin.com/api/faucet", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ address }),
      });

      const data = await res.json();

      if (res.ok) {
        setSubmitState({
          kind: "success",
          txid: data.txid,
          amount_rill: data.amount_rill,
          address: data.address,
        });
      } else if (res.status === 429) {
        setSubmitState({ kind: "rate_limit" });
      } else {
        setSubmitState({
          kind: "error",
          message: data.error ?? "Something went wrong. Please try again.",
        });
      }
    } catch {
      setSubmitState({
        kind: "error",
        message: "Network error. Please check your connection and try again.",
      });
    }
  }

  const isInFlight = submitState.kind === "loading";
  const buttonDisabled = isInFlight || address.trim().length === 0;

  const buttonStyle: React.CSSProperties = {
    background: "linear-gradient(135deg, #F97316 0%, #FB923C 100%)",
    color: "#0A0F1A",
    fontFamily: "var(--font-inter), system-ui, sans-serif",
    fontSize: 14,
    fontWeight: 600,
    borderRadius: "0 8px 8px 0",
    padding: "12px 24px",
    border: "none",
    cursor: buttonDisabled ? "not-allowed" : "pointer",
    opacity: buttonDisabled ? 0.5 : 1,
    whiteSpace: "nowrap" as const,
    transition: "box-shadow 0.15s ease",
    boxShadow: buttonHovered && !buttonDisabled
      ? "0 4px 20px rgba(249,115,22,0.35)"
      : "none",
  };

  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column", backgroundColor: "var(--void)" }}>
      <Nav />

      {/* ------------------------------------------------------------------ */}
      {/* Hero / Form Section                                                 */}
      {/* ------------------------------------------------------------------ */}
      <section
        style={{
          flex: 1,
          minHeight: "60vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          padding: "60px 20px",
          background: "radial-gradient(ellipse at 50% 40%, #0C2448 0%, transparent 60%)",
          position: "relative",
        }}
      >
        <div style={{ width: "100%", maxWidth: 512, textAlign: "center" }}>
          {/* Eyebrow */}
          <div
            className="font-mono uppercase"
            style={{
              fontSize: 11,
              color: "rgba(34,211,238,0.5)",
              letterSpacing: "3px",
              marginBottom: 20,
            }}
          >
            TESTNET FAUCET
          </div>

          {/* Headline */}
          <h1
            className="font-serif"
            style={{
              fontSize: 64,
              lineHeight: 1,
              color: "var(--text-primary)",
              margin: "0 0 20px 0",
            }}
          >
            Get Testnet RILL
          </h1>

          {/* Sub-headline */}
          <p
            className="font-sans"
            style={{
              fontSize: 16,
              color: "var(--text-muted)",
              marginBottom: 36,
              lineHeight: 1.6,
            }}
          >
            10 RILL per address, every 24 hours. No forms. No waitlist.
          </p>

          {/* Form */}
          <form onSubmit={handleSubmit} noValidate>
            {/* Input + Button row */}
            <div
              className="faucet-input-row"
              style={{
                display: "flex",
                flexDirection: "row",
                width: "100%",
              }}
            >
              <input
                type="text"
                value={address}
                onChange={handleAddressChange}
                placeholder="trill1..."
                spellCheck={false}
                autoComplete="off"
                style={{
                  flex: 1,
                  fontFamily: "var(--font-jetbrains-mono), monospace",
                  fontSize: 14,
                  backgroundColor: "#060E1C",
                  border: validationError
                    ? "1px solid rgba(239,68,68,0.4)"
                    : "1px solid rgba(34,211,238,0.2)",
                  borderRight: "none",
                  borderRadius: "8px 0 0 8px",
                  padding: "12px 16px",
                  color: "var(--text-primary)",
                  outline: "none",
                  width: "100%",
                }}
                onFocus={(e) => {
                  e.currentTarget.style.border = validationError
                    ? "1px solid rgba(239,68,68,0.5)"
                    : "1px solid rgba(34,211,238,0.5)";
                  e.currentTarget.style.borderRight = "none";
                }}
                onBlur={(e) => {
                  e.currentTarget.style.border = validationError
                    ? "1px solid rgba(239,68,68,0.4)"
                    : "1px solid rgba(34,211,238,0.2)";
                  e.currentTarget.style.borderRight = "none";
                }}
              />
              <button
                type="submit"
                disabled={buttonDisabled}
                style={buttonStyle}
                onMouseEnter={() => setButtonHovered(true)}
                onMouseLeave={() => setButtonHovered(false)}
              >
                {isInFlight ? "Sending…" : "Request RILL →"}
              </button>
            </div>

            {/* Validation error */}
            {validationError && (
              <div
                className="font-sans text-left"
                style={{
                  fontSize: 13,
                  color: "#EF4444",
                  marginTop: 8,
                  paddingLeft: 2,
                }}
              >
                {validationError}
              </div>
            )}
          </form>

          {/* Result card */}
          <ResultCard state={submitState} />

          {/* Network status bar */}
          <StatusBar status={status} />
        </div>
      </section>

      {/* ------------------------------------------------------------------ */}
      {/* How It Works Section                                                */}
      {/* ------------------------------------------------------------------ */}
      <section
        className="px-5 lg:px-20"
        style={{ paddingTop: 80, paddingBottom: 80 }}
      >
        {/* Section label */}
        <div
          className="font-mono uppercase text-center"
          style={{
            fontSize: 10,
            color: "var(--text-faint)",
            letterSpacing: "3px",
            marginBottom: 48,
          }}
        >
          HOW IT WORKS
        </div>

        {/* Steps grid */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
            gap: 0,
            maxWidth: 960,
            margin: "0 auto",
          }}
        >
          {/* Step 01 */}
          <div
            style={{
              padding: "0 40px 40px 0",
              borderRight: "1px solid var(--border-subtle)",
            }}
            className="step-item"
          >
            <div
              className="font-mono text-gradient-blue-cyan"
              style={{ fontSize: 48, lineHeight: 1, opacity: 0.6, marginBottom: 16 }}
            >
              01
            </div>
            <div
              className="font-sans font-semibold"
              style={{ fontSize: 16, color: "var(--text-primary)", marginBottom: 8 }}
            >
              Get a wallet
            </div>
            <div
              className="font-sans"
              style={{ fontSize: 14, color: "var(--text-dim)", lineHeight: 1.6 }}
            >
              Download the Rill CLI and generate a testnet address starting with{" "}
              <code
                className="font-mono"
                style={{ fontSize: 13, color: "var(--blue-400)" }}
              >
                trill1
              </code>
              .
            </div>
          </div>

          {/* Vertical divider between 01 and 02 handled by borderRight above */}

          {/* Step 02 */}
          <div
            style={{
              padding: "0 40px 40px 40px",
              borderRight: "1px solid var(--border-subtle)",
            }}
            className="step-item"
          >
            <div
              className="font-mono text-gradient-blue-cyan"
              style={{ fontSize: 48, lineHeight: 1, opacity: 0.6, marginBottom: 16 }}
            >
              02
            </div>
            <div
              className="font-sans font-semibold"
              style={{ fontSize: 16, color: "var(--text-primary)", marginBottom: 8 }}
            >
              Request RILL
            </div>
            <div
              className="font-sans"
              style={{ fontSize: 14, color: "var(--text-dim)", lineHeight: 1.6 }}
            >
              Paste your address above. 10 RILL lands in your wallet within seconds.
            </div>
          </div>

          {/* Step 03 */}
          <div
            style={{
              padding: "0 0 40px 40px",
            }}
            className="step-item"
          >
            <div
              className="font-mono text-gradient-blue-cyan"
              style={{ fontSize: 48, lineHeight: 1, opacity: 0.6, marginBottom: 16 }}
            >
              03
            </div>
            <div
              className="font-sans font-semibold"
              style={{ fontSize: 16, color: "var(--text-primary)", marginBottom: 8 }}
            >
              Start building
            </div>
            <div
              className="font-sans"
              style={{ fontSize: 14, color: "var(--text-dim)", lineHeight: 1.6 }}
            >
              Mine blocks, test transactions, or run a node on the Rill testnet.
            </div>
          </div>
        </div>

      </section>

      <Footer />
    </div>
  );
}
