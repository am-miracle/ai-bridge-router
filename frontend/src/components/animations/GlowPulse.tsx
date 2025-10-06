import { motion } from "framer-motion";
import { type ReactNode } from "react";

interface GlowPulseProps {
  children: ReactNode;
  delay?: number;
  className?: string;
}

export function GlowPulse({ children, delay = 0, className = "" }: GlowPulseProps) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{
        opacity: 1,
        scale: 1,
      }}
      transition={{
        duration: 0.5,
        delay,
      }}
      whileHover={{
        scale: 1.05,
        transition: { duration: 0.2 },
      }}
      className={className}
    >
      {children}
    </motion.div>
  );
}
