import type { FAQItem } from "@/types";

export const securityHero = {
  title: "Security First",
  description:
    "Your safety is our priority. Learn how Bridge Router protects your assets through transparent security practices and trusted protocols.",
};

export const securityPrinciples = [
  {
    title: "Non-Custodial Architecture",
    description:
      "We never hold your funds. All transfers happen directly through established bridge protocols. Your assets remain under your control at all times.",
    icon: "shield",
  },
  {
    title: "Protocol Transparency",
    description:
      "Every bridge protocol is vetted and displayed with security scores, audit reports, and historical security records. Make informed decisions with full visibility.",
    icon: "eye",
  },
  {
    title: "Smart Contract Safety",
    description:
      "We only integrate with battle-tested bridge protocols that have undergone multiple security audits and have proven track records.",
    icon: "lock",
  },
  {
    title: "Transaction Verification",
    description:
      "All transaction details are shown upfront. Verify destination addresses, amounts, and fees before confirming any transfer.",
    icon: "check",
  },
];

export const securityFAQs: FAQItem[] = [
  {
    question: "Does Bridge Router hold my funds?",
    answer:
      "No. Bridge Router is a non-custodial aggregator. We never take custody of your assets. All transfers happen directly between your wallet and the bridge protocol smart contracts. Your funds are always under your control.",
    category: "Custody",
  },
  {
    question: "How do you select which bridges to integrate?",
    answer:
      "We carefully evaluate each bridge based on security audits, TVL (Total Value Locked), historical security record, team reputation, and community trust. Only established protocols with proven track records are integrated.",
    category: "Protocol Selection",
  },
  {
    question: "What security information do you provide for each bridge?",
    answer:
      "For each bridge, we display security audit reports, historical security incidents (if any), total value locked, protocol age, and community security ratings. This helps you make informed decisions about which bridge to use.",
    category: "Transparency",
  },
  {
    question: "Can my transaction be intercepted or modified?",
    answer:
      "No. Once you approve a transaction in your wallet, it's signed with your private key and broadcast directly to the blockchain. Neither Bridge Router nor anyone else can modify or intercept signed transactions.",
    category: "Transaction Security",
  },
  {
    question: "What happens if a bridge protocol gets hacked?",
    answer:
      "Bridge Router doesn't custody funds, so our platform itself cannot be exploited to steal assets. However, if an underlying bridge protocol is compromised, funds in transit through that specific bridge could be at risk. We display historical security records to help you assess bridge reliability and immediately notify users of any security incidents.",
    category: "Risk Management",
  },
  {
    question: "Do you audit the bridge smart contracts?",
    answer:
      "We rely on third-party professional security firms to audit bridge protocols. We aggregate and display these audit reports for transparency. Each bridge we integrate must have recent audits from reputable security firms.",
    category: "Audits",
  },
  {
    question: "How can I verify my transaction on-chain?",
    answer:
      "After initiating a transfer, you'll receive a transaction hash. You can use this hash on block explorers like Etherscan to verify the transaction details, status, and destination independently.",
    category: "Verification",
  },
  {
    question: "What permissions does Bridge Router request?",
    answer:
      "We only request wallet connection permission to read your address and token balances. For transfers, you approve specific token spending limits for the bridge protocol smart contracts—not Bridge Router directly.",
    category: "Permissions",
  },
  {
    question: "Is my wallet private key shared with Bridge Router?",
    answer:
      "Absolutely not. Your wallet private keys never leave your wallet application. We use standard Web3 protocols (like WalletConnect) that allow you to sign transactions without ever exposing your private keys.",
    category: "Privacy",
  },
  {
    question: "How do you protect against phishing attacks?",
    answer:
      "Always verify you're on the official Bridge Router domain before connecting your wallet. We display clear transaction details before signing. Never approve transactions you don't understand, and always double-check recipient addresses.",
    category: "Phishing Protection",
  },
];

export const bestPractices = [
  {
    title: "Verify Transaction Details",
    description:
      "Always double-check the destination address, amount, and fees before confirming any transaction in your wallet.",
  },
  {
    title: "Start with Small Amounts",
    description:
      "When using a new bridge or route, test with a small amount first to ensure everything works as expected.",
  },
  {
    title: "Check Bridge Security Scores",
    description:
      "Review the security information we provide for each bridge. Consider audit history and TVL when choosing a route.",
  },
  {
    title: "Use Hardware Wallets",
    description:
      "For large transfers, use hardware wallets like Ledger or Trezor for an additional layer of security.",
  },
  {
    title: "Verify Website URL",
    description:
      "Always ensure you're on the official Bridge Router website. Bookmark the correct URL to avoid phishing sites.",
  },
  {
    title: "Monitor Your Transactions",
    description:
      "Track your transfer using the transaction hash on block explorers. Report any suspicious activity immediately.",
  },
];

export const auditedBridges = [
  {
    name: "Across Protocol",
    auditors: ["OpenZeppelin", "Code4rena"],
    lastAudit: "2024",
  },
  {
    name: "Stargate Finance",
    auditors: ["Quantstamp", "Zellic"],
    lastAudit: "2024",
  },
  {
    name: "Wormhole",
    auditors: ["Trail of Bits", "Neodyme", "Kudelski"],
    lastAudit: "2024",
  },
  {
    name: "Synapse Protocol",
    auditors: ["Quantstamp", "Certik"],
    lastAudit: "2023",
  },
  {
    name: "cBridge",
    auditors: ["Certik", "PeckShield", "Slowmist"],
    lastAudit: "2024",
  },
];
