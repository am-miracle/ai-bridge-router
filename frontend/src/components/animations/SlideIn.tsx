import { motion } from "framer-motion";
import { type ReactNode } from "react";

interface SlideInProps {
  children: ReactNode;
  delay?: number;
  duration?: number;
  direction?: "left" | "right";
  className?: string;
}

export function SlideIn({
  children,
  delay = 0,
  duration = 0.6,
  direction = "left",
  className = "",
}: SlideInProps) {
  return (
    <motion.div
      initial={{
        opacity: 0,
        x: direction === "left" ? -100 : 100,
      }}
      animate={{ opacity: 1, x: 0 }}
      transition={{
        duration,
        delay,
        ease: [0.25, 0.46, 0.45, 0.94],
      }}
      className={className}
    >
      {children}
    </motion.div>
  );
}
