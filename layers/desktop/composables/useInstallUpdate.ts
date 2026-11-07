import { getVersion } from "@tauri-apps/api/app";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useMainStore } from "@/stores/Main";

export const useInstallUpdate = async () => {
    const mainStore = useMainStore();

    let update = null;
    try {
        update = await check();
    } catch (error: any) {
        // When the server responds with 204 No Content (no new version),
        // @tauri-apps/plugin-updater can throw a TypeError reading 'available' from null.
        // This is a normal "up to date" response, not a network failure.
        if (
            error instanceof TypeError &&
            (error.message?.includes("available") || error.message?.includes("null"))
        ) {
            return;
        }

        // Real network failure (e.g. offline, timeout, DNS failure):
        // Re-throw so checkForUpdates can perform its 3 retry attempts with UI status messages.
        throw error;
    }

    if (!update) return;

    const autoUpdate = update.body?.includes("$auto_update=true") ?? false;

    if (autoUpdate) {
        mainStore.setUpdate({
            update: update,
            autoUpdate: true,
        });
    } else {
        mainStore.setUpdate({
            update: update,
            autoUpdate: false,
        });
    }
};

export const startUpdate = async (updateMetadata?: any) => {
    let downloaded = 0;
    let contentLength = 0;
    const mainStore = useMainStore();

    // Re-fetch the update object from Tauri to get the downloadAndInstall method
    const update = await check();
    if (!update) {
        console.error("No update available");
        return;
    }

    try {
        // Download and install with progress monitoring
        await update.downloadAndInstall((event: any) => {
            switch (event.event) {
                case "Started":
                    contentLength = event.data.contentLength ?? 0;
                    mainStore.addNotification({
                        title: `Installing update v${update.version}`,
                        message: "Will relaunch afterwards",
                        autoClose: false,
                        color: "bg-blue-500",
                        type: "update",
                    });
                    break;

                case "Progress":
                    downloaded += event.data.chunkLength;
                    console.log(`Downloaded ${downloaded} / ${contentLength} bytes`);
                    break;

                case "Finished":
                    console.log("Download and installation finished");
                    break;
            }
        });

        // Installation complete - now relaunch the app
        console.log("Update installed successfully. Relaunching...");
        mainStore.removeAllNotificationsOfType("update");
        await relaunch();
    } catch (error) {
        console.error("Update process error:", error);
        mainStore.addNotification({
            title: "Update failed",
            message: String(error),
            autoClose: true,
            color: "bg-red-500",
        });
    }
};
