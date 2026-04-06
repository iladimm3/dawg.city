import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { authApi } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { PawPrint, LogOut } from "lucide-react";
import { FloatingPawIcon } from "./FloatingPawIcon";

export function Navbar() {
  const { user, isAuthenticated } = useAuth();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await authApi.logout();
    navigate("/");
    window.location.reload();
  };

  return (
    <nav className="fixed top-0 left-0 right-0 z-50 backdrop-blur-[20px] bg-surface-variant/60">
      <div className="mx-auto max-w-7xl px-6 py-4 flex items-center justify-between relative">
        {/* Scattered paw accents */}
        <div className="absolute top-2 left-[20%] pointer-events-none">
          <FloatingPawIcon size={16} rotation={-10} />
        </div>
        <div className="absolute bottom-1 right-[35%] pointer-events-none">
          <FloatingPawIcon size={14} rotation={25} />
        </div>
        <div className="absolute top-3 right-[15%] pointer-events-none">
          <FloatingPawIcon size={12} rotation={-20} />
        </div>

        {/* Logo */}
        <Link to={isAuthenticated ? "/dashboard" : "/"} className="flex items-center gap-2">
          <PawPrint className="text-primary" size={28} />
          <span className="font-display text-xl font-bold text-on-surface">
            Dawg City
          </span>
        </Link>

        {/* Center links */}
        {isAuthenticated && (
          <div className="hidden md:flex items-center gap-8">
            <Link
              to="/dashboard"
              className="text-on-surface-variant hover:text-on-surface transition-colors font-body text-sm"
            >
              Dashboard
            </Link>
            <Link
              to="/training"
              className="text-on-surface-variant hover:text-on-surface transition-colors font-body text-sm"
            >
              Training
            </Link>
            <Link
              to="/nutrition"
              className="text-on-surface-variant hover:text-on-surface transition-colors font-body text-sm"
            >
              Nutrition
            </Link>
          </div>
        )}

        {/* Right side */}
        {isAuthenticated && user ? (
          <div className="flex items-center gap-4">
            <Avatar className="h-9 w-9">
              <AvatarImage src={user.avatar ?? undefined} alt={user.name} />
              <AvatarFallback className="bg-surface-container-high text-on-surface text-xs">
                {user.name.charAt(0).toUpperCase()}
              </AvatarFallback>
            </Avatar>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleLogout}
              className="text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high"
            >
              <LogOut size={16} />
            </Button>
          </div>
        ) : (
          <a href={authApi.loginUrl()}>
            <Button
              size="sm"
              className="bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-lg font-body"
            >
              Sign in
            </Button>
          </a>
        )}
      </div>
    </nav>
  );
}
