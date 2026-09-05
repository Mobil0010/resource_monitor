import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";

const source = readFileSync(new URL("../docs/app.js", import.meta.url), "utf8");
const base = "https://github.com/Mobil0010/resource_monitor";
const suffixes = ["macOS-Universal.dmg", "Windows-Setup.exe", "Windows-Portable.zip"];
const release = {
  tag_name: "v0.1.0",
  published_at: "2026-09-05T00:00:00Z",
  assets: suffixes.map((suffix) => ({
    name: "ResourceMonitor-0.1.0-" + suffix,
    browser_download_url: base + "/releases/download/v0.1.0/ResourceMonitor-0.1.0-" + suffix,
  })),
};

async function render(platform, payload = release, status = 200, search = "") {
  const elements = new Map();
  function element(selector) {
    if (!elements.has(selector)) elements.set(selector, {
      hidden: false, attributes: {}, textContent: "",
      querySelector: (child) => element(selector + " " + child),
      setAttribute(key, value) { this.attributes[key] = value; },
      removeAttribute(key) { delete this.attributes[key]; },
      addEventListener() {},
    });
    return elements.get(selector);
  }
  const body = { dataset: { repository: "Mobil0010/resource_monitor" } };
  const context = vm.createContext({
    document: { body, querySelector: element },
    navigator: { platform, userAgent: platform },
    location: { search },
    URLSearchParams, Intl, Date,
    AbortSignal: { timeout: () => undefined },
    fetch: async (url) => {
      assert.equal(url, "https://api.github.com/repos/Mobil0010/resource_monitor/releases/latest");
      if (payload instanceof Error) throw payload;
      return { ok: status === 200, json: async () => payload };
    },
  });
  await vm.runInContext(source, context);
  return { element, body };
}

for (const [platform, suffix] of [["MacIntel", suffixes[0]], ["Win32", suffixes[1]]]) {
  test(platform + " selects the correct installer", async () => {
    const { element } = await render(platform);
    assert.ok(element("#primary-download").href.endsWith(suffix));
    assert.equal(element("#portable-download").hidden, platform !== "Win32");
    assert.ok(element("#windows-portable-download").href.endsWith(suffixes[2]));
    assert.equal(element("#version-label").textContent, "v0.1.0");
  });
}
test("unknown OS links to releases, not a Mac installer", async () => {
  const { element } = await render("Linux");
  assert.equal(element("#primary-download").href, base + "/releases/latest");
});
test("platform override works on project Pages URLs", async () => {
  const { element } = await render("MacIntel", release, 200, "?platform=windows");
  assert.ok(element("#primary-download").href.endsWith(suffixes[1]));
});
for (const [label, payload, status] of [
  ["missing release", {}, 404],
  ["rate limited", {}, 403],
  ["network error", new Error("offline"), 200],
  ["invalid data", {}, 200],
]) {
  test(label + " keeps release links usable", async () => {
    const { element } = await render("Win32", payload, status);
    for (const id of ["primary-download", "portable-download", "mac-download", "windows-download", "windows-portable-download"]) {
      assert.equal(element("#" + id).href, base + "/releases/latest");
      assert.equal(element("#" + id).attributes["aria-disabled"], undefined);
    }
  });
}
test("missing installer preserves other downloads", async () => {
  const { element } = await render("Win32", { ...release, assets: [release.assets[0]] });
  assert.equal(element("#primary-download").href, base + "/releases/latest");
  assert.ok(element("#mac-download").href.endsWith(suffixes[0]));
});
test("untrusted asset URLs are not used", async () => {
  const { element } = await render("MacIntel", {
    ...release,
    assets: [{ ...release.assets[0], browser_download_url: "https://example.com/file.dmg" }],
  });
  assert.equal(element("#primary-download").href, base + "/releases/latest");
});
test("invalid publication date does not discard downloads", async () => {
  const { element } = await render("MacIntel", { ...release, published_at: "invalid" });
  assert.ok(element("#primary-download").href.endsWith(suffixes[0]));
});
