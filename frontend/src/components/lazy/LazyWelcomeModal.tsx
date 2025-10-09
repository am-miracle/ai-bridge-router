import { lazy, Suspense } from "react";

const WelcomeModal = lazy(() =>
  import("../WelcomeModal").then((mod) => ({ default: mod.WelcomeModal }))
);

export function LazyWelcomeModal() {
  return (
    <Suspense fallback={<div className="hidden" aria-hidden="true" />}>
      <WelcomeModal />
    </Suspense>
  );
}
