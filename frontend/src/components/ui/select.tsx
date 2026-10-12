import { forwardRef, type SelectHTMLAttributes } from "react"
import { cn } from "@/lib/utils"

const CLASE_SELECT = "flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm capitalize focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30"

/* `sinBase` usa el `className` tal cual, sin la clase base (`CLASE_SELECT`),
 * para los selects que ya traen su propio estilo completo. */
export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement> & { sinBase?: boolean }>(
  function Select({ className, sinBase = false, ...props }, ref) {
    return (
      <select /* sentinel-disable html-nativo-en-vez-de-componente */
        ref={ref}
        className={sinBase ? className : cn(CLASE_SELECT, className)}
        {...props}
      />
    )
  },
)
