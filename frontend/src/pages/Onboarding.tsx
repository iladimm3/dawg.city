import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
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
import { PawPrint } from "lucide-react";
import type { CreateDogPayload } from "@/types";

const BREEDS = [
  "Labrador Retriever",
  "German Shepherd",
  "Golden Retriever",
  "French Bulldog",
  "Bulldog",
  "Poodle",
  "Beagle",
  "Rottweiler",
  "Dachshund",
  "Corgi",
  "Siberian Husky",
  "Boxer",
  "Border Collie",
  "Australian Shepherd",
  "Shih Tzu",
  "Mixed / Other",
];

export default function Onboarding() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [form, setForm] = useState<CreateDogPayload>({
    name: "",
    breed: "",
    age_months: 12,
    weight_kg: 10,
    sex: "male",
    neutered: false,
    activity_level: "medium",
    health_notes: "",
  });

  const mutation = useMutation({
    mutationFn: dogsApi.create,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["dogs"] });
      navigate("/dashboard");
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    mutation.mutate(form);
  };

  return (
    <div className="min-h-[80vh] flex items-center justify-center px-6 py-12">
      <div className="w-full max-w-lg bg-surface-container-low rounded-xl p-10">
        {/* Overlapping icon accent */}
        <div className="flex justify-center -mt-16 mb-8">
          <div className="w-20 h-20 bg-gradient-to-br from-primary to-primary-dim rounded-xl flex items-center justify-center shadow-xl shadow-primary/20">
            <PawPrint className="text-on-primary" size={36} />
          </div>
        </div>

        <h1 className="font-display text-3xl font-bold text-on-surface text-center mb-2">
          Add your dog
        </h1>
        <p className="text-on-surface-variant text-center mb-10 font-body">
          Tell us about your pup so we can personalize everything.
        </p>

        <form onSubmit={handleSubmit} className="space-y-6">
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
              <SelectContent className="bg-surface-container-highest text-on-surface border-0 rounded-lg">
                {BREEDS.map((b) => (
                  <SelectItem key={b} value={b}>
                    {b}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

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

          <div className="grid grid-cols-2 gap-4">
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
                  <SelectItem value="low">Low</SelectItem>
                  <SelectItem value="medium">Medium</SelectItem>
                  <SelectItem value="high">High</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <Checkbox
              id="neutered"
              checked={form.neutered}
              onCheckedChange={(c) =>
                setForm({ ...form, neutered: c === true })
              }
              className="border-outline data-[state=checked]:bg-primary data-[state=checked]:border-primary"
            />
            <Label htmlFor="neutered" className="text-on-surface-variant">
              Spayed / Neutered
            </Label>
          </div>

          <div className="space-y-2">
            <Label className="text-on-surface-variant">
              Health Notes (optional)
            </Label>
            <Textarea
              placeholder="Any allergies, conditions, or notes..."
              className="bg-surface-container-high rounded-lg text-on-surface placeholder:text-outline border-0 min-h-[80px]"
              value={form.health_notes ?? ""}
              onChange={(e) =>
                setForm({ ...form, health_notes: e.target.value || undefined })
              }
            />
          </div>

          <Button
            type="submit"
            disabled={mutation.isPending || !form.name || !form.breed}
            className="w-full bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-lg py-6 text-base font-display font-semibold"
          >
            {mutation.isPending ? "Creating..." : "Add Dog & Continue"}
          </Button>

          {mutation.isError && (
            <p className="text-error text-sm text-center">
              Something went wrong. Please try again.
            </p>
          )}
        </form>
      </div>
    </div>
  );
}
