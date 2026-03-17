import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Settings, MessageSquare, Sparkles } from 'lucide-react';
import { ChatPanel } from '@/panels/ChatPanel';
import { SettingsPanel } from '@/panels/SettingsPanel';

type Panel = 'chat' | 'settings';

function App() {
  const [activePanel, setActivePanel] = useState<Panel>('chat');

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* Integrated Header */}
      <header className="relative flex h-16 items-center justify-between border-b border-border bg-gradient-to-r from-background via-background to-primary/5 px-6 backdrop-blur-sm">
        {/* Left: Logo + Version */}
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-primary to-accent shadow-lg">
            <Sparkles className="h-5 w-5 text-white" />
          </div>
          <div className="flex flex-col">
            <h1 className="text-lg font-bold tracking-tight text-foreground">AXORA</h1>
            <span className="text-xs text-muted-foreground">Multi-Agent Coding System</span>
          </div>
          <Badge variant="secondary" className="ml-2 font-mono text-xs">
            v0.1.0
          </Badge>
        </div>

        {/* Right: Navigation */}
        <div className="flex items-center gap-2 rounded-lg bg-muted/50 p-1">
          <Button
            variant={activePanel === 'chat' ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setActivePanel('chat')}
            className={`h-8 gap-2 transition-all ${
              activePanel === 'chat'
                ? 'bg-background shadow-sm'
                : 'hover:bg-muted'
            }`}
          >
            <MessageSquare className="h-4 w-4" />
            <span className="hidden sm:inline">Chat</span>
          </Button>
          <Button
            variant={activePanel === 'settings' ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setActivePanel('settings')}
            className={`h-8 gap-2 transition-all ${
              activePanel === 'settings'
                ? 'bg-background shadow-sm'
                : 'hover:bg-muted'
            }`}
          >
            <Settings className="h-4 w-4" />
            <span className="hidden sm:inline">Settings</span>
          </Button>
        </div>

        {/* Status Indicator */}
        <div className="absolute bottom-0 left-0 h-px w-full bg-gradient-to-r from-primary/0 via-primary/20 to-primary/0" />
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-hidden">
        {activePanel === 'chat' ? (
          <ChatPanel />
        ) : (
          <div className="h-full overflow-auto p-6">
            <SettingsPanel />
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
