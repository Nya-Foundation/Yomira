/** The parts of an infinite query the grid needs, named structurally so this
 *  does not depend on react-query's generics. */
interface InfiniteQueryLike {
  isLoading: boolean;
  error: unknown;
  isFetchNextPageError: boolean;
  isFetchingNextPage: boolean;
  hasNextPage: boolean | undefined;
  fetchNextPage: () => unknown;
  refetch: () => unknown;
}

/**
 * The grid props that follow mechanically from an infinite query, so every
 * feed wires them the same way. Retry resumes the failed page rather than
 * refetching the whole list, which would throw away what is already loaded.
 */
export function infiniteGridProps(query: InfiniteQueryLike) {
  return {
    loading: query.isLoading,
    error: query.error,
    onRetry: () =>
      void (query.isFetchNextPageError ? query.fetchNextPage() : query.refetch()),
    hasNextPage: query.hasNextPage ?? false,
    isFetchingNextPage: query.isFetchingNextPage,
    onLoadMore: () => void query.fetchNextPage(),
  };
}
