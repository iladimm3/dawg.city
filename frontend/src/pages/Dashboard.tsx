import { useEffect } from "react";
import { useNavigate, Link } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { dogsApi, trainingApi } from "@/lib/api";
import { useAuth } from "@/hooks/useAuth";
import { useDogs } from "@/hooks/useDogs";
import { DogHeroCard } from "@/components/DogHeroCard";
import { DogSelector } from "@/components/DogSelector";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { Dumbbell, Apple, Bone, Plus, Pencil, Trash2 } from "lucide-react";
import { FloatingPawIcon } from "@/components/FloatingPawIcon";
import type { TrainingLogEntry } from "@/types";

export default function Dashboard() {
  const { user } = useAuth();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { dogs, currentDog, isLoading: dogsLoading, selectDog } = useDogs();

  const { data: historyData } = useQuery({
    queryKey: ["training-history", currentDog?.id],
    queryFn: () => trainingApi.history(currentDog!.id, 5, 0),
    enabled: !!currentDog,
  });

  const recentSessions: TrainingLogEntry[] = historyData?.data ?? [];

  const deleteMutation = useMutation({
    mutationFn: (id: string) => dogsApi.delete(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["dogs"] }),
  });

  useEffect(() => {
    if (!dogsLoading && dogs && dogs.length === 0) {
      navigate("/onboarding");
    }
  }, [dogs, dogsLoading, navigate]);

  if (dogsLoading) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-12 space-y-8">
        <Skeleton className="h-10 w-48 bg-surface-container-high rounded-lg" />
        <Skeleton className="h-48 w-full bg-surface-container-high rounded-xl" />
        <div className="grid md:grid-cols-2 gap-6">
          <Skeleton className="h-40 bg-surface-container-high rounded-xl" />
          <Skeleton className="h-40 bg-surface-container-high rounded-xl" />
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto px-6 py-12 relative">
      {/* Background scattered accents */}
      <div className="absolute top-8 right-10 pointer-events-none">
        <FloatingPawIcon size={24} rotation={20} />
      </div>
      <div className="absolute bottom-20 left-4 pointer-events-none">
        <Bone
          className="text-outline opacity-15"
          size={20}
          style={{ transform: "rotate(-25deg)" }}
        />
      </div>

      {/* Greeting */}
      <h1 className="font-display text-3xl md:text-4xl font-bold text-on-surface mb-6">
        Hey, {user?.name?.split(" ")[0] ?? "there"} 👋
      </h1>

      {/* Dog Selector + Actions */}
      <div className="flex flex-wrap items-center gap-3 mb-4">
        <DogSelector dogs={dogs} currentDog={currentDog} onSelect={selectDog} />
        <div className="flex gap-2 mb-8">
          <Button
            size="sm"
            variant="ghost"
            onClick={() => navigate("/onboarding")}
            className="text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high rounded-lg gap-1"
          >
            <Plus size={14} />
            Add Dog
          </Button>
          {currentDog && (
            <>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => navigate(`/onboarding?edit=${currentDog.id}`)}
                className="text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high rounded-lg gap-1"
              >
                <Pencil size={14} />
                Edit
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  if (window.confirm(`Delete ${currentDog.name}? This cannot be undone.`)) {
                    deleteMutation.mutate(currentDog.id);
                  }
                }}
                className="text-error hover:text-error hover:bg-error/10 rounded-lg gap-1"
              >
                <Trash2 size={14} />
                Delete
              </Button>
            </>
          )}
        </div>
      </div>

      {/* Dog Hero Card */}
      {currentDog && (
        <div className="mb-12 mt-8">
          <DogHeroCard dog={currentDog} />
        </div>
      )}

      {/* Action Cards */}
      <div className="grid md:grid-cols-2 gap-6 mb-14">
        <Link to="/training">
          <div className="bg-surface-container-low rounded-xl p-8 hover:bg-surface-container transition-colors group cursor-pointer">
            <div className="w-12 h-12 bg-primary-container rounded-lg flex items-center justify-center mb-5">
              <Dumbbell className="text-primary" size={24} />
            </div>
            <h3 className="font-display text-xl font-bold text-on-surface mb-2">
              Start Training Session
            </h3>
            <p className="text-on-surface-variant text-sm font-body">
              Generate a personalized session based on{" "}
              {currentDog?.name ?? "your dog"}'s needs.
            </p>
            <span className="inline-block mt-4 text-secondary text-sm font-display font-semibold">
              Begin →
            </span>
          </div>
        </Link>

        <Link to="/nutrition">
          <div className="bg-surface-container-low rounded-xl p-8 hover:bg-surface-container transition-colors group cursor-pointer">
            <div className="w-12 h-12 bg-secondary-container rounded-lg flex items-center justify-center mb-5">
              <Apple className="text-secondary" size={24} />
            </div>
            <h3 className="font-display text-xl font-bold text-on-surface mb-2">
              Generate Nutrition Plan
            </h3>
            <p className="text-on-surface-variant text-sm font-body">
              Custom diet plan with feeding schedules and recommendations.
            </p>
            <span className="inline-block mt-4 text-secondary text-sm font-display font-semibold">
              Generate →
            </span>
          </div>
        </Link>
      </div>

      {/* Recent Training Sessions */}
      {recentSessions.length > 0 && (
        <section>
          <h2 className="font-display text-xl font-bold text-on-surface mb-6">
            Recent Training
          </h2>
          <div className="flex gap-4 overflow-x-auto pb-4 -mx-2 px-2">
            {recentSessions.map((session, i) => (
              <div
                key={session.id}
                className={`flex-none w-72 rounded-xl p-6 ${
                  i % 2 === 0
                    ? "bg-surface-container-low"
                    : "bg-surface-container"
                }`}
              >
                <h4 className="font-display font-semibold text-on-surface mb-2 truncate">
                  {session.session_title}
                </h4>
                <div className="flex items-center gap-3 text-sm text-on-surface-variant mb-2">
                  <span>{session.completed ? "Completed" : "Incomplete"}</span>
                  {session.rating && (
                    <span className="text-warning">
                      {"★".repeat(session.rating)}
                      {"☆".repeat(5 - session.rating)}
                    </span>
                  )}
                </div>
                {session.notes && (
                  <p className="text-on-surface-variant text-xs line-clamp-2">
                    {session.notes}
                  </p>
                )}
                <p className="text-outline text-xs mt-3">
                  {new Date(session.logged_at).toLocaleDateString()}
                </p>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
