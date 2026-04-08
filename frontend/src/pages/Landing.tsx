import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { authApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { PawPrint, Bone, Dumbbell, Apple, Brain } from "lucide-react";
import { FloatingPawIcon } from "@/components/FloatingPawIcon";

export default function Landing() {
  const { isAuthenticated, isLoading } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    if (isAuthenticated && !isLoading) {
      navigate("/dashboard");
    }
  }, [isAuthenticated, isLoading, navigate]);

  return (
    <div className="min-h-screen bg-surface relative overflow-hidden">
      {/* Scattered background accents */}
      <div className="absolute top-20 left-[8%] pointer-events-none">
        <FloatingPawIcon size={40} rotation={15} />
      </div>
      <div className="absolute top-40 right-[12%] pointer-events-none">
        <Bone className="text-outline opacity-20" size={32} style={{ transform: "rotate(-20deg)" }} />
      </div>
      <div className="absolute bottom-[30%] left-[5%] pointer-events-none">
        <FloatingPawIcon size={28} rotation={-15} />
      </div>
      <div className="absolute bottom-[20%] right-[8%] pointer-events-none">
        <Bone className="text-outline opacity-15" size={24} style={{ transform: "rotate(30deg)" }} />
      </div>
      <div className="absolute top-[60%] left-[45%] pointer-events-none">
        <FloatingPawIcon size={20} rotation={40} />
      </div>

      {/* Hero */}
      <section className="relative flex flex-col items-center justify-center min-h-screen px-6 text-center">
        {/* Overlapping accent shape */}
        <div className="absolute top-[15%] right-[10%] w-64 h-64 bg-gradient-to-br from-primary/20 to-primary-dim/10 rounded-xl blur-3xl pointer-events-none" />
        <div className="absolute bottom-[20%] left-[8%] w-48 h-48 bg-secondary/10 rounded-xl blur-2xl pointer-events-none" />

        <div className="relative z-10 max-w-4xl">
          <div className="inline-flex items-center gap-2 bg-surface-container-high rounded-xl px-5 py-2 mb-10">
            <PawPrint className="text-primary" size={18} />
            <span className="text-on-surface-variant text-sm font-body">
              AI-Powered Dog Coaching
            </span>
          </div>

          <h1 className="font-display text-5xl md:text-7xl lg:text-8xl font-extrabold text-on-surface leading-[1.05] mb-8 tracking-tight">
            Your dog deserves
            <br />
            <span className="bg-gradient-to-r from-primary to-secondary bg-clip-text text-transparent">
              a coach that remembers.
            </span>
          </h1>

          <p className="text-on-surface-variant text-lg md:text-xl max-w-2xl mx-auto mb-12 font-body font-light leading-relaxed">
            Personalized training sessions and nutrition plans powered by AI.
            Built around your dog's unique needs.
          </p>

          <Button
            size="lg"
            className="bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-xl px-10 py-6 text-lg font-display font-semibold shadow-2xl shadow-primary/25 hover:shadow-primary/40 transition-shadow"
            onClick={() => { window.location.href = authApi.loginUrl(); }}
          >
            <PawPrint className="mr-2" size={20} />
            Sign in with Google
          </Button>
        </div>
      </section>

      {/* Feature cards */}
      <section className="relative max-w-6xl mx-auto px-6 pb-32 -mt-20">
        <div className="grid md:grid-cols-3 gap-8">
          <FeatureCard
            icon={<Dumbbell className="text-primary" size={28} />}
            title="Smart Training"
            description="AI generates sessions tailored to your dog's age, breed, and skill level. Progress tracked automatically."
          />
          <FeatureCard
            icon={<Apple className="text-secondary" size={28} />}
            title="Nutrition Plans"
            description="Custom feeding schedules, portions, and supplement recommendations based on your dog's profile."
          />
          <FeatureCard
            icon={<Brain className="text-primary" size={28} />}
            title="Learns Over Time"
            description="Every session is remembered. Training adapts to your dog's progress and behaviors."
          />
        </div>
      </section>
    </div>
  );
}

function FeatureCard({
  icon,
  title,
  description,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="bg-surface-container-high rounded-xl p-8 relative group">
      {/* Scattered accent */}
      <div className="absolute top-4 right-4 pointer-events-none">
        <Bone className="text-outline opacity-20" size={16} style={{ transform: "rotate(25deg)" }} />
      </div>
      <div className="mb-5">{icon}</div>
      <h3 className="font-display text-xl font-bold text-on-surface mb-3">
        {title}
      </h3>
      <p className="text-on-surface-variant font-body text-sm leading-relaxed">
        {description}
      </p>
    </div>
  );
}
