import { useState } from "react";

interface OptimizedImageProps {
  src: string;
  alt: string;
  width?: number;
  height?: number;
  className?: string;
  priority?: boolean;
  fallback?: string;
}

/**
 * Optimized image component with lazy loading and fallback support
 */
export function OptimizedImage({
  src,
  alt,
  width,
  height,
  className = "",
  priority = false,
  fallback = "/placeholder.svg",
}: OptimizedImageProps) {
  const [imgSrc, setImgSrc] = useState(src);
  const [isLoaded, setIsLoaded] = useState(false);

  const handleError = () => {
    if (imgSrc !== fallback) {
      setImgSrc(fallback);
    }
  };

  const handleLoad = () => {
    setIsLoaded(true);
  };

  return (
    <img
      src={imgSrc}
      alt={alt}
      width={width}
      height={height}
      loading={priority ? "eager" : "lazy"}
      decoding="async"
      fetchPriority={priority ? "high" : "auto"}
      className={`${className} ${isLoaded ? "opacity-100" : "opacity-0"} transition-opacity duration-300`}
      onError={handleError}
      onLoad={handleLoad}
    />
  );
}
