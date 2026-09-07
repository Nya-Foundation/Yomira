import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createContext, useContext } from "react";
import { getPageData } from "../../lib/api";

export const OfflineContext = createContext(false);

export function usePageQuery(gid: number, token: string, index: number, enabled: boolean) {
  const localOnly = useContext(OfflineContext);
  const queryClient = useQueryClient();
  const query = useQuery(pageQueryOptions(gid, token, index, localOnly, enabled));
  return {
    query,
    retry: async () => {
      const result = await query.refetch();
      if (localOnly) {
        await queryClient.invalidateQueries({ queryKey: ["downloaded-gallery", gid, token] });
      }
      return result;
    },
  };
}

export function pageQueryOptions(gid: number, token: string, index: number, localOnly: boolean, enabled = true) {
  return {
    queryKey: ["page", gid, token, index, localOnly],
    queryFn: () => getPageData(gid, token, index, localOnly),
    enabled,
    retry: false,
    staleTime: Infinity,
  };
}
