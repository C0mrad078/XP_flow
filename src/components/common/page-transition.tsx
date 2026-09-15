import type { ReactNode } from "react";
import { motion } from "motion/react";
import { useLocation } from "react-router-dom";

/**
 * Subtle per-route fade/slide-in (section 33). Enter-only — no exit
 * animation — so navigating never feels delayed waiting for the previous
 * page to animate out.
 */
export function PageTransition({ children }: { children: ReactNode }) {
  const location = useLocation();

  return (
    <motion.div
      key={location.pathname}
      initial={{ opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.16, ease: "easeOut" }}
      className="h-full"
    >
      {children}
    </motion.div>
  );
}
