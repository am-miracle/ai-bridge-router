import { useRef, useEffect, useState } from "react";
import toast from "react-hot-toast";
import { FadeIn } from "@/components/animations/FadeIn";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { getRouteQuotes } from "@/lib/api-client";
import type { BridgeRoute } from "@/types";

interface Chain {
  id: string;
  name: string;
  symbol: string;
  chainId?: number;
  logoUrl?: string;
}

interface Token {
  address: string;
  symbol: string;
  name: string;
  logoUrl?: string;
}

interface RouteQuoteFormProps {
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
  onLoadingChange?: (loading: boolean) => void;
  onRoutesUpdate?: (routes: BridgeRoute[], formData: any) => void;
}

// Token address to symbol mapping
const TOKEN_SYMBOLS: Record<string, string> = {
  "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE": "ETH",
  "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": "USDC",
  "0xdAC17F958D2ee523a2206206994597C13D831ec7": "USDT",
  "0x6B175474E89094C44Da98b954EedeAC495271d0F": "DAI",
  "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599": "WBTC",
};

export function RouteQuoteForm({
  supportedChains,
  commonTokens,
  initialErrors = {},
  actionError,
  formData,
  onLoadingChange,
  onRoutesUpdate,
}: RouteQuoteFormProps) {
  const formRef = useRef<HTMLFormElement>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [sourceChain, setSourceChain] = useState(formData?.sourceChain || "");
  const [destinationChain, setDestinationChain] = useState(formData?.destinationChain || "");
  const [tokenAddress, setTokenAddress] = useState(formData?.tokenAddress || "");
  const [amount, setAmount] = useState(formData?.amount || "");
  const [slippage, setSlippage] = useState(formData?.slippage || "0.5");
  const debounceTimerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    // Clean up action query params while preserving form params
    if (window.location.search.includes("_action=")) {
      const params = new URLSearchParams(window.location.search);
      params.delete("_action");
      const newUrl = params.toString()
        ? `${window.location.pathname}?${params.toString()}`
        : window.location.pathname;
      window.history.replaceState({}, "", newUrl);
    }

    // Show validation errors as toasts
    if (Object.keys(initialErrors).length > 0) {
      Object.entries(initialErrors).forEach(([field, errors]) => {
        const fieldName = field.replace(/([A-Z])/g, " $1").toLowerCase();
        toast.error(`${fieldName}: ${errors.join(", ")}`);
      });
    }

    // Show action error as toast
    if (actionError) {
      console.log("Action error:", actionError);
      const errorMessage =
        typeof actionError === "string"
          ? actionError
          : (actionError as any).message || JSON.stringify(actionError) || "An error occurred";

      // Provide more helpful error messages
      let userFriendlyMessage = errorMessage;
      if (errorMessage.includes("Rate limit exceeded")) {
        userFriendlyMessage = "Too many requests. Please wait a moment and try again.";
      } else if (errorMessage.includes("No quotes available")) {
        userFriendlyMessage = "No routes found. Try a different token or chain combination.";
      } else if (errorMessage.includes("Failed to fetch")) {
        userFriendlyMessage = "Network error. Please check your connection and try again.";
      } else if (errorMessage.includes("timeout")) {
        userFriendlyMessage = "Request timed out. The bridges may be slow. Try again in a moment.";
      }

      toast.error(userFriendlyMessage);
      // Clear loading state if there was an error
      setIsLoading(false);
      onLoadingChange?.(false);
      toast.dismiss("fetching-quotes");
    }
  }, [initialErrors, actionError]);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    setIsLoading(true);
    onLoadingChange?.(true);
    toast.loading("Fetching quotes from bridges...", { id: "fetching-quotes" });

    try {
      // Validate inputs
      if (!sourceChain || !destinationChain || !tokenAddress || !amount) {
        toast.dismiss("fetching-quotes");
        toast.error("Please fill in all required fields");
        setIsLoading(false);
        onLoadingChange?.(false);
        return;
      }

      const amountNum = parseFloat(amount);
      if (isNaN(amountNum) || amountNum <= 0) {
        toast.dismiss("fetching-quotes");
        toast.error("Please enter a valid amount greater than 0");
        setIsLoading(false);
        onLoadingChange?.(false);
        return;
      }

      if (sourceChain === destinationChain) {
        toast.dismiss("fetching-quotes");
        toast.error("Source and destination chains must be different");
        setIsLoading(false);
        onLoadingChange?.(false);
        return;
      }

      // Get token symbol
      const tokenSymbol = TOKEN_SYMBOLS[tokenAddress] || "USDC";

      // Add query parameters to URL for shareability
      const params = new URLSearchParams();
      params.set("sourceChain", sourceChain);
      params.set("destinationChain", destinationChain);
      params.set("tokenAddress", tokenAddress);
      params.set("amount", amount);
      params.set("slippage", slippage || "0.5");

      // Update URL without triggering navigation
      const newUrl = `${window.location.pathname}?${params.toString()}`;
      window.history.replaceState({}, "", newUrl);

      // Call API
      const response = await getRouteQuotes({
        from_chain: sourceChain,
        to_chain: destinationChain,
        token: tokenSymbol,
        amount: amountNum,
        slippage: parseFloat(slippage) || 0.5,
      });

      toast.dismiss("fetching-quotes");

      if (response.routes && response.routes.length > 0) {
        toast.success(
          `Found ${response.routes.length} route${response.routes.length > 1 ? "s" : ""} available!`
        );

        // Update parent component with routes
        onRoutesUpdate?.(response.routes, {
          sourceChain,
          destinationChain,
          amount,
          tokenAddress,
        });

        // Scroll to results after a short delay
        setTimeout(() => {
          const resultsSection = document.querySelector('[aria-labelledby="results-heading"]');
          resultsSection?.scrollIntoView({
            behavior: "smooth",
            block: "start",
          });
        }, 300);
      } else {
        toast.error("No routes found for this combination");
        onRoutesUpdate?.([], {
          sourceChain,
          destinationChain,
          amount,
          tokenAddress,
        });
      }
    } catch (error) {
      console.error("Error fetching quotes:", error);
      toast.dismiss("fetching-quotes");

      const errorMessage = error instanceof Error ? error.message : "Failed to fetch quotes";

      // Provide more helpful error messages
      let userFriendlyMessage = errorMessage;
      if (errorMessage.includes("Rate limit exceeded")) {
        userFriendlyMessage = "Too many requests. Please wait a moment and try again.";
      } else if (errorMessage.includes("No quotes available")) {
        userFriendlyMessage = "No routes found. Try a different token or chain combination.";
      } else if (errorMessage.includes("Failed to fetch")) {
        userFriendlyMessage = "Network error. Please check your connection and try again.";
      } else if (errorMessage.includes("timeout")) {
        userFriendlyMessage = "Request timed out. The bridges may be slow. Try again in a moment.";
      }

      toast.error(userFriendlyMessage);
    } finally {
      setIsLoading(false);
      onLoadingChange?.(false);
    }
  };

  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setAmount(value);

    // Clear existing timer
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    // Debounce: wait 500ms after user stops typing
    // This prevents too many validations while typing
    debounceTimerRef.current = setTimeout(() => {
      // You can add validation or other logic here
      if (value && parseFloat(value) > 0) {
        // Valid amount entered
      }
    }, 500);
  };

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, []);

  return (
    <FadeIn>
      <section
        className="p-6 rounded-xl border bg-card shadow-lg max-w-lg mx-auto"
        aria-labelledby="quote-form-heading"
      >
        <form ref={formRef} onSubmit={handleSubmit} className="space-y-6">
          {/* Source Chain */}
          <div className="space-y-2">
            <Label htmlFor="sourceChain">
              Source Chain
              <span className="text-destructive" aria-label="required">
                *
              </span>
            </Label>
            <Select name="sourceChain" required value={sourceChain} onValueChange={setSourceChain}>
              <SelectTrigger id="sourceChain" aria-required="true" className="w-full">
                <SelectValue placeholder="Select source chain" />
              </SelectTrigger>
              <SelectContent>
                {supportedChains.map((chain) => (
                  <SelectItem key={chain.id} value={chain.id}>
                    <div className="flex items-center gap-2">
                      {chain.logoUrl && (
                        <img
                          src={chain.logoUrl}
                          alt={`${chain.name} logo`}
                          width="20"
                          height="20"
                          loading="lazy"
                          decoding="async"
                          style={{
                            width: "20px",
                            height: "20px",
                            borderRadius: "50%",
                            objectFit: "cover",
                          }}
                        />
                      )}
                      <span>{chain.name}</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Destination Chain */}
          <div className="space-y-2">
            <Label htmlFor="destinationChain">
              Destination Chain
              <span className="text-destructive" aria-label="required">
                *
              </span>
            </Label>
            <Select
              name="destinationChain"
              required
              value={destinationChain}
              onValueChange={setDestinationChain}
            >
              <SelectTrigger id="destinationChain" aria-required="true" className="w-full">
                <SelectValue placeholder="Select destination chain" />
              </SelectTrigger>
              <SelectContent>
                {supportedChains.map((chain) => (
                  <SelectItem key={chain.id} value={chain.id}>
                    <div className="flex items-center gap-2">
                      {chain.logoUrl && (
                        <img
                          src={chain.logoUrl}
                          alt={`${chain.name} logo`}
                          width="20"
                          height="20"
                          loading="lazy"
                          decoding="async"
                          style={{
                            width: "20px",
                            height: "20px",
                            borderRadius: "50%",
                            objectFit: "cover",
                          }}
                        />
                      )}
                      <span>{chain.name}</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Token */}
          <div className="space-y-2">
            <Label htmlFor="tokenAddress">
              Token
              <span className="text-destructive" aria-label="required">
                *
              </span>
            </Label>
            <Select
              name="tokenAddress"
              required
              value={tokenAddress}
              onValueChange={setTokenAddress}
            >
              <SelectTrigger id="tokenAddress" aria-required="true" className="w-full">
                <SelectValue placeholder="Select token" />
              </SelectTrigger>
              <SelectContent>
                {commonTokens.map((token) => (
                  <SelectItem key={token.address} value={token.address}>
                    <div className="flex items-center gap-2">
                      {token.logoUrl && (
                        <img
                          src={token.logoUrl}
                          alt={`${token.symbol} logo`}
                          width="20"
                          height="20"
                          loading="lazy"
                          decoding="async"
                          style={{
                            width: "20px",
                            height: "20px",
                            borderRadius: "50%",
                            objectFit: "cover",
                          }}
                        />
                      )}
                      <span>
                        {token.symbol} - {token.name}
                      </span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Amount */}
          <div className="space-y-2">
            <Label htmlFor="amount">
              Amount
              <span className="text-destructive" aria-label="required">
                *
              </span>
            </Label>
            <Input
              type="text"
              id="amount"
              name="amount"
              inputMode="decimal"
              pattern="[0-9]*[.]?[0-9]*"
              placeholder="0.0"
              required
              aria-required="true"
              aria-describedby="amount-hint"
              value={amount}
              onChange={handleAmountChange}
            />
            <p id="amount-hint" className="text-sm text-muted-foreground">
              Enter the amount you want to bridge
            </p>
          </div>

          {/* Slippage */}
          <div className="space-y-2">
            <Label htmlFor="slippage">
              Slippage Tolerance (%)
              <span className="text-muted-foreground text-xs ml-2">(Optional)</span>
            </Label>
            <Input
              type="text"
              id="slippage"
              name="slippage"
              inputMode="decimal"
              pattern="[0-9]*[.]?[0-9]*"
              placeholder="0.5"
              aria-describedby="slippage-hint"
              value={slippage}
              onChange={(e) => setSlippage(e.target.value)}
            />
            <p id="slippage-hint" className="text-sm text-muted-foreground">
              Maximum price slippage you're willing to accept (0-50%)
            </p>
          </div>

          {/* Submit Button */}
          <Button type="submit" size="lg" className="w-full" disabled={isLoading}>
            {isLoading ? (
              <span className="flex items-center gap-2">
                <LoadingSpinner size="sm" />
                Fetching Quotes...
              </span>
            ) : (
              "Get Quotes"
            )}
          </Button>
        </form>
      </section>
    </FadeIn>
  );
}
