import { FadeIn } from "@/components/animations/FadeIn";

interface HeroSectionProps {
  title: string;
  description: string;
}

export function HeroSection({ title, description }: HeroSectionProps) {
  return (
    <section className="text-center space-y-4 py-8">
      <FadeIn direction="down" duration={0.6}>
        <h1 className="text-4xl font-bold tracking-tight">{title}</h1>
      </FadeIn>
      <FadeIn delay={0.2} duration={0.6}>
        <p className="text-lg text-muted-foreground max-w-2xl mx-auto">{description}</p>
      </FadeIn>
    </section>
  );
}
