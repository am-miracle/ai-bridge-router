// Bridge protocol URLs mapping
export const BRIDGE_URLS: Record<string, string> = {
  // Stargate Finance
  stargate: "https://stargate.finance/bridge",
  "stargate finance": "https://stargate.finance/bridge",

  // Across Protocol
  across: "https://app.across.to/bridge?",
  "across protocol": "https://app.across.to/bridge?",

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
 * Get the bridge URL for a given bridge name
 * Returns the URL if found, otherwise returns null
 */
export function getBridgeUrl(bridgeName: string): string | null {
  const normalizedName = bridgeName.toLowerCase().trim();
  return BRIDGE_URLS[normalizedName] || null;
}
