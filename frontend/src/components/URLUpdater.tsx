import { useEffect } from "react";

interface URLUpdaterProps {
  queryParams: string;
}

export function URLUpdater({ queryParams }: URLUpdaterProps) {
  useEffect(() => {
    if (queryParams && window.location.search !== queryParams) {
      const newUrl = window.location.pathname + queryParams;
      window.history.replaceState(null, "", newUrl);
    }
  }, [queryParams]);

  return null;
}
