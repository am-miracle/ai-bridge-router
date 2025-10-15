import { TextReveal } from "@/components/animations/TextReveal";
import { FadeIn } from "@/components/animations/FadeIn";

interface LegalHeaderProps {
  title: string;
  lastUpdated: string;
  effectiveDate: string;
}

export function LegalHeader({ title, lastUpdated, effectiveDate }: LegalHeaderProps) {
  return (
    <div className="text-center space-y-4 py-8 border-b border-border">
      <TextReveal delay={0.1}>
        <h1 className="text-4xl font-bold tracking-tight">{title}</h1>
      </TextReveal>
      <FadeIn delay={0.3} duration={0.8}>
        <div className="flex flex-col sm:flex-row gap-2 sm:gap-6 justify-center text-sm text-muted-foreground">
          <p>
            <span className="font-medium">Last Updated:</span> {lastUpdated}
          </p>
          <p>
            <span className="font-medium">Effective Date:</span> {effectiveDate}
          </p>
        </div>
      </FadeIn>
    </div>
  );
}
