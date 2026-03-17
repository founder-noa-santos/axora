/**
 * Settings Panel
 * 
 * Main settings UI component with sections for:
 * - Model Configuration
 * - Token Limits
 * - Worker Pool
 * - Theme Preferences
 * - Advanced Settings
 */

import { useState } from 'react';
import { useSettingsStore } from '../store/settings-store';
import type { AppSettings } from '../types/settings';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Separator } from '@/components/ui/separator';
import './SettingsPanel.css';

export function SettingsPanel() {
  const { 
    settings, 
    saveSettings, 
    resetSettings, 
    updateSetting,
    exportSettings,
    importSettings,
    isLoading,
    error,
    clearError,
    hasUnsavedChanges,
  } = useSettingsStore();
  
  const [localError, setLocalError] = useState<string | null>(null);
  const [showImportExport, setShowImportExport] = useState(false);
  const [importJson, setImportJson] = useState('');

  const handleSave = async () => {
    try {
      await saveSettings(settings);
      setLocalError(null);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Failed to save settings');
    }
  };

  const handleReset = () => {
    if (window.confirm('Are you sure you want to reset all settings to defaults?')) {
      resetSettings();
    }
  };

  const handleExport = () => {
    const json = exportSettings();
    navigator.clipboard.writeText(json);
    alert('Settings copied to clipboard!');
  };

  const handleImport = async () => {
    try {
      await importSettings(importJson);
      setImportJson('');
      setShowImportExport(false);
      setLocalError(null);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : 'Failed to import settings');
    }
  };

  const displayError = localError || error;

  return (
    <div className="settings-panel">
      <header className="panel-header">
        <div className="header-content">
          <h2 className="panel-title">Settings</h2>
          {hasUnsavedChanges && (
            <span className="unsaved-indicator">Unsaved changes</span>
          )}
        </div>
        <div className="header-actions">
          <Button 
            variant="ghost" 
            size="sm"
            onClick={() => setShowImportExport(!showImportExport)}
          >
            {showImportExport ? 'Hide' : 'Import/Export'}
          </Button>
          <Button 
            variant="secondary" 
            size="sm"
            onClick={handleReset}
          >
            Reset
          </Button>
          <Button 
            variant="default" 
            size="sm"
            onClick={handleSave}
            disabled={!hasUnsavedChanges || isLoading}
          >
            {isLoading ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      </header>

      {displayError && (
        <Alert variant="destructive" className="settings-alert">
          <AlertDescription className="flex items-center justify-between">
            {displayError}
            <button onClick={clearError} className="close-error">×</button>
          </AlertDescription>
        </Alert>
      )}

      {showImportExport && (
        <Card className="import-export-card">
          <CardHeader>
            <CardTitle>Import/Export Settings</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="import-export-content">
              <textarea
                value={importJson}
                onChange={(e) => setImportJson(e.target.value)}
                placeholder="Paste settings JSON here..."
                rows={10}
                className="json-textarea"
              />
              <div className="import-export-actions">
                <Button variant="outline" onClick={handleExport}>
                  Export to Clipboard
                </Button>
                <Button variant="default" onClick={handleImport}>
                  Import from JSON
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      <div className="settings-content">
        {/* Model Configuration Section */}
        <Card>
          <CardHeader>
            <CardTitle>Model Configuration</CardTitle>
          </CardHeader>
          <CardContent className="settings-grid">
            <div className="setting-item">
              <Label>Provider</Label>
              <Select
                value={settings.model.provider}
                onValueChange={(value: string) => updateSetting('model', 'provider', value as AppSettings['model']['provider'])}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ollama">Ollama (Local)</SelectItem>
                  <SelectItem value="openai">OpenAI</SelectItem>
                  <SelectItem value="anthropic">Anthropic</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="setting-item">
              <Label>Model</Label>
              <Input
                value={settings.model.model}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('model', 'model', e.target.value)}
                placeholder="e.g., qwen2.5-coder:7b"
              />
            </div>
            
            {settings.model.provider === 'ollama' && (
              <div className="setting-item">
                <Label>Base URL</Label>
                <Input
                  value={settings.model.baseUrl || ''}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('model', 'baseUrl', e.target.value)}
                  placeholder="http://localhost:11434"
                />
              </div>
            )}
            
            {(settings.model.provider === 'openai' || settings.model.provider === 'anthropic') && (
              <div className="setting-item">
                <Label>API Key</Label>
                <Input
                  type="password"
                  value={settings.model.apiKey || ''}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('model', 'apiKey', e.target.value)}
                  placeholder="sk-..."
                />
              </div>
            )}
          </CardContent>
        </Card>

        {/* Token Limits Section */}
        <Card>
          <CardHeader>
            <CardTitle>Token Limits</CardTitle>
          </CardHeader>
          <CardContent className="settings-grid">
            <div className="setting-item">
              <Label>Max Tokens per Request</Label>
              <Input
                type="number"
                value={settings.tokens.maxTokensPerRequest}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('tokens', 'maxTokensPerRequest', parseInt(e.target.value) || 0)}
                min={100}
                max={128000}
              />
            </div>
            
            <div className="setting-item">
              <Label>Max Context Tokens</Label>
              <Input
                type="number"
                value={settings.tokens.maxContextTokens}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('tokens', 'maxContextTokens', parseInt(e.target.value) || 0)}
                min={1000}
                max={256000}
              />
            </div>
            
            <div className="setting-item">
              <Label>Token Budget (per session)</Label>
              <Input
                type="number"
                value={settings.tokens.tokenBudget}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('tokens', 'tokenBudget', parseInt(e.target.value) || 0)}
                min={10000}
                max={1000000}
              />
            </div>
          </CardContent>
        </Card>

        {/* Worker Pool Section */}
        <Card>
          <CardHeader>
            <CardTitle>Worker Pool</CardTitle>
          </CardHeader>
          <CardContent className="settings-grid">
            <div className="setting-item">
              <Label>Min Workers</Label>
              <Input
                type="number"
                value={settings.workers.minWorkers}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('workers', 'minWorkers', parseInt(e.target.value) || 0)}
                min={1}
                max={20}
              />
            </div>
            
            <div className="setting-item">
              <Label>Max Workers</Label>
              <Input
                type="number"
                value={settings.workers.maxWorkers}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('workers', 'maxWorkers', parseInt(e.target.value) || 0)}
                min={1}
                max={50}
              />
            </div>
            
            <div className="setting-item">
              <Label>Health Check Interval (seconds)</Label>
              <Input
                type="number"
                value={settings.workers.healthCheckInterval}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('workers', 'healthCheckInterval', parseInt(e.target.value) || 0)}
                min={5}
                max={300}
              />
            </div>
          </CardContent>
        </Card>

        {/* Theme Section */}
        <Card>
          <CardHeader>
            <CardTitle>Theme Preferences</CardTitle>
          </CardHeader>
          <CardContent className="settings-grid">
            <div className="setting-item">
              <Label>Theme Mode</Label>
              <Select
                value={settings.theme.mode}
                onValueChange={(value: string) => updateSetting('theme', 'mode', value as AppSettings['theme']['mode'])}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">Light</SelectItem>
                  <SelectItem value="dark">Dark</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="setting-item">
              <Label>Accent Color</Label>
              <Input
                value={settings.theme.accentColor}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => updateSetting('theme', 'accentColor', e.target.value)}
                placeholder="e.g., electric-purple"
              />
            </div>
          </CardContent>
        </Card>

        {/* Advanced Section */}
        <Card>
          <CardHeader>
            <CardTitle>Advanced Settings</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="setting-item toggle-item">
              <div className="toggle-label">
                <Label>Enable Logging</Label>
              </div>
              <Switch
                checked={settings.advanced.enableLogging}
                onCheckedChange={(value: boolean) => updateSetting('advanced', 'enableLogging', value)}
              />
            </div>
            
            <Separator className="my-4" />
            
            <div className="setting-item">
              <Label>Log Level</Label>
              <Select
                value={settings.advanced.logLevel}
                onValueChange={(value: string) => updateSetting('advanced', 'logLevel', value as AppSettings['advanced']['logLevel'])}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="debug">Debug</SelectItem>
                  <SelectItem value="info">Info</SelectItem>
                  <SelectItem value="warn">Warn</SelectItem>
                  <SelectItem value="error">Error</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <Separator className="my-4" />
            
            <div className="setting-item toggle-item">
              <div className="toggle-label">
                <Label>Auto Update</Label>
              </div>
              <Switch
                checked={settings.advanced.autoUpdate}
                onCheckedChange={(value: boolean) => updateSetting('advanced', 'autoUpdate', value)}
              />
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
