import { Outlet } from "react-router-dom";
import { Navbar } from "./Navbar";

export function Layout() {
  return (
    <div className="min-h-screen bg-surface">
      <Navbar />
      <main className="pt-20">
        <Outlet />
      </main>
    </div>
  );
}
