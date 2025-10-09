import { useState } from "react";
import { RouteQuoteForm } from "@/components/RouteQuoteForm";
import { LoadingSection } from "@/components/LoadingSection";

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
}

export function RouteQuoteFormWrapper({
  supportedChains,
  commonTokens,
  initialErrors,
  actionError,
  actionUrl,
  formData,
  hasResults,
}: RouteQuoteFormWrapperProps) {
  const [isLoading, setIsLoading] = useState(false);

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
      />

      {/* Show loading skeletons when fetching quotes and no results yet */}
      {isLoading && !hasResults && <LoadingSection />}
    </>
  );
}
