export interface User {
  id: string;
  email: string;
  name: string;
  avatar: string | null;
}

export interface Dog {
  id: string;
  owner_id: string;
  name: string;
  breed: string;
  age_months: number;
  weight_kg: number;
  sex: "male" | "female";
  neutered: boolean;
  activity_level: "low" | "medium" | "high";
  health_notes: string | null;
  created_at: string;
}

export interface CreateDogPayload {
  name: string;
  breed: string;
  age_months: number;
  weight_kg: number;
  sex: string;
  neutered: boolean;
  activity_level: string;
  health_notes?: string;
}

export interface Exercise {
  name: string;
  description: string;
  repetitions: string;
  success_criteria: string;
}

export interface TrainingSession {
  title: string;
  duration_minutes: number;
  exercises: Exercise[];
  tips: string[];
  encouragement: string;
}

export interface TrainingRequest {
  dog_id: string;
  focus_areas: string[];
  session_length_minutes: number;
  last_session_notes?: string;
  difficulty?: string;
}

export interface TrainingLogEntry {
  id: string;
  dog_id: string;
  session_title: string;
  completed: boolean;
  notes: string | null;
  rating: number | null;
  logged_at: string;
}

export interface SessionLog {
  dog_id: string;
  session_title: string;
  completed: boolean;
  notes?: string;
  rating?: number;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  limit: number;
  offset: number;
}

export interface NutritionRequest {
  dog_id: string;
  food_brand?: string;
  dietary_restrictions?: string[];
  goal?: string;
  current_issues?: string[];
}

export interface NutritionPlan {
  daily_calories: number;
  meals_per_day: number;
  portion_per_meal_grams: number;
  feeding_schedule: string[];
  recommended_foods: string[];
  foods_to_avoid: string[];
  supplements: string[];
  notes: string;
  next_review_weeks: number;
}
