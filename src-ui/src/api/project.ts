import { invoke } from '@tauri-apps/api/core';
import type { Project, RecentProject, ProjectExport, ConversionDefaults } from '../types/models';

export async function saveProject(path: string, conversionDefaults?: ConversionDefaults): Promise<void> {
    return await invoke('save_project', { path, conversionDefaults });
}

export async function loadProject(path: string): Promise<Project> {
    return await invoke('load_project', { path });
}

export async function listRecentProjects(): Promise<RecentProject[]> {
    return await invoke('list_recent_projects');
}

export async function clearRecentProjects(): Promise<void> {
    return await invoke('clear_recent_projects');
}

export async function exportProjectSummary(): Promise<ProjectExport> {
    return await invoke('export_project_summary');
}

export async function clearProject(): Promise<void> {
    return await invoke('clear_project');
}
