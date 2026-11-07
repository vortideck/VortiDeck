/**
 * useCapabilityDialog — global, singleton controller for the CapabilityDialog.
 *
 * The dialog is mounted ONCE (in `app.vue`) and triggered from ANY install flow
 * via `useCapabilityDialog().open(pkg)`. On grant, it persists the approved
 * capability/permission set via `grant_package_permissions`. This makes capability review a
 * cross-cutting concern (overlay editor, deck store / "My Plugins", marketplace
 * all share one dialog) instead of per-page wiring — mirroring how
 * `GlobalContextMenu` works.
 *
 * The dialog is PACKAGE-GENERAL: it reviews any package's contributions. A widget
 * is just one contribution type (alongside flow-node / ui-panel / macro), and a
 * runtime plugin is a package with a runtime. "Capability" is the user-facing
 * word for widget permissions; under the hood both map to `PluginPermission`.
 *
 * See `docs/WIDGET_CAPABILITY_ARCHITECTURE.md`.
 */
import { ref, readonly, shallowReadonly } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { type PackageInfo } from "../../desktop-streaming/composables/overlay/packageManifest";

// Singleton state (module-level — one dialog app-wide).
const isOpen = ref(false);
const currentManifest = ref<PackageInfo | null>(null);
const isGranting = ref(false);
const grantError = ref<string | null>(null);
const grantProgress = ref(0);
const grantStatus = ref("Ready to install");
// Optional refresh callback fired after a successful grant (e.g. My Plugins
// re-fetches list_packages so the row flips to "installed").
let onGranted: (() => void | Promise<void>) | null = null;

const close = () => {
  isOpen.value = false;
  currentManifest.value = null;
  grantError.value = null;
  grantProgress.value = 0;
  grantStatus.value = "Ready to install";
  onGranted = null;
};

export function useCapabilityDialog() {
  /**
   * Open the dialog to review a package's permissions/capabilities.
   * @param pkg the unified package info (from `list_packages`)
   * @param onGrantedCb optional callback fired after a successful grant
   */
  const open = (pkg: PackageInfo, onGrantedCb?: () => void | Promise<void>) => {
    currentManifest.value = pkg;
    grantError.value = null;
    grantProgress.value = 0;
    grantStatus.value = "Ready to install";
    isOpen.value = true;
    onGranted = onGrantedCb ?? null;
  };

  /** User cancelled. */
  const cancel = () => {
    if (isGranting.value) return;
    close();
  };

  /**
   * User approved — persist one deterministic package-level grant. Rust compiles
   * legacy runtime permissions plus every widget contribution from the
   * discovered manifest; the frontend never constructs security authority.
   */
  const grant = async () => {
    const pkg = currentManifest.value;
    if (!pkg || isGranting.value) return;
    isGranting.value = true;
    grantError.value = null;
    grantProgress.value = 5;
    grantStatus.value = "Starting installation…";
    const startedAt = performance.now();
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<{
        package_id: string;
        progress: number;
        message: string;
        error?: string | null;
      }>("package-lifecycle", ({ payload }) => {
        const canonical = (id: string) => id.replace(/^plugin\./, "");
        if (canonical(payload.package_id) !== canonical(pkg.id)) return;
        grantProgress.value = Math.max(0, Math.min(100, payload.progress));
        grantStatus.value = payload.message;
        if (payload.error) grantError.value = payload.error;
      });
      await invoke("grant_package_permissions", { packageId: pkg.id });
      // Contribution-only packages can finish inside one frame. Keep the real
      // 100% result visible briefly so the dialog never appears to ignore the
      // click; slower runtime compilation is never artificially delayed.
      const remainingFeedbackMs = 450 - (performance.now() - startedAt);
      if (remainingFeedbackMs > 0) {
        await new Promise((resolve) => window.setTimeout(resolve, remainingFeedbackMs));
      }
      await onGranted?.();
      close();
    } catch (e) {
      console.error("[useCapabilityDialog] grant failed:", e);
      grantError.value = e instanceof Error ? e.message : String(e);
    } finally {
      unlisten?.();
      isGranting.value = false;
    }
  };

  return {
    isOpen: readonly(isOpen),
    isGranting: readonly(isGranting),
    grantError: readonly(grantError),
    grantProgress: readonly(grantProgress),
    grantStatus: readonly(grantStatus),
    // Only protect ownership of the ref. Deeply readonly package data is not
    // useful here and makes it incompatible with the shared manifest helpers.
    currentManifest: shallowReadonly(currentManifest),
    open,
    cancel,
    grant,
  };
}
