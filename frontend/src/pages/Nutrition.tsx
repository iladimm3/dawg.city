import { useState, useRef } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { nutritionApi } from "@/lib/api";
import { useDogs } from "@/hooks/useDogs";
import { DogSelector } from "@/components/DogSelector";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { FloatingPawIcon } from "@/components/FloatingPawIcon";
import { Apple, Plus, X, Bone, Clock, Pill, FileText, History, FileDown } from "lucide-react";
import { toast } from "sonner";
import type { NutritionPlan, NutritionPlanRecord, NutritionRequest } from "@/types";

const DIETARY_OPTIONS = [
  "Grain-free",
  "Raw diet",
  "Limited ingredient",
  "High protein",
  "Low fat",
  "Hypoallergenic",
  "Senior formula",
  "Puppy formula",
];

const ISSUE_OPTIONS = [
  "Overweight",
  "Underweight",
  "Allergies",
  "Sensitive stomach",
  "Low energy",
  "Joint issues",
  "Dental problems",
  "Picky eater",
];

export default function Nutrition() {
  const { dogs, currentDog: dog, selectDog } = useDogs();

  const [foodBrand, setFoodBrand] = useState("");
  const [restrictions, setRestrictions] = useState<string[]>([]);
  const [goal, setGoal] = useState("");
  const [issues, setIssues] = useState<string[]>([]);
  const [plan, setPlan] = useState<NutritionPlan | null>(null);
  const [activeTab, setActiveTab] = useState<"generate" | "history">("generate");
  const pdfRef = useRef<HTMLDivElement>(null);

  const { data: historyData } = useQuery({
    queryKey: ["nutrition-history", dog?.id],
    queryFn: () => nutritionApi.history(dog!.id, 5, 0),
    enabled: !!dog,
  });
  const pastPlans: NutritionPlanRecord[] = historyData?.data ?? [];

  const toggleItem = (
    arr: string[],
    setArr: React.Dispatch<React.SetStateAction<string[]>>,
    item: string,
  ) => {
    setArr(arr.includes(item) ? arr.filter((a) => a !== item) : [...arr, item]);
  };

  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: (data: NutritionRequest) => nutritionApi.generatePlan(data),
    onSuccess: (data: NutritionPlan) => {
      setPlan(data);
      queryClient.invalidateQueries({ queryKey: ["nutrition-history", dog?.id] });
      toast.success("Nutrition plan generated!");
    },
    onError: (err: Error & { response?: { data?: { message?: string } } }) => {
      const msg = err?.response?.data?.message ?? "Failed to generate plan.";
      toast.error(msg);
    },
  });

  const handleExportPDF = async () => {
    if (!pdfRef.current) return;
    try {
      const { default: jsPDF } = await import("jspdf");
      const { default: html2canvas } = await import("html2canvas");
      const canvas = await html2canvas(pdfRef.current, { scale: 2, backgroundColor: "#1a0a2e" });
      const img = canvas.toDataURL("image/png");
      const pdf = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });
      const pdfWidth = pdf.internal.pageSize.getWidth();
      const pdfHeight = (canvas.height * pdfWidth) / canvas.width;
      pdf.addImage(img, "PNG", 0, 0, pdfWidth, pdfHeight);
      pdf.save(`nutrition-plan-${dog?.name?.toLowerCase() ?? "dog"}.pdf`);
      toast.success("PDF exported!");
    } catch {
      toast.error("Failed to export PDF.");
    }
  };

  const handleGenerate = () => {
    if (!dog) return;
    mutation.mutate({
      dog_id: dog.id,
      food_brand: foodBrand || undefined,
      dietary_restrictions: restrictions.length ? restrictions : undefined,
      goal: goal || undefined,
      current_issues: issues.length ? issues : undefined,
    });
  };

  return (
    <div className="max-w-5xl mx-auto px-6 py-12 relative">
      {/* Accents */}
      <div className="absolute top-8 right-12 pointer-events-none">
        <FloatingPawIcon size={20} rotation={18} />
      </div>
      <div className="absolute bottom-40 left-4 pointer-events-none">
        <Bone className="text-outline opacity-15" size={16} style={{ transform: "rotate(-30deg)" }} />
      </div>

      <h1 className="font-display text-3xl md:text-4xl font-bold text-on-surface mb-2">
        Nutrition Lab
      </h1>
      <p className="text-on-surface-variant font-body mb-6">
        {dog
          ? `Build the perfect diet for ${dog.name}`
          : "Loading your dog..."}
      </p>

      <DogSelector dogs={dogs} currentDog={dog} onSelect={selectDog} />

      {/* Tabs */}
      <div className="flex gap-2 mb-8">
        <button
          onClick={() => setActiveTab("generate")}
          className={`px-5 py-2 rounded-xl text-sm font-display font-semibold transition-colors ${
            activeTab === "generate"
              ? "bg-primary text-on-primary"
              : "bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest"
          }`}
        >
          <Apple size={14} className="inline mr-2" />
          Generate Plan
        </button>
        <button
          onClick={() => setActiveTab("history")}
          className={`px-5 py-2 rounded-xl text-sm font-display font-semibold transition-colors ${
            activeTab === "history"
              ? "bg-primary text-on-primary"
              : "bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest"
          }`}
        >
          <History size={14} className="inline mr-2" />
          History {pastPlans.length > 0 && `(${pastPlans.length})`}
        </button>
      </div>

      {/* History Tab */}
      {activeTab === "history" && (
        <section className="space-y-4">
          {pastPlans.length === 0 ? (
            <div className="bg-surface-container-low rounded-xl p-10 text-center text-on-surface-variant font-body">
              No nutrition plans yet. Generate your first one!
            </div>
          ) : (
            pastPlans.map((p) => (
              <div key={p.id} className="bg-surface-container-low rounded-xl p-6 space-y-3">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <p className="font-display font-semibold text-on-surface text-sm">
                      {p.goal ? `Goal: ${p.goal}` : "Maintenance plan"}
                      {p.food_brand ? ` · ${p.food_brand}` : ""}
                    </p>
                    <p className="text-outline text-xs mt-1">
                      {new Date(p.created_at).toLocaleDateString("en-US", {
                        month: "short",
                        day: "numeric",
                        year: "numeric",
                      })}
                    </p>
                  </div>
                  <div className="text-right flex-none">
                    <p className="text-on-surface font-display font-bold text-sm">{p.daily_calories} kcal</p>
                    <p className="text-on-surface-variant text-xs">{p.meals_per_day}x/day</p>
                  </div>
                </div>
                <div className="flex flex-wrap gap-1">
                  {p.recommended_foods.slice(0, 3).map((f, i) => (
                    <Badge key={i} className="bg-success/10 text-success border-0 rounded-lg text-xs px-2 py-0.5">
                      {f.split(" ")[0]}
                    </Badge>
                  ))}
                  {p.foods_to_avoid.slice(0, 2).map((f, i) => (
                    <Badge key={i} className="bg-error/10 text-error border-0 rounded-lg text-xs px-2 py-0.5">
                      ✕ {f.split(" ")[0]}
                    </Badge>
                  ))}
                </div>
                {p.notes && (
                  <p className="text-on-surface-variant text-xs line-clamp-2">{p.notes}</p>
                )}
              </div>
            ))
          )}
        </section>
      )}

      {/* Generate Tab */}
      {activeTab === "generate" && (
        <div className="space-y-6">
          <div className="space-y-2">
            <Label className="text-on-surface-variant text-sm">
              Current Food Brand (optional)
            </Label>
            <Input
              placeholder="e.g. Royal Canin, Blue Buffalo..."
              className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0 max-w-md"
              value={foodBrand}
              onChange={(e) => setFoodBrand(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label className="text-on-surface-variant text-sm">Goal (optional)</Label>
            <Input
              placeholder="e.g. Weight loss, muscle gain, maintenance..."
              className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0 max-w-md"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
            />
          </div>

          {/* Dietary Restrictions */}
          <div>
            <Label className="text-on-surface-variant text-sm mb-3 block">
              Dietary Preferences
            </Label>
            <div className="flex flex-wrap gap-2">
              {DIETARY_OPTIONS.map((opt) => (
                <Badge
                  key={opt}
                  variant="outline"
                  className={`cursor-pointer rounded-xl px-4 py-2 text-sm font-body transition-colors border-0 ${
                    restrictions.includes(opt)
                      ? "bg-secondary-container text-secondary"
                      : "bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest"
                  }`}
                  onClick={() => toggleItem(restrictions, setRestrictions, opt)}
                >
                  {restrictions.includes(opt) ? (
                    <X size={12} className="mr-1" />
                  ) : (
                    <Plus size={12} className="mr-1" />
                  )}
                  {opt}
                </Badge>
              ))}
            </div>
          </div>

          {/* Current Issues */}
          <div>
            <Label className="text-on-surface-variant text-sm mb-3 block">
              Current Issues
            </Label>
            <div className="flex flex-wrap gap-2">
              {ISSUE_OPTIONS.map((opt) => (
                <Badge
                  key={opt}
                  variant="outline"
                  className={`cursor-pointer rounded-xl px-4 py-2 text-sm font-body transition-colors border-0 ${
                    issues.includes(opt)
                      ? "bg-secondary-container text-secondary"
                      : "bg-surface-container-high text-on-surface-variant hover:bg-surface-container-highest"
                  }`}
                  onClick={() => toggleItem(issues, setIssues, opt)}
                >
                  {issues.includes(opt) ? (
                    <X size={12} className="mr-1" />
                  ) : (
                    <Plus size={12} className="mr-1" />
                  )}
                  {opt}
                </Badge>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Generate Button */}
      <Button
        onClick={handleGenerate}
        disabled={mutation.isPending || !dog}
        className="w-full max-w-md bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-xl py-6 text-base font-display font-semibold mb-12"
      >
        {mutation.isPending ? (
          "Generating..."
        ) : (
          <>
            <Apple className="mr-2" size={20} />
            Generate Plan
          </>
        )}
      </Button>

      {mutation.isError && (
        <p className="text-error text-sm mb-6">
          Failed to generate plan. Please try again.
        </p>
      )}

      {/* Results */}
      {plan && (
        <section className="space-y-8">
          <div className="flex items-center justify-between">
            <h2 className="font-display text-2xl font-bold text-on-surface">
              {dog?.name}'s Nutrition Plan
            </h2>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleExportPDF}
              className="gap-2 text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high rounded-lg"
            >
              <FileDown size={16} />
              Export PDF
            </Button>
          </div>
          <div ref={pdfRef} className="space-y-8">

          {/* Overview stats */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <StatCard label="Daily Calories" value={`${plan.daily_calories} kcal`} />
            <StatCard label="Meals/Day" value={String(plan.meals_per_day)} />
            <StatCard label="Per Meal" value={`${plan.portion_per_meal_grams}g`} />
            <StatCard label="Next Review" value={`${plan.next_review_weeks} weeks`} />
          </div>

          {/* Schedule */}
          {plan.feeding_schedule.length > 0 && (
            <div className="bg-surface-container-low rounded-xl p-6">
              <h3 className="font-display font-semibold text-on-surface mb-4 flex items-center gap-2">
                <Clock size={18} className="text-secondary" />
                Feeding Schedule
              </h3>
              <ul className="space-y-2">
                {plan.feeding_schedule.map((item, i) => (
                  <li key={i} className="text-on-surface-variant text-sm font-body">
                    {item}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Two column: Recommended vs Avoid */}
          <div className="grid md:grid-cols-2 gap-6">
            {/* Recommended */}
            <div className="bg-surface-container-low rounded-xl p-6">
              <h3 className="font-display font-semibold text-on-surface mb-4">
                Recommended Foods
              </h3>
              <div className="flex flex-wrap gap-2">
                {plan.recommended_foods.map((food, i) => (
                  <Badge
                    key={i}
                    className="bg-success/15 text-success border-0 rounded-lg px-3 py-1 font-body text-sm"
                  >
                    {food}
                  </Badge>
                ))}
              </div>
            </div>

            {/* Avoid */}
            <div className="bg-surface-container-low rounded-xl p-6">
              <h3 className="font-display font-semibold text-on-surface mb-4">
                Foods to Avoid
              </h3>
              <div className="flex flex-wrap gap-2">
                {plan.foods_to_avoid.map((food, i) => (
                  <Badge
                    key={i}
                    className="bg-error/15 text-error border-0 rounded-lg px-3 py-1 font-body text-sm"
                  >
                    {food}
                  </Badge>
                ))}
              </div>
            </div>
          </div>

          {/* Supplements */}
          {plan.supplements.length > 0 && (
            <div className="bg-surface-container rounded-xl p-6">
              <h3 className="font-display font-semibold text-on-surface mb-4 flex items-center gap-2">
                <Pill size={18} className="text-primary" />
                Supplements
              </h3>
              <div className="flex flex-wrap gap-2">
                {plan.supplements.map((supp, i) => (
                  <Badge
                    key={i}
                    className="bg-primary-container text-primary border-0 rounded-lg px-3 py-1 font-body text-sm"
                  >
                    {supp}
                  </Badge>
                ))}
              </div>
            </div>
          )}

          {/* Notes */}
          {plan.notes && (
            <div className="bg-surface-container-low rounded-xl p-6">
              <h3 className="font-display font-semibold text-on-surface mb-3 flex items-center gap-2">
                <FileText size={18} className="text-on-surface-variant" />
                Notes
              </h3>
              <p className="text-on-surface-variant text-sm font-body leading-relaxed whitespace-pre-wrap">
                {plan.notes}
              </p>
            </div>
          )}
          </div>{/* end pdfRef */}
        </section>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-surface-container-high rounded-xl p-5 text-center">
      <span className="text-on-surface-variant text-xs block mb-1">{label}</span>
      <span className="font-display text-xl font-bold text-on-surface">{value}</span>
    </div>
  );
}
