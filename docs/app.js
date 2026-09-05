const body = document.body;
const repoOverride = body.dataset.repository.trim();
const releaseApiUrl = `https://api.github.com/repos/${repoOverride}/releases/latest`;

function detectPlatform() {
  const override = new URLSearchParams(location.search).get("platform");
  if (["mac", "windows", "other"].includes(override)) return override;
  const value = `${navigator.userAgentData?.platform || ""} ${navigator.platform || ""} ${navigator.userAgent}`.toLowerCase();
  if (value.includes("win")) return "windows";
  if (value.includes("mac")) return "mac";
  return "other";
}

const platform = detectPlatform();
const githubUrl = repoOverride ? `https://github.com/${repoOverride}` : "#";
const primary = document.querySelector("#primary-download");
const primaryText = primary.querySelector("span");
const portable = document.querySelector("#portable-download");
const osMessage = document.querySelector("#os-message");
const installHeading = document.querySelector("#install-heading");
const installDescription = document.querySelector("#install-description");
const versionLabel = document.querySelector("#version-label");
const releaseNote = document.querySelector("#release-note");
const macDownload = document.querySelector("#mac-download");
const windowsDownload = document.querySelector("#windows-download");
const windowsPortableDownload = document.querySelector("#windows-portable-download");

document.querySelector("#year").textContent = new Date().getFullYear();
document.querySelector("#github-link").href = githubUrl;

const platformCopy = {
  mac: {
    status: "macOS가 감지되었습니다",
    heading: "macOS에 설치",
    description: "Universal DMG 하나로 Apple Silicon과 Intel Mac을 모두 지원합니다. DMG를 연 뒤 앱을 Applications 폴더로 옮겨 주세요.",
    primary: "macOS DMG 다운로드",
  },
  windows: {
    status: "Windows가 감지되었습니다",
    heading: "Windows에 설치",
    description: "일반 설치에는 설치 마법사를 권장합니다. 설치 권한이 없거나 USB에서 실행하려면 Portable ZIP을 선택해 주세요.",
    primary: "설치 마법사 다운로드",
  },
  other: {
    status: "운영체제를 선택해 주세요",
    heading: "설치 파일 선택",
    description: "macOS용 DMG 또는 Windows용 설치 마법사와 Portable ZIP 중에서 선택할 수 있습니다.",
    primary: "최신 릴리스 보기",
  },
};
const copy = platformCopy[platform];
body.dataset.platform = platform;
osMessage.textContent = copy.status;
installHeading.textContent = copy.heading;
installDescription.textContent = copy.description;
primaryText.textContent = copy.primary;
portable.hidden = platform !== "windows";

const menuButton = document.querySelector("#other-platforms");
const menu = document.querySelector("#platform-menu");
menuButton.addEventListener("click", () => {
  const open = menu.hidden;
  menu.hidden = !open;
  menuButton.setAttribute("aria-expanded", String(open));
});

const releasesUrl = `${githubUrl}/releases/latest`;

function useReleasePage(message) {
  [primary, portable, macDownload, windowsDownload, windowsPortableDownload].forEach((link) => {
    link.href = releasesUrl;
    link.removeAttribute("aria-disabled");
  });
  primaryText.textContent = "GitHub에서 설치 파일 보기";
  releaseNote.textContent = message;
}

async function loadLatestRelease() {
  useReleasePage("GitHub Releases에서 최신 설치 파일을 확인하고 있습니다.");

  try {
    const response = await fetch(releaseApiUrl, {
      headers: { Accept: "application/vnd.github+json" },
      signal: AbortSignal.timeout(10000),
    });
    if (!response.ok) throw new Error("release unavailable");
    const release = await response.json();
    if (release.draft || release.prerelease || !Array.isArray(release.assets)) {
      throw new Error("release invalid");
    }
    const assetUrl = (suffix) => {
      const asset = release.assets.find((item) =>
        item.name?.startsWith("ResourceMonitor-") && item.name.endsWith(suffix) &&
        item.browser_download_url?.startsWith(`${githubUrl}/releases/download/`)
      );
      return asset?.browser_download_url;
    };
    const mac = assetUrl("-macOS-Universal.dmg");
    const windows = assetUrl("-Windows-Setup.exe");
    const zip = assetUrl("-Windows-Portable.zip");
    macDownload.href = mac || releasesUrl;
    windowsDownload.href = windows || releasesUrl;
    windowsPortableDownload.href = zip || releasesUrl;
    portable.href = zip || releasesUrl;
    const selected = platform === "mac" ? mac : platform === "windows" ? windows : null;
    primary.href = selected || releasesUrl;
    primaryText.textContent = selected ? copy.primary : "최신 릴리스 보기";
    versionLabel.textContent = release.tag_name || "최신 버전";
    const published = new Date(release.published_at);
    releaseNote.textContent = release.published_at && !Number.isNaN(published.getTime())
      ? `${new Intl.DateTimeFormat("ko-KR", { dateStyle: "long" }).format(published)} 릴리스`
      : "GitHub Releases에서 제공하는 최신 버전입니다.";
    if (!mac || !windows || !zip) {
      releaseNote.textContent += " 일부 설치 파일은 릴리스 페이지에서 확인해 주세요.";
    }
  } catch {
    useReleasePage("최신 버전을 확인하지 못했습니다. GitHub 릴리스 페이지에서 설치 파일을 확인해 주세요.");
  }
}

loadLatestRelease();
