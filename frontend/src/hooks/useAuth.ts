import { useQuery } from "@tanstack/react-query";
import { authApi } from "@/lib/api";
import type { User } from "@/types";

export function useAuth() {
  const {
    data: user,
    isLoading,
    error,
  } = useQuery<User>({
    queryKey: ["me"],
    queryFn: authApi.me,
    retry: false,
    staleTime: 1000 * 60 * 5,
  });

  return {
    user: user ?? null,
    isLoading,
    isAuthenticated: !!user && !error,
  };
}
