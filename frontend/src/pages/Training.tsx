import { useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { dogsApi, trainingApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { GlassDialog } from "@/components/GlassDialog";
import { FloatingPawIcon } from "@/components/FloatingPawIcon";
import { Dumbbell, Star, Bone, CheckCircle } from "lucide-react";
import type { Dog, TrainingSession, TrainingRequest } from "@/types";

const FOCUS_AREAS = [
  "Obedience",
  "Leash Walking",
  "Recall",
  "Socialization",
  "Agility",
  "Tricks",
  "Impulse Control",
  "Crate Training",
  "Separation Anxiety",
  "Reactivity",
];

export default function Training() {
  const { data: dogs } = useQuery<Dog[]>({
    queryKey: ["dogs"],
    queryFn: dogsApi.list,
  });
  const dog = dogs?.[0];

  const [focusAreas, setFocusAreas] = useState<string[]>([]);
  const [duration, setDuration] = useState(15);
  const [difficulty, setDifficulty] = useState("medium");
  const [lastNotes, setLastNotes] = useState("");

  const [session, setSession] = useState<TrainingSession | null>(null);
  const [logOpen, setLogOpen] = useState(false);
  const [logRating, setLogRating] = useState(3);
  const [logNotes, setLogNotes] = useState("");

  const generateMutation = useMutation({
    mutationFn: (data: TrainingRequest) => trainingApi.generateSession(data),
    onSuccess: (data: TrainingSession) => setSession(data),
  });

  const logMutation = useMutation({
    mutationFn: () =>
      trainingApi.logSession({
        dog_id: dog!.id,
        session_title: session!.title,
        completed: true,
        notes: logNotes || undefined,
        rating: logRating,
      }),
    onSuccess: () => {
      setLogOpen(false);
      setLogNotes("");
      setLogRating(3);
    },
  });

  const toggleFocus = (area: string) => {
    setFocusAreas((prev) =>
      prev.includes(area) ? prev.filter((a) => a !== area) : [...prev, area],
    );
  };

  const handleGenerate = () => {
    if (!dog) return;
    generateMutation.mutate({
      dog_id: dog.id,
      focus_areas: focusAreas,
      session_length_minutes: duration,
      difficulty,
      last_session_notes: lastNotes || undefined,
    });
  };

  return (
    <div className="max-w-4xl mx-auto px-6 py-12 relative">
      {/* Scattered accents */}
      <div className="absolute top-6 right-8 pointer-events-none">
        <FloatingPawIcon size={22} rotation={-12} />
      </div>
      <div className="absolute bottom-32 left-2 pointer-events-none">
        <Bone className="text-outline opacity-15" size={18} style={{ transform: "rotate(20deg)" }} />
      </div>

      <h1 className="font-display text-3xl md:text-4xl font-bold text-on-surface mb-2">
        Training Studio
      </h1>
      <p className="text-on-surface-variant font-body mb-10">
        {dog
          ? `Craft the perfect session for ${dog.name}`
          : "Loading your dog..."}
      </p>

      {/* Focus Areas */}
      <section className="mb-8">
        <Label className="text-on-surface-variant text-sm mb-3 block">
          Focus Areas
        </Label>
        <div className="flex flex-wrap gap-2">
          {FOCUS_AREAS.map((area) => (
            <Badge
              key={area}
              variant="outline"
              className={`cursor-pointer rounded-xl px-4 py-2 text-sm font-body transition-colors border-0 ${
                focusAreas.includes(area)
                  ? "bg-secondary-container text-secondary"
                  : "bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest"
              }`}
              onClick={() => toggleFocus(area)}
            >
              {area}
            </Badge>
          ))}
        </div>
      </section>

      {/* Duration + Difficulty */}
      <div className="bg-surface-container-highest rounded-xl p-6 mb-8 space-y-6">
        <div>
          <Label className="text-on-surface-variant text-sm mb-3 block">
            Duration: {duration} min
          </Label>
          <Slider
            value={[duration]}
            onValueChange={(val) => setDuration(Array.isArray(val) ? val[0] : val)}
            min={5}
            max={60}
            step={5}
            className="[&_[role=slider]]:bg-primary"
          />
        </div>

        <div>
          <Label className="text-on-surface-variant text-sm mb-3 block">
            Difficulty
          </Label>
          <Select value={difficulty} onValueChange={(v) => v && setDifficulty(v)}>
            <SelectTrigger className="bg-surface-container-high rounded-lg text-on-surface border-0 w-48">
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="bg-surface-container-highest text-on-surface border-0 rounded-lg">
              <SelectItem value="easy">Easy</SelectItem>
              <SelectItem value="medium">Medium</SelectItem>
              <SelectItem value="hard">Hard</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div>
          <Label className="text-on-surface-variant text-sm mb-3 block">
            Notes from last session (optional)
          </Label>
          <Textarea
            placeholder="How did the last one go?"
            className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0"
            value={lastNotes}
            onChange={(e) => setLastNotes(e.target.value)}
          />
        </div>
      </div>

      {/* Generate Button */}
      <Button
        onClick={handleGenerate}
        disabled={generateMutation.isPending || !dog || focusAreas.length === 0}
        className="w-full bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-xl py-6 text-base font-display font-semibold mb-12"
      >
        {generateMutation.isPending ? (
          "Generating..."
        ) : (
          <>
            <Dumbbell className="mr-2" size={20} />
            Generate Session
          </>
        )}
      </Button>

      {generateMutation.isError && (
        <p className="text-error text-sm text-center mb-6">
          Failed to generate session. Please try again.
        </p>
      )}

      {/* Results */}
      {session && (
        <section className="space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="font-display text-2xl font-bold text-on-surface">
              {session.title}
            </h2>
            <span className="text-on-surface-variant text-sm">
              {session.duration_minutes} min
            </span>
          </div>

          <p className="text-secondary font-body italic">
            {session.encouragement}
          </p>

          {/* Exercises */}
          <div className="space-y-4">
            {session.exercises.map((ex, i) => (
              <div
                key={i}
                className="bg-surface-container-low rounded-xl p-6 relative"
              >
                {/* Overlapping icon */}
                <div className="absolute -top-3 -left-3 w-8 h-8 bg-primary-container rounded-lg flex items-center justify-center">
                  <span className="text-primary font-display font-bold text-sm">
                    {i + 1}
                  </span>
                </div>
                <h4 className="font-display font-semibold text-on-surface mb-2 ml-4">
                  {ex.name}
                </h4>
                <p className="text-on-surface-variant text-sm mb-3">
                  {ex.description}
                </p>
                <div className="flex gap-4 text-xs text-on-surface-variant">
                  <span>Reps: {ex.repetitions}</span>
                  <span>
                    <CheckCircle size={12} className="inline mr-1" />
                    {ex.success_criteria}
                  </span>
                </div>
              </div>
            ))}
          </div>

          {/* Tips */}
          {session.tips.length > 0 && (
            <div className="bg-surface-container rounded-xl p-6">
              <h3 className="font-display font-semibold text-on-surface mb-3">
                Pro Tips
              </h3>
              <ul className="space-y-2">
                {session.tips.map((tip, i) => (
                  <li
                    key={i}
                    className="text-on-surface-variant text-sm font-body flex gap-2"
                  >
                    <span className="text-secondary">•</span>
                    {tip}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Log Button */}
          <Button
            onClick={() => setLogOpen(true)}
            className="bg-secondary text-on-secondary rounded-xl px-8 py-5 font-display font-semibold"
          >
            Log this session
          </Button>
        </section>
      )}

      {/* Log Dialog */}
      <GlassDialog
        open={logOpen}
        onOpenChange={setLogOpen}
        title="Log Training Session"
        description={session?.title}
      >
        <div className="space-y-6">
          <div>
            <Label className="text-on-surface-variant text-sm mb-3 block">
              How did it go?
            </Label>
            <div className="flex gap-2">
              {[1, 2, 3, 4, 5].map((n) => (
                <button
                  key={n}
                  type="button"
                  onClick={() => setLogRating(n)}
                  className="transition-transform hover:scale-110"
                >
                  <Star
                    size={28}
                    className={
                      n <= logRating
                        ? "text-warning fill-warning"
                        : "text-outline"
                    }
                  />
                </button>
              ))}
            </div>
          </div>

          <div>
            <Label className="text-on-surface-variant text-sm mb-3 block">
              Notes (optional)
            </Label>
            <Textarea
              placeholder="What went well? Any struggles?"
              className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0"
              value={logNotes}
              onChange={(e) => setLogNotes(e.target.value)}
            />
          </div>

          <Button
            onClick={() => logMutation.mutate()}
            disabled={logMutation.isPending}
            className="w-full bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-lg py-5 font-display font-semibold"
          >
            {logMutation.isPending ? "Saving..." : "Save Log"}
          </Button>
        </div>
      </GlassDialog>
    </div>
  );
}
