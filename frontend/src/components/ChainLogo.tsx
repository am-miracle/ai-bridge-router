import { OptimizedImage } from "./OptimizedImage";

interface ChainLogoProps {
  logoUrl?: string;
  name: string;
  size?: number;
  className?: string;
}

/**
 * Optimized chain logo component with caching-friendly attributes
 */
export function ChainLogo({ logoUrl, name, size = 20, className = "" }: ChainLogoProps) {
  if (!logoUrl) return null;

  return (
    <OptimizedImage
      src={logoUrl}
      alt={`${name} logo`}
      width={size}
      height={size}
      className={className}
      fallback="/chain-fallback.svg"
    />
  );
}
