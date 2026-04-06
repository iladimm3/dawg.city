import { useState, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { dogsApi } from "@/lib/api";
import type { Dog } from "@/types";

export function useDogs() {
  const { data: dogs, isLoading } = useQuery<Dog[]>({
    queryKey: ["dogs"],
    queryFn: dogsApi.list,
  });

  const [selectedId, setSelectedId] = useState<string | null>(null);

  const currentDog: Dog | undefined =
    dogs?.find((d) => d.id === selectedId) ?? dogs?.[0];

  const selectDog = useCallback((id: string) => setSelectedId(id), []);

  return { dogs: dogs ?? [], currentDog, isLoading, selectDog };
}
