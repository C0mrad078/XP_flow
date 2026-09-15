import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useConfirmStore } from "@/stores/confirm-store";

/** Mounted once at the app root (next to `<Toaster/>`); renders whatever
 * `confirmAction()` is currently waiting on. See `stores/confirm-store.ts`. */
export function ConfirmDialogHost() {
  const pending = useConfirmStore((state) => state.pending);
  const settle = useConfirmStore((state) => state.settle);

  return (
    <Dialog open={pending !== null} onOpenChange={(open) => !open && settle(false)}>
      {pending && (
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>{pending.title}</DialogTitle>
            <DialogDescription>{pending.description}</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => settle(false)}>
              {pending.cancelLabel ?? "Cancel"}
            </Button>
            <Button variant={pending.destructive ? "destructive" : "primary"} onClick={() => settle(true)}>
              {pending.confirmLabel ?? "Confirm"}
            </Button>
          </DialogFooter>
        </DialogContent>
      )}
    </Dialog>
  );
}
