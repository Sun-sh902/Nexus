import { createContext, useCallback, useContext } from 'react';
import { useTranslation } from 'react-i18next';
import type { AgentId, AiDoctorConsultation, CreateWorkspaceRequest, ModelsView, NexusError, Overview, WorkspaceView } from './backend';
import type { ActivityView, AgentView, ExtensionView, Text, ToolView } from '../data/view';

export type DataMode = 'live' | 'demo';
export interface NexusData {
  mode: DataMode;
  status: 'loading' | 'ready' | 'error';
  error: NexusError | null;
  overview: Overview | null;
  agents: AgentView[];
  extensions: ExtensionView[];
  tools: ToolView[];
  activity: ActivityView[];
  workspaces: WorkspaceView[];
  scannedAt: string | null;
  refreshing: boolean;
  refresh: () => Promise<void>;
  models: ModelsView | null;
  modelsError: NexusError | null;
  reloadModels: () => Promise<void>;
  reloadWorkspaces: () => Promise<void>;
  createWorkspace: (req: CreateWorkspaceRequest) => Promise<WorkspaceView>;
  deleteWorkspace: (id: string) => Promise<void>;
  launchWorkspace: (id: string) => Promise<string>;
  consultAiDoctor: (agentId: AgentId) => Promise<AiDoctorConsultation>;
  findAgent: (id: string) => AgentView | undefined;
}


export const NexusContext = createContext<NexusData | null>(null);

export function useNexus(): NexusData {
  const value = useContext(NexusContext);
  if (!value) throw new Error('NexusDataProvider missing');
  return value;
}

/** Renders a view-model `Text`. Literal values are shown verbatim; `$t:` params are translated. */
export function useText() {
  const { t } = useTranslation();
  return useCallback((text: Text | null | undefined): string => {
    if (!text) return '';
    if (text.literal) return text.key;
    const params = Object.fromEntries(Object.entries(text.params ?? {}).map(([k, v]) => [k, typeof v === 'string' && v.startsWith('$t:') ? t(v.slice(3)) : v]));
    return t(text.key, params);
  }, [t]);
}
