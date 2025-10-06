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

  const formatTime = (seconds: number) => {
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
      const remainingMins = minutes % 60;
      return remainingMins > 0 ? `${hours}h ${remainingMins}m` : `${hours}h`;
    }
    return `${minutes}m`;
  };

  const formatCost = (cost: number) => {
    return `$${cost.toFixed(2)}`;
  };

  const getBadge = (rank: number) => {
    if (rank === 1)
      return {
        label: "Best Value",
        color: "bg-primary text-primary-foreground",
      };
    if (rank === 2) return { label: "Fastest", color: "bg-blue-500 text-white" };
    return null;
  };

  const badge = getBadge(rank);
  const bridgeUrl = getBridgeUrl(route.bridge);

  return (
    <article
      className="p-6 rounded-lg border bg-card hover:shadow-lg transition-all duration-300 relative"
      aria-labelledby={`route-${route.bridge}-${rank}-title`}
    >
      {badge && (
        <div
          className={`absolute top-4 right-4 px-3 py-1 rounded-full text-xs font-semibold ${badge.color}`}
          aria-label={`${badge.label} route option`}
        >
          {badge.label}
        </div>
      )}

      <div className="space-y-4">
        {/* Bridge Name */}
        <div>
          <h3 id={`route-${route.bridge}-${rank}-title`} className="text-xl font-bold mb-1">
            {route.bridge}
          </h3>
        </div>

        {/* Stats Grid */}
        <dl className="grid grid-cols-2 gap-4">
          <div>
            <dt className="text-xs text-muted-foreground mb-1">Estimated Time</dt>
            <dd className="font-semibold">{formatTime(route.est_time)}</dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground mb-1">Total Cost</dt>
            <dd className="font-semibold">{formatCost(route.cost)}</dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground mb-1">Liquidity</dt>
            <dd className="font-semibold">{route.liquidity}</dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground mb-1">Score</dt>
            <dd className={`font-semibold ${getScoreColor(route.score)}`}>
              {(route.score * 100).toFixed(0)}/100
            </dd>
          </div>
        </dl>

        {/* Action Button */}
        {bridgeUrl ? (
          <Button className="w-full" aria-label={`Select ${route.bridge} bridge route`} asChild>
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
