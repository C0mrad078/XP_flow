import { Toast } from "@/components/ui/toast";
import { useToastStore } from "@/stores/toast-store";

export function Toaster() {
  const toasts = useToastStore((state) => state.toasts);
  const dismiss = useToastStore((state) => state.dismiss);

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-(--z-toast) flex flex-col gap-2">
      {toasts.map((item) => (
        <Toast key={item.id} toast={item} onDismiss={dismiss} />
      ))}
    </div>
  );
}
