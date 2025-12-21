import { UndoRedoButtons } from './UndoRedoButtons';

export function TitleBar() {
    return (
        <div className="h-10 bg-background border-b flex items-center justify-between px-4">
            <div className="flex items-center gap-4">
                <span className="font-semibold text-sm">AudioBookConvertRs</span>
                <UndoRedoButtons />
            </div>
        </div>
    );
}
