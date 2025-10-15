import { FadeIn } from "@/components/animations/FadeIn";

interface LegalSectionProps {
  section: {
    title: string;
    content?: string[];
    subsections?: {
      subtitle: string;
      content: string[];
    }[];
  };
  index: number;
}

export function LegalSection({ section, index }: LegalSectionProps) {
  const sectionId = section.title
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9-]/g, "");

  return (
    <FadeIn delay={0.05 + index * 0.05} duration={0.6}>
      <div className="space-y-4" id={sectionId}>
        <h2 className="text-xl font-bold text-foreground scroll-mt-20">{section.title}</h2>

        {section.content && (
          <div className="space-y-3 text-muted-foreground">
            {section.content.map((paragraph, idx) => (
              <p key={idx} className="leading-relaxed">
                {paragraph}
              </p>
            ))}
          </div>
        )}

        {section.subsections && (
          <div className="space-y-6 ml-4">
            {section.subsections.map((subsection, subIdx) => (
              <div key={subIdx} className="space-y-2">
                <h3 className="text-lg font-semibold text-foreground">{subsection.subtitle}</h3>
                <div className="space-y-2 text-muted-foreground">
                  {subsection.content.map((paragraph, pIdx) => (
                    <p key={pIdx} className="leading-relaxed">
                      {paragraph}
                    </p>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </FadeIn>
  );
}
