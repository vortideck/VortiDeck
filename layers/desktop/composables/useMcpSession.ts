import { invoke } from "@tauri-apps/api/core";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";

export const useMcpSession = async (isMcpEnabled: boolean) => {
    const desktopUtils = useDesktopUtils();
    desktopUtils.setMcpTools([]);
    if (isMcpEnabled) {
        
        console.log("MCP Server Enabled, starting session");
        const res = await invoke("init_chat", {
          request: {},
        });
        console.log(res);
        // mainStore.addNotification({
        //   title: res.success ? "MCP Server Connected" : "MCP Server Error",
        //   message: res.message,
        //   buttonText: "Close",
        //   buttonFunction: () => {},
        //   autoClose: true,
    }
}
