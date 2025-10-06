import { ZapIcon, ShieldIcon, ChartIcon } from "@/components/ui/icons";
import type { Feature } from "@/types";

const iconMap = {
  zap: ZapIcon,
  shield: ShieldIcon,
  chart: ChartIcon,
};

interface FeatureCardProps {
  feature: Feature;
}

export function FeatureCard({ feature }: FeatureCardProps) {
  const Icon = iconMap[feature.icon as keyof typeof iconMap] || ZapIcon;

  return (
    <div className="p-6 rounded-xl border bg-card">
      <div className="h-12 w-12 rounded-xl bg-primary/10 flex items-center justify-center mb-4">
        <Icon className="h-6 w-6 text-primary" />
      </div>
      <h3 className="text-lg font-semibold mb-2">{feature.title}</h3>
      <p className="text-sm text-muted-foreground">{feature.description}</p>
    </div>
  );
}
