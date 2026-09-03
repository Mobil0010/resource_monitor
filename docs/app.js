const body = document.body;
const repoOverride = body.dataset.repository.trim();

function detectRepository() {
  if (repoOverride) return repoOverride;
  const hostMatch = location.hostname.match(/^([^.]+)\.github\.io$/);
  const repository = location.pathname.split("/").filter(Boolean)[0];
  return hostMatch && repository ? `${hostMatch[1]}/${repository}` : "";
}

function detectPlatform() {
  const value = `${navigator.userAgentData?.platform || ""} ${navigator.platform || ""} ${navigator.userAgent}`.toLowerCase();
  if (value.includes("win")) return "windows";
  if (value.includes("mac")) return "mac";
  return "other";
}

function findAsset(assets, platform) {
  const patterns = platform === "mac"
    ? [/\.dmg$/i, /mac.*\.zip$/i, /\.pkg$/i]
    : [/setup.*\.exe$/i, /windows?.*\.exe$/i, /\.msi$/i, /windows?.*\.zip$/i];
  return patterns.flatMap((pattern) => assets.filter((asset) => pattern.test(asset.name)))[0];
}

const repository = detectRepository();
const platform = detectPlatform();
const releasesUrl = repository ? `https://github.com/${repository}/releases/latest` : "#";
const githubUrl = repository ? `https://github.com/${repository}` : "#";
const primary = document.querySelector("#primary-download");
const primaryText = primary.querySelector("span");
const osMessage = document.querySelector("#os-message");
const versionLabel = document.querySelector("#version-label");
const releaseNote = document.querySelector("#release-note");
const macDownload = document.querySelector("#mac-download");
const windowsDownload = document.querySelector("#windows-download");

document.querySelector("#year").textContent = new Date().getFullYear();
document.querySelector("#github-link").href = githubUrl;

const platformCopy = {
  mac: ["macOS가 감지됐어", "macOS용 다운로드"],
  windows: ["Windows가 감지됐어", "Windows용 다운로드"],
  other: ["사용할 운영체제를 선택해", "최신 버전 보기"],
};
osMessage.textContent = platformCopy[platform][0];
primaryText.textContent = platformCopy[platform][1];

const menuButton = document.querySelector("#other-platforms");
const menu = document.querySelector("#platform-menu");
menuButton.addEventListener("click", () => {
  const open = menu.hidden;
  menu.hidden = !open;
  menuButton.setAttribute("aria-expanded", String(open));
});

function useReleasePage(message) {
  [primary, macDownload, windowsDownload].forEach((link) => { link.href = releasesUrl; });
  releaseNote.textContent = message;
  if (!repository) {
    primary.setAttribute("aria-disabled", "true");
    document.querySelector("#github-link").removeAttribute("target");
    releaseNote.textContent = "GitHub Pages에 배포하면 최신 릴리스에 자동으로 연결돼.";
  }
}

async function loadLatestRelease() {
  if (!repository) return useReleasePage("");
  useReleasePage("최신 릴리스 페이지에서도 모든 파일을 확인할 수 있어.");

  try {
    const response = await fetch(`https://api.github.com/repos/${repository}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) throw new Error("release unavailable");
    const release = await response.json();
    const mac = findAsset(release.assets, "mac");
    const windows = findAsset(release.assets, "windows");

    versionLabel.textContent = release.tag_name || "최신 버전";
    macDownload.href = mac?.browser_download_url || release.html_url;
    windowsDownload.href = windows?.browser_download_url || release.html_url;
    primary.href = platform === "mac" ? macDownload.href : platform === "windows" ? windowsDownload.href : release.html_url;
    releaseNote.textContent = release.published_at
      ? `${new Intl.DateTimeFormat("ko-KR", { dateStyle: "long" }).format(new Date(release.published_at))} 릴리스`
      : "GitHub Releases에서 제공하는 최신 버전이야.";
  } catch {
    releaseNote.textContent = "최신 설치 파일은 GitHub Releases에서 확인할 수 있어.";
  }
}

loadLatestRelease();
