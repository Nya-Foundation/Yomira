import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "../stores/toast";

interface ToastedMutationOptions<TData, TVariables> {
  mutationFn: (vars: TVariables) => Promise<TData>;
  /** Toast title on success. Omit (or return undefined from the fn form) to stay silent. */
  successMessage?: string | ((data: TData, vars: TVariables) => string | undefined);
  /** Optional second line of the success toast. */
  successDescription?: string | ((data: TData, vars: TVariables) => string | undefined);
  /** Toast title on failure. */
  errorMessage: string;
  onSuccess?: (data: TData, vars: TVariables) => void;
  /** react-query keys invalidated after a successful mutation. */
  invalidateKeys?: unknown[][];
}

/**
 * useMutation with the app's standard toast-on-error (and optional
 * toast-on-success) behavior folded in. Message strings are already
 * translated by the caller.
 */
export function useToastedMutation<TData = unknown, TVariables = void>(
  opts: ToastedMutationOptions<TData, TVariables>,
) {
  const queryClient = useQueryClient();
  const { mutationFn, successMessage, successDescription, errorMessage, onSuccess, invalidateKeys } =
    opts;
  return useMutation({
    mutationFn,
    onSuccess: (data, vars) => {
      for (const key of invalidateKeys ?? []) {
        void queryClient.invalidateQueries({ queryKey: key });
      }
      const title =
        typeof successMessage === "function" ? successMessage(data, vars) : successMessage;
      if (title) {
        const description =
          typeof successDescription === "function"
            ? successDescription(data, vars)
            : successDescription;
        toast.success(title, description);
      }
      onSuccess?.(data, vars);
    },
    onError: () => toast.error(errorMessage),
  });
}
