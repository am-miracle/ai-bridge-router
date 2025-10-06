// API Client for Bridge Router Backend

const API_BASE_URL = import.meta.env.PUBLIC_API_URL || "http://localhost:8080";

export interface QuoteParams {
  from_chain: string;
  to_chain: string;
  token: string;
  amount: number;
  slippage?: number;
}

export interface QuoteResponse {
  bridge: string;
  cost: number;
  est_time: number;
  liquidity: string;
  score: number;
}

export interface AggregatedQuotesResponse {
  routes: QuoteResponse[];
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
