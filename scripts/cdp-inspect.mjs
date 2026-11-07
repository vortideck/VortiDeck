#!/usr/bin/env node
// scripts/cdp-inspect.mjs
// Helper tool for agents and developers to safely inspect running Tauri app and capture screenshots without crashing windows.

import fs from "node:fs";
import path from "node:path";

async function main() {
  const args = process.argv.slice(2);
  const command = args[0] || "screenshot"; // 'screenshot', 'info', 'eval', 'eval-frame', 'list', 'navigate'

  try {
    const listRes = await fetch("http://127.0.0.1:9222/json/list");
    const targets = await listRes.json();

    if (!targets || targets.length === 0) {
      console.error(
        "❌ No active Tauri app targets found at http://127.0.0.1:9222",
      );
      process.exit(1);
    }

    if (command === "list") {
      console.log("Active CDP Targets:");
      targets.forEach((t, i) => {
        console.log(
          `[${i}] Title: "${t.title}" | Type: ${t.type} | URL: ${t.url}`,
        );
      });
      return;
    }

    const requestedTargetIndex = process.env.CDP_TARGET_INDEX;
    const requestedTargetUrl = process.env.CDP_TARGET_URL_INCLUDES;
    const requestedTarget = requestedTargetUrl
      ? targets.find((target) => target.url.includes(requestedTargetUrl))
      : requestedTargetIndex == null
        ? null
        : targets[Number(requestedTargetIndex)];

    // Prefer the titled main window over simulator/assistant child windows.
    const appTarget =
      requestedTarget ||
      targets.find(
        (t) =>
          t.type === "page" &&
          t.url.startsWith("http") &&
          !t.url.includes("splashscreen") &&
          !t.url.includes("/deck/simulator") &&
          !t.url.includes("/assistant-window"),
      ) ||
      targets.find(
        (t) =>
          t.type === "page" &&
          t.url.startsWith("http") &&
          !t.url.includes("splashscreen"),
      ) ||
      targets.find((t) => t.type === "page" && t.url.startsWith("http")) ||
      targets[0];

    if (!appTarget || !appTarget.webSocketDebuggerUrl) {
      console.error("❌ No valid WebSocket debugger URL found for app target");
      process.exit(1);
    }

    const ws = new WebSocket(appTarget.webSocketDebuggerUrl);

    let id = 1;
    const pending = new Map();
    const executionContexts = [];

    function send(method, params = {}) {
      const msgId = id++;
      return new Promise((resolve, reject) => {
        pending.set(msgId, { resolve, reject });
        ws.send(JSON.stringify({ id: msgId, method, params }));
      });
    }

    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data);
        if (msg.id && pending.has(msg.id)) {
          const { resolve, reject } = pending.get(msg.id);
          pending.delete(msg.id);
          if (msg.error) reject(new Error(msg.error.message));
          else resolve(msg.result);
        } else if (msg.method === "Runtime.executionContextCreated") {
          executionContexts.push(msg.params.context);
        }
      } catch {
        // ignore parse error
      }
    };

    ws.onopen = async () => {
      try {
        await send("Page.enable");
        await send("Runtime.enable");

        if (command === "screenshot") {
          const outPath = args[1] || "screenshot.png";
          const res = await send("Page.captureScreenshot", { format: "png" });
          fs.writeFileSync(outPath, Buffer.from(res.data, "base64"));
          console.log(`✅ Screenshot saved to: ${path.resolve(outPath)}`);
        } else if (command === "navigate") {
          const url = args[1];
          if (!url) throw new Error("navigate requires a URL");
          const res = await send("Page.navigate", { url });
          if (res.errorText) throw new Error(res.errorText);
          console.log(`✅ Navigation requested: ${url}`);
        } else if (command === "eval" || command === "eval-frame") {
          const expression = args.slice(1).join(" ") || "document.title";
          let contextId;
          if (command === "eval-frame") {
            const frameTree = await send("Page.getFrameTree");
            const childFrame = frameTree.frameTree?.childFrames?.[0]?.frame;
            if (!childFrame) throw new Error("No child frame found in the selected page");
            contextId = executionContexts.find(
              (context) =>
                context.auxData?.frameId === childFrame.id &&
                context.auxData?.isDefault,
            )?.id;
            if (!contextId) throw new Error("No default execution context found for the child frame");
          }
          const res = await send("Runtime.evaluate", {
            expression,
            returnByValue: true,
            awaitPromise: true,
            ...(contextId ? { contextId } : {}),
          });
          console.log(JSON.stringify(res.result?.value, null, 2));
        } else {
          const res = await send("Runtime.evaluate", {
            expression:
              "({ title: document.title, url: window.location.href, readyState: document.readyState })",
            returnByValue: true,
          });
          console.log("App Target Info:", res.result?.value);
        }

        // Gracefully disable before closing to avoid crashing WebView2
        await send("Page.disable").catch(() => {});
        await send("Runtime.disable").catch(() => {});
      } catch (err) {
        console.error("CDP Execution error:", err.message);
      } finally {
        ws.close();
      }
    };

    ws.onerror = (err) => {
      console.error("WebSocket connection error:", err.message || err);
      process.exit(1);
    };
  } catch (err) {
    console.error("❌ Failed to connect to CDP:", err.message);
    console.log("💡 Ensure `pnpm start:desktop` is running.");
    process.exit(1);
  }
}

main();
