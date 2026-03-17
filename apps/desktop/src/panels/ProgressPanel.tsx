import { useProgressStore } from '../store/progress-store';
import { Progress } from '../components/ui/progress';
import { Badge } from '../components/ui/badge';
import { Alert, AlertDescription } from '../components/ui/alert';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { ScrollArea } from '../components/ui/scroll-area';
import { Separator } from '../components/ui/separator';
import { formatDuration } from '../types/progress';
import { cn } from '@/lib/utils';
import {
  CheckCircle,
  AlertCircle,
  XCircle,
  Clock,
  Activity,
  AlertTriangle,
  X,
} from 'lucide-react';

export function ProgressPanel() {
  const {
    activeMissions,
    completedMissions,
    failedMissions,
    workers,
    blockers,
    isConnected,
    clearBlocker,
    getProgressSummary,
  } = useProgressStore();

  const summary = getProgressSummary();

  return (
    <div className="h-full w-full bg-background p-4">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-lg font-semibold">Progress Dashboard</h2>
        <Badge variant={isConnected ? 'default' : 'secondary'}>
          <Activity className="mr-1 h-3 w-3" />
          {isConnected ? 'Connected' : 'Disconnected'}
        </Badge>
      </div>

      <ScrollArea className="h-[calc(100vh-100px)]">
        <div className="space-y-4">
          {/* Overall Progress Summary */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Overall Progress</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <Progress value={summary.overallProgress} className="h-2" />
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>{summary.activeMissions} active</span>
                  <span>{summary.completedMissions} completed</span>
                  <span>{summary.failedMissions} failed</span>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Blocker Alerts */}
          {blockers.filter((b) => !b.resolved).length > 0 && (
            <Card className="border-destructive/50">
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-destructive flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4" />
                  Blockers ({blockers.filter((b) => !b.resolved).length})
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {blockers
                    .filter((b) => !b.resolved)
                    .map((blocker) => (
                      <Alert
                        key={blocker.id}
                        variant="destructive"
                        className="py-2"
                      >
                        <AlertCircle className="h-4 w-4" />
                        <AlertDescription className="flex items-center justify-between">
                          <span>{blocker.reason}</span>
                          <button
                            onClick={() => clearBlocker(blocker.id)}
                            className="ml-2 text-destructive hover:text-destructive/80"
                          >
                            <X className="h-4 w-4" />
                          </button>
                        </AlertDescription>
                      </Alert>
                    ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Active Missions */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Clock className="h-4 w-4" />
                Active Missions ({activeMissions.size})
              </CardTitle>
            </CardHeader>
            <CardContent>
              {activeMissions.size === 0 ? (
                <p className="text-sm text-muted-foreground">No active missions</p>
              ) : (
                <div className="space-y-3">
                  {Array.from(activeMissions.values()).map((mission) => (
                    <div key={mission.missionId} className="space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-sm font-medium">
                          {mission.missionId}
                        </span>
                        <Badge variant="outline">{mission.status}</Badge>
                      </div>
                      <Progress value={mission.progress} className="h-2" />
                      <div className="flex justify-between text-xs text-muted-foreground">
                        <span>{mission.progress}%</span>
                        {mission.eta && (
                          <span>ETA: {formatDuration(mission.eta)}</span>
                        )}
                      </div>
                      {mission.message && (
                        <p className="text-xs text-muted-foreground">
                          {mission.message}
                        </p>
                      )}
                      <Separator className="my-2" />
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Worker Status */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Activity className="h-4 w-4" />
                Workers ({workers.size})
              </CardTitle>
            </CardHeader>
            <CardContent>
              {workers.size === 0 ? (
                <p className="text-sm text-muted-foreground">No workers available</p>
              ) : (
                <div className="space-y-2">
                  {Array.from(workers.values()).map((worker) => (
                    <div
                      key={worker.workerId}
                      className={cn(
                        'flex items-center justify-between rounded-md border p-2',
                        worker.status === 'idle' && 'border-muted bg-muted/50',
                        worker.status === 'busy' && 'border-primary/50 bg-primary/5',
                        worker.status === 'unhealthy' &&
                          'border-warning/50 bg-warning/5',
                        worker.status === 'failed' &&
                          'border-destructive/50 bg-destructive/5'
                      )}
                    >
                      <div className="flex items-center gap-2">
                        <div
                          className={cn(
                            'h-2 w-2 rounded-full',
                            worker.status === 'idle' && 'bg-muted-foreground',
                            worker.status === 'busy' && 'bg-primary',
                            worker.status === 'unhealthy' && 'bg-warning',
                            worker.status === 'failed' && 'bg-destructive'
                          )}
                        />
                        <span className="text-sm font-medium">
                          {worker.workerId}
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        {worker.currentTask && (
                          <span className="text-xs text-muted-foreground">
                            {worker.currentTask}
                          </span>
                        )}
                        <Badge
                          variant={
                            worker.status === 'busy' ? 'default' : 'secondary'
                          }
                          className="text-xs"
                        >
                          {worker.status}
                        </Badge>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Completed Missions */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <CheckCircle className="h-4 w-4 text-green-500" />
                Completed ({completedMissions.length})
              </CardTitle>
            </CardHeader>
            <CardContent>
              {completedMissions.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No completed missions
                </p>
              ) : (
                <div className="space-y-2">
                  {completedMissions.map((mission) => (
                    <div
                      key={mission.missionId}
                      className="flex items-center justify-between rounded-md border border-green-500/20 bg-green-500/5 p-2"
                    >
                      <span className="text-sm font-medium">
                        {mission.missionId}
                      </span>
                      <CheckCircle className="h-4 w-4 text-green-500" />
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>

          {/* Failed Missions */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <XCircle className="h-4 w-4 text-destructive" />
                Failed ({failedMissions.length})
              </CardTitle>
            </CardHeader>
            <CardContent>
              {failedMissions.length === 0 ? (
                <p className="text-sm text-muted-foreground">No failed missions</p>
              ) : (
                <div className="space-y-2">
                  {failedMissions.map((mission) => (
                    <div
                      key={mission.missionId}
                      className="flex items-center justify-between rounded-md border border-destructive/20 bg-destructive/5 p-2"
                    >
                      <div className="flex-1">
                        <span className="text-sm font-medium">
                          {mission.missionId}
                        </span>
                        {mission.message && (
                          <p className="text-xs text-destructive">
                            {mission.message}
                          </p>
                        )}
                      </div>
                      <XCircle className="h-4 w-4 text-destructive ml-2" />
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </ScrollArea>
    </div>
  );
}
