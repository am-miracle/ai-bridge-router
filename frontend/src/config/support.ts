import type { FAQItem, SupportCategory } from "@/types";

export const supportCategories: SupportCategory[] = [
  {
    title: "Getting Started",
    description: "Learn the basics of using Bridge Router",
    icon: "book",
    href: "/docs/getting-started",
  },
  {
    title: "Bridge Protocols",
    description: "Information about supported bridges",
    icon: "bridge",
    href: "/docs/bridges",
  },
  {
    title: "Troubleshooting",
    description: "Common issues and solutions",
    icon: "tool",
    href: "/docs/troubleshooting",
  },
  {
    title: "API Reference",
    description: "Developer documentation and API",
    icon: "code",
    href: "/api",
  },
];

export const faqs: FAQItem[] = [
  {
    question: "What is Bridge Router?",
    answer:
      "Bridge Router is a cross-chain bridge aggregator that helps you move assets across multiple blockchains securely and efficiently. We compare different bridge protocols to find you the best route based on cost, speed, and security.",
    category: "General",
  },
  {
    question: "Which blockchains are supported?",
    answer:
      "We support 20+ major blockchains including Ethereum, Polygon, Arbitrum, Optimism, Avalanche, BSC, and more. Check our documentation for the complete list of supported chains.",
    category: "General",
  },
  {
    question: "How do I start bridging assets?",
    answer:
      "Connect your wallet, select the source and destination chains, enter the amount you want to bridge, and compare the available routes. Choose the best option and confirm the transaction in your wallet.",
    category: "Getting Started",
  },
  {
    question: "What fees do I need to pay?",
    answer:
      "You'll pay the bridge protocol fees (which vary by bridge), gas fees on both source and destination chains, and a small service fee to Bridge Router. All fees are shown upfront before you confirm.",
    category: "Fees",
  },
  {
    question: "How long does a bridge transfer take?",
    answer:
      "Transfer times vary by bridge protocol and chain congestion. Most transfers complete within 10-30 minutes, but some can take longer. We show estimated completion times for each route.",
    category: "Transfers",
  },
  {
    question: "Is Bridge Router safe?",
    answer:
      "Yes. We don't custody your funds - all transfers happen directly through established bridge protocols. We aggregate and compare routes to help you make informed decisions. Always verify transaction details before confirming.",
    category: "Security",
  },
  {
    question: "What if my transaction fails?",
    answer:
      "If a transaction fails, your funds will remain in your wallet on the source chain. Check the transaction hash on the block explorer for details. Contact support if you need assistance.",
    category: "Troubleshooting",
  },
  {
    question: "Can I track my transfer?",
    answer:
      "Yes! After initiating a transfer, you can track its status using the transaction hash. Our history page (coming soon) will also show all your past transactions.",
    category: "Transfers",
  },
  {
    question: "Which wallets are supported?",
    answer:
      "We support all major Web3 wallets including MetaMask, WalletConnect, Coinbase Wallet, Rainbow, and more. Make sure your wallet supports the chains you want to bridge between.",
    category: "Getting Started",
  },
  {
    question: "How is the best route determined?",
    answer:
      "We compare routes based on total cost (fees + gas), transfer speed, security score, and liquidity. Our smart router (in development) will use AI to optimize route selection based on your preferences.",
    category: "Routes",
  },
];

export const contactInfo = {
  email: "support@bridgerouter.com",
  discord: "https://discord.gg/bridgerouter",
  twitter: "https://twitter.com/bridgerouter",
  github: "https://github.com/bridgerouter",
};
