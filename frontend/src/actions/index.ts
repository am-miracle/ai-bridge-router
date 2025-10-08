import { defineAction, ActionError } from "astro:actions";
import { z } from "astro:schema";

const API_BASE_URL = import.meta.env.PUBLIC_API_URL || "http://localhost:8080";

// Token address to symbol mapping
const TOKEN_SYMBOLS: Record<string, string> = {
  "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE": "ETH",
  "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": "USDC",
  "0xdAC17F958D2ee523a2206206994597C13D831ec7": "USDT",
  "0x6B175474E89094C44Da98b954EedeAC495271d0F": "DAI",
  "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599": "WBTC",
};

export const server = {
  getRouteQuote: defineAction({
    accept: "form",
    input: z.object({
      sourceChain: z.string().min(1, "Source chain is required"),
      destinationChain: z.string().min(1, "Destination chain is required"),
      tokenAddress: z.string().min(1, "Token address is required"),
      amount: z.string().refine(
        (val) => {
          const num = parseFloat(val);
          return !isNaN(num) && num > 0;
        },
        { message: "Amount must be a positive number" }
      ),
      slippage: z
        .string()
        .refine(
          (val) => {
            const num = parseFloat(val);
            return !isNaN(num) && num >= 0 && num <= 50;
          },
          { message: "Slippage must be between 0 and 50" }
        )
        .optional()
        .default("0.5"),
      recipientAddress: z.string().optional(),
    }),
    handler: async (input) => {
      try {
        // Get token symbol from address
        const tokenSymbol = TOKEN_SYMBOLS[input.tokenAddress] || "USDC";
        const amountNum = parseFloat(input.amount);
        const slippageNum = input.slippage ? parseFloat(input.slippage) : 0.5;

        // Build query parameters
        const queryParams = new URLSearchParams({
          from_chain: input.sourceChain,
          to_chain: input.destinationChain,
          token: tokenSymbol,
          amount: amountNum.toString(),
          slippage: slippageNum.toString(),
        });

        // Call backend API
        const response = await fetch(`${API_BASE_URL}/quotes?${queryParams}`, {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
          },
        });

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({ error: "Unknown error" }));
          throw new ActionError({
            code: "BAD_REQUEST",
            message: errorData.error || `Failed to fetch quotes: ${response.status}`,
          });
        }

        const data = await response.json();

        return {
          routes: data.routes || [],
          errors: data.errors || [],
          sourceChain: input.sourceChain,
          destinationChain: input.destinationChain,
          amount: input.amount,
          tokenAddress: input.tokenAddress,
          tokenSymbol,
          slippage: input.slippage || "0.5",
          timestamp: new Date().toISOString(),
        };
      } catch (error) {
        // If it's already an ActionError, re-throw it
        if (error instanceof ActionError) {
          throw error;
        }

        // Otherwise, wrap it in a generic error
        throw new ActionError({
          code: "INTERNAL_SERVER_ERROR",
          message:
            error instanceof Error
              ? error.message
              : "Failed to fetch route quotes. Please try again.",
        });
      }
    },
  }),
};
