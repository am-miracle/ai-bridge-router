// API Client for Bridge Router Backend

const API_BASE_URL = import.meta.env.PUBLIC_API_URL || "http://localhost:8080";

export interface QuoteParams {
  from_chain: string;
  to_chain: string;
  token: string;
  amount: number;
  slippage?: number;
}

export interface CostBreakdown {
  bridge_fee: number;
  gas_estimate_usd: number;
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

export interface QuoteResponse {
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

export interface RequestMetadata {
  from: string;
  to: string;
  token: string;
  amount: number;
}

export interface ResponseMetadata {
  total_routes: number;
  available_routes: number;
  request: RequestMetadata;
}

export interface AggregatedQuotesResponse {
  routes: QuoteResponse[];
  metadata: ResponseMetadata;
  errors?: Array<{ bridge: string; error: string }>;
}

export interface ErrorResponse {
  error: string;
}

/**
 * Fetch bridge route quotes from backend API
 */
export async function getRouteQuotes(params: QuoteParams): Promise<AggregatedQuotesResponse> {
  const queryParams = new URLSearchParams({
    from_chain: params.from_chain,
    to_chain: params.to_chain,
    token: params.token,
    amount: params.amount.toString(),
    ...(params.slippage && { slippage: params.slippage.toString() }),
  });

  const response = await fetch(`${API_BASE_URL}/quotes?${queryParams}`, {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
    },
  });

  if (!response.ok) {
    const errorData: ErrorResponse = await response.json();
    throw new Error(errorData.error || `HTTP error! status: ${response.status}`);
  }

  return response.json();
}

/**
 * Check backend health
 */
export async function checkBackendHealth(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE_URL}/health`, {
      method: "GET",
    });
    return response.ok;
  } catch {
    return false;
  }
}
