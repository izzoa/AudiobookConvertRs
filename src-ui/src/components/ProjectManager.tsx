import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { saveProject, loadProject, listRecentProjects, exportProjectSummary } from "@/api/project";
import { pickSaveProjectFile, pickOpenProjectFile, pickExportSummaryFile, writeTextFile } from "@/api/files";
import { useApp } from "@/context/AppContext";
import type { RecentProject, ProjectExport } from "@/types/models";

export function ProjectManager() {
    const { setActiveAudiobook } = useApp();
    const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
    const [isSaving, setIsSaving] = useState(false);
    const [isLoading, setIsLoading] = useState(false);
    const [isExporting, setIsExporting] = useState(false);

    useEffect(() => {
        loadRecentProjects();
    }, []);

    const loadRecentProjects = async () => {
        try {
            const projects = await listRecentProjects();
            setRecentProjects(projects.slice(0, 5));
        } catch (error) {
            console.error("Failed to load recent projects:", error);
        }
    };

    const handleSaveProject = async () => {
        setIsSaving(true);
        try {
            const path = await pickSaveProjectFile();
            if (path) {
                await saveProject(path);
                await loadRecentProjects();
            }
        } catch (error) {
            console.error("Failed to save project:", error);
        } finally {
            setIsSaving(false);
        }
    };

    const handleOpenProject = async () => {
        setIsLoading(true);
        try {
            const path = await pickOpenProjectFile();
            if (path) {
                await openProjectFromPath(path);
            }
        } catch (error) {
            console.error("Failed to open project:", error);
        } finally {
            setIsLoading(false);
        }
    };

    const openProjectFromPath = async (path: string) => {
        setIsLoading(true);
        try {
            const project = await loadProject(path);
            setActiveAudiobook(project.audiobook);
            await loadRecentProjects();
        } catch (error) {
            console.error("Failed to load project:", error);
        } finally {
            setIsLoading(false);
        }
    };

    const handleExportSummary = async () => {
        setIsExporting(true);
        try {
            const summary: ProjectExport = await exportProjectSummary();
            const defaultName = `${summary.title || "project"}_summary`;
            const path = await pickExportSummaryFile(defaultName);
            if (path) {
                const content = path.endsWith(".txt")
                    ? formatSummaryAsText(summary)
                    : JSON.stringify(summary, null, 2);
                await writeTextFile(path, content);
            }
        } catch (error) {
            console.error("Failed to export summary:", error);
        } finally {
            setIsExporting(false);
        }
    };

    const formatSummaryAsText = (summary: ProjectExport): string => {
        const lines = [
            `Title: ${summary.title}`,
            summary.author ? `Author: ${summary.author}` : null,
            summary.narrator ? `Narrator: ${summary.narrator}` : null,
            `Duration: ${summary.duration_formatted}`,
            `Files: ${summary.file_count}`,
            `Chapters: ${summary.chapter_count}`,
            "",
            "Chapters:",
            ...summary.chapters.map(
                (ch) => `  ${ch.index + 1}. ${ch.title} (${ch.start_time} - ${ch.end_time}, ${ch.duration})`
            ),
        ];
        return lines.filter(Boolean).join("\n");
    };

    return (
        <div className="p-4 border rounded-lg bg-card space-y-4">
            <h3 className="font-semibold">Project</h3>

            <div className="flex flex-wrap gap-2">
                <Button onClick={handleSaveProject} disabled={isSaving}>
                    {isSaving ? "Saving..." : "Save Project"}
                </Button>
                <Button variant="secondary" onClick={handleOpenProject} disabled={isLoading}>
                    {isLoading ? "Loading..." : "Open Project"}
                </Button>
                <Button variant="outline" onClick={handleExportSummary} disabled={isExporting}>
                    {isExporting ? "Exporting..." : "Export Summary"}
                </Button>
            </div>

            {recentProjects.length > 0 && (
                <div className="space-y-2">
                    <h4 className="text-sm font-medium text-muted-foreground">Recent Projects</h4>
                    <ul className="space-y-1">
                        {recentProjects.map((project) => (
                            <li key={project.path}>
                                <button
                                    className="text-sm text-left w-full px-2 py-1 rounded hover:bg-muted/50 disabled:opacity-50 disabled:cursor-not-allowed truncate"
                                    onClick={() => openProjectFromPath(project.path)}
                                    disabled={!project.exists || isLoading}
                                    title={project.path}
                                >
                                    {project.title}
                                    {!project.exists && (
                                        <span className="text-muted-foreground ml-2">(missing)</span>
                                    )}
                                </button>
                            </li>
                        ))}
                    </ul>
                </div>
            )}
        </div>
    );
}
