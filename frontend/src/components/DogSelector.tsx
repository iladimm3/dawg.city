import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PawPrint } from "lucide-react";
import type { Dog } from "@/types";

interface DogSelectorProps {
  dogs: Dog[];
  currentDog: Dog | undefined;
  onSelect: (id: string) => void;
}

export function DogSelector({ dogs, currentDog, onSelect }: DogSelectorProps) {
  if (dogs.length <= 1) return null;

  return (
    <div className="flex items-center gap-3 mb-8">
      <PawPrint className="text-primary" size={18} />
      <Select value={currentDog?.id} onValueChange={(v) => v && onSelect(v)}>
        <SelectTrigger className="bg-surface-container-high rounded-lg text-on-surface border-0 w-56">
          <SelectValue placeholder="Select dog" />
        </SelectTrigger>
        <SelectContent className="bg-surface-container-highest text-on-surface border-0 rounded-lg">
          {dogs.map((dog) => (
            <SelectItem key={dog.id} value={dog.id}>
              {dog.name} — {dog.breed}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
