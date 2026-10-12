// End-to-end tests driving the real desktop app through tauri-driver.
// Build first: `npm run build && npx tauri build --debug --no-bundle`.
import net from "node:net";
import path from "node:path";

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { remote, type ChainablePromiseElement } from "webdriverio";

const binary =
  process.env.E2E_APP ??
  path.resolve("target", "debug", process.platform === "win32" ? "rodi-checker-app.exe" : "rodi-checker-app");

let browser: WebdriverIO.Browser;

async function open(href: string) {
  await (await browser.$(`aside a[href="${href}"]`)).click();
  await browser.waitUntil(async () => (await browser.getUrl()).includes(href), { timeout: 10_000 });
}

/**
 * WebKitWebDriver drops newlines in setValue, so multi-line text is set with
 * the native setter and an input event that React picks up.
 */
async function setTextarea(element: WebdriverIO.Element | ChainablePromiseElement, value: string) {
  await browser.execute(
    (el, text) => {
      const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!;
      setter.call(el, text);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    },
    element as unknown as HTMLTextAreaElement,
    value,
  );
}

async function heading() {
  return (await browser.$("main h1")).getText();
}

beforeAll(async () => {
  browser = await remote({
    hostname: "127.0.0.1",
    port: 4444,
    logLevel: "warn",
    // tauri-driver speaks classic WebDriver only (no BiDi).
    capabilities: { "wdio:enforceWebDriverClassic": true, "tauri:options": { application: binary } } as WebdriverIO.Capabilities,
  });
  await browser.waitUntil(async () => (await heading()) === "Panel", { timeout: 20_000 });
});

afterAll(async () => {
  await browser?.deleteSession();
});

describe("Rodi Checker", () => {
  it("shows the dashboard in Turkish by default", async () => {
    expect(await heading()).toBe("Panel");
    expect(await (await browser.$("aside")).getText()).toContain("Proxy Checker");
  });

  it("creates a TCP monitor that reports the port as up", async () => {
    const server = net.createServer().listen(0, "127.0.0.1");
    await new Promise((resolve) => server.once("listening", resolve));
    const port = (server.address() as net.AddressInfo).port;

    await open("/web/");
    await (await browser.$("button=Yeni izleyici")).click();
    const dialog = await browser.$('[role="dialog"]');
    await (await dialog.$("input")).setValue("E2E port");
    await (await dialog.$("select")).selectByAttribute("value", "tcp");
    const inputs = await dialog.$$("input");
    await inputs[1].setValue("127.0.0.1");
    await inputs[2].setValue(String(port));
    await (await dialog.$("button=Kaydet")).click();

    const row = await browser.$("tr*=E2E port");
    await row.waitForExist({ timeout: 10_000 });
    await browser.waitUntil(async () => (await row.getText()).includes("Çalışıyor"), {
      timeout: 15_000,
      timeoutMsg: "monitor never reported up",
    });
    server.close();
  });

  it("imports a proxy list", async () => {
    await open("/proxy/");
    await (await browser.$("button=Liste içe aktar")).click();
    const dialog = await browser.$('[role="dialog"]');
    await setTextarea(await dialog.$("textarea"), "127.0.0.1:1\nsocks5://user:pass@127.0.0.1:2\nbad line");
    await browser.waitUntil(async () => (await dialog.getText()).includes("2 geçerli proxy"), { timeout: 10_000 });
    await (await dialog.$("button=İçe aktar")).click();
    await browser.waitUntil(async () => (await (await browser.$("main")).getText()).includes("0/2 çalışan"), {
      timeout: 10_000,
    });
  });

  it("shows live system metrics", async () => {
    await open("/system/");
    await browser.waitUntil(async () => (await (await browser.$("main")).getText()).includes("Çekirdekler"), {
      timeout: 15_000,
    });
  });

  it("previews reports", async () => {
    await open("/reports/");
    await browser.waitUntil(async () => (await (await browser.$("main")).getText()).includes("E2E port"), {
      timeout: 10_000,
    });
  });

  it("switches the language to English", async () => {
    await open("/settings/");
    await (await browser.$("main select")).selectByAttribute("value", "en");
    await (await browser.$("button=Kaydet")).click();
    await browser.waitUntil(async () => (await (await browser.$("aside")).getText()).includes("Dashboard"), {
      timeout: 10_000,
    });
  });
});
