import { FolderOpenIcon, Cog6ToothIcon, QuestionMarkCircleIcon, ClockIcon, DocumentPlusIcon } from '@heroicons/react/24/outline';
import { useState } from 'react';
import * as FilesApi from '@/api/files';
import * as ProjectApi from '@/api/project';
import * as AudiobookApi from '@/api/audiobook';
import { useApp } from '@/context/AppContext';
import type { RecentProject } from '@/types/models';

export function Sidebar() {
    const { setActiveAudiobook, setActiveProject, view, setView } = useApp();
    const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
    const [showRecent, setShowRecent] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleOpenProject = async () => {
        try {
            setError(null);
            const path = await FilesApi.pickOpenProjectFile();
            if (path) {
                const project = await ProjectApi.loadProject(path);
                setActiveAudiobook(project.audiobook);
                setActiveProject(project);
            }
        } catch (error) {
            console.error("Failed to open project:", error);
            setError("Could not open project file.");
        }
    };

    const handleImportFiles = async () => {
        try {
            setError(null);
            const files = await FilesApi.pickFiles();
            if (files && files.length > 0) {
                const audiobook = await AudiobookApi.createAudiobookFromFiles(files);
                setActiveAudiobook(audiobook);
            }
        } catch (error) {
            console.error("Failed to import files:", error);
            setError("Import failed. Please verify file types and try again.");
        }
    };

    const handleToggleRecent = async () => {
        if (!showRecent) {
            try {
                const projects = await ProjectApi.listRecentProjects();
                setRecentProjects(projects.slice(0, 5));
            } catch (error) {
                console.error("Failed to load recent projects:", error);
                setError("Unable to load recent projects.");
            }
        }
        setShowRecent(!showRecent);
    };

    const handleOpenRecentProject = async (path: string) => {
        try {
            const project = await ProjectApi.loadProject(path);
            setActiveAudiobook(project.audiobook);
            setActiveProject(project);
        } catch (error) {
            console.error("Failed to open recent project:", error);
            setError("Could not open selected recent project.");
        }
    };

    const handleSettings = () => {
        setView('settings');
    };

    const handleHelp = () => {
        setView('help');
    };

    return (
        <aside className="w-[220px] bg-muted/30 border-r flex flex-col py-4">
            <nav className="flex-1 space-y-1 px-2">
                <NavItem 
                    icon={<DocumentPlusIcon className="w-5 h-5" />} 
                    label="Import Files" 
                    onClick={handleImportFiles}
                />
                <NavItem 
                    icon={<FolderOpenIcon className="w-5 h-5" />} 
                    label="Open Project" 
                    onClick={handleOpenProject}
                />
                <NavItem 
                    icon={<ClockIcon className="w-5 h-5" />} 
                    label="Recent Projects" 
                    onClick={handleToggleRecent}
                    active={showRecent}
                />
                {showRecent && recentProjects.length > 0 && (
                    <div className="ml-4 space-y-1">
                        {recentProjects.map((project) => (
                            <button
                                key={project.path}
                                className="w-full text-left text-xs px-2 py-1 rounded hover:bg-muted truncate text-muted-foreground hover:text-foreground disabled:opacity-50"
                                onClick={() => handleOpenRecentProject(project.path)}
                                disabled={!project.exists}
                                title={project.path}
                            >
                                {project.title}
                            </button>
                        ))}
                    </div>
                )}
                <NavItem
                    icon={<Cog6ToothIcon className="w-5 h-5" />}
                    label="Settings"
                    onClick={handleSettings}
                    active={view === 'settings'}
                />
            </nav>

            {error && (
                <div className="px-4 py-2 text-xs text-destructive">
                    {error}
                </div>
            )}

            <div className="px-2 mt-auto">
                <NavItem
                    icon={<QuestionMarkCircleIcon className="w-5 h-5" />}
                    label="Help"
                    onClick={handleHelp}
                    active={view === 'help'}
                />
            </div>
        </aside>
    );
}

interface NavItemProps {
    icon: React.ReactNode;
    label: string;
    onClick?: () => void;
    active?: boolean;
    disabled?: boolean;
}

function NavItem({ icon, label, onClick, active = false, disabled = false }: NavItemProps) {
    return (
        <button
            onClick={onClick}
            disabled={disabled}
            className={`
                w-full flex items-center gap-3 px-3 py-2 text-sm font-medium rounded-md transition-colors
                ${active
                    ? 'bg-primary/10 text-primary'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                }
                disabled:opacity-50 disabled:cursor-not-allowed
            `}
        >
            {icon}
            <span>{label}</span>
        </button>
    );
}
