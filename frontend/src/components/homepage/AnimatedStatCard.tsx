import { motion } from "framer-motion";
import type { Stat } from "@/types";

interface AnimatedStatCardProps {
  stat: Stat;
  index: number;
}

export function AnimatedStatCard({ stat, index }: AnimatedStatCardProps) {
  const isEmoji = /[\u{1F300}-\u{1F9FF}]/u.test(stat.value);
  const isComingSoon = stat.status === "coming-soon";

  const getTrendIcon = () => {
    if (!stat.trend) return null;

    if (stat.trend === "up") {
      return (
        <motion.svg
          initial={{ opacity: 0, y: 5 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: index * 0.15 + 0.5 }}
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
        </motion.svg>
      );
    }

    if (stat.trend === "down") {
      return (
        <motion.svg
          initial={{ opacity: 0, y: 5 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: index * 0.15 + 0.5 }}
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
        </motion.svg>
      );
    }

    return null;
  };

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.5 }}
      whileInView={{ opacity: 1, scale: 1 }}
      viewport={{ once: true, margin: "-50px" }}
      transition={{
        duration: 0.5,
        delay: index * 0.15,
        type: "spring",
        stiffness: 100,
      }}
      whileHover={{
        scale: 1.1,
        rotate: [0, -2, 2, 0],
        transition: { duration: 0.3 },
      }}
      className="group relative text-center p-6 rounded-xl transition-all hover:bg-accent/50 cursor-default"
    >
      {isComingSoon && (
        <motion.div
          initial={{ scale: 0 }}
          animate={{ scale: 1 }}
          transition={{ delay: index * 0.15 + 0.3, type: "spring" }}
          className="absolute top-2 right-2 px-2 py-1 text-xs font-semibold rounded-full bg-primary/10 text-primary border border-primary/20"
        >
          Soon
        </motion.div>
      )}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ delay: index * 0.15 + 0.2 }}
        className={`mb-2 transition-all ${
          isEmoji
            ? "text-5xl filter grayscale-0 group-hover:grayscale-0"
            : "text-4xl font-bold bg-gradient-to-br from-primary to-primary/60 bg-clip-text text-transparent group-hover:from-primary/80 group-hover:to-primary"
        }`}
      >
        {stat.value}
        {!isEmoji && getTrendIcon()}
      </motion.div>
      <motion.div
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        viewport={{ once: true }}
        transition={{ delay: index * 0.15 + 0.3 }}
        className="text-sm font-medium text-muted-foreground group-hover:text-foreground transition-colors"
      >
        {stat.label}
      </motion.div>
      {stat.description && (
        <div className="text-xs text-muted-foreground/70 mt-2 opacity-0 max-h-0 overflow-hidden group-hover:opacity-100 group-hover:max-h-20 transition-all duration-300">
          {stat.description}
        </div>
      )}
    </motion.div>
  );
}
