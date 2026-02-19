import Nav from "@/components/Nav";
import Hero from "@/components/Hero";
import BentoSection from "@/components/BentoSection";
import StatsSection from "@/components/StatsSection";
import DecayRingSection from "@/components/DecayRingSection";
import CliSection from "@/components/CliSection";
import CtaSection from "@/components/CtaSection";
import Footer from "@/components/Footer";

export default function Home() {
  return (
    <main style={{ backgroundColor: "var(--void)" }}>
      <Nav />
      <Hero />
      <BentoSection />
      <StatsSection />
      <DecayRingSection />
      <CliSection />
      <CtaSection />
      <Footer />
    </main>
  );
}
