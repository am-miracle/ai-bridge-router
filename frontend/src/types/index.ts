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

// New detailed bridge route types (MVP v1)
export interface GasDetails {
  source_gas_usd: number;
  destination_gas_usd: number;
  source_chain: string;
  destination_chain: string;
  source_gas_price_gwei: number;
  destination_gas_price_gwei: number;
  source_gas_limit: number;
  destination_gas_limit: number;
}

export interface CostBreakdown {
  bridge_fee: number;
  gas_estimate_usd: number;
  gas_details?: GasDetails;
}

export interface CostDetails {
  total_fee: number;
  total_fee_usd: number;
  breakdown: CostBreakdown;
}

export interface OutputDetails {
  expected: number;
  minimum: number;
  input: number;
}

export interface TimingDetails {
  seconds: number;
  display: string;
  category: "fast" | "medium" | "slow";
}

export interface SecurityDetails {
  score: number;
  level: "high" | "medium" | "low";
  has_audit: boolean;
  has_exploit: boolean;
}

export interface BridgeRoute {
  bridge: string;
  score: number;
  cost: CostDetails;
  output: OutputDetails;
  timing: TimingDetails;
  security: SecurityDetails;
  available: boolean;
  status: "operational" | "degraded" | "unavailable";
  warnings: string[];
}

export interface RouteQuoteRequest {
  sourceChain: string;
  destinationChain: string;
  tokenAddress: string;
  amount: string;
  recipientAddress?: string;
}
