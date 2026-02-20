import type { Metadata } from "next";
import Nav from "@/components/Nav";
import Footer from "@/components/Footer";
import WalletPage from "@/components/WalletPage";

export const metadata: Metadata = {
  title: "Wallet — RillCoin Testnet",
  description:
    "Create a testnet wallet, get RILL from the faucet, and send payments — all in your browser.",
};

export default function Wallet() {
  return (
    <main style={{ backgroundColor: "var(--void)" }}>
      <Nav />
      <WalletPage />
      <Footer />
    </main>
  );
}
