import type { Stat } from "@/types";

interface StatCardProps {
  stat: Stat;
}

export function StatCard({ stat }: StatCardProps) {
  const isEmoji = /[\u{1F300}-\u{1F9FF}]/u.test(stat.value);
  const isComingSoon = stat.status === "coming-soon";

  const getTrendIcon = () => {
    if (!stat.trend) return null;

    if (stat.trend === "up") {
      return (
        <svg
          className="w-4 h-4 text-green-500 inline-block ml-2"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"
          />
        </svg>
      );
    }

    if (stat.trend === "down") {
      return (
        <svg
          className="w-4 h-4 text-red-500 inline-block ml-2"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M13 17h8m0 0V9m0 8l-8-8-4 4-6-6"
          />
        </svg>
      );
    }

    return null;
  };

  return (
    <div className="group relative text-center p-6 rounded-lg transition-all hover:bg-accent/50 hover:scale-105 duration-300 cursor-default">
      {isComingSoon && (
        <div className="absolute top-2 right-2 px-2 py-1 text-xs font-semibold rounded-full bg-primary/10 text-primary border border-primary/20">
          Soon
        </div>
      )}
      <div
        className={`mb-2 group-hover:scale-110 transition-all ${
          isEmoji
            ? "text-5xl filter grayscale-0 group-hover:grayscale-0"
            : "text-4xl font-bold bg-gradient-to-br from-primary to-primary/60 bg-clip-text text-transparent group-hover:from-primary/80 group-hover:to-primary"
        }`}
      >
        {stat.value}
        {!isEmoji && getTrendIcon()}
      </div>
      <div className="text-sm font-medium text-muted-foreground group-hover:text-foreground transition-colors">
        {stat.label}
      </div>
      {stat.description && (
        <div className="text-xs text-muted-foreground/70 mt-2 max-h-0 opacity-0 group-hover:max-h-20 group-hover:opacity-100 transition-all duration-300 overflow-hidden">
          {stat.description}
        </div>
      )}
    </div>
  );
}
