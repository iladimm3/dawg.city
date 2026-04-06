import type { Dog } from "@/types";
import { PawPrint } from "lucide-react";

interface DogHeroCardProps {
  dog: Dog;
}

export function DogHeroCard({ dog }: DogHeroCardProps) {
  const ageYears = Math.floor(dog.age_months / 12);
  const ageMonths = dog.age_months % 12;
  const ageStr =
    ageYears > 0
      ? `${ageYears}y ${ageMonths > 0 ? `${ageMonths}m` : ""}`
      : `${ageMonths}m`;

  return (
    <div className="relative">
      {/* Photo breaking container bounds */}
      <div className="absolute -top-8 right-8 z-10">
        <div className="w-28 h-28 rounded-xl bg-gradient-to-br from-primary to-primary-dim flex items-center justify-center shadow-2xl shadow-primary/20">
          <PawPrint className="text-on-primary" size={48} />
        </div>
      </div>

      <div className="bg-surface-container-low rounded-xl p-8 pt-10 relative overflow-visible">
        <h2 className="font-display text-3xl font-bold text-on-surface mb-2">
          {dog.name}
        </h2>
        <p className="text-on-surface-variant font-body text-lg mb-6">
          {dog.breed}
        </p>

        <div className="flex flex-wrap gap-6">
          <div>
            <span className="text-on-surface-variant text-sm block">Age</span>
            <span className="text-on-surface font-display font-semibold text-lg">
              {ageStr}
            </span>
          </div>
          <div>
            <span className="text-on-surface-variant text-sm block">Weight</span>
            <span className="text-on-surface font-display font-semibold text-lg">
              {dog.weight_kg} kg
            </span>
          </div>
          <div>
            <span className="text-on-surface-variant text-sm block">Activity</span>
            <span className="text-on-surface font-display font-semibold text-lg capitalize">
              {dog.activity_level}
            </span>
          </div>
          <div>
            <span className="text-on-surface-variant text-sm block">Sex</span>
            <span className="text-on-surface font-display font-semibold text-lg capitalize">
              {dog.sex} {dog.neutered ? "(neutered)" : ""}
            </span>
          </div>
        </div>

        {dog.health_notes && (
          <p className="mt-4 text-on-surface-variant text-sm">
            {dog.health_notes}
          </p>
        )}
      </div>
    </div>
  );
}
