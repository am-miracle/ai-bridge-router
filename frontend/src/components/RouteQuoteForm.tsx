import { useRef, useEffect, useState } from "react";
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
  actionError?: { message: string };
  actionUrl: string;
}

export function RouteQuoteForm({
  supportedChains,
  commonTokens,
  initialErrors = {},
  actionError,
  actionUrl,
}: RouteQuoteFormProps) {
  const formRef = useRef<HTMLFormElement>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    // Clear query string on mount if present
    if (window.location.search.includes("_action=")) {
      window.history.replaceState({}, "", window.location.pathname);
    }
  }, []);

  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    setIsLoading(true);
  };

  return (
    <FadeIn>
      <section
        className="p-6 rounded-xl border bg-card shadow-lg"
        aria-labelledby="quote-form-heading"
      >
        <h2 id="quote-form-heading" className="text-2xl font-bold mb-6">
          Get Route Quotes
        </h2>

        <form
          ref={formRef}
          method="POST"
          action={actionUrl}
          onSubmit={handleSubmit}
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
            <Select name="sourceChain" required>
              <SelectTrigger
                id="sourceChain"
                aria-required="true"
                aria-invalid={initialErrors.sourceChain ? "true" : "false"}
                aria-describedby={initialErrors.sourceChain ? "sourceChain-error" : undefined}
                className="w-full"
              >
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
            {initialErrors.sourceChain && (
              <p id="sourceChain-error" className="text-sm text-destructive" role="alert">
                {initialErrors.sourceChain.join(", ")}
              </p>
            )}
          </div>

          {/* Destination Chain */}
          <div className="space-y-2">
            <Label htmlFor="destinationChain">
              Destination Chain
              <span className="text-destructive" aria-label="required">
                *
              </span>
            </Label>
            <Select name="destinationChain" required>
              <SelectTrigger
                id="destinationChain"
                aria-required="true"
                aria-invalid={initialErrors.destinationChain ? "true" : "false"}
                aria-describedby={
                  initialErrors.destinationChain ? "destinationChain-error" : undefined
                }
                className="w-full"
              >
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
            {initialErrors.destinationChain && (
              <p id="destinationChain-error" className="text-sm text-destructive" role="alert">
                {initialErrors.destinationChain.join(", ")}
              </p>
            )}
          </div>

          {/* Token */}
          <div className="space-y-2">
            <Label htmlFor="tokenAddress">
              Token
              <span className="text-destructive" aria-label="required">
                *
              </span>
            </Label>
            <Select name="tokenAddress" required>
              <SelectTrigger
                id="tokenAddress"
                aria-required="true"
                aria-invalid={initialErrors.tokenAddress ? "true" : "false"}
                aria-describedby={initialErrors.tokenAddress ? "tokenAddress-error" : undefined}
                className="w-full"
              >
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
            {initialErrors.tokenAddress && (
              <p id="tokenAddress-error" className="text-sm text-destructive" role="alert">
                {initialErrors.tokenAddress.join(", ")}
              </p>
            )}
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
              aria-invalid={initialErrors.amount ? "true" : "false"}
              aria-describedby={initialErrors.amount ? "amount-error" : "amount-hint"}
            />
            {initialErrors.amount ? (
              <p id="amount-error" className="text-sm text-destructive" role="alert">
                {initialErrors.amount.join(", ")}
              </p>
            ) : (
              <p id="amount-hint" className="text-sm text-muted-foreground">
                Enter the amount you want to bridge
              </p>
            )}
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
              defaultValue="0.5"
              aria-invalid={initialErrors.slippage ? "true" : "false"}
              aria-describedby={initialErrors.slippage ? "slippage-error" : "slippage-hint"}
            />
            {initialErrors.slippage ? (
              <p id="slippage-error" className="text-sm text-destructive" role="alert">
                {initialErrors.slippage.join(", ")}
              </p>
            ) : (
              <p id="slippage-hint" className="text-sm text-muted-foreground">
                Maximum price slippage you're willing to accept (0-50%)
              </p>
            )}
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

          {/* General Error Message */}
          {actionError && (
            <div
              className="p-4 rounded-lg bg-destructive/10 border border-destructive"
              role="alert"
            >
              <p className="text-sm text-destructive font-medium">{actionError.message}</p>
            </div>
          )}
        </form>
      </section>
    </FadeIn>
  );
}
