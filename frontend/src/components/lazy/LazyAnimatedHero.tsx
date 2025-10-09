import { lazy, Suspense } from "react";
import type { ComponentProps } from "react";

const AnimatedHero = lazy(() =>
  import("../homepage/AnimatedHero").then((mod) => ({ default: mod.AnimatedHero }))
);

type AnimatedHeroProps = ComponentProps<typeof AnimatedHero>;

export function LazyAnimatedHero(props: AnimatedHeroProps) {
  return (
    <Suspense
      fallback={
        <div className="text-center space-y-8">
          <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold">{props.title}</h1>
          <p className="text-xl text-muted-foreground max-w-3xl mx-auto">{props.description}</p>
        </div>
      }
    >
      <AnimatedHero {...props} />
    </Suspense>
  );
}
