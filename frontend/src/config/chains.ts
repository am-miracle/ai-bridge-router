export const supportedChains = [
  {
    id: "ethereum",
    name: "Ethereum",
    symbol: "ETH",
    chainId: 1,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/mainnet/assets/logo.svg",
  },
  {
    id: "optimism",
    name: "Optimism",
    symbol: "OP",
    chainId: 10,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/optimism/assets/logo.svg",
  },
  {
    id: "bsc",
    name: "BNB Smart Chain",
    symbol: "BNB",
    chainId: 56,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/bsc/assets/logo.svg",
  },
  {
    id: "polygon",
    name: "Polygon",
    symbol: "MATIC",
    chainId: 137,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/polygon/assets/logo.svg",
  },
  {
    id: "zksync",
    name: "zkSync",
    symbol: "ETH",
    chainId: 324,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/zk-sync/assets/logo.svg",
  },
  {
    id: "base",
    name: "Base",
    symbol: "ETH",
    chainId: 8453,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/base/assets/logo.svg",
  },
  {
    id: "arbitrum",
    name: "Arbitrum",
    symbol: "ARB",
    chainId: 42161,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/arbitrum/assets/logo.svg",
  },
  {
    id: "linea",
    name: "Linea",
    symbol: "ETH",
    chainId: 59144,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/linea/assets/logo.svg",
  },
  {
    id: "avalanche",
    name: "Avalanche",
    symbol: "AVAX",
    chainId: 43114,
    logoUrl:
      "https://assets.coingecko.com/coins/images/12559/small/Avalanche_Circle_RedWhite_Trans.png",
  },
  {
    id: "fantom",
    name: "Fantom",
    symbol: "FTM",
    chainId: 250,
    logoUrl: "https://assets.coingecko.com/coins/images/4001/small/Fantom_round.png",
  },
  {
    id: "mode",
    name: "Mode",
    symbol: "ETH",
    chainId: 34443,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/mode/assets/logo.svg",
  },
  {
    id: "blast",
    name: "Blast",
    symbol: "ETH",
    chainId: 81457,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/blast/assets/logo.svg",
  },
  {
    id: "scroll",
    name: "Scroll",
    symbol: "ETH",
    chainId: 534352,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/scroll/assets/logo.svg",
  },
  {
    id: "lisk",
    name: "Lisk",
    symbol: "ETH",
    chainId: 1135,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/lisk/assets/logo.svg",
  },
  {
    id: "redstone",
    name: "Redstone",
    symbol: "ETH",
    chainId: 690,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/redstone/assets/logo.svg",
  },
  {
    id: "zora",
    name: "Zora",
    symbol: "ETH",
    chainId: 7777777,
    logoUrl:
      "https://raw.githubusercontent.com/across-protocol/frontend/master/scripts/chain-configs/zora/assets/logo.svg",
  },
] as const;

export const commonTokens = [
  {
    address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
    symbol: "ETH",
    name: "Ethereum",
    logoUrl: "https://assets.coingecko.com/coins/images/279/small/ethereum.png",
  },
  {
    address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    symbol: "USDC",
    name: "USD Coin",
    logoUrl: "https://assets.coingecko.com/coins/images/6319/small/usdc.png",
  },
  {
    address: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
    symbol: "USDT",
    name: "Tether USD",
    logoUrl: "https://assets.coingecko.com/coins/images/325/small/Tether.png",
  },
  {
    address: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
    symbol: "DAI",
    name: "Dai Stablecoin",
    logoUrl: "https://assets.coingecko.com/coins/images/9956/small/Badge_Dai.png",
  },
  {
    address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
    symbol: "WBTC",
    name: "Wrapped Bitcoin",
    logoUrl: "https://assets.coingecko.com/coins/images/7598/small/wrapped_bitcoin_wbtc.png",
  },
  {
    address: "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9",
    symbol: "AAVE",
    name: "Aave Token",
    logoUrl: "https://assets.coingecko.com/coins/images/12645/small/AAVE.png",
  },
  {
    address: "0x514910771AF9Ca656af840dff83E8264EcF986CA",
    symbol: "LINK",
    name: "Chainlink",
    logoUrl: "https://assets.coingecko.com/coins/images/877/small/chainlink-new-logo.png",
  },
  {
    address: "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984",
    symbol: "UNI",
    name: "Uniswap",
    logoUrl: "https://assets.coingecko.com/coins/images/12504/small/uni.jpg",
  },
  {
    address: "0x0D8775F648430679A709E98d2b0Cb6250d2887EF",
    symbol: "BAT",
    name: "Basic Attention Token",
    logoUrl: "https://assets.coingecko.com/coins/images/677/small/basic-attention-token.png",
  },
  {
    address: "0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F",
    symbol: "SNX",
    name: "Synthetix",
    logoUrl: "https://assets.coingecko.com/coins/images/3406/small/SNX.png",
  },
] as const;
