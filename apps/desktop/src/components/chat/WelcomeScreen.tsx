import { Sparkles, Code2, Zap, Shield } from 'lucide-react';

export function WelcomeScreen() {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-primary to-accent">
        <Sparkles className="h-8 w-8 text-white" />
      </div>
      
      <h1 className="mb-2 text-2xl font-bold">Welcome to AXORA</h1>
      <p className="mb-8 max-w-md text-muted-foreground">
        Your AI-powered coding assistant. Ask me to write code, debug issues, 
        or help with any programming task.
      </p>
      
      <div className="grid max-w-lg grid-cols-3 gap-4">
        <FeatureCard
          icon={<Code2 className="h-5 w-5" />}
          title="Write Code"
          description="Generate code in any language"
        />
        <FeatureCard
          icon={<Zap className="h-5 w-5" />}
          title="Debug"
          description="Find and fix bugs quickly"
        />
        <FeatureCard
          icon={<Shield className="h-5 w-5" />}
          title="Review"
          description="Get code review suggestions"
        />
      </div>
    </div>
  );
}

function FeatureCard({ icon, title, description }: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="flex flex-col items-center rounded-lg border border-border bg-card p-4">
      <div className="mb-2 text-primary">{icon}</div>
      <h3 className="mb-1 text-sm font-medium">{title}</h3>
      <p className="text-xs text-muted-foreground">{description}</p>
    </div>
  );
}
