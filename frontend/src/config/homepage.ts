import type { Feature, Stat } from "@/types";

export const heroContent = {
  title: "Cross-Chain Bridge Aggregator",
  description:
    "Move assets across multiple blockchains securely, quickly, and at the lowest possible cost. Compare bridge options in real-time to find the best route.",
  primaryCTA: {
    label: "Start Bridging",
    href: "/",
  },
  secondaryCTA: {
    label: "View Routes",
    href: "/routes",
  },
};

export const features: Feature[] = [
  {
    title: "Bridge Aggregation",
    description:
      "Connect to top protocols like Everclear, Hop, Axelar, and Wormhole with real-time data.",
    icon: "zap",
  },
  {
    title: "Security Scoring",
    description: "Heuristic scoring based on audits, exploit history, and custodial risk analysis.",
    icon: "shield",
  },
  {
    title: "Quote Comparison",
    description:
      "Compare fees, speeds, and risks side by side to find the best route for your transfer.",
    icon: "chart",
  },
];

export const stats: Stat[] = [
  {
    value: "8+",
    label: "Bridge Protocols",
    description: "Everclear, Hop, Axelar & more",
    status: "active",
  },
  {
    value: "20+",
    label: "Supported Chains",
    description: "Ethereum, Polygon, Arbitrum & more",
    status: "active",
  },
  {
    value: "🚀",
    label: "Volume Tracking",
    description: "Real-time analytics coming soon",
    status: "coming-soon",
  },
  {
    value: "⚡",
    label: "Smart Router",
    description: "AI-powered route optimization in development",
    status: "coming-soon",
  },
];
