import { FadeIn } from "@/components/animations/FadeIn";
import { StaggerContainer } from "@/components/animations/StaggerContainer";
import { StaggerItem } from "@/components/animations/StaggerItem";
import { RouteCardSkeleton } from "@/components/RouteCardSkeleton";

export function LoadingSection() {
  return (
    <FadeIn delay={0.2}>
      <section
        className="space-y-6"
        aria-labelledby="loading-heading"
        role="region"
        aria-busy="true"
      >
        <div className="lg:flex items-center justify-between space-3 mt-3">
          <h2 id="loading-heading" className="text-lg lg:text-2xl font-bold">
            Fetching routes...
          </h2>
          <p className="text-sm text-muted-foreground">Comparing bridge quotes</p>
        </div>

        <StaggerContainer className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          {[1, 2, 3].map((index) => (
            <StaggerItem key={index}>
              <RouteCardSkeleton />
            </StaggerItem>
          ))}
        </StaggerContainer>
      </section>
    </FadeIn>
  );
}
