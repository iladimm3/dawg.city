import { useState, useEffect } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { dogsApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { PawPrint, ChevronRight, ChevronLeft, Camera } from "lucide-react";
import { toast } from "sonner";
import type { CreateDogPayload, Dog } from "@/types";

const BREEDS = [
  // Popular
  "Labrador Retriever",
  "Golden Retriever",
  "German Shepherd",
  "French Bulldog",
  "Bulldog",
  "Poodle (Standard)",
  "Poodle (Miniature)",
  "Poodle (Toy)",
  "Beagle",
  "Rottweiler",
  // Medium / Large
  "Boxer",
  "Dachshund",
  "Siberian Husky",
  "Border Collie",
  "Australian Shepherd",
  "Doberman Pinscher",
  "Great Dane",
  "Bernese Mountain Dog",
  "Weimaraner",
  "Vizsla",
  // Small
  "Corgi (Pembroke)",
  "Corgi (Cardigan)",
  "Shih Tzu",
  "Chihuahua",
  "Maltese",
  "Yorkshire Terrier",
  "Pomeranian",
  "Cavalier King Charles Spaniel",
  "Bichon Frisé",
  "Miniature Schnauzer",
  // Working / Sport
  "Belgian Malinois",
  "Dutch Shepherd",
  "Australian Cattle Dog",
  "Jack Russell Terrier",
  "Staffordshire Bull Terrier",
  "American Pit Bull Terrier",
  "Cocker Spaniel",
  "Springer Spaniel",
  "Irish Setter",
  "Dalmatian",
  // Other
  "Samoyed",
  "Akita",
  "Chow Chow",
  "Shar Pei",
  "Basset Hound",
  "Saint Bernard",
  "Newfoundland",
  "Mixed / Other",
];

const STEPS = ["Basic Info", "Health & Activity", "Photo"];

export default function Onboarding() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const editId = searchParams.get("edit");
  const [step, setStep] = useState(0);

  const { data: editDog } = useQuery<Dog>({
    queryKey: ["dog", editId],
    queryFn: () => dogsApi.get(editId!),
    enabled: !!editId,
  });

  const [form, setForm] = useState<CreateDogPayload>({
    name: "",
    breed: "",
    age_months: 12,
    weight_kg: 10,
    sex: "male",
    neutered: false,
    activity_level: "medium",
    health_notes: "",
    photo_url: "",
  });

  useEffect(() => {
    if (editDog) {
      setForm({
        name: editDog.name,
        breed: editDog.breed,
        age_months: editDog.age_months,
        weight_kg: editDog.weight_kg,
        sex: editDog.sex,
        neutered: editDog.neutered,
        activity_level: editDog.activity_level,
        health_notes: editDog.health_notes ?? "",
        photo_url: editDog.photo_url ?? "",
      });
    }
  }, [editDog]);

  const createMutation = useMutation({
    mutationFn: (data: CreateDogPayload) => dogsApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["dogs"] });
      toast.success("Dog profile created!");
      navigate("/dashboard");
    },
    onError: () => {
      toast.error("Something went wrong. Please try again.");
    },
  });

  const updateMutation = useMutation({
    mutationFn: (data: CreateDogPayload) => dogsApi.update(editId!, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["dogs"] });
      toast.success("Dog profile saved!");
      navigate("/dashboard");
    },
    onError: () => {
      toast.error("Something went wrong. Please try again.");
    },
  });

  const mutation = editId ? updateMutation : createMutation;

  const handleNext = () => {
    if (step === 0 && (!form.name || !form.breed)) return;
    setStep((s) => Math.min(s + 1, STEPS.length - 1));
  };

  const handleBack = () => setStep((s) => Math.max(s - 1, 0));

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const payload: CreateDogPayload = {
      ...form,
      photo_url: form.photo_url || undefined,
      health_notes: form.health_notes || undefined,
    };
    mutation.mutate(payload);
  };

  return (
    <div className="min-h-[80vh] flex items-center justify-center px-6 py-12">
      <div className="w-full max-w-lg bg-surface-container-low rounded-xl p-10">
        {/* Icon accent */}
        <div className="flex justify-center -mt-16 mb-8">
          <div className="w-20 h-20 bg-gradient-to-br from-primary to-primary-dim rounded-xl flex items-center justify-center shadow-xl shadow-primary/20">
            <PawPrint className="text-on-primary" size={36} />
          </div>
        </div>

        <h1 className="font-display text-3xl font-bold text-on-surface text-center mb-2">
          {editId ? "Edit your dog" : "Add your dog"}
        </h1>
        <p className="text-on-surface-variant text-center mb-8 font-body">
          {editId ? "Update your pup's details." : "Tell us about your pup so we can personalize everything."}
        </p>

        {/* Step indicator */}
        <div className="flex items-center gap-2 mb-10">
          {STEPS.map((label, i) => (
            <div key={label} className="flex items-center gap-2 flex-1">
              <div className="flex flex-col items-center gap-1 flex-none">
                <div
                  className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-display font-bold transition-colors ${
                    i < step
                      ? "bg-primary text-on-primary"
                      : i === step
                      ? "bg-primary text-on-primary ring-2 ring-primary/30"
                      : "bg-surface-container-high text-on-surface-variant"
                  }`}
                >
                  {i < step ? "✓" : i + 1}
                </div>
                <span
                  className={`text-xs font-body whitespace-nowrap ${
                    i === step ? "text-on-surface" : "text-on-surface-variant"
                  }`}
                >
                  {label}
                </span>
              </div>
              {i < STEPS.length - 1 && (
                <div
                  className={`h-px flex-1 mt-[-14px] transition-colors ${
                    i < step ? "bg-primary" : "bg-surface-container-high"
                  }`}
                />
              )}
            </div>
          ))}
        </div>

        <form onSubmit={handleSubmit} className="space-y-6">
          {/* ── Step 0: Basic Info ─────────────────────── */}
          {step === 0 && (
            <>
              <div className="space-y-2">
                <Label className="text-on-surface-variant">Name</Label>
                <Input
                  required
                  placeholder="e.g. Buddy"
                  className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                />
              </div>

              <div className="space-y-2">
                <Label className="text-on-surface-variant">Breed</Label>
                <Select
                  value={form.breed}
                  onValueChange={(v) => v && setForm({ ...form, breed: v })}
                >
                  <SelectTrigger className="bg-surface-container-high rounded-lg text-on-surface border-0">
                    <SelectValue placeholder="Select breed" />
                  </SelectTrigger>
                  <SelectContent className="bg-surface-container-highest text-on-surface border-0 rounded-lg max-h-60">
                    {BREEDS.map((b) => (
                      <SelectItem key={b} value={b}>
                        {b}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2">
                <Label className="text-on-surface-variant">Sex</Label>
                <Select
                  value={form.sex}
                  onValueChange={(v) => v && setForm({ ...form, sex: v })}
                >
                  <SelectTrigger className="bg-surface-container-high rounded-lg text-on-surface border-0">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="bg-surface-container-highest text-on-surface border-0 rounded-lg">
                    <SelectItem value="male">Male</SelectItem>
                    <SelectItem value="female">Female</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </>
          )}

          {/* ── Step 1: Health & Activity ───────────────── */}
          {step === 1 && (
            <>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label className="text-on-surface-variant">Age (months)</Label>
                  <Input
                    type="number"
                    min={1}
                    required
                    className="bg-surface-container-high rounded-lg text-on-surface border-0"
                    value={form.age_months}
                    onChange={(e) =>
                      setForm({ ...form, age_months: Number(e.target.value) })
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label className="text-on-surface-variant">Weight (kg)</Label>
                  <Input
                    type="number"
                    min={0.5}
                    step={0.1}
                    required
                    className="bg-surface-container-high rounded-lg text-on-surface border-0"
                    value={form.weight_kg}
                    onChange={(e) =>
                      setForm({ ...form, weight_kg: Number(e.target.value) })
                    }
                  />
                </div>
              </div>

              <div className="space-y-2">
                <Label className="text-on-surface-variant">Activity Level</Label>
                <Select
                  value={form.activity_level}
                  onValueChange={(v) => v && setForm({ ...form, activity_level: v })}
                >
                  <SelectTrigger className="bg-surface-container-high rounded-lg text-on-surface border-0">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent className="bg-surface-container-highest text-on-surface border-0 rounded-lg">
                    <SelectItem value="low">Low — calm, short walks</SelectItem>
                    <SelectItem value="medium">Medium — daily walks & play</SelectItem>
                    <SelectItem value="high">High — very active, sports</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="flex items-center gap-3">
                <Checkbox
                  id="neutered"
                  checked={form.neutered}
                  onCheckedChange={(c) => setForm({ ...form, neutered: c === true })}
                  className="border-outline data-[state=checked]:bg-primary data-[state=checked]:border-primary"
                />
                <Label htmlFor="neutered" className="text-on-surface-variant">
                  Spayed / Neutered
                </Label>
              </div>

              <div className="space-y-2">
                <Label className="text-on-surface-variant">Health Notes (optional)</Label>
                <Textarea
                  placeholder="Any allergies, conditions, or notes..."
                  className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0 min-h-[80px]"
                  value={form.health_notes ?? ""}
                  onChange={(e) =>
                    setForm({ ...form, health_notes: e.target.value || undefined })
                  }
                />
              </div>
            </>
          )}

          {/* ── Step 2: Photo ──────────────────────────── */}
          {step === 2 && (
            <div className="space-y-6">
              <div className="flex flex-col items-center gap-4">
                {form.photo_url ? (
                  <img
                    src={form.photo_url}
                    alt="Dog preview"
                    className="w-32 h-32 rounded-xl object-cover shadow-lg shadow-primary/20"
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = "none";
                    }}
                  />
                ) : (
                  <div className="w-32 h-32 rounded-xl bg-surface-container-high flex items-center justify-center shadow-inner">
                    <Camera className="text-on-surface-variant" size={40} />
                  </div>
                )}
                <p className="text-on-surface-variant text-sm text-center font-body">
                  Paste a public image URL below to give {form.name || "your dog"} a photo.
                </p>
              </div>

              <div className="space-y-2">
                <Label className="text-on-surface-variant">Photo URL (optional)</Label>
                <Input
                  type="url"
                  placeholder="https://example.com/my-dog.jpg"
                  className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0"
                  value={form.photo_url ?? ""}
                  onChange={(e) => setForm({ ...form, photo_url: e.target.value })}
                />
              </div>
            </div>
          )}

          {/* Navigation */}
          <div className="flex gap-3 pt-2">
            {step > 0 && (
              <Button
                type="button"
                variant="ghost"
                onClick={handleBack}
                className="flex-1 text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high rounded-lg py-6 font-display font-semibold gap-1"
              >
                <ChevronLeft size={18} />
                Back
              </Button>
            )}
            {step < STEPS.length - 1 ? (
              <Button
                type="button"
                onClick={handleNext}
                disabled={step === 0 && (!form.name || !form.breed)}
                className="flex-1 bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-lg py-6 text-base font-display font-semibold gap-1"
              >
                Next
                <ChevronRight size={18} />
              </Button>
            ) : (
              <Button
                type="submit"
                disabled={mutation.isPending}
                className="flex-1 bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-lg py-6 text-base font-display font-semibold"
              >
                {mutation.isPending
                  ? editId ? "Saving..." : "Creating..."
                  : editId ? "Save Changes" : "Add Dog & Continue"}
              </Button>
            )}
          </div>
        </form>
      </div>
    </div>
  );
}
