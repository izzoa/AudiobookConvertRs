import { getCurrentWindow } from '@tauri-apps/api/window';
import { MinusIcon, Square2StackIcon, XMarkIcon } from '@heroicons/react/24/outline';

export function TitleBar() {
    const appWindow = getCurrentWindow();

    return (
        <div
            className="
        h-8 bg-background border-b
        flex items-center justify-between px-4
        select-none
        fixed top-0 left-0 right-0 z-50
      "
            data-tauri-drag-region
        >
            <span className="font-semibold text-sm">AudioBookConvertRs</span>

            <div className="flex gap-2">
                <button
                    onClick={() => appWindow.minimize()}
                    className="hover:bg-accent/50 p-1 rounded transition-colors"
                >
                    <MinusIcon className="w-4 h-4" />
                </button>
                <button
                    onClick={() => appWindow.toggleMaximize()}
                    className="hover:bg-accent/50 p-1 rounded transition-colors"
                >
                    <Square2StackIcon className="w-4 h-4" />
                </button>
                <button
                    onClick={() => appWindow.close()}
                    className="hover:bg-destructive hover:text-destructive-foreground p-1 rounded transition-colors"
                >
                    <XMarkIcon className="w-4 h-4" />
                </button>
            </div>
        </div>
    );
}
