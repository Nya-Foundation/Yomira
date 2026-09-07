import { Skeleton } from "../ui/skeleton";

/** Loading placeholder shaped like the cover-led detail layout. */
export function DetailSkeleton() {
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-6xl p-6 pb-16">
      <div className="flex flex-col gap-8 lg:flex-row">
        <div className="w-full shrink-0 lg:w-72">
          <Skeleton className="aspect-[3/4] w-full rounded-lg" />
        </div>
        <div className="min-w-0 flex-1 space-y-4">
          <div className="flex items-center gap-2">
            <Skeleton className="h-5 w-20 rounded" />
            <Skeleton className="h-3 w-16 rounded" />
          </div>
          <div className="space-y-2">
            <Skeleton className="h-6 w-3/4 rounded" />
            <Skeleton className="h-4 w-1/2 rounded" />
          </div>
          <div className="grid max-w-lg grid-cols-2 gap-x-6 gap-y-3 pt-1 sm:grid-cols-3">
            {Array.from({ length: 6 }, (_, i) => (
              <div key={i} className="space-y-1.5">
                <Skeleton className="h-3 w-12 rounded" />
                <Skeleton className="h-4 w-20 rounded" />
              </div>
            ))}
          </div>
          <div className="flex items-center gap-3 pt-1">
            <Skeleton className="h-6 w-28 rounded" />
            <Skeleton className="h-9 w-28 rounded-md" />
            <Skeleton className="h-9 w-24 rounded-md" />
          </div>
        </div>
      </div>
      <div className="mt-8 space-y-2.5">
        {Array.from({ length: 3 }, (_, i) => (
          <div key={i} className="flex items-center gap-1.5">
            <Skeleton className="h-3.5 w-24 rounded-full" />
            <Skeleton className="h-6 w-28 rounded-full" />
            <Skeleton className="h-6 w-20 rounded-full" />
          </div>
        ))}
      </div>
      </div>
    </div>
  );
}
