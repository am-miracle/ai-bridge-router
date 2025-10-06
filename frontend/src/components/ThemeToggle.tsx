import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { MoonIcon, SunIcon } from "@/components/ui/icons";
import {
  getInitialTheme,
  setStoredTheme,
  applyTheme,
  toggleTheme as toggleThemeUtil,
} from "@/lib/theme";
import type { Theme } from "@/types";

export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>("light");

  useEffect(() => {
    const initialTheme = getInitialTheme();
    setTheme(initialTheme);
  }, []);

  const handleToggle = () => {
    const newTheme = toggleThemeUtil(theme);
    setTheme(newTheme);
    setStoredTheme(newTheme);
    applyTheme(newTheme);
  };

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={handleToggle}
      aria-label="Toggle theme"
      className="rounded-full"
    >
      {theme === "light" ? <MoonIcon /> : <SunIcon />}
    </Button>
  );
}
