import { FolderOpenIcon, Cog6ToothIcon, QuestionMarkCircleIcon, ClockIcon } from '@heroicons/react/24/outline'; // Using outline for nav

export function Sidebar() {
    return (
        <aside className="
      w-[220px] 
      bg-muted/30 
      border-r 
      flex flex-col
      py-4
    ">
            <nav className="flex-1 space-y-1 px-2">
                <NavItem icon={<FolderOpenIcon className="w-5 h-5" />} label="Open Project" active />
                <NavItem icon={<ClockIcon className="w-5 h-5" />} label="Recent Files" />
                <NavItem icon={<Cog6ToothIcon className="w-5 h-5" />} label="Settings" />
            </nav>

            <div className="px-2 mt-auto">
                <NavItem icon={<QuestionMarkCircleIcon className="w-5 h-5" />} label="Help" />
            </div>
        </aside>
    );
}

function NavItem({ icon, label, active = false }: { icon: React.ReactNode, label: string, active?: boolean }) {
    return (
        <button
            className={`
        w-full flex items-center gap-3 px-3 py-2 text-sm font-medium rounded-md transition-colors
        ${active
                    ? 'bg-primary/10 text-primary'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                }
      `}
        >
            {icon}
            <span>{label}</span>
        </button>
    );
}
