import type { FooterSection } from "@/types";

export const footerSections: FooterSection[] = [
  {
    title: "Product",
    links: [
      { label: "Bridge", href: "/" },
      { label: "Routes", href: "/routes" },
      { label: "History", href: "/#" },
      { label: "Analytics", href: "/#" },
    ],
  },
  {
    title: "Resources",
    links: [
      // { label: "Documentation", href: "/docs" },
      // { label: "API Reference", href: "/api" },
      { label: "GitHub", href: "https://github.com", external: true },
      { label: "Blog", href: "/#" },
    ],
  },
  {
    title: "Community",
    links: [
      { label: "Discord", href: "https://discord.com", external: true },
      { label: "Twitter", href: "https://twitter.com", external: true },
      { label: "Support", href: "/support" },
      { label: "Security", href: "/#" },
    ],
  },
];

export const legalLinks = [
  { label: "Privacy Policy", href: "/privacy" },
  { label: "Terms of Service", href: "/terms" },
];

export const brandInfo = {
  name: "Bridge Router",
  description:
    "Cross-chain bridge aggregator for secure and efficient asset transfers across multiple blockchains.",
};
