export const supportedChains = [
  { id: "ethereum", name: "Ethereum", symbol: "ETH" },
  { id: "polygon", name: "Polygon", symbol: "MATIC" },
  { id: "arbitrum", name: "Arbitrum", symbol: "ARB" },
  { id: "optimism", name: "Optimism", symbol: "OP" },
  { id: "avalanche", name: "Avalanche", symbol: "AVAX" },
  { id: "bsc", name: "BNB Smart Chain", symbol: "BNB" },
  { id: "base", name: "Base", symbol: "ETH" },
  { id: "linea", name: "Linea", symbol: "ETH" },
] as const;

export const commonTokens = [
  { address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE", symbol: "ETH", name: "Ethereum" },
  { address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", symbol: "USDC", name: "USD Coin" },
  { address: "0xdAC17F958D2ee523a2206206994597C13D831ec7", symbol: "USDT", name: "Tether USD" },
  { address: "0x6B175474E89094C44Da98b954EedeAC495271d0F", symbol: "DAI", name: "Dai Stablecoin" },
  {
    address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
    symbol: "WBTC",
    name: "Wrapped Bitcoin",
  },
] as const;
