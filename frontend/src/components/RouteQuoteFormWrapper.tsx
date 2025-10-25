import { useState } from "react";
import { RouteQuoteForm } from "@/components/RouteQuoteForm";
import { LoadingSection } from "@/components/LoadingSection";
import { ResultsSection } from "@/components/ResultsSection";
import type { BridgeRoute } from "@/types";

interface Chain {
  id: string;
  name: string;
  symbol: string;
}

interface Token {
  address: string;
  symbol: string;
  name: string;
}

interface RouteQuoteFormWrapperProps {
  supportedChains: readonly Chain[];
  commonTokens: readonly Token[];
  initialErrors?: Record<string, string[]>;
  actionError?: { message?: string } | string;
  actionUrl: string;
  formData?: {
    sourceChain?: string;
    destinationChain?: string;
    tokenAddress?: string;
    amount?: string;
    slippage?: string;
  };
  hasResults: boolean;
  initialRoutes?: BridgeRoute[];
  initialAmount?: string;
  initialSourceChain?: string;
  initialDestinationChain?: string;
}

export function RouteQuoteFormWrapper({
  supportedChains,
  commonTokens,
  initialErrors,
  actionError,
  actionUrl,
  formData,
  initialRoutes = [],
  initialAmount = "",
  initialSourceChain = "",
  initialDestinationChain = "",
}: RouteQuoteFormWrapperProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [routes, setRoutes] = useState<BridgeRoute[]>(initialRoutes);
  const [routeFormData, setRouteFormData] = useState({
    amount: initialAmount,
    sourceChain: initialSourceChain,
    destinationChain: initialDestinationChain,
  });

  const handleRoutesUpdate = (newRoutes: BridgeRoute[], newFormData: any) => {
    setRoutes(newRoutes);
    setRouteFormData({
      amount: newFormData.amount || "",
      sourceChain: newFormData.sourceChain || "",
      destinationChain: newFormData.destinationChain || "",
    });
  };

  return (
    <>
      <RouteQuoteForm
        supportedChains={supportedChains}
        commonTokens={commonTokens}
        initialErrors={initialErrors}
        actionError={actionError}
        actionUrl={actionUrl}
        formData={formData}
        onLoadingChange={setIsLoading}
        onRoutesUpdate={handleRoutesUpdate}
      />

      {/* Show loading skeletons when fetching quotes */}
      {isLoading && <LoadingSection />}

      {/* Show results when available */}
      {!isLoading && routes.length > 0 && (
        <ResultsSection
          routes={routes}
          amount={routeFormData.amount}
          sourceChain={routeFormData.sourceChain}
          destinationChain={routeFormData.destinationChain}
        />
      )}
    </>
  );
}
