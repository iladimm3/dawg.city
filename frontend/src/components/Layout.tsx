import { Outlet } from "react-router-dom";
import { Navbar } from "./Navbar";
import { Toaster } from "@/components/ui/sonner";

export function Layout() {
  return (
    <div className="min-h-screen bg-surface">
      <Navbar />
      <main className="pt-20">
        <Outlet />
      </main>
      <Toaster position="bottom-right" richColors />
    </div>
  );
}
