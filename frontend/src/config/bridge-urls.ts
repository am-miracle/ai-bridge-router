// Bridge protocol URLs mapping
export const BRIDGE_URLS: Record<string, string> = {
  // Stargate Finance
  stargate: "https://stargate.finance/bridge",
  "stargate finance": "https://stargate.finance/bridge",

  // Across Protocol
  across: "https://app.across.to/bridge",
  "across protocol": "https://app.across.to/bridge",

  // Hop Protocol
  hop: "https://app.hop.exchange",
  "hop protocol": "https://app.hop.exchange",

  // Synapse Protocol
  synapse: "https://synapseprotocol.com/",
  "synapse protocol": "https://synapseprotocol.com/",

  // Everclear
  everclear: "https://explorer.everclear.org/intents/create",

  // Axelar
  axelar: "https://interchain.axelar.dev/",
  "axelar protocol": "https://interchain.axelar.dev/",

  // Celer cBridge
  celer: "https://cbridge.celer.network",
  "celer cbridge": "https://cbridge.celer.network",
  cbridge: "https://cbridge.celer.network",

  // Multichain (Anyswap)
  multichain: "https://app.multichain.org/#/router",
  anyswap: "https://app.multichain.org/#/router",

  // Orbiter Finance
  orbiter: "https://www.orbiter.finance/bridge",
  "orbiter finance": "https://www.orbiter.finance/bridge",

  // LayerZero (via Stargate)
  layerzero: "https://lz.superbridge.app/",

  // Wormhole
  wormhole: "https://portalbridge.com/",
  "wormhole bridge": "https://portalbridge.com/",
};

/**
 * Affiliate/Referral configuration for each bridge
 * Add your referral codes here once you're accepted into their programs
 */
export const BRIDGE_REFERRAL_CONFIG: Record<
  string,
  {
    paramName: string;
    referralCode: string;
    enabled: boolean;
  }
> = {
  // Hop Protocol Referral Program
  // Apply at: https://docs.hop.exchange/hop-protocol/community-programs/referral-program
  hop: {
    paramName: "ref",
    referralCode: "bridgerouter", // TODO: Replace with your actual referral code
    enabled: false, // Set to true once approved
  },

  // Across Protocol Affiliate Program
  // Contact: partnerships@across.to
  across: {
    paramName: "referrer",
    referralCode: "bridgerouter", // TODO: Replace with your actual referral address
    enabled: false, // Set to true once approved
  },

  // Stargate Finance Partner Program
  // Apply through: https://stargate.finance/
  stargate: {
    paramName: "ref",
    referralCode: "bridgerouter", // TODO: Replace with your actual referral code
    enabled: false, // Set to true once approved
  },

  // Synapse Protocol Affiliate Program
  // Contact: partnerships@synapseprotocol.com
  synapse: {
    paramName: "ref",
    referralCode: "bridgerouter", // TODO: Replace with your actual referral code
    enabled: false, // Set to true once approved
  },
};

interface GetBridgeUrlOptions {
  fromChain?: string;
  toChain?: string;
  token?: string;
  amount?: number;
}

/**
 * Get the bridge URL for a given bridge name with optional referral parameters
 * Returns the URL if found, otherwise returns null
 *
 * @param bridgeName - Name of the bridge protocol
 * @param options - Optional parameters to pre-fill the bridge UI
 * @returns Complete URL with referral code (if configured) and pre-filled parameters
 */
export function getBridgeUrl(bridgeName: string, options?: GetBridgeUrlOptions): string | null {
  const normalizedName = bridgeName.toLowerCase().trim();
  const baseUrl = BRIDGE_URLS[normalizedName];

  if (!baseUrl) {
    return null;
  }

  // Build URL with query parameters
  const url = new URL(baseUrl);

  // Add referral code if configured and enabled
  const referralConfig = BRIDGE_REFERRAL_CONFIG[normalizedName];
  if (referralConfig?.enabled && referralConfig.referralCode) {
    url.searchParams.set(referralConfig.paramName, referralConfig.referralCode);
  }

  // Add optional pre-fill parameters (if supported by bridge)
  if (options?.fromChain) {
    url.searchParams.set("from", options.fromChain);
  }
  if (options?.toChain) {
    url.searchParams.set("to", options.toChain);
  }
  if (options?.token) {
    url.searchParams.set("token", options.token);
  }
  if (options?.amount) {
    url.searchParams.set("amount", options.amount.toString());
  }

  // Add UTM tracking for analytics
  url.searchParams.set("utm_source", "bridgerouter");
  url.searchParams.set("utm_medium", "aggregator");
  url.searchParams.set("utm_campaign", "bridge_comparison");

  return url.toString();
}
