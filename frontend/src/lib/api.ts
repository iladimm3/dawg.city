import axios from "axios";

const api = axios.create({
  baseURL: "",
  withCredentials: true,
});

api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      if (window.location.pathname !== "/") {
        window.location.href = "/";
      }
    }
    return Promise.reject(error);
  },
);

export const authApi = {
  loginUrl: () => "/auth/google",
  logout: () => api.get("/auth/logout"),
  me: () => api.get("/auth/me").then((r) => r.data),
};

export const dogsApi = {
  list: () => api.get("/api/dogs").then((r) => r.data),
  get: (id: string) => api.get(`/api/dogs/${id}`).then((r) => r.data),
  create: (data: import("@/types").CreateDogPayload) =>
    api.post("/api/dogs", data).then((r) => r.data),
  update: (id: string, data: import("@/types").CreateDogPayload) =>
    api.put(`/api/dogs/${id}`, data).then((r) => r.data),
  delete: (id: string) => api.delete(`/api/dogs/${id}`).then((r) => r.data),
};

export const trainingApi = {
  generateSession: (data: import("@/types").TrainingRequest) =>
    api.post("/api/training/session", data).then((r) => r.data),
  logSession: (data: import("@/types").SessionLog) =>
    api.post("/api/training/log", data).then((r) => r.data),
  history: (dogId: string, limit = 10, offset = 0) =>
    api
      .get("/api/training/history", { params: { dog_id: dogId, limit, offset } })
      .then((r) => r.data),
};

export const nutritionApi = {
  generatePlan: (data: import("@/types").NutritionRequest) =>
    api.post("/api/nutrition/plan", data).then((r) => r.data),
};

export default api;
