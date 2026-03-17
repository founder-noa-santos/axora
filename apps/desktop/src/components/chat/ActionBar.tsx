import { ActionBarPrimitive, MessagePrimitive } from '@assistant-ui/react';
import { Copy, RefreshCw, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useState } from 'react';

export function ActionBar() {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="mt-2 flex items-center gap-1">
      <ActionBarPrimitive.Copy asChild>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 text-xs"
          onClick={handleCopy}
        >
          {copied ? (
            <Check className="h-3 w-3" />
          ) : (
            <Copy className="h-3 w-3" />
          )}
          {copied ? 'Copied' : 'Copy'}
        </Button>
      </ActionBarPrimitive.Copy>
      
      <MessagePrimitive.If assistant>
        <ActionBarPrimitive.Reload asChild>
          <Button variant="ghost" size="sm" className="h-7 gap-1 text-xs">
            <RefreshCw className="h-3 w-3" />
            Regenerate
          </Button>
        </ActionBarPrimitive.Reload>
      </MessagePrimitive.If>
    </div>
  );
}
