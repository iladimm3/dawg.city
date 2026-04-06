import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { authApi } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { PawPrint, LogOut, Menu, X } from "lucide-react";
import { FloatingPawIcon } from "./FloatingPawIcon";

export function Navbar() {
  const { user, isAuthenticated } = useAuth();
  const navigate = useNavigate();
  const [mobileOpen, setMobileOpen] = useState(false);

  const handleLogout = async () => {
    await authApi.logout();
    navigate("/");
    window.location.reload();
  };

  const navLinks = [
    { to: "/dashboard", label: "Dashboard" },
    { to: "/training", label: "Training" },
    { to: "/nutrition", label: "Nutrition" },
  ];

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

        {/* Center links (desktop) */}
        {isAuthenticated && (
          <div className="hidden md:flex items-center gap-8">
            {navLinks.map((link) => (
              <Link
                key={link.to}
                to={link.to}
                className="text-on-surface-variant hover:text-on-surface transition-colors font-body text-sm"
              >
                {link.label}
              </Link>
            ))}
          </div>
        )}

        {/* Right side */}
        <div className="flex items-center gap-3">
          {isAuthenticated && user ? (
            <>
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
                className="hidden md:inline-flex text-on-surface-variant hover:text-on-surface hover:bg-surface-container-high"
              >
                <LogOut size={16} />
              </Button>
              {/* Mobile hamburger */}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setMobileOpen(!mobileOpen)}
                className="md:hidden text-on-surface-variant hover:text-on-surface"
              >
                {mobileOpen ? <X size={20} /> : <Menu size={20} />}
              </Button>
            </>
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
      </div>

      {/* Mobile menu */}
      {isAuthenticated && mobileOpen && (
        <div className="md:hidden backdrop-blur-[20px] bg-surface-variant/80 border-t border-outline/10 px-6 pb-4 space-y-1">
          {navLinks.map((link) => (
            <Link
              key={link.to}
              to={link.to}
              onClick={() => setMobileOpen(false)}
              className="block py-3 text-on-surface-variant hover:text-on-surface transition-colors font-body text-sm"
            >
              {link.label}
            </Link>
          ))}
          <button
            onClick={() => { setMobileOpen(false); handleLogout(); }}
            className="flex items-center gap-2 py-3 text-on-surface-variant hover:text-on-surface transition-colors font-body text-sm w-full"
          >
            <LogOut size={14} />
            Sign out
          </button>
        </div>
      )}
    </nav>
  );
}
