import { TextReveal } from "@/components/animations/TextReveal";
import { FadeIn } from "@/components/animations/FadeIn";
import { FloatingElement } from "@/components/animations/FloatingElement";
import { Button } from "@/components/ui/button";

interface HeroProps {
  title: string;
  description: string;
  primaryCTA: { label: string; href: string };
  secondaryCTA: { label: string; href: string };
}

export function AnimatedHero({ title, description, primaryCTA, secondaryCTA }: HeroProps) {
  return (
    <div className="text-center space-y-8 md:space-y-10 relative">
      {/* Animated background gradient */}
      <FloatingElement delay={0} duration={8} className="absolute inset-0 -z-10 opacity-30">
        <div className="absolute top-1/4 left-1/4 w-64 h-64 md:w-96 md:h-96 bg-primary/20 rounded-full blur-3xl" />
      </FloatingElement>
      <FloatingElement delay={2} duration={10} className="absolute inset-0 -z-10 opacity-20">
        <div className="absolute bottom-1/4 right-1/4 w-64 h-64 md:w-96 md:h-96 bg-blue-500/20 rounded-full blur-3xl" />
      </FloatingElement>
      <FloatingElement delay={4} duration={12} className="absolute inset-0 -z-10 opacity-15">
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-80 h-80 md:w-[500px] md:h-[500px] bg-purple-500/10 rounded-full blur-3xl" />
      </FloatingElement>

      <TextReveal delay={0.1}>
        <h1 className="text-4xl sm:text-5xl md:text-6xl lg:text-7xl font-bold tracking-tight bg-gradient-to-r from-foreground via-primary to-foreground bg-clip-text text-transparent animate-gradient px-4">
          {title}
        </h1>
      </TextReveal>

      <FadeIn delay={0.3} duration={0.8}>
        <p className="text-lg sm:text-xl md:text-2xl text-muted-foreground max-w-3xl mx-auto px-6">
          {description}
        </p>
      </FadeIn>

      <FadeIn delay={0.5} direction="up" duration={0.6}>
        <div className="flex flex-col sm:flex-row gap-4 justify-center items-center pt-4 px-6">
          <Button
            size="lg"
            className="w-full sm:w-auto text-base md:text-lg px-8 py-6 rounded-xl"
            asChild
          >
            <a href={primaryCTA.href}>{primaryCTA.label}</a>
          </Button>
          <Button
            size="lg"
            variant="outline"
            className="w-full sm:w-auto text-base md:text-lg px-8 py-6 rounded-xl"
            asChild
          >
            <a href={secondaryCTA.href}>{secondaryCTA.label}</a>
          </Button>
        </div>
      </FadeIn>
    </div>
  );
}
