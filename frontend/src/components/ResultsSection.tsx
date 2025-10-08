import { useEffect } from "react";
import toast from "react-hot-toast";
import { FadeIn } from "@/components/animations/FadeIn";
import { StaggerContainer } from "@/components/animations/StaggerContainer";
import { StaggerItem } from "@/components/animations/StaggerItem";
import { RouteCard } from "@/components/RouteCard";
import type { BridgeRoute } from "@/types";

interface ResultsSectionProps {
  routes: BridgeRoute[];
  amount: string;
  sourceChain: string;
  destinationChain: string;
}

export function ResultsSection({
  routes,
  amount,
  sourceChain,
  destinationChain,
}: ResultsSectionProps) {
  useEffect(() => {
    // Dismiss the loading toast
    toast.dismiss("fetching-quotes");

    if (routes.length > 0) {
      toast.success(`Found ${routes.length} route${routes.length > 1 ? "s" : ""} available!`);
    } else {
      toast.error("No routes found for this combination");
    }
  }, [routes]);

  return (
    <FadeIn delay={0.2}>
      <section
        className="space-y-6"
        aria-labelledby="results-heading"
        role="region"
        aria-live="polite"
      >
        <div className="lg:flex items-center justify-between space-3 mt-3">
          <h2 id="results-heading" className="text-lg lg:text-2xl font-bold">
            Available Routes ({routes.length})
          </h2>
          <p className="text-sm text-muted-foreground">
            Bridging {amount} from {sourceChain} to {destinationChain}
          </p>
        </div>

        <StaggerContainer className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          {routes.map((route, index) => (
            <StaggerItem key={`${route.bridge}-${index}`}>
              <RouteCard route={route} rank={index + 1} />
            </StaggerItem>
          ))}
        </StaggerContainer>
      </section>
    </FadeIn>
  );
}
