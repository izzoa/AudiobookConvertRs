import * as React from "react"

import type {
    ToastActionElement,
    ToastProps,
} from "@/components/ui/toast"

const TOAST_LIMIT = 5
const TOAST_REMOVE_DELAY = 1000

type ToasterToast = ToastProps & {
    id: string
    title?: React.ReactNode
    description?: React.ReactNode
    action?: ToastActionElement
}

type ToasterState = {
    toasts: ToasterToast[]
}

const toastTimeouts = new Map<string, ReturnType<typeof setTimeout>>()

type ActionType =
    | { type: "ADD_TOAST"; toast: ToasterToast }
    | { type: "UPDATE_TOAST"; toast: Partial<ToasterToast> }
    | { type: "DISMISS_TOAST"; toastId?: string }
    | { type: "REMOVE_TOAST"; toastId?: string }

const addToRemoveQueue = (toastId: string) => {
    if (toastTimeouts.has(toastId)) {
        return
    }

    const timeout = setTimeout(() => {
        toastTimeouts.delete(toastId)
        dispatch({ type: "REMOVE_TOAST", toastId })
    }, TOAST_REMOVE_DELAY)

    toastTimeouts.set(toastId, timeout)
}

const toasterReducer = (state: ToasterState, action: ActionType): ToasterState => {
    switch (action.type) {
    case "ADD_TOAST":
        return {
            ...state,
            toasts: [action.toast, ...state.toasts].slice(0, TOAST_LIMIT),
        }

    case "UPDATE_TOAST":
        return {
            ...state,
            toasts: state.toasts.map((toast) =>
                toast.id === action.toast.id ? { ...toast, ...action.toast } : toast,
            ),
        }

    case "DISMISS_TOAST": {
        const { toastId } = action

        if (toastId) {
            addToRemoveQueue(toastId)
        } else {
            state.toasts.forEach((toast) => {
                addToRemoveQueue(toast.id)
            })
        }

        return {
            ...state,
            toasts: state.toasts.map((toast) =>
                toast.id === toastId || toastId === undefined
                    ? {
                        ...toast,
                        open: false,
                    }
                    : toast,
            ),
        }
    }
    case "REMOVE_TOAST":
        return {
            ...state,
            toasts: state.toasts.filter((toast) => toast.id !== action.toastId),
        }
    default:
        return state
    }
}

const listeners: Array<(state: ToasterState) => void> = []

let memoryState: ToasterState = { toasts: [] }

function dispatch(action: ActionType) {
    memoryState = toasterReducer(memoryState, action)
    listeners.forEach((listener) => listener(memoryState))
}

type Toast = Omit<ToasterToast, "id">

function toast(props: Toast) {
    const id = crypto.randomUUID()

    const update = (props: ToasterToast) =>
        dispatch({
            type: "UPDATE_TOAST",
            toast: { ...props, id },
        })
    const dismiss = () => dispatch({ type: "DISMISS_TOAST", toastId: id })

    dispatch({
        type: "ADD_TOAST",
        toast: {
            ...props,
            id,
            open: true,
            onOpenChange: (open) => {
                if (!open) dismiss()
            },
        },
    })

    return {
        id,
        dismiss,
        update,
    }
}

toast.dismiss = (toastId?: string) => dispatch({ type: "DISMISS_TOAST", toastId })
toast.remove = (toastId?: string) => dispatch({ type: "REMOVE_TOAST", toastId })

function useToast() {
    const [state, setState] = React.useState<ToasterState>(memoryState)

    React.useEffect(() => {
        listeners.push(setState)
        return () => {
            const index = listeners.indexOf(setState)
            if (index > -1) {
                listeners.splice(index, 1)
            }
        }
    }, [])

    return {
        ...state,
        toast,
        dismiss: toast.dismiss,
        remove: toast.remove,
    }
}

export { useToast, toast }

