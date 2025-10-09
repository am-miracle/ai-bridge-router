import { Skeleton } from "@/components/ui/skeleton";

export function RouteCardSkeleton() {
  return (
    <article className="p-5 rounded-lg border bg-card flex flex-col h-full space-y-3">
      {/* Header */}
      <div className="space-y-2">
        <Skeleton className="h-6 w-32" />
        <div className="flex gap-1.5">
          <Skeleton className="h-5 w-20" />
          <Skeleton className="h-5 w-24" />
        </div>
      </div>

      {/* Score */}
      <div className="space-y-1">
        <Skeleton className="h-4 w-16" />
        <Skeleton className="h-8 w-20" />
      </div>

      {/* Details */}
      <div className="space-y-2">
        <div className="flex justify-between">
          <Skeleton className="h-4 w-24" />
          <Skeleton className="h-4 w-20" />
        </div>
        <div className="flex justify-between">
          <Skeleton className="h-4 w-28" />
          <Skeleton className="h-4 w-24" />
        </div>
        <div className="flex justify-between">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="h-4 w-16" />
        </div>
      </div>

      {/* Button */}
      <Skeleton className="h-10 w-full mt-auto" />
    </article>
  );
}
