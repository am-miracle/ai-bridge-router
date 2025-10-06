import { motion } from "framer-motion";
import { ZapIcon, ShieldIcon, ChartIcon } from "@/components/ui/icons";
import type { Feature } from "@/types";

const iconMap = {
  zap: ZapIcon,
  shield: ShieldIcon,
  chart: ChartIcon,
};

interface AnimatedFeatureCardProps {
  feature: Feature;
  index: number;
}

export function AnimatedFeatureCard({ feature, index }: AnimatedFeatureCardProps) {
  const Icon = iconMap[feature.icon as keyof typeof iconMap] || ZapIcon;

  return (
    <motion.div
      initial={{ opacity: 0, y: 50 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-100px" }}
      transition={{
        duration: 0.6,
        delay: index * 0.2,
        ease: [0.21, 0.45, 0.27, 0.9],
      }}
      whileHover={{
        y: -8,
        transition: { duration: 0.3 },
      }}
      className="p-6 rounded-xl border bg-card hover:shadow-xl transition-all group"
    >
      <motion.div
        initial={{ scale: 0 }}
        whileInView={{ scale: 1 }}
        viewport={{ once: true }}
        transition={{
          duration: 0.5,
          delay: index * 0.2 + 0.3,
          type: "spring",
          stiffness: 200,
        }}
        className="h-12 w-12 rounded-xl bg-primary/10 flex items-center justify-center mb-4 group-hover:bg-primary/20 transition-colors"
      >
        <Icon className="h-6 w-6 text-primary" />
      </motion.div>
      <h3 className="text-lg font-semibold mb-2">{feature.title}</h3>
      <p className="text-sm text-muted-foreground">{feature.description}</p>
    </motion.div>
  );
}
