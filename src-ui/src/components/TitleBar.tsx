import { useApp } from '@/context/AppContext';
import { UndoRedoButtons } from './UndoRedoButtons';
import { QueueStatusPill } from './QueueStatusPill';

export function TitleBar() {
    const { setView } = useApp();

    return (
        <div className="h-10 bg-background border-b flex items-center justify-between px-4">
            <div className="flex items-center gap-4">
                <button
                    onClick={() => setView('main')}
                    className="font-semibold text-sm hover:text-primary transition-colors"
                >
                    AudioBookConvertRs
                </button>
                <UndoRedoButtons />
            </div>
            <div className="flex items-center gap-2">
                <QueueStatusPill />
            </div>
        </div>
    );
}
