import { Button } from "@/components/ui/button";
import { getBridgeUrl } from "@/config/bridge-urls";
import type { BridgeRoute } from "@/types";

interface RouteCardProps {
  route: BridgeRoute;
  rank: number;
}

export function RouteCard({ route, rank }: RouteCardProps) {
  const getScoreColor = (score: number) => {
    if (score >= 0.9) return "text-green-600 dark:text-green-400";
    if (score >= 0.75) return "text-yellow-600 dark:text-yellow-400";
    return "text-red-600 dark:text-red-400";
  };

  const getTimingBadgeColor = (category: string) => {
    if (category === "fast")
      return "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400";
    if (category === "medium")
      return "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400";
    return "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400";
  };

  const getSecurityBadgeColor = (level: string) => {
    if (level === "high")
      return "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400";
    if (level === "medium")
      return "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400";
    return "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400";
  };

  const formatAmount = (amount: number) => {
    return amount.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    });
  };

  const getBadge = (rank: number) => {
    if (rank === 1) return { label: "Best", color: "bg-primary text-primary-foreground" };
    if (rank === 2) return { label: "Fast", color: "bg-blue-500 text-white" };
    return null;
  };

  const badge = getBadge(rank);
  const bridgeUrl = getBridgeUrl(route.bridge);

  return (
    <article
      className="p-5 rounded-lg border bg-card hover:shadow-lg transition-all duration-300 relative flex flex-col h-full"
      aria-labelledby={`route-${route.bridge}-${rank}-title`}
    >
      {badge && (
        <div
          className={`absolute top-3 right-3 px-2 py-0.5 rounded-full text-xs font-semibold ${badge.color}`}
          aria-label={`${badge.label} route option`}
        >
          {badge.label}
        </div>
      )}

      <div className="flex-1 space-y-3">
        {/* Header: Bridge Name + Badges */}
        <div className="space-y-2">
          <h3 id={`route-${route.bridge}-${rank}-title`} className="text-lg font-bold pr-12">
            {route.bridge}
          </h3>

          {/* Inline badges for warnings and security */}
          <div className="flex flex-wrap gap-1.5">
            {/* Timing badge */}
            <span
              className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium ${getTimingBadgeColor(route.timing.category)}`}
              title={
                route.timing.category === "fast"
                  ? "Fast route - Completes in under 2 minutes"
                  : route.timing.category === "medium"
                    ? "Medium speed route - Takes 2-10 minutes"
                    : "Slow route - Takes over 10 minutes"
              }
            >
              {route.timing.display}
            </span>

            {/* Security badge */}
            <span
              className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium ${getSecurityBadgeColor(route.security.level)}`}
              title={
                route.security.has_audit
                  ? "Security audited - This bridge has undergone professional security audits"
                  : "Not audited - No public security audit available"
              }
            >
              {route.security.has_audit && "Audited • "}
              {route.security.level.charAt(0).toUpperCase() + route.security.level.slice(1)}
            </span>

            {/* Warning badges */}
            {route.warnings?.includes("slow_route") && (
              <span
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400"
                title="This route is slower than alternatives"
              >
                Warning: Slow
              </span>
            )}
            {route.warnings?.includes("low_security") && (
              <span
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                title="Low security score - Use with caution"
              >
                Warning: Security
              </span>
            )}
            {route.security.has_exploit && (
              <span
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                title="This bridge has a history of security exploits"
              >
                Warning: Exploit History
              </span>
            )}
          </div>
        </div>

        {/* Cost */}
        <div className="py-2 border-y space-y-1.5">
          <div className="flex items-baseline justify-between">
            <dt className="text-sm text-muted-foreground">Total Cost</dt>
            <dd className="text-xl font-bold">${route.cost.total_fee_usd.toFixed(2)}</dd>
          </div>

          {/* Gas breakdown */}
          {route.cost.breakdown.gas_estimate_usd > 0 && (
            <div className="text-xs text-muted-foreground space-y-0.5">
              <div className="flex justify-between">
                <span>Bridge Fee</span>
                <span className="font-medium">${route.cost.breakdown.bridge_fee.toFixed(4)}</span>
              </div>
              <div className="flex justify-between">
                <span>Gas (Est.)</span>
                <span className="font-medium">
                  ${route.cost.breakdown.gas_estimate_usd.toFixed(4)}
                </span>
              </div>

              {/* Detailed gas info tooltip */}
              {route.cost.breakdown.gas_details && (
                <details className="cursor-pointer hover:text-foreground transition-colors">
                  <summary className="text-[10px] uppercase tracking-wide font-semibold mt-1">
                    Gas Details ▼
                  </summary>
                  <div className="mt-1 pl-2 border-l-2 border-muted space-y-0.5">
                    <div className="flex justify-between">
                      <span>{route.cost.breakdown.gas_details.source_chain}</span>
                      <span className="font-medium">
                        ${route.cost.breakdown.gas_details.source_gas_usd.toFixed(4)}
                      </span>
                    </div>
                    <div className="text-[10px]">
                      {route.cost.breakdown.gas_details.source_gas_price_gwei.toFixed(2)} Gwei ×{" "}
                      {route.cost.breakdown.gas_details.source_gas_limit.toLocaleString()} gas
                    </div>
                    <div className="flex justify-between mt-1">
                      <span>{route.cost.breakdown.gas_details.destination_chain}</span>
                      <span className="font-medium">
                        ${route.cost.breakdown.gas_details.destination_gas_usd.toFixed(4)}
                      </span>
                    </div>
                    <div className="text-[10px]">
                      {route.cost.breakdown.gas_details.destination_gas_price_gwei.toFixed(2)} Gwei
                      × {route.cost.breakdown.gas_details.destination_gas_limit.toLocaleString()}{" "}
                      gas
                    </div>
                  </div>
                </details>
              )}
            </div>
          )}
        </div>

        {/* Output */}
        <div className="bg-muted/50 rounded p-2.5 space-y-1">
          <div className="flex justify-between items-baseline">
            <dt className="text-xs text-muted-foreground">You'll Receive</dt>
            <dd className="text-sm font-semibold">{formatAmount(route.output.expected)}</dd>
          </div>
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Minimum</span>
            <span>{formatAmount(route.output.minimum)}</span>
          </div>
        </div>

        {/* Compact Stats */}
        <div className="grid grid-cols-2 gap-2 text-center">
          <div className="bg-muted/30 rounded p-2">
            <dt className="text-xs text-muted-foreground mb-0.5">Score</dt>
            <dd className={`text-lg font-bold ${getScoreColor(route.score)}`}>
              {(route.score * 100).toFixed(0)}
            </dd>
          </div>
          <div className="bg-muted/30 rounded p-2">
            <dt className="text-xs text-muted-foreground mb-0.5">Status</dt>
            <dd className="text-sm font-bold capitalize">{route.status}</dd>
          </div>
        </div>
      </div>

      {/* Action Button */}
      <div className="mt-4">
        {bridgeUrl ? (
          <Button
            className="w-full"
            disabled={!route.available}
            aria-label={`Select ${route.bridge} bridge route`}
            asChild={route.available}
          >
            {route.available ? (
              <a href={bridgeUrl} target="_blank" rel="noopener noreferrer">
                Select Route
                <svg
                  className="ml-2 h-4 w-4 inline-block"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                  />
                </svg>
              </a>
            ) : (
              <span>Route Unavailable</span>
            )}
          </Button>
        ) : (
          <Button
            className="w-full"
            disabled
            aria-label={`${route.bridge} bridge URL not available`}
          >
            Bridge URL Not Available
          </Button>
        )}
      </div>
    </article>
  );
}
