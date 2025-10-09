import type { Feature, Stat } from "@/types";

export const heroContent = {
  title: "Find the Best Bridge Route. Every Time.",
  description:
    "Compare 9+ cross-chain bridges in real-time. Save up to 30% on fees while getting the fastest, most secure routes across Ethereum, Polygon, Arbitrum, Base, and 16+ blockchains.",
  primaryCTA: {
    label: "Compare Bridges Now",
    href: "/routes",
  },
  secondaryCTA: {
    label: "How It Works",
    href: "/support",
  },
};

export const features: Feature[] = [
  {
    title: "Real-Time Rate Comparison",
    description:
      "Instantly compare fees from Across, Stargate, Wormhole, Synapse, cBridge, LayerZero, and more. Always get the best rate.",
    icon: "zap",
  },
  {
    title: "Security-First Approach",
    description:
      "View audit history, security scores, and exploit records for each bridge. Make informed decisions with transparent security data.",
    icon: "shield",
  },
  {
    title: "Speed & Cost Optimized",
    description:
      "See exact transfer times and fees side-by-side. Choose the perfect balance between speed, cost, and security for your needs.",
    icon: "chart",
  },
];

export const stats: Stat[] = [
  {
    value: "10+",
    label: "Bridge Protocols",
    description: "Across, Stargate, Wormhole, Synapse & more",
    status: "active",
  },
  {
    value: "20+",
    label: "Supported Chains",
    description: "Ethereum, Polygon, Arbitrum, Base, zkSync & more",
    status: "active",
  },
  {
    value: "30%",
    label: "Average Savings",
    description: "Users save up to 30% on bridge fees",
    status: "active",
  },
  {
    value: "<5s",
    label: "Quote Speed",
    description: "Get instant quotes from all bridges",
    status: "active",
  },
  {
    value: "🚀",
    label: "Volume Tracking",
    description: "Real-time analytics and insights",
    status: "coming-soon",
  },
  {
    value: "🤖",
    label: "AI Route Optimizer",
    description: "Machine learning-powered route selection",
    status: "coming-soon",
  },
];
