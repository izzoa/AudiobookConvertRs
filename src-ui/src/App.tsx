import { Layout } from "@/components/Layout";
import { FileInput } from "@/components/FileInput";
import { MetadataEditor } from "@/components/MetadataEditor";
import { ChapterEditor } from "@/components/ChapterEditor";
import { ConversionQueuePanel } from "@/components/ConversionQueuePanel";
import { NormalizationPanel } from "@/components/NormalizationPanel";
import { DryRunPreview } from "@/components/DryRunPreview";
import { ProjectManager } from "@/components/ProjectManager";
import { SettingsPage } from "@/components/SettingsPage";
import { HelpPage } from "@/components/HelpPage";
import { AudioPlayer } from "@/components/AudioPlayer";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useApp } from "@/context/AppContext";

function WorkflowContent() {
  return (
    <div className="p-6 space-y-6 max-w-4xl mx-auto">
      {/* Project Management */}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold tracking-tight">Project</h2>
        <ProjectManager />
      </section>

      {/* Step 1: Import */}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold tracking-tight">Step 1: Import Files</h2>
        <FileInput />
      </section>

      {/* Step 2: Metadata */}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold tracking-tight">Step 2: Edit Metadata</h2>
        <MetadataEditor />
      </section>

      {/* Step 2b: Chapters */}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold tracking-tight">Step 2b: Edit Chapters</h2>
        <ChapterEditor />
      </section>

      {/* Audio Quality */}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold tracking-tight">Audio Quality</h2>
        <NormalizationPanel />
      </section>

      {/* Step 3: Convert */}
      <section className="space-y-3">
        <h2 className="text-xl font-semibold tracking-tight">Step 3: Convert</h2>
        <DryRunPreview />
        <ConversionQueuePanel />
      </section>
    </div>
  );
}

function App() {
  const { view } = useApp();

  useKeyboardShortcuts({
    onOpen: () => {
      console.log("Open project shortcut triggered");
    },
    onSave: () => {
      console.log("Save project shortcut triggered");
    },
  });

  const renderView = () => {
    switch (view) {
      case 'settings':
        return <SettingsPage />;
      case 'help':
        return <HelpPage />;
      default:
        return <WorkflowContent />;
    }
  };

  return (
    <Layout>
      {renderView()}
      <AudioPlayer />
    </Layout>
  );
}

export default App;
