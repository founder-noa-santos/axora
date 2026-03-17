/**
 * AXORA Application Settings
 * 
 * Configuration for model selection, token limits, worker pool, and theme preferences.
 */

export interface AppSettings {
  // Model configuration
  model: {
    provider: 'ollama' | 'openai' | 'anthropic';
    model: string;
    baseUrl?: string; // For Ollama
    apiKey?: string;  // For OpenAI/Anthropic
  };
  
  // Token limits
  tokens: {
    maxTokensPerRequest: number;  // Default: 4096
    maxContextTokens: number;     // Default: 8192
    tokenBudget: number;          // Default: 100000
  };
  
  // Worker pool
  workers: {
    minWorkers: number;   // Default: 2
    maxWorkers: number;   // Default: 10
    healthCheckInterval: number; // Default: 30 (seconds)
  };
  
  // Theme
  theme: {
    mode: 'light' | 'dark' | 'system';
    accentColor: string;  // Default: 'electric-purple'
  };
  
  // Advanced
  advanced: {
    enableLogging: boolean;
    logLevel: 'debug' | 'info' | 'warn' | 'error';
    autoUpdate: boolean;
  };
}

export const defaultSettings: AppSettings = {
  model: {
    provider: 'ollama',
    model: 'qwen2.5-coder:7b',
    baseUrl: 'http://localhost:11434',
  },
  tokens: {
    maxTokensPerRequest: 4096,
    maxContextTokens: 8192,
    tokenBudget: 100000,
  },
  workers: {
    minWorkers: 2,
    maxWorkers: 10,
    healthCheckInterval: 30,
  },
  theme: {
    mode: 'dark',
    accentColor: 'electric-purple',
  },
  advanced: {
    enableLogging: true,
    logLevel: 'info',
    autoUpdate: true,
  },
};

/**
 * Validation rules for settings
 */
export const settingsValidation = {
  model: {
    provider: (value: string) => 
      ['ollama', 'openai', 'anthropic'].includes(value),
    model: (value: string) => value.length > 0 && value.length <= 100,
    baseUrl: (value?: string) => {
      if (!value) return true;
      try {
        new URL(value);
        return true;
      } catch {
        return false;
      }
    },
  },
  tokens: {
    maxTokensPerRequest: (value: number) => 
      value >= 100 && value <= 128000,
    maxContextTokens: (value: number) => 
      value >= 1000 && value <= 256000,
    tokenBudget: (value: number) => 
      value >= 10000 && value <= 1000000,
  },
  workers: {
    minWorkers: (value: number) => value >= 1 && value <= 20,
    maxWorkers: (value: number) => value >= 1 && value <= 50,
    healthCheckInterval: (value: number) => value >= 5 && value <= 300,
  },
  theme: {
    mode: (value: string) => 
      ['light', 'dark', 'system'].includes(value),
    accentColor: (value: string) => value.length > 0 && value.length <= 50,
  },
  advanced: {
    enableLogging: (value: boolean) => typeof value === 'boolean',
    logLevel: (value: string) => 
      ['debug', 'info', 'warn', 'error'].includes(value),
    autoUpdate: (value: boolean) => typeof value === 'boolean',
  },
};

/**
 * Validate a settings object
 */
export function validateSettings(settings: Partial<AppSettings>): {
  valid: boolean;
  errors: Record<string, string>;
} {
  const errors: Record<string, string> = {};
  
  if (settings.model) {
    if (!settingsValidation.model.provider(settings.model.provider)) {
      errors['model.provider'] = 'Invalid provider. Must be ollama, openai, or anthropic.';
    }
    if (!settingsValidation.model.model(settings.model.model)) {
      errors['model.model'] = 'Model name must be 1-100 characters.';
    }
    if (settings.model.baseUrl && !settingsValidation.model.baseUrl(settings.model.baseUrl)) {
      errors['model.baseUrl'] = 'Invalid URL format.';
    }
  }
  
  if (settings.tokens) {
    if (!settingsValidation.tokens.maxTokensPerRequest(settings.tokens.maxTokensPerRequest)) {
      errors['tokens.maxTokensPerRequest'] = 'Must be between 100 and 128,000.';
    }
    if (!settingsValidation.tokens.maxContextTokens(settings.tokens.maxContextTokens)) {
      errors['tokens.maxContextTokens'] = 'Must be between 1,000 and 256,000.';
    }
    if (!settingsValidation.tokens.tokenBudget(settings.tokens.tokenBudget)) {
      errors['tokens.tokenBudget'] = 'Must be between 10,000 and 1,000,000.';
    }
  }
  
  if (settings.workers) {
    if (!settingsValidation.workers.minWorkers(settings.workers.minWorkers)) {
      errors['workers.minWorkers'] = 'Must be between 1 and 20.';
    }
    if (!settingsValidation.workers.maxWorkers(settings.workers.maxWorkers)) {
      errors['workers.maxWorkers'] = 'Must be between 1 and 50.';
    }
    if (!settingsValidation.workers.healthCheckInterval(settings.workers.healthCheckInterval)) {
      errors['workers.healthCheckInterval'] = 'Must be between 5 and 300 seconds.';
    }
    if (settings.workers.minWorkers > settings.workers.maxWorkers) {
      errors['workers.minWorkers'] = 'Min workers cannot exceed max workers.';
    }
  }
  
  if (settings.theme) {
    if (!settingsValidation.theme.mode(settings.theme.mode)) {
      errors['theme.mode'] = 'Invalid mode. Must be light, dark, or system.';
    }
  }
  
  if (settings.advanced) {
    if (!settingsValidation.advanced.logLevel(settings.advanced.logLevel)) {
      errors['advanced.logLevel'] = 'Invalid log level. Must be debug, info, warn, or error.';
    }
  }
  
  return {
    valid: Object.keys(errors).length === 0,
    errors,
  };
}
