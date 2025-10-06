import { TextReveal } from "@/components/animations/TextReveal";
import { FadeIn } from "@/components/animations/FadeIn";

export function AnimatedSupportHero() {
  return (
    <section className="text-center space-y-4 py-8">
      <TextReveal delay={0.1}>
        <h1 className="text-4xl font-bold tracking-tight">How can we help you?</h1>
      </TextReveal>
      <FadeIn delay={0.3} duration={0.8}>
        <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
          Find answers to common questions or get in touch with our team.
        </p>
      </FadeIn>
    </section>
  );
}
