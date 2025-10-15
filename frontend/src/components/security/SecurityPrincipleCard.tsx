import { ShieldIcon, EyeIcon, LockIcon, CheckCircleIcon } from "@/components/ui/icons";
import { FadeIn } from "@/components/animations/FadeIn";

const iconMap = {
  shield: ShieldIcon,
  eye: EyeIcon,
  lock: LockIcon,
  check: CheckCircleIcon,
};

interface SecurityPrincipleCardProps {
  principle: {
    title: string;
    description: string;
    icon: string;
  };
  index: number;
}

export function SecurityPrincipleCard({ principle, index }: SecurityPrincipleCardProps) {
  const Icon = iconMap[principle.icon as keyof typeof iconMap] || ShieldIcon;

  return (
    <FadeIn delay={0.1 + index * 0.1} duration={0.6}>
      <div className="p-6 rounded-xl border bg-card hover:bg-card/80 transition-colors">
        <div className="h-12 w-12 rounded-xl bg-primary/10 flex items-center justify-center mb-4">
          <Icon className="h-6 w-6 text-primary" />
        </div>
        <h3 className="text-lg font-semibold mb-2">{principle.title}</h3>
        <p className="text-sm text-muted-foreground">{principle.description}</p>
      </div>
    </FadeIn>
  );
}
