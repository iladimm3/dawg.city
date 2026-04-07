import { useQuery, useMutation } from "@tanstack/react-query";
import { billingApi } from "@/lib/api";
import { useSearchParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FloatingPawIcon } from "@/components/FloatingPawIcon";
import { Bone, Zap, CheckCircle, Settings } from "lucide-react";
import { toast } from "sonner";
import type { SubscriptionStatus } from "@/types";

const FREE_FEATURES = [
  "3 AI sessions per day (training + nutrition combined)",
  "Unlimited dog profiles",
  "Full training & nutrition history",
  "All exercise & meal types",
];

const PRO_FEATURES = [
  "Unlimited AI training sessions",
  "Unlimited AI nutrition plans",
  "Priority Anthropic model access",
  "Everything in Free",
];

export default function Billing() {
  const [searchParams] = useSearchParams();
  const justUpgraded = searchParams.get("success") === "1";

  const { data: status, isLoading } = useQuery<SubscriptionStatus>({
    queryKey: ["billing-status"],
    queryFn: billingApi.status,
  });

  const checkoutMutation = useMutation({
    mutationFn: billingApi.createCheckout,
    onSuccess: (data: { url: string }) => {
      window.location.href = data.url;
    },
    onError: () => {
      toast.error("Could not start checkout — please try again.");
    },
  });

  const portalMutation = useMutation({
    mutationFn: billingApi.createPortal,
    onSuccess: (data: { url: string }) => {
      window.location.href = data.url;
    },
    onError: () => {
      toast.error("Could not open billing portal — please try again.");
    },
  });

  const isPro = status?.tier === "pro";

  return (
    <div className="max-w-4xl mx-auto px-6 py-12 relative">
      {/* Background accents */}
      <div className="absolute top-8 right-12 pointer-events-none">
        <FloatingPawIcon size={24} rotation={15} />
      </div>
      <div className="absolute bottom-32 left-4 pointer-events-none">
        <Bone className="text-outline opacity-15" size={18} style={{ transform: "rotate(-20deg)" }} />
      </div>

      <h1 className="font-display text-3xl md:text-4xl font-bold text-on-surface mb-2">
        Plans & Billing
      </h1>
      <p className="text-on-surface-variant font-body mb-10">
        Choose the plan that fits your pack.
      </p>

      {justUpgraded && (
        <div className="bg-success/15 border-0 rounded-xl p-5 mb-8 flex items-center gap-3">
          <CheckCircle className="text-success" size={20} />
          <p className="text-success font-body font-semibold">
            You're now on Pro! Enjoy unlimited AI sessions.
          </p>
        </div>
      )}

      <div className="grid md:grid-cols-2 gap-6 mb-10">
        {/* Free Plan */}
        <div className={`bg-surface-container-low rounded-xl p-8 flex flex-col gap-6 ${!isPro && !isLoading ? "ring-2 ring-primary/40" : ""}`}>
          <div>
            <div className="flex items-center justify-between mb-2">
              <h2 className="font-display text-xl font-bold text-on-surface">Free</h2>
              {!isPro && (
                <Badge className="bg-primary/20 text-primary border-0 rounded-lg text-xs">
                  Current plan
                </Badge>
              )}
            </div>
            <p className="text-on-surface-variant font-body text-sm">
              Get started with AI-powered dog coaching.
            </p>
            <p className="font-display text-3xl font-extrabold text-on-surface mt-4">
              $0 <span className="text-on-surface-variant text-base font-normal">/mo</span>
            </p>
          </div>
          <ul className="space-y-3 flex-1">
            {FREE_FEATURES.map((f) => (
              <li key={f} className="flex items-start gap-2 text-on-surface-variant text-sm font-body">
                <CheckCircle size={14} className="text-outline mt-0.5 flex-none" />
                {f}
              </li>
            ))}
          </ul>
          <Button
            disabled
            className="w-full bg-surface-container-high text-on-surface-variant rounded-xl py-5 cursor-not-allowed"
          >
            Current plan
          </Button>
        </div>

        {/* Pro Plan */}
        <div className={`bg-surface-container-high rounded-xl p-8 flex flex-col gap-6 relative overflow-hidden ${isPro ? "ring-2 ring-secondary/60" : ""}`}>
          {/* Gradient glow */}
          <div className="absolute -top-10 -right-10 w-40 h-40 bg-secondary/10 rounded-full blur-3xl pointer-events-none" />
          <div className="relative">
            <div className="flex items-center justify-between mb-2">
              <h2 className="font-display text-xl font-bold text-on-surface">Pro</h2>
              {isPro ? (
                <Badge className="bg-secondary/20 text-secondary border-0 rounded-lg text-xs">
                  Active
                </Badge>
              ) : (
                <Badge className="bg-gradient-to-r from-primary to-secondary text-on-primary border-0 rounded-lg text-xs px-3">
                  Recommended
                </Badge>
              )}
            </div>
            <p className="text-on-surface-variant font-body text-sm">
              Unlimited AI for serious dog parents.
            </p>
            <p className="font-display text-3xl font-extrabold text-on-surface mt-4">
              $9 <span className="text-on-surface-variant text-base font-normal">/mo</span>
            </p>
          </div>
          <ul className="space-y-3 flex-1">
            {PRO_FEATURES.map((f) => (
              <li key={f} className="flex items-start gap-2 text-on-surface text-sm font-body">
                <CheckCircle size={14} className="text-secondary mt-0.5 flex-none" />
                {f}
              </li>
            ))}
          </ul>
          {isPro ? (
            <Button
              onClick={() => portalMutation.mutate()}
              disabled={portalMutation.isPending}
              className="w-full bg-surface-container-highest text-on-surface rounded-xl py-5 gap-2"
            >
              <Settings size={16} />
              {portalMutation.isPending ? "Opening..." : "Manage Subscription"}
            </Button>
          ) : (
            <Button
              onClick={() => checkoutMutation.mutate()}
              disabled={checkoutMutation.isPending || isLoading}
              className="w-full bg-gradient-to-br from-primary to-primary-dim text-on-primary rounded-xl py-5 font-display font-semibold gap-2"
            >
              <Zap size={16} />
              {checkoutMutation.isPending ? "Loading..." : "Upgrade to Pro"}
            </Button>
          )}
        </div>
      </div>

      {/* FAQ */}
      <section className="space-y-4">
        <h2 className="font-display text-lg font-bold text-on-surface">FAQ</h2>
        {[
          {
            q: "What counts as an AI session?",
            a: "Any AI generation — training session or nutrition plan — counts as one session. Free users get 3 per day.",
          },
          {
            q: "Can I cancel at any time?",
            a: "Yes. Open the billing portal above to cancel. You keep Pro access until the end of the billing period.",
          },
          {
            q: "How do I update my payment method?",
            a: 'Click "Manage Subscription" above to open the Stripe billing portal and update your card.',
          },
        ].map(({ q, a }) => (
          <div key={q} className="bg-surface-container-low rounded-xl p-6">
            <p className="font-display font-semibold text-on-surface text-sm mb-2">{q}</p>
            <p className="text-on-surface-variant text-sm font-body">{a}</p>
          </div>
        ))}
      </section>
    </div>
  );
}
