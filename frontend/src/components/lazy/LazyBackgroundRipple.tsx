import { lazy, Suspense } from "react";

const BackgroundRippleEffect = lazy(() =>
  import("../ui/background-ripple-effect").then((mod) => ({ default: mod.BackgroundRippleEffect }))
);

export function LazyBackgroundRipple() {
  return (
    <Suspense fallback={null}>
      <BackgroundRippleEffect />
    </Suspense>
  );
}
