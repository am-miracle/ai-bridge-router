export interface NavLink {
  label: string;
  href: string;
  external?: boolean;
}

export interface FooterSection {
  title: string;
  links: NavLink[];
}

export interface SocialLink {
  name: string;
  href: string;
  icon: string;
}

export interface Feature {
  title: string;
  description: string;
  icon: string;
}

export interface Stat {
  value: string;
  label: string;
  description?: string;
  trend?: "up" | "down" | "neutral";
  status?: "active" | "coming-soon";
}

export type Theme = "light" | "dark";

export interface FAQItem {
  question: string;
  answer: string;
  category?: string;
}

export interface SupportCategory {
  title: string;
  description: string;
  icon: string;
  href: string;
}

export interface BridgeRoute {
  bridge: string;
  cost: number;
  est_time: number;
  liquidity: string;
  score: number;
}

export interface RouteQuoteRequest {
  sourceChain: string;
  destinationChain: string;
  tokenAddress: string;
  amount: string;
  recipientAddress?: string;
}
