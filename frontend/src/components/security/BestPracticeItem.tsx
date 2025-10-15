import { CheckCircleIcon } from "@/components/ui/icons";
import { FadeIn } from "@/components/animations/FadeIn";

interface BestPracticeItemProps {
  practice: {
    title: string;
    description: string;
  };
  index: number;
}

export function BestPracticeItem({ practice, index }: BestPracticeItemProps) {
  return (
    <FadeIn delay={0.05 + index * 0.05} duration={0.5}>
      <div className="flex gap-3 p-4 rounded-lg bg-muted/50 hover:bg-muted transition-colors">
        <div className="flex-shrink-0 mt-0.5">
          <CheckCircleIcon className="h-5 w-5 text-green-500" />
        </div>
        <div className="space-y-1">
          <h4 className="font-medium">{practice.title}</h4>
          <p className="text-sm text-muted-foreground">{practice.description}</p>
        </div>
      </div>
    </FadeIn>
  );
}
