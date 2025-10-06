import { ScaleIn } from "@/components/animations/ScaleIn";

export function EmptyState() {
  return (
    <ScaleIn delay={0.3}>
      <section className="text-center py-12 space-y-4" aria-label="Get started instructions">
        <h3 className="text-xl font-semibold">Ready to bridge your assets?</h3>
        <p className="text-muted-foreground">
          Fill out the form above to compare routes from multiple bridge protocols.
        </p>
      </section>
    </ScaleIn>
  );
}
