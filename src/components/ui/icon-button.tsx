import * as React from "react";

import { Button, type ButtonProps } from "./button";
import { cn } from "@/lib/utilities/cn";

export interface IconButtonProps extends Omit<ButtonProps, "size"> {
  label: string;
  size?: "sm" | "md" | "lg";
}

/** A `Button` that renders icon-only, with a mandatory accessible label
 * (section 52 — every icon-only control keeps a screen-reader name). */
export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ label, size = "md", variant = "ghost", className, children, ...props }, ref) => {
    const dimension = size === "sm" ? "size-7" : size === "lg" ? "size-10" : "size-9";
    return (
      <Button
        ref={ref}
        variant={variant}
        size="icon"
        aria-label={label}
        title={label}
        className={cn(dimension, className)}
        {...props}
      >
        {children}
      </Button>
    );
  },
);
IconButton.displayName = "IconButton";
