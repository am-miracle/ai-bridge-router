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
}

export function RouteQuoteForm({
  supportedChains,
  commonTokens,
  initialErrors = {},
  actionError,
  actionUrl,
  formData,
}: RouteQuoteFormProps) {
  const formRef = useRef<HTMLFormElement>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [sourceChain, setSourceChain] = useState(formData?.sourceChain || "");
  const [destinationChain, setDestinationChain] = useState(formData?.destinationChain || "");
  const [tokenAddress, setTokenAddress] = useState(formData?.tokenAddress || "");
  const [amount, setAmount] = useState(formData?.amount || "");
  const [slippage, setSlippage] = useState(formData?.slippage || "0.5");

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
      toast.error(errorMessage);
      // Clear loading state if there was an error
      setIsLoading(false);
      toast.dismiss("fetching-quotes");
    }
  }, [initialErrors, actionError]);

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    setIsLoading(true);
    toast.loading("Fetching quotes from bridges...", { id: "fetching-quotes" });

    // Add query parameters to URL for shareability
    const formData = new FormData(e.currentTarget);
    const params = new URLSearchParams();
    params.set("sourceChain", formData.get("sourceChain") as string);
    params.set("destinationChain", formData.get("destinationChain") as string);
    params.set("tokenAddress", formData.get("tokenAddress") as string);
    params.set("amount", formData.get("amount") as string);
    params.set("slippage", (formData.get("slippage") as string) || "0.5");

    // Update URL without triggering navigation
    const newUrl = `${window.location.pathname}?${params.toString()}`;
    window.history.replaceState({}, "", newUrl);
  };

  return (
    <FadeIn>
      <section
        className="p-6 rounded-xl border bg-card shadow-lg max-w-lg mx-auto"
        aria-labelledby="quote-form-heading"
      >
        {/*<h2 id="quote-form-heading" className="text-2xl font-bold mb-6">
          Get Route Quotes
        </h2>*/}

        <form
          ref={formRef}
          method="POST"
          action={actionUrl}
          onSubmit={(e) => {
            handleSubmit(e);
          }}
          className="space-y-6"
        >
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
                    {chain.name}
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
                    {chain.name}
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
                    {token.symbol} - {token.name}
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
              onChange={(e) => setAmount(e.target.value)}
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

          {/* Recipient Address (Optional) */}
          {/*<div className="space-y-2">
                    <Label htmlFor="recipientAddress">
                        Recipient Address
                        <span className="text-muted-foreground text-xs ml-2">
                            (Optional)
                        </span>
                    </Label>
                    <Input
                        type="text"
                        id="recipientAddress"
                        name="recipientAddress"
                        placeholder="0x..."
                        aria-describedby="recipientAddress-hint"
                    />
                    <p
                        id="recipientAddress-hint"
                        className="text-sm text-muted-foreground"
                    >
                        Leave empty to send to your connected wallet
                    </p>
                </div>*/}

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
