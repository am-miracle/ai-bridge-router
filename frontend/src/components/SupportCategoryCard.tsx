import { BookIcon, BridgeIcon, ToolIcon, CodeIcon } from "@/components/ui/icons";
import type { SupportCategory } from "@/types";

const iconMap = {
  book: BookIcon,
  bridge: BridgeIcon,
  tool: ToolIcon,
  code: CodeIcon,
};

interface SupportCategoryCardProps {
  category: SupportCategory;
}

export function SupportCategoryCard({ category }: SupportCategoryCardProps) {
  const Icon = iconMap[category.icon as keyof typeof iconMap] || BookIcon;

  return (
    <a
      href={category.href}
      className="group p-6 rounded-lg border bg-card hover:bg-accent/50 hover:border-primary/50 transition-all duration-300"
    >
      <div className="flex items-start gap-4">
        <div className="p-3 rounded-lg bg-primary/10 group-hover:bg-primary/20 transition-colors">
          <Icon className="h-6 w-6 text-primary" />
        </div>
        <div className="flex-1">
          <h3 className="font-semibold mb-1 group-hover:text-primary transition-colors">
            {category.title}
          </h3>
          <p className="text-sm text-muted-foreground">{category.description}</p>
        </div>
        <svg
          className="w-5 h-5 text-muted-foreground group-hover:text-primary group-hover:translate-x-1 transition-all"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
        </svg>
      </div>
    </a>
  );
}
